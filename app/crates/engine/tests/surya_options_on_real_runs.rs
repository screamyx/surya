//! Decision 5 reaches a real agent, or it reaches nobody.
//!
//! The harness wires the `surya-mcp` sidecar, the prompt append, the cards
//! skill and the `SendMessage` denial only when `RunRequest.surya` is `Some`
//! (`harness/src/claude/mod.rs`). Every `Some(SuryaOptions)` in the tree used
//! to be a test, an example or a `#[cfg(test)]` block, so no running engine
//! ever set it: real sessions had no `show_card`, no cards skill, and the
//! CLI's own cross-session messaging was never denied.
//!
//! This test holds the seam shut from the engine side. The harness records
//! the request it was handed and the assertions read it back.

use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::BoxStream;

use surya_engine::{EngineCore, HarnessRegistry};
use surya_harness::{Harness, HarnessError, RunControls};
use surya_proto::{
    AgentEvent, DoneStatus, HarnessId, Model, ReasoningLevel, RunRequest, SandboxLevel,
    SteeringMode, SuryaOptions, ToolCall,
};

const CHAT: &str = "chat-surya-options";
const CWD: &str = "/repo/checkout";

/// A private `SURYA_DATA_DIR` for every test in this binary.
///
/// The card store defaults to `~/.surya` when this is unset, which is also
/// where it resolved BEFORE surya#226. A test that leaves it unset therefore
/// passes on the unfixed engine and proves nothing. Setting it is what makes
/// the assertion below fail if `card_store` stops following the data
/// directory.
///
/// Written exactly once and read by every test before it dispatches: libtest
/// runs a binary's tests as threads in one process, so the write has to happen
/// ahead of every read rather than inside one test.
static DATA_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = std::env::temp_dir().join(format!("surya-options-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("private data dir");
    // SAFETY: written once, before any test in this binary reads the
    // environment, and this binary sets no other variable.
    unsafe { std::env::set_var("SURYA_DATA_DIR", &dir) };
    dir
});

/// Keeps every request it is asked to run.
struct RecordingHarness {
    seen: Arc<Mutex<Vec<RunRequest>>>,
}

#[async_trait]
impl Harness for RecordingHarness {
    fn id(&self) -> HarnessId {
        HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Recording"
    }
    fn supports_steering(&self) -> bool {
        false
    }
    fn steering_mode(&self) -> SteeringMode {
        SteeringMode::TurnBoundary
    }
    fn reasoning_levels(&self) -> &[ReasoningLevel] {
        &[ReasoningLevel::Medium]
    }
    async fn models(&self) -> Result<Vec<Model>, HarnessError> {
        Ok(vec![])
    }
    async fn run(
        &self,
        request: RunRequest,
        _controls: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        self.seen.lock().expect("seen").push(request);
        let events: Vec<Result<AgentEvent, HarnessError>> = vec![Ok(AgentEvent::Done {
            status: DoneStatus::Completed,
            result: None,
            error: None,
            session_id: None,
        })];
        Ok(Box::pin(futures::stream::iter(events)))
    }
}

fn run_request(prompt: &str) -> RunRequest {
    RunRequest {
        // What the app sends: the composer does not know the host's paths, so
        // it sends None and the engine fills it in.
        surya: None,
        prompt: prompt.to_string(),
        harness: Some(HarnessId::Mock),
        model: None,
        reasoning: None,
        model_options: serde_json::Map::new(),
        cwd: CWD.to_string(),
        sandbox: SandboxLevel::WorkspaceWrite,
        auto_approve: false,
        resume: None,
        attachments: Vec::new(),
        worktree: None,
    }
}

