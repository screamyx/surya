//! Unit tests for the derived-state store: the five states, the roll-up, the
//! inbox order, and the rule that answers before the user sees the card.

use super::*;
// Named here rather than in the parent: the parent only mentions these
// two in `derive.rs`, so importing them there would read as unused.
use zeron_proto::{AgentState, NeedsYouKind, RuleScope};

fn states() -> AgentStates {
    AgentStates::new(AllowRules::open(
        tempfile::tempdir().expect("tempdir").keep(),
    ))
}

fn request(id: &str, tool: &str, command: &str) -> PermissionRequest {
    PermissionRequest {
        request_id: id.into(),
        tool_name: tool.into(),
        command: command.into(),
        input: None,
    }
}

fn question(id: &str, text: &str) -> UserInputQuestion {
    UserInputQuestion {
        id: id.into(),
        header: "Question".into(),
        question: text.into(),
        options: vec!["Yes".into(), "No".into()],
        multi_select: false,
    }
}

fn row<'a>(rows: &'a [AgentStateRow], id: &str) -> &'a AgentStateRow {
    rows.iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id} in {:?}", rows.iter().map(|r| &r.id).collect::<Vec<_>>()))
}

#[test]
fn a_working_chat_reads_working_and_a_finished_one_reads_done() {
    let s = states();
    s.note_session("chat-1", SessionStatus::Working);
    assert_eq!(row(&s.states(), "chat-1").state, AgentState::Working);

    s.note_session("chat-1", SessionStatus::Idle);
    s.note_run_end("chat-1", DoneStatus::Completed, "");
    assert_eq!(row(&s.states(), "chat-1").state, AgentState::Done);

    // Opening it clears the unread badge.
    s.mark_seen("chat-1");
    assert_eq!(row(&s.states(), "chat-1").state, AgentState::Idle);
}

#[test]
fn an_interrupt_reads_stopped_and_a_crash_reads_needs_you_failed() {
    let s = states();
    s.note_session("chat-stop", SessionStatus::Working);
    s.note_run_end("chat-stop", DoneStatus::Interrupted, "");
    assert_eq!(row(&s.states(), "chat-stop").state, AgentState::Stopped);

    s.note_session("chat-crash", SessionStatus::Working);
    s.note_run_end("chat-crash", DoneStatus::Errored, "claude exited (code 1)");
    assert_eq!(
        row(&s.states(), "chat-crash").state,
        AgentState::NeedsYou {
            kind: NeedsYouKind::Failed
        }
    );
    // Decision 17: the crash sits in the inbox with its detail and a Retry.
    let inbox = s.needs_you();
    let failed = inbox
        .iter()
        .find(|i| i.agent_id == "chat-crash")
        .expect("crash is in the inbox");
    assert_eq!(failed.kind, NeedsYouKind::Failed);
    assert_eq!(failed.prompt, "claude exited (code 1)");
    assert!(failed.retryable);

    // A new turn clears both.
    s.note_session("chat-crash", SessionStatus::Working);
    assert_eq!(row(&s.states(), "chat-crash").state, AgentState::Working);
    assert!(s.needs_you().iter().all(|i| i.agent_id != "chat-crash"));
}

#[test]
fn a_parked_permission_puts_the_chat_in_the_inbox_and_answering_frees_it() {
    let s = states();
    s.note_session("chat-1", SessionStatus::Working);
    let (tx, rx) = oneshot::channel();
    let open = s.open_permission(
        "chat-1",
        "chat-1",
        "/repos/project-jag",
        request("req-1", "Bash", "php artisan migrate"),
        tx,
    );
    assert!(matches!(open, PermissionOpen::Parked));

    assert_eq!(
        row(&s.states(), "chat-1").state,
        AgentState::NeedsYou {
            kind: NeedsYouKind::Permission
        }
    );
    let inbox = s.needs_you();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].kind, NeedsYouKind::Permission);
    assert_eq!(inbox[0].tool_name.as_deref(), Some("Bash"));
    assert_eq!(inbox[0].tool_command.as_deref(), Some("php artisan migrate"));

    let resolved = s
        .resolve_permission("req-1", PermissionDecision::Allow, None)
        .expect("resolves");
    assert_eq!(resolved.chat_id, "chat-1");
    assert!(resolved.created_rule.is_none());
    assert_eq!(rx.blocking_recv().unwrap(), PermissionDecision::Allow);
    assert!(s.needs_you().is_empty());
    assert_eq!(row(&s.states(), "chat-1").state, AgentState::Working);

    let c = s.counters();
    assert_eq!(
        (c.permissions_asked, c.permissions_auto_allowed, c.permissions_answered),
        (1, 0, 1)
    );
}

