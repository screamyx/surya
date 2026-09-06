//! The agent list: every agent grouped by state, children nested under their
//! spawner, with the roll-up badge a parent shows for a blocked descendant.
//!
//! Decision 15: agents form a tree, a child's needs-you marks every ancestor,
//! and rows sort needs-you, working, done, idle. Clicking a row emits
//! [`SelectChat`]; the shell owns routing, not this pane.

use std::collections::HashSet;

use gpui::{Context, Entity, Render, SharedString, Task, Window, div, prelude::*, px};
use surya_proto::{AgentStateRow, Chat};
use surya_rpc::methods;

use crate::inbox::chrome::{badge, dot, empty_state, heading, state_color};
use crate::inbox::model::{AgentRow, AgentSection, agent_sections};
use crate::inbox::watch::spawn_engine_watch;
use crate::state::AppState;
use crate::theme::Theme;
use crate::typography::ui_rems;

/// Emitted when a row is clicked. The chat id, not the agent id: tapping a
/// subagent opens the chat it lives in, which is where its transcript is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectChat(pub String);

impl gpui::EventEmitter<SelectChat> for AgentsRail {}

pub struct AgentsRail {
    /// `None` in the demo and in tests.
    state: Option<Entity<AppState>>,
    rows: Vec<AgentStateRow>,
    chats: Vec<Chat>,
    selected: Option<String>,
    _watch: Option<Task<()>>,
    _chats_watch: Option<Task<()>>,
}

impl AgentsRail {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let mut rail = Self {
            state: Some(state),
            rows: Vec::new(),
            chats: Vec::new(),
            selected: None,
            _watch: None,
            _chats_watch: None,
        };
        rail.start_watches(cx);
        rail
    }

    /// Fixtures instead of an engine — the demo entry and the tests.
    pub fn demo(rows: Vec<AgentStateRow>, chats: Vec<Chat>) -> Self {
        Self {
            state: None,
            rows,
            chats,
            selected: None,
            _watch: None,
            _chats_watch: None,
        }
    }

    pub fn set_rows(&mut self, rows: Vec<AgentStateRow>, chats: Vec<Chat>) {
        self.rows = rows;
        self.chats = chats;
    }

    /// The rows whose chat this client still has.
    ///
    /// A removed project takes its chats with it, and a row nobody can open
    /// is worse than no row: with the chat gone there is no title left, so it
    /// draws as "Untitled chat" under a heading that says it wants you, and
    /// clicking it does nothing. The engine now drops these on its side too
    /// (`deleteSpace` in `engine/src/rpc.rs`); this is the client half, so a
    /// row cannot outlive its chat whatever the feed says.
    ///
    /// `chats` is the chat feed's own mirror, so "missing" means the chat is
    /// really gone, not that the name is unknown - `agent_sections` still
    /// falls back to "Untitled chat" for a chat it has no title for.
    fn live_rows(&self) -> Vec<AgentStateRow> {
        let known: HashSet<&str> = self.chats.iter().map(|chat| chat.id.as_str()).collect();
        self.rows
            .iter()
            .filter(|row| known.contains(row.chat_id.as_str()))
            .cloned()
            .collect()
    }

    pub fn sections(&self) -> Vec<AgentSection> {
        agent_sections(&self.live_rows(), &self.chats)
    }

    /// How many rows want the user, counting descendants once each — the
    /// number a collapsed workspace row shows (decision 15).
    ///
    /// Through [`Self::live_rows`] as well: `Stopped` counts as needs-you
    /// (`AgentState::needs_you`), and a run the `deleteSpace` cascade
    /// interrupted ends up exactly there, so a ghost row kept the badge red
    /// as well as the rail.
    pub fn needs_you_count(&self) -> usize {
        self.live_rows()
            .iter()
            .filter(|row| row.state.needs_you())
            .count()
    }

    pub fn set_selected(&mut self, chat_id: Option<String>) {
        self.selected = chat_id;
    }

    fn start_watches(&mut self, cx: &mut Context<Self>) {
        // `None` in the demo and in tests: no engine to watch.
        let Some(state) = self.state.clone() else {
            return;
        };
        self._watch = Some(spawn_engine_watch(
            cx,
            state.clone(),
            methods::WATCH_AGENT_STATES,
            |rail: &mut Self, rows: Vec<AgentStateRow>, cx| {
                rail.rows = rows;
                cx.notify();
            },
        ));
        self._chats_watch = Some(spawn_engine_watch(
            cx,
            state,
            methods::WATCH_CHATS,
            |rail: &mut Self, chats: Vec<Chat>, cx| {
                rail.chats = chats;
                cx.notify();
            },
        ));
    }

    fn render_row(&self, row: &AgentRow, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = Theme::of(cx);
        let selected = self.selected.as_deref() == Some(row.chat_id.as_str());
        let chat_id = row.chat_id.clone();
        div()
            .id(SharedString::from(format!("agent-row-{}", row.id)))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            // Children step in; the indent IS the tree.
            .pl(px(6.0 + row.depth as f32 * 14.0))
            .pr(px(6.0))
            .py(px(5.0))
            .rounded(px(7.0))
            .cursor_pointer()
            .when(selected, |el| el.bg(theme.element_active))
            .when(!selected, |el| el.hover(|s| s.bg(theme.element_hover)))
            .child(dot(state_color(row.state, theme)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .whitespace_nowrap()
                    .text_size(ui_rems(13.0))
                    .text_color(if selected { theme.text } else { theme.text_muted })
                    .child(SharedString::from(row.name.clone())),
            )
            // The roll-up badge: this row is calm, something under it is not.
            .when(row.rolled_up_only, |el| {
                el.child(badge(
                    theme,
                    SharedString::from(row.needs_you_children.to_string()),
                    state_color(row.rolled_up, theme),
                ))
            })
            .on_click(cx.listener(move |_, _, _, cx| {
                cx.emit(SelectChat(chat_id.clone()));
            }))
            .into_any_element()
    }
}

