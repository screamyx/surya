//! Model tests for the inbox and the agent tree. These carry the seat's
//! proof counters; run with `--nocapture` to read them.

use super::*;
use chrono::{TimeZone, Utc};
use zeron_proto::child_agent_id;

fn chat(id: &str, title: Option<&str>) -> Chat {
    Chat {
        id: id.into(),
        device_id: "dev".into(),
        title: title.map(str::to_string),
        archived: false,
        cwd: None,
        branch: None,
        checkout_id: None,
        source_context: None,
        config: None,
        last_message_preview: None,
        last_message_at: None,
        created_at: Utc::now(),
        harness_session_id: None,
        harness_session_cwd: None,
        space_id: None,
        last_seen_at: None,
        room_gen: None,
    }
}

fn item(id: &str, chat_id: &str, kind: NeedsYouKind) -> NeedsYouItem {
    NeedsYouItem {
        id: id.into(),
        chat_id: chat_id.into(),
        agent_id: chat_id.into(),
        kind,
        title: "Allow Bash?".into(),
        prompt: "php artisan migrate".into(),
        options: Vec::new(),
        multi_select: false,
        tool_name: Some("Bash".into()),
        tool_command: Some("php artisan migrate".into()),
        retryable: false,
        created_at: Utc.timestamp_opt(1_757_000_000, 0).unwrap(),
    }
}

fn state_row(id: &str, parent: Option<&str>, state: AgentState) -> AgentStateRow {
    AgentStateRow {
        id: id.into(),
        parent_id: parent.map(str::to_string),
        chat_id: parent.unwrap_or(id).into(),
        state,
        rolled_up: state,
        needs_you_children: 0,
        label: None,
        updated_at: Utc::now(),
    }
}

const NEEDS_PERMISSION: AgentState = AgentState::NeedsYou {
    kind: NeedsYouKind::Permission,
};
const NEEDS_QUESTION: AgentState = AgentState::NeedsYou {
    kind: NeedsYouKind::Question,
};

#[test]
fn every_pending_item_becomes_a_row_in_the_order_the_engine_sent() {
    let items = [
        item("perm-1", "chat-a", NeedsYouKind::Permission),
        item("input-1:q1", "chat-b", NeedsYouKind::Question),
        item("chat-c:failure", "chat-c", NeedsYouKind::Failed),
    ];
    let chats = [
        chat("chat-a", Some("Migrate the database")),
        chat("chat-b", Some("Rewrite the fold")),
        chat("chat-c", None),
    ];
    let rows = inbox_rows(&items, &chats);
    println!("items={} shown={}", items.len(), rows.len());
    assert_eq!(rows.len(), 3, "items=3 shown=3");

    assert_eq!(rows[0].chat_name, "Migrate the database");
    assert_eq!(rows[0].badge, "Permission");
    assert_eq!(rows[0].tool_command.as_deref(), Some("php artisan migrate"));
    assert_eq!(rows[1].badge, "Question");
    // Decision 17 shows a failure as "Stopped", and an unnamed chat says so
    // rather than printing its id at the user.
    assert_eq!(rows[2].badge, "Stopped");
    assert_eq!(rows[2].chat_name, UNTITLED_CHAT);
    // The engine's newest-first order is kept verbatim.
    assert_eq!(
        rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["perm-1", "input-1:q1", "chat-c:failure"]
    );
}

#[test]
fn a_blank_title_reads_as_untitled_not_as_an_empty_row() {
    let items = [item("perm-1", "chat-a", NeedsYouKind::Permission)];
    let rows = inbox_rows(&items, &[chat("chat-a", Some("   "))]);
    assert_eq!(rows[0].chat_name, UNTITLED_CHAT);
}

/// Answering removes the row. The list is a projection of the engine's
/// stream, so "removed" means the next frame no longer carries the item —
/// this is the projection doing its half.
#[test]
fn answering_one_item_leaves_the_rest() {
    let items = [
        item("perm-1", "chat-a", NeedsYouKind::Permission),
        item("input-1:q1", "chat-b", NeedsYouKind::Question),
        item("chat-c:failure", "chat-c", NeedsYouKind::Failed),
    ];
    let before = inbox_rows(&items, &[]);
    let after_frame: Vec<NeedsYouItem> = items
        .iter()
        .filter(|i| i.id != "perm-1")
        .cloned()
        .collect();
    let after = inbox_rows(&after_frame, &[]);
    println!(
        "before={} answered=1 remaining={}",
        before.len(),
        after.len()
    );
    assert_eq!((before.len(), after.len()), (3, 2), "answered=1 remaining=2");
    assert!(after.iter().all(|r| r.id != "perm-1"));
}