#[test]
fn a_remembered_answer_auto_allows_the_identical_next_request() {
    let s = states();
    let (tx, rx) = oneshot::channel();
    s.open_permission(
        "chat-1",
        "chat-1",
        "/repos/project-jag",
        request("req-1", "Bash", "php artisan migrate"),
        tx,
    );
    let resolved = s
        .resolve_permission(
            "req-1",
            PermissionDecision::Allow,
            Some(&RememberRule {
                scope: RuleScope::Workspace,
                pattern: "php artisan migrate*".into(),
                name: None,
            }),
        )
        .expect("resolves");
    assert_eq!(rx.blocking_recv().unwrap(), PermissionDecision::Allow);
    let rule = resolved.created_rule.expect("a rule was created");
    assert_eq!(rule.name, "Bash php artisan migrate* in project-jag");

    // The next, identical request never reaches the inbox.
    let (tx2, rx2) = oneshot::channel();
    let open = s.open_permission(
        "chat-1",
        "chat-1",
        "/repos/project-jag",
        request("req-2", "Bash", "php artisan migrate --seed"),
        tx2,
    );
    match open {
        PermissionOpen::AutoAllowed(matched) => {
            assert_eq!(matched.id, rule.id);
            assert_eq!(matched.transcript_line(), format!("allowed by rule {}", rule.name));
        }
        other => panic!("expected AutoAllowed, got {other:?}"),
    }
    assert_eq!(rx2.blocking_recv().unwrap(), PermissionDecision::Allow);
    assert!(s.needs_you().is_empty());

    let c = s.counters();
    assert_eq!((c.permissions_asked, c.permissions_auto_allowed), (2, 1));
}

#[test]
fn a_denied_answer_never_becomes_a_rule() {
    let s = states();
    let (tx, rx) = oneshot::channel();
    s.open_permission("c", "c", "/repos/x", request("r", "Bash", "rm -rf /"), tx);
    let resolved = s
        .resolve_permission(
            "r",
            PermissionDecision::Deny,
            Some(&RememberRule {
                scope: RuleScope::Global,
                pattern: "*".into(),
                name: None,
            }),
        )
        .expect("resolves");
    assert!(resolved.created_rule.is_none());
    assert!(s.rules().list().is_empty());
    assert_eq!(rx.blocking_recv().unwrap(), PermissionDecision::Deny);
}

/// Deleting a chat with a permission still parked drops its responder. The
/// gate turns that into a Deny, so the tool does not run — the fail-closed
/// half of `PermissionGate::ask`.
#[test]
fn dropping_a_parked_permission_denies_it_rather_than_running_the_tool() {
    let s = states();
    let (tx, rx) = oneshot::channel();
    s.open_permission("c", "c", "/repos/x", request("r", "Bash", "rm -rf /"), tx);
    assert_eq!(s.needs_you().len(), 1);

    s.drop_chat("c");
    assert!(s.needs_you().is_empty());
    // The responder went with the chat, so the receiver sees a closed
    // channel — which the gate reads as Deny.
    assert!(rx.blocking_recv().is_err(), "the responder was dropped");
}

