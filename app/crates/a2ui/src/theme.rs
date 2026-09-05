//! The paint tokens a card renders with. The host fills this from its own
//! theme (comet: `zeron_ui::theme::Theme`), so a card carries no colors of
//! its own and always sits on the transcript's palette. Numbers are layout,
//! colors are paint — the same rule the transcript follows.

use gpui::{Hsla, SharedString};

#[derive(Debug, Clone)]
pub struct CardTheme {
    /// The card plate (comet: `surface_card`).
    pub surface: Hsla,
    /// Raised controls on the plate: default buttons, fields (`surface_raised`).
    pub surface_raised: Hsla,
    /// Hover wash for buttons, tabs, checkboxes (`element_hover`).
    pub element_hover: Hsla,
    /// Hairline border (`border`) and its focused/raised step (`border_strong`).
    pub border: Hsla,
    pub border_strong: Hsla,
    /// Text ladder (`text`, `text_muted`, `text_faint`).
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_faint: Hsla,
    /// The primary button plate and its label (`solid`, `on_solid`).
    pub solid: Hsla,
    pub on_solid: Hsla,
    /// The selected accent, for the active tab rule and focus (`accent`).
    pub accent: Hsla,
    /// Inline code (`code_text` over `code_wash`).
    pub code_text: Hsla,
    pub code_wash: Hsla,
    /// Input plate (`input_bg`).
    pub input_bg: Hsla,
    /// Diagnostics (`danger_muted`).
    pub danger: Hsla,
    pub font_sans: SharedString,
    pub font_mono: SharedString,
    /// SVG asset path for the checked-box glyph (comet: `icons::CHECK`);
    /// `None` draws a filled inner square instead.
    pub check_icon: Option<SharedString>,
    /// Card corner radius (comet: `PANEL_RADIUS`) and control radius
    /// (`CONTROL_RADIUS`).
    pub radius: f32,
    pub control_radius: f32,
}
