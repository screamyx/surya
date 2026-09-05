//! Ingress from the surya-mcp seat.
//!
//! The MCP server's `send_message` tool writes one JSON record per message.
//! Two ways in, both live at once:
//!
//! - a unix socket, default `$XDG_RUNTIME_DIR/surya/mail.sock` — one JSON
//!   object per line, the reply is `{"ids":[...],"recipients":[...]}`;
//! - an append-only file, default `~/.surya/mail.jsonl` — the fallback when
//!   there is no runtime dir, tailed from its end so a restart does not
//!   redeliver history.
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
    /// The defaults from the brief: runtime socket first, home file always.
    pub fn detect() -> Self {
        let socket = std::env::var_os("XDG_RUNTIME_DIR")
            .map(|dir| PathBuf::from(dir).join("surya").join("mail.sock"));
        let jsonl = Some(crate::repos::home_dir().join(".surya").join("mail.jsonl"));
        Self { socket, jsonl }
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

fn start_socket(mail: Mail, path: PathBuf) -> std::io::Result<tokio::task::JoinHandle<()>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // A stale socket file from a crashed engine would refuse the bind.
    let _ = std::fs::remove_file(&path);
    let listener = tokio::net::UnixListener::bind(&path)?;
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
            // The channel is a file and a socket: anyone who can write to
            // either can claim any sender, so the envelope marks it.
            false,
        )
        .await?;
    Ok(Some(receipt))
}

#[cfg(test)]
mod tests {
    use super::*;

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
