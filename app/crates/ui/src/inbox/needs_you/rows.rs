//! How one needs-you row is drawn: the full card, the collapsed line for a
//! question whose sheet is already on screen, and the buttons that answer it.
//!
//! Split out of `needs_you.rs` to keep both files under the 500-line rule
//! (decision 13). The pane owns the data and the engine calls; this owns the
//! layout.

use gpui::{Context, SharedString, div, prelude::*, px};
use zeron_proto::{NeedsYouKind, PermissionDecision};

use super::{NeedsYouPane, OpenChat};
use crate::inbox::chrome::{
    ButtonTone, badge, body_text, button, command_text, hint_chip, kind_color, row_card,
    setting_chip,
};
use crate::inbox::model::{InboxRow, answered_in_the_open_chat, title_adds_to_badge};
use crate::theme::Theme;
use crate::typography::ui_rems;

impl NeedsYouPane {
    pub(super) fn render_row(
        &self,
        row: &InboxRow,
        open_chat: Option<&str>,
        open_sheet: Option<&str>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = Theme::of(cx);
        let tone = kind_color(row.kind, theme);
        let busy = self.answering.contains(&row.id);
        if answered_in_the_open_chat(row, open_chat, open_sheet) {
            return self.render_collapsed_row(row, tone, busy, cx);
        }
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
            .when(title_adds_to_badge(row), |el| el.child(title))
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

    /// The one-line form for a question whose sheet is open in the chat
    /// below. It keeps the row visible - the queue has not lost it - without
    /// offering a second set of buttons for the same answer.
    fn render_collapsed_row(
        &self,
        row: &InboxRow,
        tone: gpui::Hsla,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = Theme::of(cx);
        let text = if row.prompt.trim().is_empty() {
            row.title.clone()
        } else {
            row.prompt.clone()
        };
        row_card(theme)
            .id(SharedString::from(format!("needs-you-{}", row.id)))
            .when(busy, |el| el.opacity(0.55))
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .child(badge(theme, row.badge, tone))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .whitespace_nowrap()
                    .text_size(ui_rems(13.0))
                    .text_color(theme.text)
                    .child(SharedString::from(text)),
            )
            .child(hint_chip(theme, "answer below"))
            // Tapping the row hands the user to the sheet it points at, so
            // the line is a way there and not just a label.
            .cursor_pointer()
            .hover(|s| s.border_color(theme.border_strong))
            .on_click(cx.listener(|pane, _, window, cx| {
                if let Some(focus) = pane
                    .state
                    .as_ref()
                    .and_then(|state| state.read(cx).composer_focus.clone())
                {
                    window.focus(&focus, cx);
                }
            }))
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
