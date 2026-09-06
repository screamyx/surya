//! The editor's in-pane notices: the save-refused banner and the
//! unsaved-edits prompt. One shape for both, built to fit the narrowest
//! pane the shell allows (`RIGHT_PANE_MIN` less the tree column): the
//! message on its own line (two at most, then an ellipsis), and
//! the actions on the line below, wrapping if even they do not fit. A
//! one-row layout put the buttons past the pane edge at the default width
//! (e2e FILES-03), and a filled red bar put near-white text on pink at
//! 1.83:1 (e2e UI-07); the notice now sits on comet's raised surface with
//! its body text, and says its tone with a two-pixel edge.

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

/// The message row: a short lead in medium weight, then the detail, at
/// most two lines and then an ellipsis, so a long path is read on the
/// second line and never clips mid-word.
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
                .items_baseline()
                .gap(px(6.0))
                .min_w_0()
                .child(
                    div()
                        .flex_none()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(lead.into()),
                )
                .child(div().flex_1().min_w_0().line_clamp(2).child(detail.into())),
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