/// The card said "Bash: ls". A client that answers with pattern `rm*` is
/// asking for a rule that does not cover what the user approved, so the
/// whole answer is refused rather than half-applied.
#[test]
fn a_remember_pattern_that_misses_the_shown_command_refuses_the_answer() {
    let s = states();
    let (tx, rx) = oneshot::channel();
    s.open_permission("c", "c", "/repos/x", request("r", "Bash", "ls"), tx);
    let refused = s.resolve_permission(
        "r",
        PermissionDecision::Allow,
        Some(&RememberRule {
            scope: RuleScope::Global,
            pattern: "rm*".into(),
            name: None,
        }),
    );
    assert!(refused.is_err(), "the widened rule is refused");
    assert!(s.rules().list().is_empty(), "and no rule was stored");
    // The request stays parked, so nothing ran and the card is still there
    // for an honest answer.
    assert_eq!(s.needs_you().len(), 1);
    drop(rx);
}

#[test]
fn answering_an_unknown_permission_is_an_error_not_a_silent_success() {
    let s = states();
    assert!(
        s.resolve_permission("nope", PermissionDecision::Allow, None)
            .is_err()
    );
}

#[test]
fn questions_reach_the_inbox_with_their_options() {
    let s = states();
    s.note_session("chat-q", SessionStatus::Working);
    s.open_questions(
        "chat-q",
        "chat-q",
        "input-1",
        vec![question("q1", "Ship it?")],
    );
    assert_eq!(
        row(&s.states(), "chat-q").state,
        AgentState::NeedsYou {
            kind: NeedsYouKind::Question
        }
    );
    let inbox = s.needs_you();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].kind, NeedsYouKind::Question);
    assert_eq!(inbox[0].prompt, "Ship it?");
    assert_eq!(inbox[0].options, vec!["Yes".to_string(), "No".to_string()]);
    assert_eq!(inbox[0].id, "input-1:q1");

    s.close_questions("input-1");
    assert!(s.needs_you().is_empty());
}

#[test]
fn a_permission_outranks_a_question_on_the_same_row() {
    let s = states();
    s.open_questions("c", "c", "input-1", vec![question("q1", "Ship it?")]);
    let (tx, _rx) = oneshot::channel();
    s.open_permission("c", "c", "/repos/x", request("r", "Bash", "ls"), tx);
    assert_eq!(
        row(&s.states(), "c").state,
        AgentState::NeedsYou {
            kind: NeedsYouKind::Permission
        }
    );
    assert_eq!(s.needs_you().len(), 2);
}

#[test]
fn a_child_needing_you_rolls_up_the_parent_row() {
    let s = states();
    s.note_session("parent", SessionStatus::Working);
    let child = s.note_subagent("parent", "tool-9", Some("scan the repo"));
    assert_eq!(child, "parent:tool-9");

    let rows = s.states();
    assert_eq!(rows.len(), 2, "parent plus one child");
    assert_eq!(row(&rows, &child).parent_id.as_deref(), Some("parent"));
    assert_eq!(row(&rows, &child).label.as_deref(), Some("scan the repo"));
    assert_eq!(row(&rows, &child).chat_id, "parent");
    // A working child leaves the parent working, with no needs-you count.
    assert_eq!(row(&rows, "parent").rolled_up, AgentState::Working);
    assert_eq!(row(&rows, "parent").needs_you_children, 0);

    // The child asks. The parent's own state is untouched; the rolled-up one
    // is not (decision 15: "a child that needs you marks every ancestor").
    s.open_questions("parent", &child, "input-1", vec![question("q1", "Which?")]);
    let rows = s.states();
    assert_eq!(row(&rows, "parent").state, AgentState::Working);
    assert_eq!(
        row(&rows, "parent").rolled_up,
        AgentState::NeedsYou {
            kind: NeedsYouKind::Question
        }
    );
    assert_eq!(row(&rows, "parent").needs_you_children, 1);
    // Both rows now rank as needs-you, so their relative order is by
    // recency, not by depth — the rail nests from `parent_id`, not from
    // this list's order.
    assert!(rows.iter().all(|r| r.rolled_up.needs_you()));

    s.close_questions("input-1");
    assert_eq!(row(&s.states(), "parent").rolled_up, AgentState::Working);
    assert_eq!(row(&s.states(), "parent").needs_you_children, 0);
}

