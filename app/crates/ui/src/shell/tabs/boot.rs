//! Initial conversation selection follows the saved project filter.
use crate::state::AppState;
use chrono::Utc;

/// The outer option distinguishes an unfinished boot from an empty landing.
/// Wait for both lists so a late project frame cannot change the decision.
pub(super) fn landing(state: &AppState, filter: Option<&str>) -> Option<Option<String>> {
    if !state.spaces_synced
        || !state.chats_synced
        || state.selected_chat.is_some()
        || state.auto_selected
    {
        return None;
    }
    // A removed project has the same All-projects fallback as the sidebar.
    let filter = filter.filter(|id| state.space_row(id).is_some());
    Some(
        state
            .sidebar_chats(Utc::now(), filter)
            .first()
            .map(|(_, chat)| chat.id.clone()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use surya_proto::{Chat, Space};

    fn state() -> AppState {
        let mut state = AppState::new();
        state.apply_spaces(
            ["a", "b"]
                .into_iter()
                .map(|id| Space {
                    id: id.into(),
                    device_id: "device".into(),
                    path: format!("/projects/{id}"),
                    name: None,
                    git_detected: false,
                    git_checked_at: None,
                    checkout_id: None,
                    created_at: Utc::now(),
                })
                .collect(),
        );
        state.apply_chats(vec![chat("a-chat", "a")]);
        state
    }

    fn chat(id: &str, project: &str) -> Chat {
        Chat {
            id: id.into(),
            device_id: "device".into(),
            space_id: Some(project.into()),
            title: None,
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
            last_seen_at: None,
            room_gen: None,
        }
    }

    #[test]
    fn relaunch_empty_project_does_not_open_another_projects_chat() {
        assert_eq!(landing(&state(), Some("b")), Some(None));
    }

    #[test]
    fn relaunch_project_uses_its_chat_even_when_another_is_newer() {
        let mut state = state();
        let b = chat("b-chat", "b");
        let mut a = chat("a-chat", "a");
        a.last_message_at = Some(Utc::now() + chrono::TimeDelta::minutes(1));
        state.apply_chats(vec![b, a]);
        assert_eq!(landing(&state, Some("b")), Some(Some("b-chat".into())));
        assert_eq!(landing(&state, None), Some(Some("a-chat".into())));
    }

    #[test]
    fn removed_filter_falls_back_to_all_projects() {
        assert_eq!(
            landing(&state(), Some("removed")),
            Some(Some("a-chat".into()))
        );
    }

    #[test]
    fn either_unsynced_list_defers_landing() {
        let mut state = state();
        state.spaces_synced = false;
        assert_eq!(landing(&state, Some("b")), None);
        state.spaces_synced = true;
        state.chats_synced = false;
        assert_eq!(landing(&state, Some("b")), None);
    }

    #[test]
    fn manual_selection_and_completed_empty_landing_are_preserved() {
        let mut state = state();
        state.selected_chat = Some("a-chat".into());
        assert_eq!(landing(&state, Some("b")), None);
        state.selected_chat = None;
        state.auto_selected = true;
        assert_eq!(landing(&state, None), None);
    }
}
