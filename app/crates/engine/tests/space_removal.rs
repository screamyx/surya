//! E2E-NAV-02: removing a project must not leave its chats in the rail.
//!
//! Both doc backends tombstone the chat row and the session-status row on a
//! `deleteSpace` cascade, so the sidebar loses the chat. The engine's own
//! agent-state node is a separate, in-memory record, and nothing dropped it:
//! `deleteChat` calls `sessions.drop_chat`, `deleteSpace` never did. The rail
//! reads that node, so it kept drawing a row for a chat the app could no
//! longer open, with no title left to draw ("Untitled chat"), through as many
//! client restarts as you like.
//!
//! Driven over the real RPC surface, the same `Mutate` the app sends.

use std::sync::Arc;
use std::time::Duration;

use surya_engine::{EngineCore, HarnessRegistry};
use surya_harness::Harness;
use surya_harness::mock::MockHarness;
use surya_proto::{AgentEvent, DoneStatus, HarnessId, RunRequest, SandboxLevel};
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

/// A project with one chat that is mid-run when the project is removed.
/// After the cascade the rail must have nothing left to draw for that chat.
#[tokio::test(flavor = "multi_thread")]
async fn removing_a_project_takes_its_chats_out_of_the_rail() {
    let dir = tempfile::tempdir().expect("tempdir");
    // The script parks on the gate, so the run is still live when the project
    // goes away - the state the bug was reported in.
    let core = assemble(
        dir.path(),
        vec![
            started(),
            asks_permission("perm-1", "php artisan migrate"),
            done(DoneStatus::Completed),
        ],
    );
    let states = core.sessions.agent_states().clone();
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
        .dispatch(
            "chat-1",
            HarnessId::Mock,
            run_request("migrate the database"),
            None,
        )
        .await
        .expect("dispatch");
    wait_for(
        || states.states().iter().any(|r| r.chat_id == "chat-1"),
        "the chat to reach the rail",
    )
    .await;

    client
        .call(
            methods::MUTATE,
            serde_json::json!({ "op": "deleteSpace", "spaceId": "space-1" }),
        )
        .await
        .expect("delete space");

    // The doc side has always been right: chat row and session-status row gone.
    wait_for(
        || core.workspace.chat("chat-1").ok().flatten().is_none(),
        "the chat row to go",
    )
    .await;

    // The rail reads the agent-state rows, and this is what was left behind.
    wait_for(
        || !states.states().iter().any(|r| r.chat_id == "chat-1"),
        "the rail row to go with the project",
    )
    .await;
    // Nothing parked under a chat nobody can open, either.
    assert!(
        !states.needs_you().iter().any(|i| i.chat_id == "chat-1"),
        "needs-you still lists the removed chat: {:?}",
        states.needs_you()
    );
}
