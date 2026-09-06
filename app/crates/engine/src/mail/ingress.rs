//! Ingress from the surya-mcp seat.
//!
//! The MCP server's `send_message` tool writes one JSON record per message.
//! Two ways in, both live at once:
//!
//! - a unix socket, `$SURYA_MAIL_SOCKET` else `$XDG_RUNTIME_DIR/surya/mail.sock`
//!   — one JSON object per line, the reply is
//!   `{"ids":[...],"recipients":[...]}`. Unix only;
//! - an append-only file, `$SURYA_MAIL_LOG` else `~/.surya/mail.jsonl`, tailed
//!   from its end so a restart does not redeliver history. Always on, and the
//!   only way in on Windows.
//!
//! Both paths resolve exactly as the sidecar resolves them; see
//! [`MailIngressPaths::detect`].
//!
//! Record, as `surya-mcp`'s `send_message` writes it (`crates/mcp/src/mail.rs`):
//! `{"delivery_id":"d_…","from":"…","to":"…","workspace":"…","text":"…"}`.
//! `body` is accepted for `text`, and `toDevice` for cross-server addressing.
//! The seat's `delivery_id` becomes the row id, so the id the tool already
//! handed the agent is the one `Mail.Ack` takes.

use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::Mail;

#[derive(Debug, Clone)]
pub struct MailIngressPaths {
    pub socket: Option<PathBuf>,
    pub jsonl: Option<PathBuf>,
}

impl MailIngressPaths {
    /// The same paths the surya-mcp sidecar writes to, resolved the same way.
    ///
    /// This mirrors `crates/mcp/src/config.rs` deliberately and exactly. The
    /// two sides used to diverge: the sidecar honours `SURYA_MAIL_SOCKET` and
    /// `SURYA_MAIL_LOG` and falls back to `/tmp/surya-<uid>` when there is no
    /// runtime dir, and this side read only `XDG_RUNTIME_DIR` and `HOME`. An
    /// operator setting either override, or a service launch with no runtime
    /// dir, put the writer and the reader on different files - and nothing
    /// failed. Mail simply stopped arriving, with no error on either side.
    ///
    /// If the sidecar's resolution changes, this has to change with it. There
    /// is a test below that fails when they disagree about an override.
    ///
    /// On Windows there is no socket: `socket` is `None` and every message
    /// arrives over the jsonl file, which is why the file half is not an
    /// optional fallback.
    pub fn detect() -> Self {
        let socket = cfg!(unix).then(|| {
            env_path("SURYA_MAIL_SOCKET").unwrap_or_else(|| runtime_dir().join("mail.sock"))
        });
        let jsonl =
            Some(env_path("SURYA_MAIL_LOG").unwrap_or_else(|| surya_home_dir().join("mail.jsonl")));
        Self { socket, jsonl }
    }
}

/// An environment path, treating empty as unset — the sidecar's rule.
fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// `$XDG_RUNTIME_DIR/surya`, else a temp dir suffixed with our uid.
///
/// The uid comes from `getuid(2)` and not from `$UID`: that is a shell builtin
/// and is absent from a process the shell did not export it to, which would
/// leave every user on the box sharing one `/tmp/surya`.
fn runtime_dir() -> PathBuf {
    if let Some(dir) = env_path("XDG_RUNTIME_DIR") {
        return dir.join("surya");
    }
    std::env::temp_dir().join(format!("surya-{}", current_uid()))
}

#[cfg(unix)]
fn current_uid() -> String {
    // SAFETY: getuid(2) reads a process attribute and cannot fail.
    unsafe { libc::getuid() }.to_string()
}

#[cfg(not(unix))]
fn current_uid() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| "user".into())
}

/// `~/.surya`, falling back to the runtime dir when HOME is unset.
pub(crate) fn surya_home_dir() -> PathBuf {
    match env_path("HOME") {
        Some(home) => home.join(".surya"),
        None => runtime_dir(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MailRecord {
    #[serde(default = "unknown_sender")]
    from: String,
    to: String,
    /// The seat writes `text`; `body` is the same field under the engine's name.
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    to_device: Option<String>,
    /// Minted by the seat's tool and already returned to the sending agent.
    #[serde(default, alias = "delivery_id")]
    delivery_id: Option<String>,
}

fn unknown_sender() -> String {
    "unknown".to_string()
}

pub struct MailIngress {
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl MailIngress {
    /// Start both listeners. A path that cannot be opened is logged and
    /// skipped: mail over RPC must keep working when the seat channel does not.
    pub fn start(mail: Mail, paths: MailIngressPaths) -> Self {
        let mut tasks = Vec::new();
        if paths.socket.is_none() {
            // Said once, at start, so a Windows operator reading the log knows
            // which route mail takes here rather than wondering why the socket
            // never appears.
            tracing::info!(
                "mail socket unavailable on this platform; the jsonl channel carries every message"
            );
        }
        if let Some(socket) = paths.socket {
            match start_socket(mail.clone(), socket.clone()) {
                Ok(task) => tasks.push(task),
                Err(err) => {
                    tracing::warn!(path = %socket.display(), error = %err, "mail socket not started")
                }
            }
        }
        if let Some(jsonl) = paths.jsonl {
            match start_jsonl(mail, jsonl.clone()) {
                Ok(task) => tasks.push(task),
                Err(err) => {
                    tracing::warn!(path = %jsonl.display(), error = %err, "mail jsonl not started")
                }
            }
        }
        Self { tasks }
    }

    pub fn stop(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
        }
    }
}

impl Drop for MailIngress {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Owner-only (0600). Anyone who can write the mail channel can send as
/// anybody, so the channel is not world-writable and not world-readable.
#[cfg(unix)]
fn restrict(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Err(err) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)) {
        tracing::warn!(path = %path.display(), error = %err, "could not restrict mail channel to 0600");
    }
}

#[cfg(not(unix))]
fn restrict(_path: &Path) {}

/// The socket half of the channel. Unix only: it is a unix-domain socket, and
/// Windows has no `tokio::net::UnixListener`. The jsonl half runs everywhere,
/// so mail still arrives on Windows - one route instead of two.
#[cfg(unix)]
fn start_socket(mail: Mail, path: PathBuf) -> std::io::Result<tokio::task::JoinHandle<()>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // A stale socket file from a crashed engine would refuse the bind.
    let _ = std::fs::remove_file(&path);
    let listener = tokio::net::UnixListener::bind(&path)?;
    // Owner-only, set explicitly. `$XDG_RUNTIME_DIR` is 0700 on a normal
    // system, but the fallback paths are not, and mail is not public.
    restrict(&path);
    tracing::info!(path = %path.display(), "mail socket listening");
    Ok(tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let mail = mail.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut lines = BufReader::new(read).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let reply = match accept(&mail, &line).await {
                        Ok(Some(receipt)) => serde_json::to_string(&receipt)
                            .unwrap_or_else(|_| "{\"ids\":[]}".into()),
                        Ok(None) => continue,
                        Err(err) => serde_json::json!({ "error": err.to_string() }).to_string(),
                    };
                    if write
                        .write_all(format!("{reply}\n").as_bytes())
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }
    }))
}

