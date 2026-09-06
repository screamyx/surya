//! The needs-you inbox, the agent tree, and the approval-policy page.
//!
//! Decision 20 makes the needs-you queue the spine of the app: what needs you
//! at full weight, then what is running, then what is done. These are the
//! views for the data the engine already publishes (`WatchNeedsYou`,
//! `WatchAgentStates`, `ListAllowRules`).
//!
//! Self-contained on purpose. Nothing here touches `shell.rs` — the panes
//! emit [`needs_you::OpenChat`] and [`agents::SelectChat`] and let whoever
//! mounts them decide what that means. Wave 3 does the mounting.

pub mod agents;
pub mod chrome;
pub mod demo_run;
pub mod model;
pub mod needs_you;
pub mod rules;
pub(crate) mod watch;

pub use agents::{AgentsRail, SelectChat};
pub use needs_you::{NeedsYouPane, OpenChat};
pub use rules::RulesPane;

use gpui::{Context, Entity, Render, Window, div, prelude::*, px};

use crate::state::AppState;
use crate::theme::Theme;

/// The inbox surface: the needs-you list with the agent tree beside it.
///
/// One entity so a caller mounts a single thing; the two panes stay separate
/// entities so either can be mounted alone (the rail belongs in the sidebar,
/// the list in the main feed).
pub struct InboxPane {
    needs_you: Entity<NeedsYouPane>,
    agents: Entity<AgentsRail>,
}

impl InboxPane {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let needs_you = cx.new(|cx| NeedsYouPane::new(state.clone(), cx));
        let agents = cx.new(|cx| AgentsRail::new(state, cx));
        Self { needs_you, agents }
    }

    pub fn needs_you(&self) -> &Entity<NeedsYouPane> {
        &self.needs_you
    }

    pub fn agents(&self) -> &Entity<AgentsRail> {
        &self.agents
    }
}

impl Render for InboxPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx);
        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(theme.bg)
            .child(
                div()
                    .w(px(260.0))
                    .h_full()
                    .flex_none()
                    .border_r_1()
                    .border_color(theme.border)
                    .child(self.agents.clone()),
            )
            .child(div().flex_1().min_w_0().h_full().child(self.needs_you.clone()))
    }
}

/// Sample rows for `--inbox-demo`: one permission, one question, one stopped
/// run, and a parent whose child is the one asking.
///
/// Made-up work in a made-up repo (decision 18: nothing in this repo is
/// specific to one person or one network).
pub mod demo {
    use chrono::{TimeZone, Utc};
    use surya_proto::{
        AgentState, AgentStateRow, AllowRule, Chat, NeedsYouItem, NeedsYouKind, RuleScope,
        child_agent_id,
    };

    const CWD: &str = "/repos/orchard";

    fn at(offset: i64) -> chrono::DateTime<Utc> {
        Utc.timestamp_opt(1_757_000_000 + offset, 0).unwrap()
    }

    fn chat(id: &str, title: &str) -> Chat {
        Chat {
            id: id.into(),
            device_id: "demo-device".into(),
            title: Some(title.into()),
            archived: false,
            cwd: Some(CWD.into()),
            branch: None,
            checkout_id: None,
            source_context: None,
            config: None,
            last_message_preview: None,
            last_message_at: None,
            created_at: at(0),
            harness_session_id: None,
            harness_session_cwd: None,
            space_id: None,
            last_seen_at: None,
            room_gen: None,
        }
    }

    pub fn chats() -> Vec<Chat> {
        vec![
            chat("chat-migrate", "Add the follow-up column"),
            chat("chat-fold", "Rewrite the fold"),
            chat("chat-import", "Import last season's rows"),
        ]
    }

