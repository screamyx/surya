//! What surya hands a real agent when it starts one (decision 5).
//!
//! The harness only wires the `surya-mcp` sidecar, the prompt append, the
//! cards skill and the SendMessage denial when `RunRequest.surya` is `Some`
//! (`harness/src/claude/mod.rs`). Until this module existed nothing in a
//! running engine ever set it: every `Some(SuryaOptions)` in the tree was a
//! test, an example or a `#[cfg(test)]` block, so a real Claude Code session
//! had no `show_card` tool, no card prompt, no cards skill, and the CLI's own
//! `SendMessage` was never denied - decision 19's single mail channel was not
//! enforced on any real run.
//!
//! The engine fills this in, not the app: these are all host paths, and the
//! app may be on another machine entirely.

use std::path::PathBuf;

use surya_proto::SuryaOptions;

use crate::mail::MailIngressPaths;

/// Where a run's shown cards are recorded for the app to read back.
///
/// One file per chat under `~/.surya`, the same root the mail ingress
/// resolves - NOT the headless engine's data dir, which is `SURYA_DATA_DIR`
/// or `~/.local/share/surya-engine`. There was no prior convention to follow
/// - the app reads whatever path the options carry - so this is a choice,
/// recorded in `docs/decisions.md` under decision 14.
fn card_store(chat_id: &str) -> PathBuf {
    crate::mail::surya_home_dir()
        .join("cards")
        .join(format!("{}.jsonl", safe_name(chat_id)))
}

/// A chat id is opaque and may hold a slash. One file, not a surprise
/// directory - the sidecar's `run_dir` takes the same precaution.
fn safe_name(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

/// The options for one real user run.
///
/// `cwd` doubles as the workspace: it is what the harness already passes as
/// `SURYA_WORKSPACE_ROOT`, and it is where the sidecar looks for a workspace
/// catalog (`<cwd>/.surya/cards`), so the mail channel and the catalog agree
/// on what "this workspace" means.
///
/// `mcp_binary` stays `None` unless overridden: the harness already looks
/// beside the running executable and then on PATH, which is where the
/// installer puts it.
pub(crate) fn options_for(chat_id: &str, cwd: &str) -> SuryaOptions {
    SuryaOptions {
        // The mail system addresses a top-level agent by its chat id
        // (`mail/delivery.rs` settles agents straight off `Session::chat_id`,
        // and `child_agent_id` hangs subagents off the same string).
        agent_id: chat_id.to_string(),
        workspace: cwd.to_string(),
        mcp_binary: std::env::var("SURYA_MCP_BINARY")
            .ok()
            .filter(|v| !v.is_empty()),
        card_store: Some(card_store(chat_id).to_string_lossy().into_owned()),
        // Windows has no socket; mail arrives over the jsonl file there, and
        // the sidecar falls back to it on its own.
        mail_socket: MailIngressPaths::detect()
            .socket
            .map(|p| p.to_string_lossy().into_owned()),
        // Basic catalog unless the workspace ships its own. The sidecar
        // resolves `<cwd>/.surya/cards` itself, so naming a catalog here is
        // only for an explicit override.
        catalog_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_run_is_addressed_by_its_chat_id() {
        let options = options_for("chat-7", "/repo");
        assert_eq!(options.agent_id, "chat-7");
        assert_eq!(options.workspace, "/repo");
    }

    #[test]
    fn the_card_store_is_one_file_per_chat_even_with_a_slash_in_the_id() {
        let store = card_store("space/chat-7");
        assert_eq!(
            store.file_name().and_then(|n| n.to_str()),
            Some("space-chat-7.jsonl"),
            "a slash must not turn the store into a directory"
        );
        assert_eq!(store.parent().and_then(|p| p.file_name()), Some("cards".as_ref()));
    }

    #[test]
    fn the_binary_is_left_to_the_harness_unless_overridden() {
        assert_eq!(options_for("c", "/repo").mcp_binary, None);
    }
}
