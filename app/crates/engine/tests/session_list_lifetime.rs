//! A removed chat leaves the live session list.
//!
//! `SessionsEngine::inner.statuses` is the in-memory status map behind
//! `session_status`, `any_active` and the `WatchSessions` feed. `drop_chat`
//! cleared the yolo flag, the parked permissions and the last run
//! configuration, but not this - so a cascaded chat stayed in a client's
//! session list, and stayed inside the updater's "do not restart from under a
//! run" gate.

use std::sync::Arc;
use std::time::Duration;

use surya_engine::{EngineCore, HarnessRegistry};
use surya_harness::Harness;
use surya_harness::mock::MockHarness;
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
            // Parks on the gate, so the run is live when the project goes.
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

#[tokio::test(flavor = "multi_thread")]
async fn a_removed_project_takes_its_chat_out_of_the_session_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(dir.path());
    let client = surya_rpc::memory_client(core.rpc_service());
    // Subscribed before the run, so the list is watched across the whole
    // thing rather than sampled after it.
    let sessions = core.sessions.watch_sessions();

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

    // It has to be IN the list first, or its absence later proves nothing.
    wait_for(
        || sessions.borrow().iter().any(|s| s.chat_id == "chat-1"),
        "the run to reach the session list",
    )
    .await;
    assert!(
        core.sessions.session_status("chat-1").is_some(),
        "and session_status to know it"
    );

    client
        .call(
            methods::MUTATE,
            serde_json::json!({ "op": "deleteSpace", "spaceId": "space-1" }),
        )
        .await
        .expect("delete space");

    wait_for(
        || !sessions.borrow().iter().any(|s| s.chat_id == "chat-1"),
        "the removed chat to leave the session list",
    )
    .await;

    // And it STAYS gone. Leaving on the first look is the easy half: the
    // cascade drops the chat before its teardown task, so this assertion used
    // to land while the interrupted run was still settling, and the run's
    // terminal transition put the entry straight back afterwards. A fixed
    // window on purpose - it asserts that nothing arrives.
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !sessions.borrow().iter().any(|s| s.chat_id == "chat-1"),
            "the removed chat came back to the session list: {:?}",
            sessions.borrow().clone()
        );
    }
    assert!(
        core.sessions.session_status("chat-1").is_none(),
        "session_status still answers for a chat that is gone"
    );
}

/// `deleteChat` mid-run, which is the certain case rather than the racy one.
///
/// That path does not interrupt the run (`rpc.rs`, `MutateParams::DeleteChat`
/// deletes, purges and drops, and returns). So the run keeps going and its
/// terminal `set_status` lands after the drop every time - re-creating the
/// entry unless creation is gated on the chat still having a row.
#[tokio::test(flavor = "multi_thread")]
async fn deleting_a_chat_mid_run_does_not_leave_it_in_the_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(dir.path());
    let client = surya_rpc::memory_client(core.rpc_service());
    let sessions = core.sessions.watch_sessions();

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
        || sessions.borrow().iter().any(|s| s.chat_id == "chat-1"),
        "the run to reach the session list",
    )
    .await;

    client
        .call(
            methods::MUTATE,
            serde_json::json!({ "op": "deleteChat", "chatId": "chat-1" }),
        )
        .await
        .expect("delete chat");

    // The run is still live and will settle on its own. Nothing it does on
    // the way out may put the chat back.
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !sessions.borrow().iter().any(|s| s.chat_id == "chat-1"),
            "the deleted chat is in the session list: {:?}",
            sessions.borrow().clone()
        );
    }
    assert!(
        core.sessions.session_status("chat-1").is_none(),
        "session_status still answers for a deleted chat"
    );
}
