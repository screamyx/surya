//! Proof for the seat's deliverable, driven through the mock harness and the
//! real engine assembly: three sessions in three states, the needs-you inbox
//! that lists two of them, a rule made from one answer that auto-allows the
//! identical next request, and a subagent frame that produces a child row
//! whose needs-you rolls up its parent.
//!
//! Every assertion prints its counters as a pair. Run it with
//! `--nocapture` to read them.

use std::sync::Arc;
use std::time::Duration;

use surya_engine::agent_states::AgentStates;
use surya_engine::{EngineCore, HarnessRegistry};
use surya_harness::Harness;
use surya_harness::mock::MockHarness;
use surya_proto::{
    AgentEvent, AgentState, AgentStateRow, DoneStatus, HarnessId, NeedsYouKind, NeedsYouItem,
    PermissionDecision, RememberRule, RunRequest, RuleScope, SandboxLevel, UserInputQuestion,
};

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

fn done(status: DoneStatus) -> AgentEvent {
    AgentEvent::Done {
        status,
        result: None,
        error: None,
        session_id: Some("hs-1".into()),
    }
}

/// A scripted permission ask: the mock blocks on the gate here.
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

fn row<'a>(rows: &'a [AgentStateRow], id: &str) -> &'a AgentStateRow {
    rows.iter().find(|r| r.id == id).unwrap_or_else(|| {
        panic!(
            "no row {id}; have {:?}",
            rows.iter().map(|r| &r.id).collect::<Vec<_>>()
        )
    })
}

fn of_kind(inbox: &[NeedsYouItem], kind: NeedsYouKind) -> Vec<&NeedsYouItem> {
    inbox.iter().filter(|i| i.kind == kind).collect()
}

/// Three sessions — one waiting on a permission, one on a question, one done —
/// and `WatchNeedsYou` lists exactly the two that want the user.
#[tokio::test(flavor = "multi_thread")]
async fn three_sessions_two_of_them_need_you() {
    let dir = tempfile::tempdir().expect("tempdir");
    // The permission chat's script blocks on the gate; the other two settle.
    let core = assemble(
        dir.path(),
        vec![
            started(),
            asks_permission("perm-1", "php artisan migrate"),
            done(DoneStatus::Completed),
        ],
    );
    let states = core.sessions.agent_states().clone();

    core.sessions
        .dispatch("chat-permission", HarnessId::Mock, run_request("migrate the database"), None)
        .await
        .expect("dispatch");
    wait_for(
        || !of_kind(&states.needs_you(), NeedsYouKind::Permission).is_empty(),
        "the permission to park",
    )
    .await;

    // The question chat: the engine's input bridge owns the lifecycle, so it
    // is opened the same way a harness ask arrives.
    states.open_questions(
        "chat-question",
        "chat-question",
        "input-1",
        vec![UserInputQuestion {
            id: "q1".into(),
            header: "Question".into(),
            question: "Which sync strategy should the rewrite use?".into(),
            options: vec!["Poll".into(), "Event-driven".into()],
            multi_select: false,
        }],
    );

    // The done chat: finished, unread.
    states.note_session("chat-done", surya_proto::SessionStatus::Working);
    states.note_run_end("chat-done", DoneStatus::Completed, "");

    let rows = states.states();
    let sessions = ["chat-permission", "chat-question", "chat-done"];
    let inbox = states.needs_you();
    let needs = rows.iter().filter(|r| r.rolled_up.needs_you()).count();
    println!(
        "sessions={} needs_you={} inbox={} asked={} auto_allowed={}",
        sessions.len(),
        needs,
        inbox.len(),
        states.counters().permissions_asked,
        states.counters().permissions_auto_allowed,
    );

    assert_eq!(
        row(&rows, "chat-permission").state,
        AgentState::NeedsYou {
            kind: NeedsYouKind::Permission
        }
    );
    assert_eq!(
        row(&rows, "chat-question").state,
        AgentState::NeedsYou {
            kind: NeedsYouKind::Question
        }
    );
    assert_eq!(row(&rows, "chat-done").state, AgentState::Done);
    assert_eq!(needs, 2, "sessions=3 needs_you=2");
    assert_eq!(inbox.len(), 2, "the done session is not in the inbox");

    // The permission card carries what the UI has to draw.
    let permission = of_kind(&inbox, NeedsYouKind::Permission)[0];
    assert_eq!(permission.chat_id, "chat-permission");
    assert_eq!(permission.tool_name.as_deref(), Some("Bash"));
    assert_eq!(
        permission.tool_command.as_deref(),
        Some("php artisan migrate")
    );
    // The question card carries its options.
    let question = of_kind(&inbox, NeedsYouKind::Question)[0];
    assert_eq!(question.options, vec!["Poll".to_string(), "Event-driven".to_string()]);

    // Answering the permission unblocks the run, which then completes.
    core.sessions
        .respond_permission("perm-1", PermissionDecision::Allow, None)
        .expect("answers");
    wait_for(
        || {
            states
                .states()
                .iter()
                .any(|r| r.id == "chat-permission" && r.state == AgentState::Done)
        },
        "the unblocked run to finish",
    )
    .await;
    let c = states.counters();
    println!(
        "after answer: asked={} auto_allowed={} answered={}",
        c.permissions_asked, c.permissions_auto_allowed, c.permissions_answered
    );
    assert_eq!((c.permissions_asked, c.permissions_answered), (1, 1));
}