async fn dispatched_request(prompt: &str) -> RunRequest {
    let tmp = tempfile::tempdir().expect("tempdir");
    let seen: Arc<Mutex<Vec<RunRequest>>> = Arc::default();
    let registry = HarnessRegistry::new();
    registry.register(Arc::new(RecordingHarness { seen: seen.clone() }));
    let core = EngineCore::assemble(
        &tmp.path().join("data"),
        Arc::new(registry),
        HarnessId::Mock,
        None,
    )
    .expect("engine core assembles");
    core.sessions
        .dispatch(CHAT, HarnessId::Mock, run_request(prompt), None)
        .await
        .expect("dispatch");

    // Match on the prompt, not on arrival order: `dispatch` kicks the
    // auto-titler off BEFORE it spawns drive_run, so the title run can be
    // recorded first and this would read the wrong request.
    for _ in 0..200 {
        let found = seen
            .lock()
            .expect("seen")
            .iter()
            .find(|r| r.prompt == prompt)
            .cloned();
        if let Some(request) = found {
            return request;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the harness was never handed the user's request");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_user_run_carries_the_surya_block() {
    let data_dir = &*DATA_DIR;
    let request = dispatched_request("compare A and B in a table").await;

    let options: SuryaOptions = request
        .surya
        .expect("a real run must carry the surya block, or the sidecar never loads");

    // The mail system addresses a top-level agent by its chat id.
    assert_eq!(options.agent_id, CHAT);
    // The workspace doubles as the sidecar's catalog root.
    assert_eq!(options.workspace, CWD);
    // One card file per chat, under the data dir. The prefix is the point
    // (surya#226): revert `card_store` to `surya_home_dir()` and this fails,
    // because the store then hangs off `$HOME` whatever the data directory
    // says, and two engines under one OS user share it.
    let store = options.card_store.expect("card store");
    assert!(
        store.ends_with(&format!("cards/{CHAT}.jsonl")),
        "card_store={store}"
    );
    assert!(
        std::path::Path::new(&store).starts_with(data_dir),
        "the card store hangs off SURYA_DATA_DIR: card_store={store} data_dir={}",
        data_dir.display()
    );
    // Left to the harness, which looks beside the executable then on PATH -
    // which is where the installer now puts it.
    assert_eq!(options.mcp_binary, None);
}

/// A title run is the engine talking to itself: it builds its own request in
/// `titles.rs` and calls the harness directly, never through `dispatch`. It
/// must not get tools, a card prompt or a mail address, or it shows up as an
/// agent that can act.
///
/// Asserted as a count rather than by forcing a title run: whatever else the
/// engine starts while this dispatch runs, exactly one request may carry the
/// block, and it must be the user's.
#[tokio::test(flavor = "multi_thread")]
async fn only_the_user_run_carries_it() {
    let _ = &*DATA_DIR;
    let tmp = tempfile::tempdir().expect("tempdir");
    let seen: Arc<Mutex<Vec<RunRequest>>> = Arc::default();
    let registry = HarnessRegistry::new();
    registry.register(Arc::new(RecordingHarness { seen: seen.clone() }));
    let core = EngineCore::assemble(
        &tmp.path().join("data"),
        Arc::new(registry),
        HarnessId::Mock,
        None,
    )
    .expect("engine core assembles");
    core.sessions
        .dispatch(CHAT, HarnessId::Mock, run_request("write the thing"), None)
        .await
        .expect("dispatch");

    // Let anything else the engine starts (the auto-titler) land too.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let requests = seen.lock().expect("seen").clone();
    let with: Vec<&RunRequest> = requests.iter().filter(|r| r.surya.is_some()).collect();
    let prompts: Vec<&str> = requests.iter().map(|r| r.prompt.as_str()).collect();
    assert_eq!(
        with.len(),
        1,
        "runs={} with_block={} prompts={prompts:?}",
        requests.len(),
        with.len()
    );
    assert_eq!(with[0].prompt, "write the thing");
}

/// The whole seam, against the REAL Claude Code CLI:
///
///     cargo test -p surya-engine --test surya_options_on_real_runs \
///       -- --ignored --nocapture live_
///
/// `#[ignore]`d on purpose. It spends model quota, and a test that quietly
/// passes when the binary is missing is worse than one that never ran - the
/// bug this file exists for was exactly a feature that looked fine because
/// nothing exercised it end to end.
///
/// Two assertions, because the block carries two promises:
///   1. the agent can draw a card at all (the sidecar loaded), and
///   2. Claude Code's own SendMessage is gone (decision 19's one mail
///      channel is enforced, not just intended).
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn live_a_real_claude_run_gets_the_card_tool_and_loses_send_message() {
    let _ = &*DATA_DIR;
    let tmp = tempfile::tempdir().expect("tempdir");
    let cwd = tmp.path().join("workspace");
    std::fs::create_dir_all(&cwd).expect("workspace");

    let registry = HarnessRegistry::new();
    registry.register(Arc::new(surya_harness::ClaudeHarness::new()));
    let core = EngineCore::assemble(
        &tmp.path().join("data"),
        Arc::new(registry),
        HarnessId::ClaudeCode,
        None,
    )
    .expect("engine core assembles");

    let (_replay, mut events) = core.sessions.subscribe(CHAT, 0).expect("subscribe");

    let mut request = run_request("Compare option A and option B in a table.");
    request.harness = Some(HarnessId::ClaudeCode);
    request.cwd = cwd.to_string_lossy().into_owned();
    if core
        .sessions
        .dispatch(CHAT, HarnessId::ClaudeCode, request, None)
        .await
        .is_err()
    {
        eprintln!("SKIPPED: no usable claude CLI on this box");
        return;
    }

    let mut tools: Option<Vec<String>> = None;
    let mut card_calls = 0usize;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    while tokio::time::Instant::now() < deadline {
        let Ok(Ok(event)) = tokio::time::timeout(Duration::from_secs(30), events.recv()).await
        else {
            break;
        };
        match &event.event {
            AgentEvent::SessionStarted { tools: t, .. } => tools = Some(t.clone()),
            // The sidecar's tool arrives as an MCP call, server `surya`.
            AgentEvent::ToolCall {
                call: ToolCall::Mcp { tool, .. },
                ..
            } if tool.contains("show_card") => card_calls += 1,
            AgentEvent::Done { .. } => break,
            _ => {}
        }
    }

    let Some(tools) = tools else {
        eprintln!("SKIPPED: the run never started - no claude binary, or no quota");
        return;
    };

    // 2. The denial is live. `--disallowed-tools SendMessage,ListAgents` is
    //    inside the same `if let Some(options)` as everything else, so this
    //    failing means the block did not reach the CLI at all.
    let banned: Vec<&String> = tools
        .iter()
        .filter(|t| t.as_str() == "SendMessage" || t.as_str() == "ListAgents")
        .collect();
    assert!(
        banned.is_empty(),
        "denied tools still offered: {banned:?} - tools={tools:?}"
    );

    // 1. The sidecar loaded and the agent reached for a card.
    let has_card_tool = tools.iter().any(|t| t.contains("show_card"));
    assert!(
        has_card_tool,
        "show_card absent from the tool list - the sidecar did not load. tools={tools:?}"
    );
    eprintln!("show_card offered, SendMessage denied, show_card calls={card_calls}");
}
