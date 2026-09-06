//! Yolo mode: the owner's "on new and running sessions".
//!
//! Driven through the mock harness and the real engine assembly. The mock
//! asks the gate for every scripted `PermissionRequested` regardless of the
//! run's `auto_approve` flag (surya-harness mock.rs), which is exactly what
//! makes it able to prove the ENGINE side: whether a request parks in the
//! needs-you inbox or is answered without waking anyone.

use std::sync::Arc;
use std::time::Duration;

use surya_engine::{EngineCore, HarnessRegistry};
use surya_harness::Harness;
use surya_harness::mock::MockHarness;
use surya_proto::{
    AgentEvent, AgentState, ChatConfig, DoneStatus, HarnessId, NeedsYouKind, RunRequest,
    SandboxLevel,
};

const CWD: &str = "/repos/project-jag";
const SPACE: &str = "space-yolo";

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
        // What the composer sends with yolo off. Every test that expects the
        // flag to arrive at the harness expects the ENGINE to have put it
        // there, not the caller.
        auto_approve: false,
        attachments: Vec::new(),
        worktree: None,
        surya: None,
        resume: None,
    }
}

fn chat_config(auto_approve: bool) -> ChatConfig {
    ChatConfig {
        harness: HarnessId::Mock,
        model: None,
        reasoning: None,
        model_options: Default::default(),
        sandbox: SandboxLevel::WorkspaceWrite,
        auto_approve,
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

fn done() -> AgentEvent {
    AgentEvent::Done {
        status: DoneStatus::Completed,
        result: None,
        error: None,
        session_id: Some("hs-1".into()),
    }
}

fn asks_permission(request_id: &str, command: &str) -> AgentEvent {
    AgentEvent::PermissionRequested {
        request_id: request_id.into(),
        tool_name: "Bash".into(),
        command: command.into(),
        input: Some(serde_json::json!({ "command": command })),
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

fn seed_chat(core: &EngineCore, chat_id: &str, auto_approve: bool) {
    core.workspace
        .create_space(SPACE, &core.device_id, "/tmp/yolo-space", None, false)
        .expect("space");
    core.workspace
        .create_chat(
            chat_id,
            Some(SPACE),
            Some(&core.device_id),
            Some(chat_config(auto_approve)),
            Some(CWD.to_string()),
        )
        .expect("chat");
}

/// Switched ON mid-run: the tool that is blocked RIGHT NOW is allowed, the
/// card leaves the inbox, and the run finishes. No restart.
#[tokio::test(flavor = "multi_thread")]
async fn switching_on_allows_what_is_already_parked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(
        dir.path(),
        vec![started(), asks_permission("perm-1", "rm -rf build"), done()],
    );
    let states = core.sessions.agent_states().clone();
    seed_chat(&core, "chat-1", false);

    core.sessions
        .dispatch("chat-1", HarnessId::Mock, run_request("clean the build"), None)
        .await
        .expect("dispatch");
    wait_for(|| !states.needs_you().is_empty(), "the ask to park").await;
    assert_eq!(
        states.needs_you()[0].kind,
        NeedsYouKind::Permission,
        "the run is waiting on the user before yolo goes on"
    );

    core.sessions.set_auto_approve("chat-1", true);

    wait_for(
        || {
            states
                .states()
                .iter()
                .any(|r| r.id == "chat-1" && r.state == AgentState::Done)
        },
        "the unblocked run to finish",
    )
    .await;
    let c = states.counters();
    assert!(
        states.needs_you().is_empty(),
        "the card is gone: nobody has to answer it now"
    );
    assert_eq!(
        (c.permissions_asked, c.permissions_auto_allowed, c.permissions_answered),
        (1, 1, 0),
        "answered by the mode, not by a person"
    );
}

/// Switched OFF: the parked card stays parked. Turning yolo off must never
/// answer anything on the user's behalf.
#[tokio::test(flavor = "multi_thread")]
async fn switching_off_leaves_the_card_for_the_user() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(
        dir.path(),
        vec![started(), asks_permission("perm-1", "rm -rf build"), done()],
    );
    let states = core.sessions.agent_states().clone();
    seed_chat(&core, "chat-1", false);

    core.sessions
        .dispatch("chat-1", HarnessId::Mock, run_request("clean the build"), None)
        .await
        .expect("dispatch");
    wait_for(|| !states.needs_you().is_empty(), "the ask to park").await;

    core.sessions.set_auto_approve("chat-1", false);
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(
        states.needs_you().len(),
        1,
        "off is not an answer; the card is still the user's to decide"
    );
    assert_eq!(states.counters().permissions_auto_allowed, 0);
}

/// Switched ON before the ask: the request never parks at all.
#[tokio::test(flavor = "multi_thread")]
async fn a_later_ask_never_reaches_the_inbox() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(
        dir.path(),
        vec![started(), asks_permission("perm-1", "rm -rf build"), done()],
    );
    let states = core.sessions.agent_states().clone();
    seed_chat(&core, "chat-1", false);
    core.sessions.set_auto_approve("chat-1", true);

    core.sessions
        .dispatch("chat-1", HarnessId::Mock, run_request("clean the build"), None)
        .await
        .expect("dispatch");
    wait_for(
        || {
            states
                .states()
                .iter()
                .any(|r| r.id == "chat-1" && r.state == AgentState::Done)
        },
        "the run to finish without asking anyone",
    )
    .await;
    let c = states.counters();
    assert_eq!(
        (c.permissions_asked, c.permissions_auto_allowed),
        (1, 1),
        "the gate saw it and the mode answered it"
    );
    assert!(states.needs_you().is_empty());
}

/// A new run on a chat whose ROW says yolo carries the flag even when the
/// caller sent false: the composer, a queued command and the crash-resume
/// rebuild all go through this one stamp.
#[tokio::test(flavor = "multi_thread")]
async fn a_run_on_a_yolo_chat_launches_with_the_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(dir.path(), vec![started(), done()]);
    seed_chat(&core, "chat-1", true);

    core.sessions
        .dispatch("chat-1", HarnessId::Mock, run_request("go"), None)
        .await
        .expect("dispatch");

    assert!(
        core.sessions.auto_approve("chat-1"),
        "the row's flag reached the live session"
    );
    let last = core.sessions.last_request("chat-1").expect("a last request");
    assert!(
        last.auto_approve,
        "the run was dispatched with auto_approve, so the CLI never prompts"
    );
}