/// A rule made from one answer auto-allows the identical next request:
/// two asks, one of them never reaches the inbox.
#[tokio::test(flavor = "multi_thread")]
async fn a_rule_made_from_one_answer_auto_allows_the_next_request() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(
        dir.path(),
        vec![
            started(),
            asks_permission("perm-1", "php artisan migrate"),
            done(DoneStatus::Completed),
        ],
    );
    let states = core.sessions.agent_states().clone();

    // First ask: parks, and the answer is remembered as a rule.
    core.sessions
        .dispatch("chat-1", HarnessId::Mock, run_request("migrate"), None)
        .await
        .expect("dispatch");
    wait_for(|| !states.needs_you().is_empty(), "the first ask").await;
    core.sessions
        .respond_permission(
            "perm-1",
            PermissionDecision::Allow,
            Some(&RememberRule {
                scope: RuleScope::Workspace,
                pattern: "php artisan migrate*".into(),
                name: None,
            }),
        )
        .expect("answers and remembers");
    let rules = states.rules().list();
    assert_eq!(rules.len(), 1, "one answer made one rule");
    println!("rule created: {:?} in {:?}", rules[0].name, rules[0].workspace_path);
    assert_eq!(rules[0].name, "Bash php artisan migrate* in project-jag");

    wait_for(
        || {
            states
                .states()
                .iter()
                .any(|r| r.id == "chat-1" && r.state == AgentState::Done)
        },
        "the first run to finish",
    )
    .await;

    // Second ask, a different chat, a command the rule covers: never parks.
    core.registry.register(Arc::new(MockHarness {
        script: vec![
            started(),
            asks_permission("perm-2", "php artisan migrate --seed"),
            done(DoneStatus::Completed),
        ],
    }) as Arc<dyn Harness>);
    core.sessions
        .dispatch("chat-2", HarnessId::Mock, run_request("migrate again"), None)
        .await
        .expect("dispatch");
    wait_for(
        || {
            states
                .states()
                .iter()
                .any(|r| r.id == "chat-2" && r.state == AgentState::Done)
        },
        "the auto-allowed run to finish",
    )
    .await;

    let c = states.counters();
    println!(
        "asked={} auto_allowed={} answered={} parked={}",
        c.permissions_asked,
        c.permissions_auto_allowed,
        c.permissions_answered,
        states.needs_you().len()
    );
    assert_eq!(
        (c.permissions_asked, c.permissions_auto_allowed),
        (2, 1),
        "asked=2 auto_allowed=1"
    );
    assert!(
        states.needs_you().is_empty(),
        "the rule answered before anyone was woken"
    );
}

/// A denied permission reaches the harness as a refusal rather than a hang,
/// and never becomes a rule.
#[tokio::test(flavor = "multi_thread")]
async fn a_denied_permission_ends_the_turn_and_makes_no_rule() {
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(
        dir.path(),
        vec![
            started(),
            asks_permission("perm-1", "rm -rf /"),
            done(DoneStatus::Completed),
        ],
    );
    let states = core.sessions.agent_states().clone();
    core.sessions
        .dispatch("chat-1", HarnessId::Mock, run_request("clean up"), None)
        .await
        .expect("dispatch");
    wait_for(|| !states.needs_you().is_empty(), "the ask").await;

    core.sessions
        .respond_permission(
            "perm-1",
            PermissionDecision::Deny,
            Some(&RememberRule {
                scope: RuleScope::Global,
                pattern: "*".into(),
                name: None,
            }),
        )
        .expect("answers");
    wait_for(
        || {
            states
                .states()
                .iter()
                .any(|r| r.id == "chat-1" && r.state == AgentState::Stopped)
        },
        "the refused run to stop",
    )
    .await;
    println!(
        "asked={} denied_rules={} inbox={}",
        states.counters().permissions_asked,
        states.rules().list().len(),
        states.needs_you().len()
    );
    assert!(states.rules().list().is_empty(), "a Deny is never remembered");
}