#[test]
fn always_allow_sends_remember_with_the_chosen_scope() {
    let rows = inbox_rows(&[item("perm-1", "chat-a", NeedsYouKind::Permission)], &[]);
    let row = &rows[0];

    let workspace = remember_for(row, AlwaysAllowScope::ThisWorkspace);
    let everywhere = remember_for(row, AlwaysAllowScope::Everywhere);
    println!(
        "asked=2 remembered=2 scopes={:?}/{:?}",
        workspace.scope, everywhere.scope
    );
    assert_eq!(workspace.scope, RuleScope::Workspace);
    assert_eq!(everywhere.scope, RuleScope::Global);
    // EMPTY, which the engine reads as "pin this exact command" and stores
    // literally. Sending the command as a pattern would store it as a glob:
    // `rm -rf build/*` would become a standing wildcard rule, and `git *`
    // would be refused by the anchoring check so the click would just error.
    assert_eq!(workspace.pattern, "");
    assert_eq!(everywhere.pattern, "");

    let params = respond_permission_params("perm-1", PermissionDecision::Allow, Some(&workspace));
    assert_eq!(params["requestId"], "perm-1");
    assert_eq!(params["decision"], "allow");
    assert_eq!(params["remember"]["scope"], "workspace");
    assert_eq!(params["remember"]["pattern"], "");
}

#[test]
fn allow_and_deny_without_remember_send_no_rule() {
    for decision in [PermissionDecision::Allow, PermissionDecision::Deny] {
        let params = respond_permission_params("perm-1", decision, None);
        assert!(
            params.get("remember").is_none(),
            "a plain answer never carries a rule"
        );
    }
    assert_eq!(
        respond_permission_params("perm-1", PermissionDecision::Deny, None)["decision"],
        "deny"
    );
}

#[test]
fn the_scope_toggle_has_two_positions_and_says_which() {
    let scope = AlwaysAllowScope::default();
    assert_eq!(scope, AlwaysAllowScope::ThisWorkspace);
    assert_eq!(scope.label(), "this workspace");
    assert_eq!(scope.toggled(), AlwaysAllowScope::Everywhere);
    assert_eq!(scope.toggled().label(), "everywhere");
    assert_eq!(scope.toggled().toggled(), scope);
}

#[test]
fn a_question_row_id_carries_both_halves_of_the_answer() {
    let (request, question) = split_question_id("input-1:q1").expect("splits");
    assert_eq!((request, question), ("input-1", "q1"));
    assert!(split_question_id("no-colon").is_none());

    let params = respond_input_params(
        "chat-b",
        request,
        vec![UserInputAnswer {
            question_id: question.into(),
            labels: vec!["Event-driven".into()],
        }],
    )
    .expect("serializes");
    assert_eq!(params["chatId"], "chat-b");
    // The same RespondInput doc command the composer's question panel
    // sends: tagged `kind`, as SessionCommandPayload spells it.
    assert_eq!(params["command"]["kind"], "respondInput");
    assert_eq!(params["command"]["requestId"], "input-1");
    assert_eq!(params["command"]["answers"][0]["labels"][0], "Event-driven");
}

#[test]
fn a_multi_select_question_carries_every_picked_label() {
    let mut raw = item("input-2:q-gates", "chat-b", NeedsYouKind::Question);
    raw.multi_select = true;
    raw.options = vec!["Unit tests".into(), "End-to-end".into()];
    let rows = inbox_rows(&[raw], &[]);
    assert!(rows[0].multi_select, "the view needs to know to accumulate");

    let (request, question_id) = split_question_id(&rows[0].id).expect("splits");
    let params = respond_input_params(
        "chat-b",
        request,
        vec![UserInputAnswer {
            question_id: question_id.into(),
            labels: vec!["Unit tests".into(), "End-to-end".into()],
        }],
    )
    .expect("serializes");
    let labels = &params["command"]["answers"][0]["labels"];
    assert_eq!(labels.as_array().map(Vec::len), Some(2));
    assert_eq!(labels[0], "Unit tests");
    assert_eq!(labels[1], "End-to-end");
}

#[test]
fn a_child_nests_under_its_parent_and_rides_its_parent_group() {
    let child_id = child_agent_id("chat-1", "tool-9");
    let mut parent = state_row("chat-1", None, AgentState::Working);
    parent.rolled_up = NEEDS_QUESTION;
    parent.needs_you_children = 1;
    let mut child = state_row(&child_id, Some("chat-1"), NEEDS_QUESTION);
    child.label = Some("scan the repo".into());

    let sections = agent_sections(&[parent, child], &[chat("chat-1", Some("Fold rewrite"))]);
    let children = sections
        .iter()
        .flat_map(|s| &s.rows)
        .filter(|r| r.depth > 0)
        .count();
    let nested = sections
        .iter()
        .flat_map(|s| s.rows.windows(2))
        .filter(|w| w[1].depth == w[0].depth + 1)
        .count();
    println!("children={children} nested={nested}");
    assert_eq!((children, nested), (1, 1), "children=1 nested=1");

    // The parent is calm itself but grouped by its roll-up, so the user sees
    // it without unfolding anything.
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].group, AgentGroup::WaitingForYou);
    assert_eq!(sections[0].group.heading(), "Waiting for you");
    let rows = &sections[0].rows;
    assert_eq!(rows[0].name, "Fold rewrite");
    assert_eq!(rows[0].depth, 0);
    assert!(rows[0].rolled_up_only, "the badge case: calm row, loud child");
    assert_eq!(rows[0].needs_you_children, 1);
    // The child is named by its spawn description, and sits one level in.
    assert_eq!(rows[1].name, "scan the repo");
    assert_eq!(rows[1].depth, 1);
    assert!(!rows[1].rolled_up_only, "the child is loud on its own");
}