    pub fn needs_you() -> Vec<NeedsYouItem> {
        vec![
            NeedsYouItem {
                id: "perm-1".into(),
                chat_id: "chat-migrate".into(),
                agent_id: "chat-migrate".into(),
                kind: NeedsYouKind::Permission,
                title: "Allow Bash?".into(),
                prompt: "php artisan migrate".into(),
                options: Vec::new(),
                multi_select: false,
                tool_name: Some("Bash".into()),
                tool_command: Some(
                    "php artisan migrate --seed --force --database=orchard --path=database/migrations/2026_09_05_follow_up_column.php".into(),
                ),
                retryable: false,
                created_at: at(300),
            },
            NeedsYouItem {
                id: "input-1:q-sync".into(),
                chat_id: "chat-fold".into(),
                agent_id: child_agent_id("chat-fold", "tool-9"),
                kind: NeedsYouKind::Question,
                title: "Question".into(),
                prompt: "Which sync strategy should the rewrite use?".into(),
                options: vec![
                    "Event-driven fold".into(),
                    "Poll every 120ms".into(),
                    "Hybrid, with a polling fallback".into(),
                ],
                multi_select: false,
                tool_name: None,
                tool_command: None,
                retryable: false,
                created_at: at(200),
            },
            NeedsYouItem {
                id: "input-2:q-gates".into(),
                chat_id: "chat-fold".into(),
                agent_id: "chat-fold".into(),
                kind: NeedsYouKind::Question,
                title: "Question".into(),
                prompt: "Which suites should gate the merge?".into(),
                options: vec![
                    "Unit tests".into(),
                    "End-to-end".into(),
                    "Golden screenshots".into(),
                ],
                multi_select: true,
                tool_name: None,
                tool_command: None,
                retryable: false,
                created_at: at(150),
            },
            NeedsYouItem {
                id: "chat-import:failure".into(),
                chat_id: "chat-import".into(),
                agent_id: "chat-import".into(),
                kind: NeedsYouKind::Failed,
                title: "The run failed".into(),
                prompt: "ran out of context".into(),
                options: Vec::new(),
                multi_select: false,
                tool_name: None,
                tool_command: None,
                retryable: true,
                created_at: at(100),
            },
        ]
    }

    pub fn agent_states() -> Vec<AgentStateRow> {
        let child = child_agent_id("chat-fold", "tool-9");
        let needs_question = AgentState::NeedsYou {
            kind: NeedsYouKind::Question,
        };
        vec![
            AgentStateRow {
                id: "chat-migrate".into(),
                parent_id: None,
                chat_id: "chat-migrate".into(),
                state: AgentState::NeedsYou {
                    kind: NeedsYouKind::Permission,
                },
                rolled_up: AgentState::NeedsYou {
                    kind: NeedsYouKind::Permission,
                },
                needs_you_children: 0,
                label: None,
                updated_at: at(300),
            },
            // The roll-up case: the parent is working, its child is blocked.
            AgentStateRow {
                id: "chat-fold".into(),
                parent_id: None,
                chat_id: "chat-fold".into(),
                state: AgentState::Working,
                rolled_up: needs_question,
                needs_you_children: 1,
                label: None,
                updated_at: at(250),
            },
            AgentStateRow {
                id: child,
                parent_id: Some("chat-fold".into()),
                chat_id: "chat-fold".into(),
                state: needs_question,
                rolled_up: needs_question,
                needs_you_children: 0,
                label: Some("scan the fold call sites".into()),
                updated_at: at(200),
            },
            AgentStateRow {
                id: "chat-import".into(),
                parent_id: None,
                chat_id: "chat-import".into(),
                state: AgentState::Stopped,
                rolled_up: AgentState::Stopped,
                needs_you_children: 0,
                label: None,
                updated_at: at(100),
            },
        ]
    }

    pub fn rules() -> Vec<AllowRule> {
        vec![
            AllowRule {
                id: "rule-1".into(),
                name: "Bash php artisan migrate* in orchard".into(),
                scope: RuleScope::Workspace,
                workspace_path: Some(CWD.into()),
                tool_name: "Bash".into(),
                pattern: "php artisan migrate*".into(),
                exact: false,
                created_at: at(0),
            },
            AllowRule {
                id: "rule-2".into(),
                name: "read-only git".into(),
                scope: RuleScope::Global,
                workspace_path: None,
                tool_name: "Bash".into(),
                pattern: "git status*".into(),
                exact: false,
                created_at: at(0),
            },
        ]
    }
}