impl Render for AgentsRail {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sections = self.sections();
        // Built before the container: `render_row` needs `&mut cx` for its
        // click listeners, and `Theme::of(cx)` holds a shared borrow, so the
        // two cannot be live inside one `.children()` closure.
        let groups: Vec<gpui::AnyElement> = sections
            .into_iter()
            .map(|section| {
                let heading_text = section.group.heading();
                let rows: Vec<gpui::AnyElement> = section
                    .rows
                    .iter()
                    .map(|row| self.render_row(row, cx))
                    .collect();
                let theme = Theme::of(cx);
                div()
                    .flex()
                    .flex_col()
                    .child(heading(theme, heading_text))
                    .children(rows)
                    .into_any_element()
            })
            .collect();
        let empty = groups.is_empty();
        let theme = Theme::of(cx);
        div()
            .id("agents-rail")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .p(px(8.0))
            .when(empty, |el| {
                el.child(empty_state(theme, "No agents yet."))
            })
            .children(groups)
    }
}

#[cfg(test)]
mod tests {
    use surya_proto::{AgentState, AgentStateRow};

    use super::AgentsRail;
    use crate::inbox::demo;

    fn row(chat_id: &str) -> AgentStateRow {
        AgentStateRow {
            id: chat_id.into(),
            chat_id: chat_id.into(),
            parent_id: None,
            label: None,
            state: AgentState::Stopped,
            rolled_up: AgentState::Stopped,
            needs_you_children: 0,
            updated_at: chrono::Utc::now(),
        }
    }

    /// E2E-NAV-02: a row whose chat is gone must not reach the rail. Removing
    /// a project left one behind, drawn as "Untitled chat" under "Waiting for
    /// you", with nothing to open and no way to dismiss it.
    #[test]
    fn a_row_whose_chat_is_gone_does_not_draw() {
        let rail = AgentsRail::demo(vec![row("chat-removed")], demo::chats());
        assert!(
            rail.sections().is_empty(),
            "a chat the client does not have must not draw: {:?}",
            rail.sections()
        );
        // Stopped counts as needs-you, so the same ghost also kept the
        // collapsed workspace row's badge red.
        assert_eq!(rail.needs_you_count(), 0, "and it must not count either");
    }

    #[test]
    fn rows_whose_chats_are_present_still_draw() {
        let rail = AgentsRail::demo(demo::agent_states(), demo::chats());
        assert!(!rail.sections().is_empty(), "the demo rail still draws");
    }
}
