use super::*;
use chrono::Utc;

fn row(id: &str, parent: Option<&str>, state: AgentState) -> AgentStateRow {
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

#[test]
fn stopped_response_has_a_stopped_heading_not_waiting_for_you() {
    let sections = agent_sections(&[row("stopped", None, AgentState::Stopped)], &[]);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].group.heading(), "Stopped");
    assert_eq!(sections[0].rows[0].state, AgentState::Stopped);
}

#[test]
fn failures_are_stopped_but_questions_and_permissions_wait_for_answers() {
    for kind in [
        NeedsYouKind::Permission,
        NeedsYouKind::Question,
        NeedsYouKind::Failed,
    ] {
        let sections = agent_sections(&[row("agent", None, AgentState::NeedsYou { kind })], &[]);
        let expected = if kind == NeedsYouKind::Failed {
            "Stopped"
        } else {
            "Waiting for you"
        };
        assert_eq!(sections[0].group.heading(), expected);
    }
}

#[test]
fn attention_groups_still_precede_running_done_and_idle() {
    let rows = [
        row("idle", None, AgentState::Idle),
        row("done", None, AgentState::Done),
        row("working", None, AgentState::Working),
        row(
            "blocked",
            None,
            AgentState::NeedsYou {
                kind: NeedsYouKind::Permission,
            },
        ),
        row("stopped", None, AgentState::Stopped),
    ];
    let sections = agent_sections(&rows, &[]);
    let headings: Vec<_> = sections.iter().map(|s| s.group.heading()).collect();
    assert_eq!(
        headings,
        ["Waiting for you", "Stopped", "Running", "Done", "Idle"]
    );
    assert_eq!(sections[0].rows.len(), 1);
    assert_eq!(sections[1].rows.len(), 1);
}

#[test]
fn stopped_descendants_remain_nested_under_their_parent() {
    let mut parent = row("parent", None, AgentState::Working);
    parent.rolled_up = AgentState::Stopped;
    let child = row("child", Some("parent"), AgentState::Stopped);
    let sections = agent_sections(&[parent, child], &[]);
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].group.heading(), "Stopped");
    assert_eq!(
        sections[0].rows.iter().map(|r| r.depth).collect::<Vec<_>>(),
        [0, 1]
    );
    assert!(sections[0].rows[0].rolled_up_only);
}
