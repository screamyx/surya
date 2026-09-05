//! The needs-you list: everything waiting on the user, newest first.
//!
//! Decision 20 makes this the spine of the app. A row is answerable in place
//! — a permission takes Allow, Deny, or Always allow with a scope; a question
//! shows its options as buttons; a stopped run offers Open chat. Answering
//! sends the command and the engine's next `WatchNeedsYou` frame drops the
//! row, so the list never has to guess what it did.

use std::collections::HashSet;

use gpui::{Context, Entity, Render, SharedString, Task, Window, div, prelude::*, px};
use zeron_proto::{Chat, NeedsYouItem, NeedsYouKind, PermissionDecision, UserInputAnswer};
use zeron_rpc::methods;

use crate::inbox::chrome::{
    ButtonTone, badge, body_text, button, command_text, empty_state, hint_chip, kind_color,
    row_card, setting_chip,
};
use crate::inbox::model::{
    AlwaysAllowScope, InboxRow, inbox_rows, remember_for, respond_input_params,
    respond_permission_params, split_question_id,
};
use crate::state::AppState;
use crate::theme::Theme;
use crate::typography::ui_rems;

/// What the pane emits when a row is tapped. The pane never navigates
/// itself: the shell owns routing, and wave 3 wires this up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenChat(pub String);

impl gpui::EventEmitter<OpenChat> for NeedsYouPane {}

pub struct NeedsYouPane {
    /// `None` in the demo and in tests: the pane draws fixtures and every
    /// send becomes a visible failure instead of a silent no-op.
    state: Option<Entity<AppState>>,
    items: Vec<NeedsYouItem>,
    chats: Vec<Chat>,
    /// The scope the "Always allow" toggle is on, per row. Absent means the
    /// default, this workspace.
    scopes: std::collections::HashMap<String, AlwaysAllowScope>,
    /// Rows whose answer is in flight. They stay on screen but stop taking
    /// clicks, so a double tap cannot answer twice.
    answering: HashSet<String>,
    /// Options picked so far on a multi-select question, keyed by row id.
    /// A multi-select is not sent until the user says they are done, so it
    /// needs somewhere to accumulate.
    picked: std::collections::HashMap<String, Vec<String>>,
    /// The last failure, shown inline — an answer that did not leave the
    /// device must say so rather than looking accepted.
    failure: Option<SharedString>,
    _watch: Option<Task<()>>,
    _chats_watch: Option<Task<()>>,
}

