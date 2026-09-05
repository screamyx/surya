//! Where the server reads and writes. Every path is overridable by an
//! environment variable the harness sets in the generated `--mcp-config`, so
//! the binary itself hardcodes no user, host or project.

use std::path::PathBuf;

/// Resolved once at startup and threaded through the handlers.
#[derive(Debug, Clone)]
pub struct Config {
    /// The address mail is sent *from*. Empty when the host did not name us.
    pub agent_id: String,
    /// Workspace the agent is working in; the mail default channel.
    pub workspace: String,
    /// Workspace root scanned for `.surya/cards/*.json`.
    pub workspace_root: PathBuf,
    /// Append-only record of every shown card, read by the engine.
    pub card_store: PathBuf,
    /// Unix socket the daemon listens on for mail. Tried first.
    pub mail_socket: PathBuf,
    /// Append-only fallback when the socket is absent or refuses.
    pub mail_log: PathBuf,
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn env_string(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}

/// `$XDG_RUNTIME_DIR/surya`, falling back to a temp dir suffixed with our uid
/// when the runtime dir is unset (a service launch, or a non-systemd box).
///
/// The uid comes from `getuid(2)`, not from `$UID`: that variable is a shell
/// builtin and is simply absent from a process the shell did not export it
/// to, which would leave every user on the box sharing one `/tmp/surya`.
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
fn home_dir() -> PathBuf {
    match env_path("HOME") {
        Some(home) => home.join(".surya"),
        None => runtime_dir(),
    }
}

impl Config {
    pub fn from_env() -> Self {
        let workspace_root = env_path("SURYA_WORKSPACE_ROOT")
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            agent_id: env_string("SURYA_AGENT_ID"),
            workspace: env_string("SURYA_WORKSPACE"),
            workspace_root,
            card_store: env_path("SURYA_CARD_STORE")
                .unwrap_or_else(|| home_dir().join("cards.jsonl")),
            mail_socket: env_path("SURYA_MAIL_SOCKET")
                .unwrap_or_else(|| runtime_dir().join("mail.sock")),
            mail_log: env_path("SURYA_MAIL_LOG").unwrap_or_else(|| home_dir().join("mail.jsonl")),
        }
    }

    /// The folder a workspace keeps its own card catalog in (decision 14).
    pub fn cards_dir(&self) -> PathBuf {
        self.workspace_root.join(".surya").join("cards")
    }
}

/// Append one JSON line to `path`, creating parent directories as needed.
///
/// Owner-only on unix, directory and file both. These are the cards the agent
/// drew and the mail it sent: another user on the box has no business reading
/// them, and none writing them. The modes are set at creation rather than
/// chmod-ed after, so there is no window where the file is world-readable.
pub fn append_line(path: &std::path::Path, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        create_private_dir(parent)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()
}

/// Create `dir` and every parent, owner-only on unix.
fn create_private_dir(dir: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_dir_hangs_off_the_workspace_root() {
        let cfg = Config {
            agent_id: "a".into(),
            workspace: "w".into(),
            workspace_root: PathBuf::from("/repo"),
            card_store: PathBuf::from("/tmp/cards.jsonl"),
            mail_socket: PathBuf::from("/tmp/mail.sock"),
            mail_log: PathBuf::from("/tmp/mail.jsonl"),
        };
        assert_eq!(cfg.cards_dir(), PathBuf::from("/repo/.surya/cards"));
    }

    #[cfg(unix)]
    #[test]
    fn the_store_and_its_directory_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("private").join("cards.jsonl");
        append_line(&path, "{}").unwrap();
        let mode = |p: &std::path::Path| {
            std::fs::metadata(p).unwrap().permissions().mode() & 0o777
        };
        assert_eq!(mode(path.parent().unwrap()), 0o700, "the directory");
        assert_eq!(mode(&path), 0o600, "the file");
    }

    #[test]
    fn append_line_creates_the_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("out.jsonl");
        append_line(&path, "{\"a\":1}").unwrap();
        append_line(&path, "{\"a\":2}").unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(body, "{\"a\":1}\n{\"a\":2}\n");
    }
}
