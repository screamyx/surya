//! Locating the device's Claude Code CLI, and reading a boolean off a run
//! request's model options. Moved out of `mod.rs` under the 500-line rule
//! (issue #159); the code itself is unchanged.

use std::path::PathBuf;

use serde_json::Value;

/// Locate the device's installed Claude Code CLI: `CLAUDE_CODE_EXECUTABLE`,
/// then our own PATH, then the login-shell PATH snapshot (the user's shell
/// init shapes PATH in ways a GUI/service launch never sees - see
/// [`crate::shell_env`]), then known install locations as a last resort.
/// Resolved per call - cheap after the snapshot is cached.
pub(super) fn resolve_claude_executable() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("CLAUDE_CODE_EXECUTABLE")
        && !p.is_empty()
    {
        return Some(PathBuf::from(p));
    }
    let exe = if cfg!(windows) {
        "claude.exe"
    } else {
        "claude"
    };
    let mut candidates: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .filter(|d| !d.as_os_str().is_empty())
                .map(|d| d.join(exe))
                .collect()
        })
        .unwrap_or_default();
    if let Some(shell_path) = crate::shell_env::login_shell_path() {
        candidates.extend(
            std::env::split_paths(shell_path)
                .filter(|d| !d.as_os_str().is_empty())
                .map(|d| d.join(exe)),
        );
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        candidates.push(home.join(".claude").join("local").join("claude"));
        candidates.push(home.join(".local").join("bin").join("claude"));
    }
    candidates.push(PathBuf::from("/opt/homebrew/bin/claude"));
    candidates.push(PathBuf::from("/usr/local/bin/claude"));
    candidates.extend(
        crate::node_version_manager_bins()
            .into_iter()
            .map(|d| d.join(exe)),
    );
    candidates.into_iter().find(|p| p.exists())
}

pub(super) fn option_is_on(options: &serde_json::Map<String, Value>, key: &str) -> bool {
    match options.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s == "on" || s == "true",
        _ => false,
    }
}