#[test]
fn a_grandchild_rolls_up_through_both_ancestors() {
    let s = states();
    s.note_session("root", SessionStatus::Working);
    let child = s.note_subagent("root", "t1", None);
    // A grandchild is a child of the child row, keyed under the same chat.
    let grandchild = s.note_subagent(&child, "t2", None);
    s.open_questions("root", &grandchild, "input-1", vec![question("q", "?")]);

    let rows = s.states();
    for ancestor in ["root", child.as_str()] {
        assert_eq!(
            row(&rows, ancestor).rolled_up,
            AgentState::NeedsYou {
                kind: NeedsYouKind::Question
            },
            "{ancestor} must show its grandchild's ask"
        );
        assert_eq!(row(&rows, ancestor).needs_you_children, 1);
    }
    assert_eq!(row(&rows, "root").state, AgentState::Working);
}

#[test]
fn registering_the_same_subagent_twice_updates_one_row() {
    let s = states();
    s.note_subagent("parent", "tool-9", None);
    s.note_subagent("parent", "tool-9", Some("named later"));
    let rows = s.states();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        row(&rows, "parent:tool-9").label.as_deref(),
        Some("named later")
    );
    assert_eq!(s.counters().children_registered, 1);
}

#[test]
fn a_subagent_done_event_settles_the_child_row() {
    let s = states();
    s.note_session("parent", SessionStatus::Working);
    s.note_subagent_event(
        "parent",
        "t1",
        &AgentEvent::Done {
            status: DoneStatus::Errored,
            result: None,
            error: Some("out of context".into()),
            session_id: None,
        },
    );
    let rows = s.states();
    assert_eq!(
        row(&rows, "parent:t1").state,
        AgentState::NeedsYou {
            kind: NeedsYouKind::Failed
        }
    );
    assert_eq!(row(&rows, "parent").needs_you_children, 1);
    let inbox = s.needs_you();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].prompt, "out of context");
    assert_eq!(inbox[0].chat_id, "parent", "the tap opens the parent chat");
}

#[test]
fn the_inbox_is_newest_first_across_chats() {
    let s = states();
    for (i, chat) in ["a", "b", "c"].iter().enumerate() {
        let (tx, _rx) = oneshot::channel();
        s.open_permission(
            chat,
            chat,
            "/repos/x",
            request(&format!("r{i}"), "Bash", "ls"),
            tx,
        );
        // The store stamps Utc::now(); a beat apart keeps the order readable.
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let inbox = s.needs_you();
    assert_eq!(inbox.len(), 3);
    assert_eq!(inbox[0].chat_id, "c");
    assert_eq!(inbox[2].chat_id, "a");
}

#[test]
fn dropping_a_chat_takes_its_children_and_its_asks_with_it() {
    let s = states();
    s.note_session("keep", SessionStatus::Working);
    s.note_session("go", SessionStatus::Working);
    s.note_subagent("go", "t1", None);
    let (tx, _rx) = oneshot::channel();
    s.open_permission("go", "go", "/repos/x", request("r", "Bash", "ls"), tx);
    s.open_questions("keep", "keep", "input-1", vec![question("q", "?")]);
    assert_eq!(s.states().len(), 3);

    s.drop_chat("go");
    let rows = s.states();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "keep");
    let inbox = s.needs_you();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].chat_id, "keep");
}

#[test]
fn the_watch_channel_pushes_every_change() {
    let s = states();
    let mut rx = s.watch_needs_you();
    assert!(rx.borrow_and_update().is_empty());
    let (tx, _keep) = oneshot::channel();
    s.open_permission("c", "c", "/repos/x", request("r", "Bash", "ls"), tx);
    assert!(rx.has_changed().unwrap());
    assert_eq!(rx.borrow_and_update().len(), 1);
}
