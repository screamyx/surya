//! Carry spawn descriptions into the live agent tree.

use super::AgentStates;
use surya_proto::ToolCall;

impl AgentStates {
    /// Register the same spawn the transcript chip displays. Tagged child
    /// events do not carry its description, so capture it on the parent call.
    pub(crate) fn note_spawn(&self, chat_id: &str, tool_id: &str, call: &ToolCall) {
        if !call.is_subagent_spawn() {
            return;
        }
        let (name, input) = match call {
            ToolCall::Unknown { name, input } => (name.as_str(), input),
            ToolCall::Mcp { tool, input, .. } => (tool.as_str(), input),
            _ => return,
        };
        let label = name
            .strip_prefix("Agent: ")
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .or_else(|| {
                input
                    .as_ref()?
                    .get("description")?
                    .as_str()
                    .map(str::trim)
                    .filter(|label| !label.is_empty())
            });
        self.note_subagent(chat_id, tool_id, label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::AllowRules;
    use surya_proto::{AgentEvent, AgentState, DoneStatus};

    fn store() -> AgentStates {
        AgentStates::new(AllowRules::open(tempfile::tempdir().unwrap().keep()))
    }

    fn spawn(description: &str) -> ToolCall {
        ToolCall::Unknown {
            name: format!("Agent: {description}"),
            input: None,
        }
    }

    #[test]
    fn two_spawn_names_remain_distinct_while_running_and_after_done() {
        let states = store();
        for (id, label) in [
            ("first", "Audit the fold path"),
            ("second", "Verify the commit cadence"),
        ] {
            states.note_spawn("parent", id, &spawn(label));
            states.note_subagent_event(
                "parent",
                id,
                &AgentEvent::TextDelta {
                    text: "Working".into(),
                },
            );
        }
        let children = || {
            states
                .states()
                .into_iter()
                .filter(|row| row.parent_id.is_some())
                .collect::<Vec<_>>()
        };
        let rows = children();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.state == AgentState::Working));
        for label in ["Audit the fold path", "Verify the commit cadence"] {
            assert!(rows.iter().any(|row| row.label.as_deref() == Some(label)));
        }
        for id in ["first", "second"] {
            states.note_subagent_event(
                "parent",
                id,
                &AgentEvent::Done {
                    status: DoneStatus::Completed,
                    result: None,
                    error: None,
                    session_id: None,
                },
            );
        }
        let done = children();
        assert!(done.iter().all(|row| row.state == AgentState::Done));
        assert!(done.iter().all(|row| {
            rows.iter()
                .any(|old| old.id == row.id && old.label == row.label)
        }));
    }

    #[test]
    fn late_spawn_description_names_existing_child_without_restarting_it() {
        let states = store();
        states.note_subagent_event(
            "parent",
            "first",
            &AgentEvent::Done {
                status: DoneStatus::Completed,
                result: None,
                error: None,
                session_id: None,
            },
        );
        states.note_spawn("parent", "first", &spawn("Audit"));
        let rows = states.states();
        let child = rows.iter().find(|row| row.parent_id.is_some()).unwrap();
        assert_eq!(child.label.as_deref(), Some("Audit"));
        assert_eq!(child.state, AgentState::Done);
    }

    #[test]
    fn bare_agent_uses_description_but_ordinary_tools_never_create_children() {
        let states = store();
        for (id, name) in [("child", "Agent"), ("other", "Read")] {
            states.note_spawn(
                "parent",
                id,
                &ToolCall::Unknown {
                    name: name.into(),
                    input: Some(serde_json::json!({"description": "Inspect files"})),
                },
            );
        }
        let rows = states.states();
        let children = rows
            .iter()
            .filter(|row| row.parent_id.is_some())
            .collect::<Vec<_>>();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].label.as_deref(), Some("Inspect files"));
    }
}
