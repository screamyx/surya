//! The editor's in-pane notices: the save-refused banner and the
//! unsaved-edits prompt. One shape for both, built to fit the default
//! right-pane width, where the old one-row layout put the buttons past the
//! pane edge (e2e FILES-03): the message on its own line (two at most, then
//! an ellipsis), and the actions on the line below, wrapping if even they
//! do not fit. Below about 180 px of editor column (the pane dragged to its
//! minimum, 360 less the 240 tree) the widest button still overflows; that
//! width is out of this shape's scope. The old filled red bar also put
//! near-white text on pink at 1.83:1 (e2e UI-07); the notice now sits on
//! comet's raised surface with its body text, and says its tone with a
//! two-pixel edge.

use gpui::{AnyElement, SharedString, div, prelude::*, px};

use crate::theme::Theme;

/// What the notice is about; only the edge colour differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Something was refused: the edge is `theme.danger`.
    Refused,
    /// A question before the buffer moves: the edge is `theme.warning`,
    /// the dirty dot's colour.
    Ask,
}

/// The save-refused message: lead and detail. Was one string,
/// `Save refused: {reason}`.
pub fn refused_text(reason: &str) -> (&'static str, String) {
    ("Save refused.", reason.to_string())
}

/// The unsaved-edits message: lead and detail. Was one string,
/// `{here} has unsaved changes. Save or discard them before opening {next}?`;
/// the file being left is already in the header above, so the detail leads
/// with the choice and names the file being opened.
pub fn prompt_text(here: &str, next: &str) -> (&'static str, String) {
    ("Unsaved changes.", format!("Save or discard {here} before opening {next}?"))
}

/// The message row: a short lead in medium weight, then the detail, at
/// most two lines and then an ellipsis (`line_clamp` alone sets the line
/// count and hides the overflow; `text_ellipsis` is what draws the cue), so
/// a long path is read on the second line and never clips mid-word.
pub fn notice(
    theme: &Theme,
    tone: Tone,
    lead: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    actions: Vec<AnyElement>,
) -> gpui::Div {
    let edge = match tone {
        Tone::Refused => theme.danger,
        Tone::Ask => theme.warning,
    };
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .px(px(10.0))
        .py(px(8.0))
        .bg(theme.surface_raised) // TOKEN: surya.notice.bg
        .border_l_2()
        .border_color(edge) // TOKEN: surya.notice.edge
        .text_size(px(12.0))
        .text_color(theme.text)
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(6.0))
                .min_w_0()
                .child(
                    div()
                        .flex_none()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(lead.into()),
                )
                .child(div().flex_1().min_w_0().line_clamp(2).text_ellipsis().child(detail.into())),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_end()
                .gap(px(8.0))
                .children(actions),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_messages_split_into_a_short_lead_and_the_detail() {
        let (lead, detail) = refused_text("changed on disk since it was read");
        assert_eq!(lead, "Save refused.");
        assert_eq!(detail, "changed on disk since it was read");
        assert!(!detail.starts_with("Save refused"), "the reason is not repeated in the detail");

        let (lead, detail) = prompt_text("a.rs", "b.rs");
        assert_eq!(lead, "Unsaved changes.");
        assert_eq!(detail, "Save or discard a.rs before opening b.rs?");
        assert!(lead.len() < 20, "the lead is short enough never to need the clamp");
    }
}
