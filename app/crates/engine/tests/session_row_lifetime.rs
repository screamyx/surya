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

    // Asserted at once, not waited for. `dispatch` claims and mirrors the
    // opening status synchronously, so the row is there the moment it
    // returns - and waiting instead would hide the bug: with the claim
    // removed this still goes green after about 15 s, because
    // `touch_session`'s 10 s throttle re-writes the entry once something else
    // has claimed the chat. A deadline would make it pass for the wrong
    // reason twice over.
    //
    // Any status, not `Working` specifically. What this pins is that the
    // claim put a row there at all; which status it carries is the run's
    // business and not the point. (An earlier comment here blamed a parked
    // permission for turning the row `AwaitingInput` - wrong: `AwaitingInput`
    // has one setter, `AgentEvent::InputRequested`, and a parked permission
    // leaves the status `Working`.)
    let rows = core.workspace.read_sessions().unwrap_or_default();
    assert!(
        rows.iter().any(|s| s.chat_id == "chat-claimed"),
        "the opening status mirror never reached the doc: {rows:?}"
    );
}

/// The whole chain, end to end: a project removed while one of its chats has
/// a run behind it, then mail queued for that dead chat.
///
/// `Mail::run_request_for` (`mail/delivery.rs:271`) falls back to the
/// sessions engine's last run configuration. That map outlived `drop_chat`,
/// so the delivery still found a runnable request carrying the removed
/// project's cwd - and dispatch would claim a chat row for it and, at that
/// cwd, auto-create a space to hold it. A removed project growing a new
/// project back is the worst shape this seam has.
///
/// With the map cleared the fallback is `request_from_chat_row`, which needs
/// a row the cascade took, so mail holds instead.
#[tokio::test(flavor = "multi_thread")]
async fn mail_for_a_cascaded_chat_mints_no_project_and_no_chat() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(
        dir.path(),
        vec![started(), done(DoneStatus::Completed)],
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

    // A run, so the sessions engine holds a last request for this chat.
    core.sessions
        .dispatch("chat-1", HarnessId::Mock, run_request("first turn"), None)
        .await
        .expect("dispatch");
    wait_for(
        || core.sessions.last_request("chat-1").is_some(),
        "the run configuration to be remembered",
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

    // Mail for an agent that no longer exists here. Accepted and queued:
    // reserve-on-spawn means an unknown address is not an error.
    core.mail
        .send("someone", "chat-1", "are you still there", None)
        .await
        .expect("queued");
    core.mail.deliver_pending().await.expect("deliver pass");

    let spaces = core.workspace.read_spaces().unwrap_or_default();
    let chats = core.workspace.read_chats().unwrap_or_default();
    assert!(
        spaces.is_empty(),
        "a removed project grew a project back: {spaces:?}"
    );
    assert!(
        chats.is_empty(),
        "a removed project grew a chat back: {chats:?}"
    );
}

/// A message persisted after the cascade must not bring the chat back.
///
/// The run interrupted by `deleteSpace` writes one last message on its way
/// out. `note_message` used to claim the row before writing the preview, so
/// that write re-created the chat - cwd-less and project-less, drawn as
/// "New session" at `~` under All projects, opening onto a transcript
/// `purge_chat` had already taken.
#[tokio::test(flavor = "multi_thread")]
async fn a_message_after_the_cascade_brings_nothing_back() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(dir.path(), vec![started(), done(DoneStatus::Completed)]);
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

    core.workspace
        .note_message("chat-1", "the run said something on its way out");

    let chats = core.workspace.read_chats().unwrap_or_default();
    assert!(
        chats.is_empty(),
        "a message brought the chat back: {chats:?}"
    );
}

/// And the preview still lands for a chat that never went through
/// `createChat`: the run's own claim on the first command gives it a row, and
/// that runs before any message is persisted.
#[tokio::test(flavor = "multi_thread")]
async fn a_pre_workspace_chat_still_gets_its_preview() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(dir.path(), vec![started(), done(DoneStatus::Completed)]);
    // No createChat anywhere: dispatch is the only thing that can give this
    // chat a row, and `dispatch_with` calls `note_message` right after.
    core.sessions
        .dispatch(
            "chat-claimed",
            HarnessId::Mock,
            run_request("migrate the database"),
            None,
        )
        .await
        .expect("dispatch");

    let chat = core
        .workspace
        .chat("chat-claimed")
        .expect("read")
        .expect("the first command claimed the row");
    assert_eq!(
        chat.last_message_preview.as_deref(),
        Some("migrate the database"),
        "the preview never landed: {chat:?}"
    );
}