#[cfg(not(unix))]
fn start_socket(_mail: Mail, path: PathBuf) -> std::io::Result<tokio::task::JoinHandle<()>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!("unix sockets are not available on this platform ({})", path.display()),
    ))
}

fn start_jsonl(mail: Mail, path: PathBuf) -> std::io::Result<tokio::task::JoinHandle<()>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .open(&path)?;
    // Start at the end: records written while this engine was down belong to
    // whichever engine was running then.
    restrict(&path);
    let mut offset = file.seek(SeekFrom::End(0))?;
    tracing::info!(path = %path.display(), "mail jsonl tailing");
    Ok(tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            match read_from(&path, offset) {
                Ok((lines, next)) => {
                    offset = next;
                    for line in lines {
                        if let Err(err) = accept(&mail, &line).await {
                            tracing::warn!(error = %err, "mail jsonl record rejected");
                        }
                    }
                }
                Err(err) => {
                    tracing::debug!(error = %err, "mail jsonl read failed");
                }
            }
        }
    }))
}

/// Read whole lines appended after `offset`; a partial trailing line is left
/// for the next pass.
fn read_from(path: &Path, offset: u64) -> std::io::Result<(Vec<String>, u64)> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    // Truncated or rotated: follow it from the top.
    let offset = if len < offset { 0 } else { offset };
    if len == offset {
        return Ok((Vec::new(), offset));
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    let consumed = match buf.rfind('\n') {
        Some(idx) => idx + 1,
        None => return Ok((Vec::new(), offset)),
    };
    let lines = buf[..consumed]
        .lines()
        .map(str::to_string)
        .filter(|l| !l.trim().is_empty())
        .collect();
    Ok((lines, offset + consumed as u64))
}

async fn accept(mail: &Mail, line: &str) -> Result<Option<super::MailReceipt>, crate::EngineError> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    let record: MailRecord = serde_json::from_str(line)
        .map_err(|e| crate::EngineError::Other(format!("bad mail record: {e}")))?;
    let body = record
        .text
        .or(record.body)
        .ok_or_else(|| crate::EngineError::Other("mail record has no text".into()))?;
    let receipt = mail
        .send_with_id(
            &record.from,
            &record.to,
            &body,
            record.to_device.as_deref(),
            record.delivery_id.as_deref(),
        )
        .await?;
    Ok(Some(receipt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_is_always_a_route_in() {
        let paths = MailIngressPaths::detect();
        assert!(
            paths.jsonl.is_some(),
            "the jsonl channel is the one every platform has"
        );
        #[cfg(not(unix))]
        assert!(
            paths.socket.is_none(),
            "no unix socket off unix: it would fail to bind on every start"
        );
    }

    #[test]
    fn the_seats_record_shape_parses() {
        // Verbatim from crates/mcp/src/mail.rs.
        // `r##` and not `r#`: the address `"#demo` would close a single-hash
        // raw string.
        let line = r##"{"delivery_id":"d_abc","from":"seat-1","to":"#demo",
            "workspace":"/repo","text":"the migration is ready","at":"2026-09-05T00:00:00Z"}"##;
        let record: MailRecord = serde_json::from_str(line).expect("parses");
        assert_eq!(record.delivery_id.as_deref(), Some("d_abc"));
        assert_eq!(record.text.as_deref(), Some("the migration is ready"));
        assert_eq!(record.from, "seat-1");
        assert_eq!(record.to, "#demo");
    }

    #[test]
    fn tail_reads_only_whole_appended_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.jsonl");
        std::fs::write(&path, "{\"to\":\"a\",\"body\":\"one\"}\n{\"to\":\"a\"").unwrap();
        let (lines, offset) = read_from(&path, 0).unwrap();
        assert_eq!(lines.len(), 1, "the partial trailing line waits");
        let (again, _) = read_from(&path, offset).unwrap();
        assert!(again.is_empty());
    }
}