#[test]
fn a_grandchild_keeps_going_deeper_and_stays_in_one_group() {
    let child = child_agent_id("chat-1", "t1");
    let grandchild = child_agent_id(&child, "t2");
    let mut parent = state_row("chat-1", None, AgentState::Working);
    parent.rolled_up = NEEDS_QUESTION;
    let mut mid = state_row(&child, Some("chat-1"), AgentState::Working);
    mid.rolled_up = NEEDS_QUESTION;
    let deep = state_row(&grandchild, Some(&child), NEEDS_QUESTION);

    let sections = agent_sections(&[parent, mid, deep], &[]);
    assert_eq!(sections.len(), 1, "one tree, one group");
    let depths: Vec<usize> = sections[0].rows.iter().map(|r| r.depth).collect();
    assert_eq!(depths, vec![0, 1, 2]);
}

#[test]
fn the_four_groups_appear_in_the_decision_15_order() {
    let rows = [
        state_row("idle", None, AgentState::Idle),
        state_row("done", None, AgentState::Done),
        state_row("working", None, AgentState::Working),
        state_row("blocked", None, NEEDS_PERMISSION),
        state_row("stopped", None, AgentState::Stopped),
    ];
    let sections = agent_sections(&rows, &[]);
    let headings: Vec<&str> = sections.iter().map(|s| s.group.heading()).collect();
    assert_eq!(headings, ["Waiting for you", "Running", "Done", "Idle"]);
    // Stopped shares the waiting group: it wants a decision too (decision 17).
    assert_eq!(sections[0].rows.len(), 2);
}

#[test]
fn an_empty_group_is_not_drawn() {
    let sections = agent_sections(&[state_row("a", None, AgentState::Working)], &[]);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].group, AgentGroup::Running);
}

#[test]
fn nothing_pending_is_an_empty_list_not_a_panic() {
    assert!(inbox_rows(&[], &[]).is_empty());
    assert!(agent_sections(&[], &[]).is_empty());
}

/// A child whose parent row never arrived must not vanish. The engine always
/// creates the parent first, but a dropped frame should cost one row's
/// nesting, not the row.
#[test]
fn an_orphan_child_is_dropped_rather_than_drawn_at_the_root() {
    let orphan = state_row("chat-9:tool-1", Some("chat-9"), NEEDS_QUESTION);
    let sections = agent_sections(&[orphan], &[]);
    assert!(
        sections.is_empty(),
        "no parent means no tree to hang it under; the next frame carries both"
    );
}

/// Round 4, I2: the open chat's question is already answerable in its sheet,
/// so the card must not offer the same options a second time. Every other row
/// keeps its full card - there is nowhere else to answer those.
#[test]
fn only_the_open_chat_s_question_is_the_one_drawn_twice() {
    let items = [
        item("q-open", "chat-1", NeedsYouKind::Question),
        item("q-other", "chat-2", NeedsYouKind::Question),
        item("p-open", "chat-1", NeedsYouKind::Permission),
        item("f-open", "chat-1", NeedsYouKind::Failed),
    ];
    let rows = inbox_rows(&items, &[]);
    let collapsed: Vec<&str> = rows
        .iter()
        .filter(|row| answered_in_the_open_chat(row, Some("chat-1")))
        .map(|row| row.id.as_str())
        .collect();
    assert_eq!(
        collapsed,
        vec!["q-open"],
        "only the question whose sheet is on screen collapses"
    );
    for row in &rows {
        assert!(
            !answered_in_the_open_chat(row, None),
            "with no chat open nothing is duplicated: {}",
            row.id
        );
    }
}

/// Round 4, I3: the engine fills a question's title from the model's own
/// header, and a model that answers "Question" leaves the card saying it
/// twice - once in the badge, once in bold under it.
#[test]
fn a_title_that_only_repeats_the_badge_is_not_worth_drawing() {
    let mut items = [item("q-1", "chat-1", NeedsYouKind::Question)];
    items[0].title = "Question".into();
    let rows = inbox_rows(&items, &[]);
    assert!(!title_adds_to_badge(&rows[0]));

    items[0].title = "  question  ".into();
    let rows = inbox_rows(&items, &[]);
    assert!(
        !title_adds_to_badge(&rows[0]),
        "spacing and case are not meaning"
    );

    items[0].title = String::new();
    let rows = inbox_rows(&items, &[]);
    assert!(!title_adds_to_badge(&rows[0]), "an empty title says nothing");

    items[0].title = "Which sync strategy?".into();
    let rows = inbox_rows(&items, &[]);
    assert!(
        title_adds_to_badge(&rows[0]),
        "a real header still gets its line"
    );
}
