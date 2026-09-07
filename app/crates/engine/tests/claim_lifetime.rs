//! A deleted chat is not claimable, and a run for one does not start.
//!
//! `claim_chat` is how a chat gets its row on the first run command, so it
//! creates when the row is absent. A row can be absent for two reasons, and
//! only one of them is a chat: it may never have been written (a crash can
//! predate the debounced registry write), or the cascade may have taken it.
//! Claiming the second minted the row back, and at the removed project's cwd
//! `space_for_path` minted the project to hold it.

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

/// The dispatch fails, and it fails loudly: a run for a chat nobody has must
/// not start at all, because a run with no row is the state every guard added
/// since #147 exists to prevent.
#[tokio::test(flavor = "multi_thread")]
async fn a_dispatch_after_the_cascade_mints_nothing_and_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
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
    client
        .call(
            methods::MUTATE,
            serde_json::json!({ "op": "deleteSpace", "spaceId": "space-1" }),
        )
        .await
        .expect("delete space");
    wait_for(
        || core.workspace.read_chats().unwrap_or_default().is_empty(),
        "the cascade to land",
    )
    .await;

    let dispatched = core
        .sessions
        .dispatch("chat-1", HarnessId::Mock, run_request("hello again"), None)
        .await;
    assert!(
        dispatched.is_err(),
        "a run started for a chat the cascade removed"
    );

    // Nothing left behind either: the refusal happens before the run handle
    // is registered, so there is no half-started run to trip over.
    assert!(
        core.sessions.session_status("chat-1").is_none(),
        "a refused dispatch left a session status"
    );
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let spaces = core.workspace.read_spaces().unwrap_or_default();
        let chats = core.workspace.read_chats().unwrap_or_default();
        assert!(spaces.is_empty(), "a project came back: {spaces:?}");
        assert!(chats.is_empty(), "a chat came back: {chats:?}");
    }
}

/// The other side: a chat with no row at all is still claimable, because that
/// is how every new chat starts.
#[tokio::test(flavor = "multi_thread")]
async fn a_chat_that_never_had_a_row_still_claims() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(dir.path());

    core.sessions
        .dispatch("chat-fresh", HarnessId::Mock, run_request("first turn"), None)
        .await
        .expect("a brand-new chat dispatches");

    let chat = core
        .workspace
        .chat("chat-fresh")
        .expect("read")
        .expect("the claim created the row");
    assert_eq!(chat.cwd.as_deref(), Some(CWD));
}
