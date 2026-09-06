//! A session-status row must not outlive its chat.
//!
//! The `deleteSpace` cascade tombstones the chat row and the session row in
//! one commit, then interrupts the run it was hosting. That interrupt settles
//! into a status transition, which used to write the session row straight
//! back - a row for a chat nobody has, that nothing afterwards clears.
//!
//! Driven over the real RPC surface, the same `Mutate` the app sends.

use std::sync::Arc;
use std::time::Duration;

use surya_engine::{EngineCore, HarnessRegistry};
use surya_harness::Harness;
use surya_harness::mock::MockHarness;
use surya_proto::{AgentEvent, DoneStatus, HarnessId, RunRequest, SandboxLevel, Session,
                  SessionStatus};
use surya_rpc::methods;

const CWD: &str = "/repos/project-jag";

fn assemble(dir: &std::path::Path, script: Vec<AgentEvent>) -> EngineCore {
    let registry = HarnessRegistry::new();
    registry.register(Arc::new(MockHarness { script }) as Arc<dyn Harness>);
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

fn started() -> AgentEvent {
    AgentEvent::SessionStarted {
        harness: HarnessId::Mock,
        model: "mock-1".into(),
        tools: vec![],
        cwd: CWD.into(),
        session_id: "hs-1".into(),
        assistant_message_id: "a-1".into(),
    }
}

fn asks_permission(request_id: &str, command: &str) -> AgentEvent {
    AgentEvent::PermissionRequested {
        request_id: request_id.into(),
        tool_name: "Bash".into(),
        command: command.into(),
        input: None,
    }
}

fn done(status: DoneStatus) -> AgentEvent {
    AgentEvent::Done {
        status,
        result: None,
        error: None,
        session_id: None,
    }
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

/// The reported shape: a project removed while one of its chats is mid-run,
/// then the late status transition that settles afterwards.
///
/// The late write is driven explicitly rather than timed. Racing the
/// interrupt is what made this leak hard to see in the first place: the same
/// setup leaves a row on one run and not the next, so a test that waits for
/// the race to land is a test that passes for the wrong reason. The cascade
/// here is real, the chat is really gone, and the write under test is the
/// one the settling run makes.
#[tokio::test(flavor = "multi_thread")]
async fn a_status_after_the_cascade_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(
        dir.path(),
        vec![
            started(),
            asks_permission("perm-1", "php artisan migrate"),
            done(DoneStatus::Completed),
        ],
    );
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
    wait_for(
        || {
            core.workspace
                .read_sessions()
                .unwrap_or_default()
                .iter()
                .any(|s| s.chat_id == "chat-1")
        },
        "the run to write a session row",
    )
    .await;

    client
        .call(
            methods::MUTATE,
            serde_json::json!({ "op": "deleteSpace", "spaceId": "space-1" }),
        )
        .await
        .expect("delete space");
    wait_for(
        || core.workspace.chat("chat-1").ok().flatten().is_none(),
        "the chat row to go",
    )
    .await;

    // What the settling run does on its way out.
    core.workspace.record_session(&Session {
        chat_id: "chat-1".into(),
        device_id: core.device_id.clone(),
        status: SessionStatus::Idle,
        started_at: None,
        updated_at: chrono::Utc::now(),
    });

    let rows = core.workspace.read_sessions().unwrap_or_default();
    assert!(
        !rows.iter().any(|s| s.chat_id == "chat-1"),
        "a session row outlived its chat: {rows:?}"
    );
}

/// The guard is on the chat existing, not on the cascade, so a status for a
/// chat that never had a row is refused too.
#[tokio::test(flavor = "multi_thread")]
async fn a_status_for_an_unknown_chat_writes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(dir.path(), vec![started(), done(DoneStatus::Completed)]);
    core.workspace.record_session(&Session {
        chat_id: "never-existed".into(),
        device_id: core.device_id.clone(),
        status: SessionStatus::Idle,
        started_at: None,
        updated_at: chrono::Utc::now(),
    });
    assert!(
        core.workspace.read_sessions().unwrap_or_default().is_empty(),
        "no chat, no session row"
    );
}

/// The guard must not cost a live run its OPENING status.
///
/// This is the one thing the change could plausibly have broken, so it
/// asserts the Working mirror specifically, not "a row shows up eventually".
/// An earlier version asserted the latter and passed for the wrong reason:
/// the Working write was refused, and the row only appeared later at
/// Done -> Idle, after `note_message` had claimed the chat. The dispatch path
/// now claims before its first `set_status`, and that is what this pins.
#[tokio::test(flavor = "multi_thread")]
async fn a_run_with_no_createchat_records_its_opening_working() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Parks on the gate, so the run sits in its opening state and the only
    // status that can have been written is the Working one.
    let core = assemble(
        dir.path(),
        vec![
            started(),
            asks_permission("perm-1", "php artisan migrate"),
            done(DoneStatus::Completed),
        ],
    );
    // No createChat: the dispatch path's own claim is the only thing that
    // gives this chat a row.
    core.sessions
        .dispatch("chat-claimed", HarnessId::Mock, run_request("hello"), None)
        .await
        .expect("dispatch");

    // Asserted at once, not waited for. `dispatch` claims and mirrors
    // Working synchronously, so the row is there the moment it returns - and
    // waiting instead would hide the bug: with the claim removed this still
    // goes green after about 15 s, because `touch_session`'s 10 s throttle
    // re-writes the entry once something else has claimed the chat. A
    // deadline would have made the test pass for the wrong reason twice over.
    let rows = core.workspace.read_sessions().unwrap_or_default();
    assert!(
        rows.iter()
            .any(|s| s.chat_id == "chat-claimed" && s.status == SessionStatus::Working),
        "the opening Working mirror never reached the doc: {rows:?}"
    );
}