/// A caller's per-run `auto_approve` must NOT become the chat's mode.
///
/// This is the shape `scripts/smoke.sh` drives: every turn it queues carries
/// `autoApprove: true`, and section f then waits for the fake harness's
/// permission request to appear in the needs-you inbox. When the run-config
/// backfill copied that flag onto the row, the next read of the row turned
/// yolo on, the gate answered the request, and the card the rig was waiting
/// for never arrived (CI 34040834906).
#[tokio::test(flavor = "multi_thread")]
async fn a_runs_own_auto_approve_is_not_the_chats_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(
        dir.path(),
        vec![started(), asks_permission("perm-1", "run the migration"), done()],
    );
    let states = core.sessions.agent_states().clone();
    // A chat with NO config row, exactly as `Mutate createChat` leaves it.
    core.workspace
        .create_space(SPACE, &core.device_id, "/tmp/yolo-space", None, false)
        .expect("space");
    core.workspace
        .create_chat(
            "chat-1",
            Some(SPACE),
            Some(&core.device_id),
            None,
            Some(CWD.to_string()),
        )
        .expect("chat");

    let mut request = run_request("run the migration");
    request.auto_approve = true; // what the smoke rig sends on every turn
    core.sessions
        .dispatch("chat-1", HarnessId::Mock, request, None)
        .await
        .expect("dispatch");

    wait_for(|| !states.needs_you().is_empty(), "the ask to reach the inbox").await;
    assert_eq!(
        states.needs_you()[0].kind,
        NeedsYouKind::Permission,
        "a run that asked to bypass its CLI still parks what the CLI asks anyway"
    );
    assert!(
        !core.sessions.auto_approve("chat-1"),
        "the chat is not in yolo mode: nobody switched it on"
    );
    // The run-config backfill itself lives on the doc-command drain path
    // (`doc_host`), which this direct dispatch does not go through; when it
    // does run, the row it writes must not claim a mode either.
    assert!(
        core.workspace
            .chat_config("chat-1")
            .is_none_or(|config| !config.auto_approve),
        "no row may come out of a run claiming the chat is in yolo mode"
    );
}

/// The flag is on the chat ROW, so it survives the engine that was holding
/// the live copy: a fresh engine over the same data dir reads it back.
#[tokio::test(flavor = "multi_thread")]
async fn the_flag_survives_a_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    {
        let core = assemble(dir.path(), vec![started(), done()]);
        seed_chat(&core, "chat-1", true);
        assert!(
            core.workspace
                .chat_config("chat-1")
                .expect("row")
                .auto_approve
        );
        // Persist now rather than on the debounce: the next engine reads the
        // snapshot from disk.
        core.workspace.flush();
    }
    let core = assemble(dir.path(), vec![started(), done()]);
    assert!(
        core.workspace
            .chat_config("chat-1")
            .expect("the row is still there")
            .auto_approve,
        "a restarted engine reads yolo back off the row"
    );
    core.sessions
        .dispatch("chat-1", HarnessId::Mock, run_request("go"), None)
        .await
        .expect("dispatch");
    assert!(
        core.sessions.auto_approve("chat-1"),
        "and applies it to the first run after the restart"
    );
}
