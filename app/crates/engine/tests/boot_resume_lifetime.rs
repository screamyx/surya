//! The boot auto-resume must not revive a chat that is gone.
//!
//! `recover_stale` runs once, from `EngineCore::assemble` (`lib.rs:285`), so
//! this is a restart and not a race: remove a project, then start the engine
//! again. The journal outlives the chat - the cascade tombstones the row and
//! purges the transcript, and nothing clears the journal - so the resume's
//! last-resort fallback would build a run from the journal's own cwd, which
//! is the removed project's path. Dispatching that claims a chat row and, at
//! that cwd, auto-creates a space: the project comes back.
//!
//! The purge normally stops it earlier, by leaving no transcript to find a
//! prompt in. This is the crash that lands between the cascade's commit and
//! its purge, pinned directly rather than by timing.

use std::sync::Arc;
use std::time::Duration;

use surya_engine::{EngineCore, HarnessRegistry};
use surya_harness::Harness;
use surya_harness::mock::MockHarness;
use surya_doc::MessageStatus;
use surya_proto::{AgentEvent, DoneStatus, HarnessId, RunRequest, SandboxLevel};
use surya_rpc::methods;

const CWD: &str = "/repos/project-jag";

fn assemble(dir: &std::path::Path) -> EngineCore {
    let registry = HarnessRegistry::new();
    registry.register(Arc::new(MockHarness {
        script: vec![
            AgentEvent::SessionStarted {
                harness: HarnessId::Mock,
                model: "mock-1".into(),
                tools: vec![],
                cwd: CWD.into(),
                session_id: "hs-1".into(),
                assistant_message_id: "a-1".into(),
            },
            // Parks on the gate: the run never finishes, so its journal
            // entry is still open when the process goes away.
            AgentEvent::PermissionRequested {
                request_id: "perm-1".into(),
                tool_name: "Bash".into(),
                command: "php artisan migrate".into(),
                input: None,
            },
            AgentEvent::Done {
                status: DoneStatus::Completed,
                result: None,
                error: None,
                session_id: None,
            },
        ],
    }) as Arc<dyn Harness>);
    EngineCore::assemble(dir, Arc::new(registry), HarnessId::Mock, None)
        .expect("engine core assembles")
}

fn run_request(prompt: &str) -> RunRequest {
    RunRequest {
        prompt: prompt.into(),
        harness: None,
        model: None,
        reasoning: None,
        model_options: Default::default(),
        cwd: CWD.into(),
        sandbox: SandboxLevel::WorkspaceWrite,
        auto_approve: false,
        attachments: Vec::new(),
        worktree: None,
        surya: None,
        resume: None,
    }
}

