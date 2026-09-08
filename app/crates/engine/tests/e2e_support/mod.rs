//! Shared poll helpers for the M2 end-to-end tests.
//!
//! `e2e.rs` is 2179 lines, far over decision 13's 500, so a helper it needs
//! lives here rather than growing it further. `mail_support` next door holds
//! its tests' poll helpers for the same reason.
#![allow(dead_code)]

use std::time::Duration;

use surya_doc::SessionCommandStatus;
use surya_engine::EngineCore;

/// Poll `predicate` until it holds, or fail on the suite's shared deadline.
pub async fn wait_for<F>(mut predicate: F, what: &str)
where
    F: FnMut() -> bool,
{
    let deadline = tokio::time::Instant::now() + surya_test_deadlines::WAIT;
    while !predicate() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for {what}"
        );
        tokio::time::sleep(Duration::from_millis(15)).await;
    }
}

/// Wait for the host to write a command's outcome row.
///
/// The drain runs a command's side effect and writes its status row AFTER the
/// effect returns (`doc_host.rs`, the `Execute` arm). So everything a test can
/// see a command do - an abort stamp, a completed assistant entry, a resolved
/// question part - is in the doc BEFORE the row that says what the command
/// resolved to. Reading the row straight after waiting on one of those effects
/// reads `Pending` whenever the runner is loaded enough to interleave them,
/// which is what failed `interrupt_stamps_streaming_entry_aborted` in CI.
///
/// This waits for ANY terminal status and never for the expected one. A
/// command that resolves the wrong way then still fails on the `assert_eq!`
/// that follows, which prints both values, instead of timing out on a wait
/// that says only which id it was waiting for.
pub async fn wait_for_outcome_row(core: &EngineCore, chat_id: &str, command_id: &str) {
    wait_for(
        || status_of(core, chat_id, command_id).is_some_and(|s| s != SessionCommandStatus::Pending),
        &format!("the outcome row for {command_id}"),
    )
    .await;
}

fn status_of(core: &EngineCore, chat_id: &str, command_id: &str) -> Option<SessionCommandStatus> {
    core.doc_host
        .open(chat_id)
        .expect("open chat")
        .doc()
        .read_commands()
        .expect("read commands")
        .into_iter()
        .find(|c| c.id == command_id)
        .map(|c| c.status)
}
