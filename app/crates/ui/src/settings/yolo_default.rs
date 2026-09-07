//! Settings -> Agents: the yolo default for new sessions.
//!
//! Yolo mode runs an agent's tools without asking (see [`crate::yolo`]).
//! This row sets what a NEW session starts as. It never reaches a session
//! that already exists: those carry their own flag on the chat row, and the
//! composer chip is where they change it.
//!
//! Device-local, like the rest of `ui-settings.json`. The setting is a
//! preference about this machine's prompts, not something to push at the
//! user's other devices mid-run.

use gpui::{App, ClickEvent, SharedString, Window, div, prelude::*, px};

use crate::icons;
use crate::settings::widgets;
use crate::theme::Theme;

/// The section card. `toggle` is the caller's click handler (it owns the
/// page entity, so it owns the notify).
pub fn section(
    theme: &Theme,
    on: bool,
    toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui::AnyElement {
    widgets::section_card(theme)
        .child(
            widgets::card_row(theme, true)
                .id("yolo-default-row")
                .child(widgets::row_tile(theme, icons::DANGER_TRIANGLE))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(widgets::row_title(theme, "Yolo mode for new sessions"))
                        .child(widgets::meta_line(
                            theme,
                            vec![
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .whitespace_normal()
                                    .child(SharedString::from(
                                        "New sessions run tools without asking. Sessions you \
                                         already have keep their own setting; switch those in \
                                         the composer.",
                                    ))
                                    .into_any_element(),
                            ],
                        )),
                )
                .child(
                    widgets::toggle_switch(theme, on)
                        .id("yolo-default-toggle")
                        .cursor_pointer()
                        .on_click(toggle),
                ),
        )
        .mt(px(16.0))
        .into_any_element()
}