/// A real restart, because that is the only place `recover_stale` runs.
///
/// The cascade is driven through `WorkspaceHost::delete_space` directly (it
/// commits `RegistryDoc::delete_space`), NOT through the `deleteSpace`
/// mutate: that path also purges the transcript and drops the run
/// configuration, and those are what make the ordinary cascade safe.
/// Committing the row deletions while the transcript and the journal stay is
/// precisely the state a crash between the commit and the purge leaves on
/// disk.
#[tokio::test(flavor = "multi_thread")]
async fn a_boot_resume_refuses_a_chat_with_no_row() {
    let dir = tempfile::tempdir().expect("tempdir");
    {
        let core = assemble(dir.path());
        let client = surya_rpc::memory_client(core.rpc_service());
        client
            .call(
                methods::MUTATE,
                serde_json::json!({
                    "op": "createSpace", "spaceId": "space-1",
                    "deviceId": core.device_id, "path": CWD
                }),
            )
            .await
            .expect("create space");
        client
            .call(
                methods::MUTATE,
                serde_json::json!({ "op": "createChat", "chatId": "chat-1", "spaceId": "space-1" }),
            )
            .await
            .expect("create chat");
        core.sessions
            .dispatch("chat-1", HarnessId::Mock, run_request("migrate"), None)
            .await
            .expect("dispatch");
        tokio::time::sleep(Duration::from_millis(400)).await;

        // The cascade's COMMIT, and nothing after it:
        // `RegistryDoc::delete_space` tombstones the space row, the chat rows
        // and the session rows in one go, while `purge_chat` and `drop_chat`
        // live in the teardown task the crash is standing in for.
        core.workspace.delete_space("space-1").expect("delete space");
        // Both the space row and the chat tombstone have to reach disk, or
        // the restart reads a doc where none of this happened and the test
        // proves nothing. The registry persist is debounced.
        tokio::time::sleep(Duration::from_secs(4)).await;
        assert!(
            !core.workspace.chat_exists("chat-1"),
            "the chat row should be gone"
        );
    }
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Boot again. `recover_stale` runs inside `assemble`.
    let core = assemble(dir.path());

    // Wait on a positive signal from THIS boot, not on a fixed span: the same
    // `recover_stale` pass that would have resumed also stamps the crashed
    // streaming entry aborted, so once that stamp is visible the pass has run
    // and the assertions below are reading a settled state. A sleep here
    // would pass by simply not waiting long enough.
    wait_for(
        || {
            core.doc_host
                .open("chat-1")
                .ok()
                .and_then(|h| h.doc().read_entries().ok())
                .is_some_and(|entries| {
                    entries
                        .iter()
                        .any(|e| e.status == Some(MessageStatus::Aborted))
                })
        },
        "recover_stale to stamp the crashed entry",
    )
    .await;

    // The stamp says the pass RAN; it does not say the resume it spawns has
    // finished, and the resurrection lands in that task. So hold a quiet
    // window after it and fail the moment anything appears. This one is a
    // fixed span on purpose - it asserts that nothing arrives, which is the
    // one shape `surya_test_deadlines::WAIT` must not be used for.
    //
    // Sized against the unguarded run: the chat comes back well inside it.
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let chats = core.workspace.read_chats().unwrap_or_default();
        assert!(
            chats.is_empty(),
            "the boot resume brought a chat back: {chats:?}"
        );
        let spaces = core.workspace.read_spaces().unwrap_or_default();
        assert!(
            spaces.is_empty(),
            "the boot resume minted a project back: {spaces:?}"
        );
    }
}

/// The other side of the same coin, and the reason the guard reads the
/// tombstone rather than the row's absence.
///
/// A chat that never got a row at all must still resume: the claim on the
/// first command is what creates the row, and the journal fallback exists
/// precisely because "a crash can predate the debounced workspace-row write".
/// `restart_resume::fresh_crash_auto_resumes_and_notes_the_interruption`
/// covers the full revival; this pins the distinction the guard turns on.
#[tokio::test(flavor = "multi_thread")]
async fn a_chat_that_never_had_a_row_is_not_a_tombstone() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(dir.path());

    assert!(
        !core.workspace.chat_exists("never-written"),
        "no row, as the crash-predates-the-write case has"
    );
    assert!(
        !core.workspace.chat_tombstoned("never-written"),
        "and it must not read as deleted, or the resume refuses the case the \
         journal fallback was added for"
    );

    // A deleted chat is the opposite answer to the same question.
    core.workspace
        .create_chat("chat-1", None, Some(&core.device_id), None, Some(CWD.into()))
        .expect("create chat");
    core.workspace.delete_chat("chat-1").expect("delete");
    assert!(!core.workspace.chat_exists("chat-1"), "row gone");
    assert!(
        core.workspace.chat_tombstoned("chat-1"),
        "a deleted chat must read as a tombstone"
    );
}

async fn wait_for<F: FnMut() -> bool>(mut predicate: F, what: &str) {
    let deadline = tokio::time::Instant::now() + surya_test_deadlines::WAIT;
    while !predicate() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for {what}"
        );
        tokio::time::sleep(Duration::from_millis(15)).await;
    }
}
