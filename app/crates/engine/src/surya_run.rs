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

use std::path::{Path, PathBuf};

use surya_proto::SuryaOptions;

use crate::mail::MailIngressPaths;

/// Where a run's shown cards are recorded for the app to read back.
///
/// One file per chat under the engine's data directory, the same root the
/// mail channel resolves: `SURYA_DATA_DIR`, else `~/.surya`. It used to be
/// `~/.surya` whatever the data directory said, so two engines under one OS
/// user appended to one file per shared chat id and each showed the other's
/// cards (surya#226). Decision 14 records the move, and that files written
/// at the old path are left there and not read.
fn card_store(chat_id: &str) -> PathBuf {
    card_store_under(&crate::mail::data_dir(), chat_id)
}

/// [`card_store`] with the data directory named rather than resolved.
///
/// Split out so the layout and the move are testable without setting
/// `SURYA_DATA_DIR`. libtest runs a binary's tests as threads in one process,
/// so a test that writes the environment is read by its siblings. The one
/// place in this crate that does it, `tests/mail_ingress_paths.rs`, is alone
/// in its binary for that reason.
fn card_store_under(data_dir: &Path, chat_id: &str) -> PathBuf {
    data_dir
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
        // Named, not left to the sidecar's own default. Both sides derive it
        // from SURYA_DATA_DIR now, so they agree either way, but only if the
        // sidecar inherited that variable - and the mcp-config env block is
        // the one place that does not depend on inheritance.
        mail_log: MailIngressPaths::detect()
            .jsonl
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
        let store = card_store_under(Path::new("/data"), "space/chat-7");
        assert_eq!(
            store,
            Path::new("/data/cards/space-chat-7.jsonl"),
            "a slash must not turn the store into a directory"
        );
    }

    /// surya#226. Two engines under one OS user each get their own data
    /// directory; before this they appended to one file per shared chat id
    /// and each showed the other's cards.
    #[test]
    fn two_data_directories_are_two_card_stores() {
        let one = card_store_under(Path::new("/data/one"), "chat-7");
        let two = card_store_under(Path::new("/data/two"), "chat-7");
        assert_ne!(
            one, two,
            "the same chat id under two data directories must not share a file"
        );
        assert_eq!(one, Path::new("/data/one/cards/chat-7.jsonl"));
        assert_eq!(two, Path::new("/data/two/cards/chat-7.jsonl"));
    }

    /// The wiring, not the layout: the resolved store hangs off the same root
    /// the mail channel resolves, so `SURYA_DATA_DIR` moves both together.
    /// Reads the environment, never writes it, so it cannot race a sibling.
    #[test]
    fn the_resolved_card_store_hangs_off_the_engine_data_dir() {
        assert_eq!(
            card_store("chat-7"),
            crate::mail::data_dir().join("cards").join("chat-7.jsonl")
        );
    }

    #[test]
    fn the_binary_is_left_to_the_harness_unless_overridden() {
        assert_eq!(options_for("c", "/repo").mcp_binary, None);
    }
}
