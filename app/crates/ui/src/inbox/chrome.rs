//! The small pieces all three inbox views share: section headings, kind
//! badges, buttons, and the empty state.
//!
//! Kept together so the needs-you list, the agent rail and the rules page
//! cannot drift into three different-looking surfaces.

use gpui::{Div, Hsla, SharedString, div, prelude::*, px};
use zeron_proto::{AgentState, NeedsYouKind};

use crate::theme::Theme;
use crate::typography::ui_rems;

/// The tone a kind is drawn in.
// TOKEN: surya-theme's palette lands on feat/surya-look; these map onto the
// existing theme roles so the switch is a rename, not a redesign.
pub fn kind_color(kind: NeedsYouKind, theme: &Theme) -> Hsla {
    match kind {
        // A blocked tool is the one the user must answer to unfreeze a run.
        NeedsYouKind::Permission => theme.accent,
        NeedsYouKind::Question => theme.warning,
        NeedsYouKind::Failed => theme.danger,
    }
}

pub fn state_color(state: AgentState, theme: &Theme) -> Hsla {
    match state {
        AgentState::NeedsYou { kind } => kind_color(kind, theme),
        AgentState::Stopped => theme.danger,
        AgentState::Working => theme.busy,
        AgentState::Done => theme.success,
        AgentState::Idle => theme.text_faint,
    }
}

/// A small pill: the kind badge on an inbox row, the roll-up count on a
/// parent agent row.
pub fn badge(_theme: &Theme, text: impl Into<SharedString>, tone: Hsla) -> Div {
    div()
        .flex_none()
        .px(px(6.0))
        .py(px(1.0))
        .rounded(px(999.0))
        .bg(tone.opacity(0.14))
        .text_size(ui_rems(10.0))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(tone)
        .child(text.into())
}

/// A round status dot, for an agent row.
pub fn dot(color: Hsla) -> Div {
    div()
        .flex_none()
        .size(px(6.0))
        .rounded(px(999.0))
        .bg(color)
}

/// How loud a button is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonTone {
    /// The one the user most likely wants: a filled plate.
    Primary,
    /// Everything else: an outline.
    Quiet,
    /// Destructive or refusing.
    Danger,
}

/// A small action button. The caller adds `.id(..)` and `.on_click(..)`;
/// this only decides how it looks, so every inbox button matches.
pub fn button(theme: &Theme, tone: ButtonTone, label: impl Into<SharedString>) -> Div {
    let base = div()
        .flex_none()
        .px(px(9.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .text_size(ui_rems(12.0))
        .font_weight(gpui::FontWeight::MEDIUM)
        .cursor_pointer();
    match tone {
        ButtonTone::Primary => base
            .bg(theme.solid)
            .text_color(theme.on_solid)
            .hover(|s| s.bg(theme.solid.opacity(0.88))),
        ButtonTone::Quiet => base
            .border_1()
            .border_color(theme.border_strong)
            .text_color(theme.text_muted)
            .hover(|s| s.bg(theme.element_hover).text_color(theme.text)),
        ButtonTone::Danger => base
            .border_1()
            .border_color(theme.danger.opacity(0.5))
            .text_color(theme.danger)
            .hover(|s| s.bg(theme.danger.opacity(0.1))),
    }
    .child(label.into())
}

/// A quieter chip for a SETTING rather than an action: no border, muted
/// text, so it does not read as a fourth button next to Allow and Deny.
pub fn setting_chip(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .px(px(6.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .text_size(ui_rems(11.0))
        .text_color(theme.text_faint)
        .cursor_pointer()
        .hover(|s| s.bg(theme.element_hover).text_color(theme.text_muted))
        .child(text.into())
}

/// A group heading: "Waiting for you", "Running", and the like.
pub fn heading(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .px(px(4.0))
        .pt(px(10.0))
        .pb(px(4.0))
        .text_size(ui_rems(10.0))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme.text_faint)
        .child(text.into())
}

/// The quiet state. Decision 20: "The quiet state says nothing needs
/// attention" — it is a sentence, not a blank pane.
pub fn empty_state(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .py(px(40.0))
        .text_size(ui_rems(13.0))
        .text_color(theme.text_faint)
        .child(text.into())
}

/// The card every inbox row sits in.
pub fn row_card(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(10.0))
        .rounded(px(10.0))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface_card)
}

/// One line of secondary text: the prompt under a title, the command under a
/// permission badge.
pub fn body_text(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .min_w_0()
        .text_size(ui_rems(12.0))
        .text_color(theme.text_muted)
        .child(text.into())
}

/// A monospace line for a tool command — it is literal text the user is
/// approving, so it must not be reflowed into prose.
pub fn command_text(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .min_w_0()
        .truncate()
        .whitespace_nowrap()
        .px(px(6.0))
        .py(px(3.0))
        .rounded(px(5.0))
        .bg(theme.surface_raised)
        .font_family(theme.font_mono.clone())
        .text_size(ui_rems(11.5))
        .text_color(theme.text)
        .child(text.into())
}