/// A subagent frame produces a child row under its spawner, and the child's
/// needs-you rolls up the parent (decision 15).
#[tokio::test(flavor = "multi_thread")]
async fn a_subagent_frame_makes_a_child_that_rolls_up() {
    let tagged = |event: AgentEvent| AgentEvent::Subagent {
        parent_tool_use_id: "tool-9".into(),
        event: Box::new(event),
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let core = assemble(
        dir.path(),
        vec![
            started(),
            AgentEvent::ToolCall {
                id: "tool-9".into(),
                call: surya_proto::ToolCall::Unknown {
                    name: "Agent: scan the repo".into(),
                    input: Some(serde_json::json!({ "description": "scan the repo" })),
                },
            },
            tagged(AgentEvent::TextDelta {
                text: "scanning".into(),
            }),
            tagged(AgentEvent::Done {
                status: DoneStatus::Errored,
                result: None,
                error: Some("out of context".into()),
                session_id: None,
            }),
            done(DoneStatus::Completed),
        ],
    );
    let states = core.sessions.agent_states().clone();
    core.sessions
        .dispatch("chat-1", HarnessId::Mock, run_request("scan"), None)
        .await
        .expect("dispatch");

    let child = "chat-1:tool-9";
    wait_for(
        || states.states().iter().any(|r| r.id == child),
        "the child row",
    )
    .await;
    wait_for(
        || {
            states
                .states()
                .iter()
                .any(|r| r.id == "chat-1" && r.needs_you_children == 1)
        },
        "the roll-up",
    )
    .await;

    let rows = states.states();
    let children = rows.iter().filter(|r| r.parent_id.is_some()).count();
    let rolled = rows
        .iter()
        .filter(|r| r.parent_id.is_none() && r.rolled_up.needs_you() && !r.state.needs_you())
        .count();
    println!(
        "rows={} children={} rolled_up={} counters.children={}",
        rows.len(),
        children,
        rolled,
        states.counters().children_registered
    );
    assert_eq!(children, 1, "children=1");
    assert_eq!(rolled, 1, "rolled_up=1");

    let child_row = row(&rows, child);
    assert_eq!(child_row.parent_id.as_deref(), Some("chat-1"));
    assert_eq!(child_row.chat_id, "chat-1", "the tap opens the parent chat");
    // The name the spawn gave itself reaches the row. This is the assertion
    // that covers `sessions.rs`: without the `note_spawn` call there, nothing
    // in production ever passes a label, the child's label is `None` and the
    // tree shows "Untitled". The tests in `spawns.rs` call `note_spawn`
    // directly, so they pass either way.
    assert_eq!(
        child_row.label.as_deref(),
        Some("scan the repo"),
        "the child carries the name its spawn gave it"
    );
    assert_eq!(
        child_row.state,
        AgentState::NeedsYou {
            kind: NeedsYouKind::Failed
        }
    );
    // The parent's OWN state is untouched; only the rolled-up one moves.
    let parent = row(&rows, "chat-1");
    assert!(!parent.state.needs_you(), "the parent itself wants nothing");
    assert_eq!(
        parent.rolled_up,
        AgentState::NeedsYou {
            kind: NeedsYouKind::Failed
        }
    );
    assert_eq!(parent.needs_you_children, 1);
    // Every needs-you row sorts ahead of every settled one. Parent and child
    // share a rank here, so their order between themselves is by recency —
    // the rail nests them from `parent_id`.
    let first_settled = rows.iter().position(|r| !r.rolled_up.needs_you());
    assert!(
        first_settled.is_none_or(|ix| rows[..ix].iter().all(|r| r.rolled_up.needs_you())),
        "needs-you rows sort first"
    );

    // The failure is in the inbox with its detail and a Retry.
    let inbox = states.needs_you();
    let failed = of_kind(&inbox, NeedsYouKind::Failed);
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].prompt, "out of context");
    assert!(failed[0].retryable);
}

/// The rules table survives an engine restart: it is the one durable piece of
/// this feature, and it is per-device.
#[test]
fn rules_persist_across_an_engine_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let states = AgentStates::new(surya_engine::rules::AllowRules::open(dir.path()));
    states
        .rules()
        .add(surya_proto::AllowRule {
            id: "r-1".into(),
            name: "read-only git".into(),
            scope: RuleScope::Global,
            workspace_path: None,
            tool_name: "Bash".into(),
            pattern: "git status*".into(),
            exact: false,
            created_at: chrono::Utc::now(),
        })
        .expect("adds");

    let reopened = AgentStates::new(surya_engine::rules::AllowRules::open(dir.path()));
    let rules = reopened.rules().list();
    println!("written=1 read_back={}", rules.len());
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "read-only git");
}