impl NeedsYouPane {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let mut pane = Self {
            state: Some(state),
            items: Vec::new(),
            chats: Vec::new(),
            scopes: std::collections::HashMap::new(),
            answering: HashSet::new(),
            picked: std::collections::HashMap::new(),
            failure: None,
            _watch: None,
            _chats_watch: None,
        };
        pane.start_watches(cx);
        pane
    }

    /// Fixtures instead of an engine — the demo entry and the tests.
    pub fn demo(items: Vec<NeedsYouItem>, chats: Vec<Chat>) -> Self {
        Self {
            state: None,
            items,
            chats,
            scopes: std::collections::HashMap::new(),
            answering: HashSet::new(),
            picked: std::collections::HashMap::new(),
            failure: None,
            _watch: None,
            _chats_watch: None,
        }
    }

    pub fn set_items(&mut self, items: Vec<NeedsYouItem>, chats: Vec<Chat>) {
        self.items = items;
        self.chats = chats;
    }

    pub fn rows(&self) -> Vec<InboxRow> {
        inbox_rows(&self.items, &self.chats)
    }

    fn start_watches(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self
            .state
            .as_ref()
            .and_then(|state| state.read(cx).engine().cloned())
        else {
            return;
        };
        let needs_you = engine.clone();
        self._watch = Some(cx.spawn(async move |this, cx| {
            // Resubscribe loop, same contract as the chats watch: a daemon
            // restart ends the stream, and returning here would freeze the
            // inbox until the app restarts.
            const RETRY: std::time::Duration = std::time::Duration::from_secs(2);
            loop {
                if let Ok(mut rx) = needs_you
                    .client()
                    .subscribe(methods::WATCH_NEEDS_YOU, serde_json::json!({}))
                    .await
                {
                    while let Some(value) = rx.recv().await {
                        let Ok(items) = serde_json::from_value::<Vec<NeedsYouItem>>(value) else {
                            tracing::warn!("dropping malformed needs-you frame");
                            continue;
                        };
                        let alive = this.update(cx, |pane, cx| {
                            // Anything the engine no longer lists has been
                            // answered; stop holding its row disabled.
                            pane.answering
                                .retain(|id| items.iter().any(|i| &i.id == id));
                            pane.items = items;
                            cx.notify();
                        });
                        if alive.is_err() {
                            return;
                        }
                    }
                }
                if this.update(cx, |_, _| {}).is_err() {
                    return;
                }
                cx.background_executor().timer(RETRY).await;
            }
        }));

        self._chats_watch = Some(cx.spawn(async move |this, cx| {
            const RETRY: std::time::Duration = std::time::Duration::from_secs(2);
            loop {
                if let Ok(mut rx) = engine
                    .client()
                    .subscribe(methods::WATCH_CHATS, serde_json::json!({}))
                    .await
                {
                    while let Some(value) = rx.recv().await {
                        let Ok(chats) = serde_json::from_value::<Vec<Chat>>(value) else {
                            continue;
                        };
                        let alive = this.update(cx, |pane, cx| {
                            pane.chats = chats;
                            cx.notify();
                        });
                        if alive.is_err() {
                            return;
                        }
                    }
                }
                if this.update(cx, |_, _| {}).is_err() {
                    return;
                }
                cx.background_executor().timer(RETRY).await;
            }
        }));
    }

    fn scope_for(&self, row_id: &str) -> AlwaysAllowScope {
        self.scopes.get(row_id).copied().unwrap_or_default()
    }

    fn toggle_scope(&mut self, row_id: &str, cx: &mut Context<Self>) {
        let next = self.scope_for(row_id).toggled();
        self.scopes.insert(row_id.to_string(), next);
        cx.notify();
    }

    fn answer_permission(
        &mut self,
        row: &InboxRow,
        decision: PermissionDecision,
        remember: bool,
        cx: &mut Context<Self>,
    ) {
        let remember = remember.then(|| remember_for(row, self.scope_for(&row.id)));
        let params = respond_permission_params(&row.id, decision, remember.as_ref());
        self.send(row.id.clone(), methods::RESPOND_PERMISSION, params, cx);
    }

    /// Single-select: one tap is the answer. Multi-select: one tap toggles
    /// the option and the user sends when they are done.
    fn pick_option(&mut self, row: &InboxRow, label: &str, cx: &mut Context<Self>) {
        if !row.multi_select {
            self.answer_question(row, vec![label.to_string()], cx);
            return;
        }
        let picked = self.picked.entry(row.id.clone()).or_default();
        match picked.iter().position(|p| p == label) {
            Some(ix) => {
                picked.remove(ix);
            }
            None => picked.push(label.to_string()),
        }
        cx.notify();
    }

    fn is_picked(&self, row_id: &str, label: &str) -> bool {
        self.picked
            .get(row_id)
            .is_some_and(|picked| picked.iter().any(|p| p == label))
    }

    fn answer_question(&mut self, row: &InboxRow, labels: Vec<String>, cx: &mut Context<Self>) {
        let Some((request_id, question_id)) = split_question_id(&row.id) else {
            return;
        };
        let answers = vec![UserInputAnswer {
            question_id: question_id.to_string(),
            labels,
        }];
        let Some(params) = respond_input_params(&row.chat_id, request_id, answers) else {
            return;
        };
        self.picked.remove(&row.id);
        self.send(row.id.clone(), methods::QUEUE_COMMAND, params, cx);
    }

    fn send(
        &mut self,
        row_id: String,
        method: &'static str,
        params: serde_json::Value,
        cx: &mut Context<Self>,
    ) {
        let Some(engine) = self
            .state
            .as_ref()
            .and_then(|state| state.read(cx).engine().cloned())
        else {
            self.failure = Some("Not connected to an engine.".into());
            cx.notify();
            return;
        };
        self.answering.insert(row_id.clone());
        self.failure = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = engine.client().call(method, params).await;
            this.update(cx, |pane, cx| {
                if let Err(err) = result {
                    // The answer never left the device: release the row so
                    // the user can try again, and say what happened.
                    pane.answering.remove(&row_id);
                    pane.failure = Some(format!("Answer failed: {err}").into());
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn render_row(&self, row: &InboxRow, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = Theme::of(cx);
        let tone = kind_color(row.kind, theme);
        let busy = self.answering.contains(&row.id);
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .child(badge(theme, row.badge, tone))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .whitespace_nowrap()
                    .text_size(ui_rems(12.0))
                    .text_color(theme.text_faint)
                    .child(SharedString::from(row.chat_name.clone())),
            );
        let title = div()
            .min_w_0()
            .text_size(ui_rems(13.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(theme.text)
            .child(SharedString::from(row.title.clone()));

        row_card(theme)
            .id(SharedString::from(format!("needs-you-{}", row.id)))
            .when(busy, |el| el.opacity(0.55))
            .child(header)
            .child(title)
            .when(
                !row.prompt.is_empty() && row.kind != NeedsYouKind::Permission,
                |el| el.child(body_text(theme, row.prompt.clone())),
            )
            .when_some(row.tool_command.clone(), |el, command| {
                el.child(command_text(theme, command))
            })
            .child(self.render_actions(row, busy, cx))
            .into_any_element()
    }

    fn render_actions(
        &self,
        row: &InboxRow,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = Theme::of(cx);
        let actions = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(px(6.0))
            .pt(px(2.0));
        match row.kind {
            NeedsYouKind::Permission => {
                let scope = self.scope_for(&row.id);
                let (allow, deny, always, toggle) =
                    (row.clone(), row.clone(), row.clone(), row.id.clone());
                actions
                    .child(
                        button(theme, ButtonTone::Primary, "Allow")
                            .id(SharedString::from(format!("allow-{}", row.id)))
                            .when(!busy, |el| {
                                el.on_click(cx.listener(move |pane, _, _, cx| {
                                    pane.answer_permission(
                                        &allow,
                                        PermissionDecision::Allow,
                                        false,
                                        cx,
                                    );
                                }))
                            }),
                    )
                    .child(
                        button(theme, ButtonTone::Danger, "Deny")
                            .id(SharedString::from(format!("deny-{}", row.id)))
                            .when(!busy, |el| {
                                el.on_click(cx.listener(move |pane, _, _, cx| {
                                    pane.answer_permission(
                                        &deny,
                                        PermissionDecision::Deny,
                                        false,
                                        cx,
                                    );
                                }))
                            }),
                    )
                    // Decision 20: the second, quieter action.
                    .child(
                        button(theme, ButtonTone::Quiet, "Always allow")
                            .id(SharedString::from(format!("always-{}", row.id)))
                            .when(!busy, |el| {
                                el.on_click(cx.listener(move |pane, _, _, cx| {
                                    pane.answer_permission(
                                        &always,
                                        PermissionDecision::Allow,
                                        true,
                                        cx,
                                    );
                                }))
                            }),
                    )
                    // The scope is a SETTING on "Always allow", not a fourth
                    // action. Drawn quieter and named so, or a user taps it
                    // expecting something to happen.
                    .child(
                        setting_chip(theme, format!("scope: {}", scope.label()))
                            .id(SharedString::from(format!("scope-{}", row.id)))
                            .on_click(cx.listener(move |pane, _, _, cx| {
                                pane.toggle_scope(&toggle, cx);
                            })),
                    )
                    .into_any_element()
            }
            NeedsYouKind::Question => {
                let picked_count = self.picked.get(&row.id).map_or(0, Vec::len);
                let send_row = row.clone();
                actions
                    .children(row.options.iter().enumerate().map(|(ix, option)| {
                        let (answer_row, label) = (row.clone(), option.clone());
                        // On a multi-select a picked option reads as chosen
                        // but not yet sent, so it takes the primary plate
                        // while the send button is what actually answers.
                        let tone = if self.is_picked(&row.id, option) {
                            ButtonTone::Primary
                        } else {
                            ButtonTone::Quiet
                        };
                        button(theme, tone, option.clone())
                            .id(SharedString::from(format!("option-{}-{ix}", row.id)))
                            .when(!busy, |el| {
                                el.on_click(cx.listener(move |pane, _, _, cx| {
                                    pane.pick_option(&answer_row, &label, cx);
                                }))
                            })
                    }))
                    .when(row.multi_select && picked_count == 0, |el| {
                        // Nothing picked yet: a HINT, not a button. Drawn as
                        // a button it read as a fourth option sitting in the
                        // same row as the real ones.
                        el.child(hint_chip(theme, "pick one or more"))
                    })
                    .when(row.multi_select && picked_count > 0, |el| {
                        el.child(
                            button(theme, ButtonTone::Primary, format!("Send {picked_count}"))
                                .id(SharedString::from(format!("send-{}", row.id)))
                                .when(!busy, |el| {
                                    el.on_click(cx.listener(move |pane, _, _, cx| {
                                        let labels = pane
                                            .picked
                                            .get(&send_row.id)
                                            .cloned()
                                            .unwrap_or_default();
                                        pane.answer_question(&send_row, labels, cx);
                                    }))
                                }),
                        )
                    })
                    .into_any_element()
            }
            NeedsYouKind::Failed => {
                let chat_id = row.chat_id.clone();
                actions
                    .child(
                        button(theme, ButtonTone::Quiet, "Open chat")
                            .id(SharedString::from(format!("open-{}", row.id)))
                            .on_click(cx.listener(move |_, _, _, cx| {
                                cx.emit(OpenChat(chat_id.clone()));
                            })),
                    )
                    .into_any_element()
            }
        }
    }
}

impl Render for NeedsYouPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.rows();
        // Rows first: their listeners need `&mut cx`, which cannot be live
        // alongside the shared borrow `Theme::of(cx)` holds.
        let cards: Vec<gpui::AnyElement> =
            rows.iter().map(|row| self.render_row(row, cx)).collect();
        let empty = cards.is_empty();
        let theme = Theme::of(cx);
        div()
            .id("needs-you-pane")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .when_some(self.failure.clone(), |el, failure| {
                el.child(
                    div()
                        .text_size(ui_rems(12.0))
                        .text_color(theme.danger)
                        .child(failure),
                )
            })
            .when(empty, |el| {
                el.child(empty_state(theme, "Nothing needs you."))
            })
            .children(cards)
    }
}
