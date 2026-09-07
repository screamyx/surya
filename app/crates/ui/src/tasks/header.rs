//! The Tasks board header: project name, task count, and the hint beside
//! them (E2E-TASK-02).
//!
//! The three strings sit in one row with the ends pushed apart. That is fine
//! until the row fills up, and then `justify_between` simply stops pushing
//! and the ends touch: at the default 520px pane width a longish project
//! name produced "1 taskAgents pull from Queued when they go idle." with
//! nothing between the count and the hint.
//!
//! Split out of `board.rs` because the fix took that file over the 500-line
//! rule (decision 13).

use gpui::prelude::*;
use gpui::{div, px, SharedString};

use crate::theme::Theme;
use crate::typography::ui_rems;

/// The least space allowed between the count and the hint.
///
/// `justify_between` gives no minimum of its own, so this gap is the only
/// thing standing between the two strings when the row is full.
pub const HEADER_GAP: f32 = 16.0;

/// The header row: `<project name> <n tasks>` on the left, the hint right.
pub fn render(space_name: &SharedString, count: usize, theme: &Theme) -> gpui::Div {
    div()
        .flex()
        .flex_row()
        .items_end()
        .justify_between()
        .gap(px(HEADER_GAP))
        .px(px(16.0))
        .pt(px(24.0))
        .pb(px(12.0))
        .child(
            div()
                .flex()
                .flex_row()
                .items_end()
                .gap(px(8.0))
                // The title is what gives way when the row runs out of room:
                // it is the one part that can be elided and still understood.
                // Without min_w(0) a flex child refuses to shrink below its
                // content and `truncate` never fires.
                .min_w(px(0.0))
                .child(
                    div()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(ui_rems(30.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text) // TOKEN: surya.heading
                        .child(space_name.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .pb(px(6.0))
                        .text_size(ui_rems(13.0))
                        .text_color(theme.text_muted)
                        .child(SharedString::from(count_label(count))),
                ),
        )
        .child(
            div()
                .flex_none()
                .pb(px(6.0))
                .text_size(ui_rems(13.0))
                .text_color(theme.text_muted)
                .child(HINT),
        )
}

/// The line under the count. Never elided: it is short, and half of it is
/// no use.
const HINT: &str = "Agents pull from Queued when they go idle.";

/// "1 task", "2 tasks", "0 tasks".
fn count_label(count: usize) -> String {
    format!("{count} task{}", if count == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The finding: the count and the hint are separate strings with nothing
    /// between them, so at a narrow width they render as one word.
    #[test]
    fn the_count_and_the_hint_are_held_apart() {
        assert!(
            HEADER_GAP > 0.0,
            "justify_between gives no minimum separation of its own"
        );
        // What the frame showed, and what must never render again.
        let ran_together = format!("{}{}", count_label(1), HINT);
        assert_eq!(ran_together, "1 taskAgents pull from Queued when they go idle.");
        assert!(
            !HINT.starts_with(char::is_whitespace),
            "the gap belongs to the layout, not to a padded string"
        );
    }

    #[test]
    fn the_count_reads_naturally_at_every_size() {
        assert_eq!(count_label(0), "0 tasks");
        assert_eq!(count_label(1), "1 task");
        assert_eq!(count_label(2), "2 tasks");
    }
}
