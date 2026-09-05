//! The address row: history buttons, the address field, the zoom reading,
//! and the slot the device picker fills.
//!
//! The field is comet's own [`ComposerInput`], the same entity the composer
//! and the palette searches use, wrapped in the border and background the
//! settings dialogs give a text field. Nothing here is hand-drawn text.

use gpui::{div, prelude::*, px, App, Context, Entity, Focusable as _, SharedString, Window};

use crate::composer::ComposerInput;
use crate::icons::{self, icon};
use crate::theme::Theme;

use super::{state, BrowserPane, RightSlot, BAR_HEIGHT};

pub struct BarProps<'a> {
    pub url: &'a Entity<ComposerInput>,
    pub can_back: bool,
    pub can_forward: bool,
    pub loading: bool,
    pub zoom_percent: u32,
    pub right_slot: Option<&'a RightSlot>,
}

pub fn row(
    props: BarProps<'_>,
    theme: &Theme,
    window: &mut Window,
    cx: &mut Context<BrowserPane>,
) -> gpui::Div {
    let focused = props.url.focus_handle(cx).is_focused(window);
    let field = div()
        .id("browser-url")
        .flex_1()
        .min_w_0()
        .h(px(26.0))
        .px(px(10.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(if focused { theme.border_strong } else { theme.border })
        .bg(theme.input_bg)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .cursor_text()
        .text_size(crate::typography::ui_rems(12.5))
        .child(div().flex_1().min_w_0().child(props.url.clone()))
        .children(zoom_reading(props.zoom_percent, theme, cx))
        .on_click(cx.listener(|this, _, window: &mut Window, cx| this.focus_url(window, cx)));

    let mut row = div()
        .flex_none()
        .h(px(BAR_HEIGHT))
        .px(px(8.0))
        .gap(px(4.0))
        .flex()
        .flex_row()
        .items_center()
        .border_b_1()
        .border_color(theme.border)
        .child(button(
            "browser-back",
            icons::ARROW_LEFT,
            "Back",
            props.can_back,
            theme,
            cx.listener(|this, _, _, cx| this.go_back(cx)),
        ))
        .child(button(
            "browser-forward",
            icons::ARROW_RIGHT,
            "Forward",
            props.can_forward,
            theme,
            cx.listener(|this, _, _, cx| this.go_forward(cx)),
        ))
        .child(button(
            "browser-reload",
            if props.loading { icons::CLOSE } else { icons::REFRESH },
            if props.loading { "Stop" } else { "Reload" },
            true,
            theme,
            cx.listener(|this, _, _, cx| this.reload_or_stop(cx)),
        ))
        .child(field);
    if let Some(slot) = props.right_slot {
        row = row.child(div().flex_none().child(slot(window, cx)));
    }
    row
}

/// The zoom, inside the field's right edge, and only when it is not 100%.
/// Clicking it puts the page back to 100, as Chrome's own reading does.
fn zoom_reading(
    percent: u32,
    theme: &Theme,
    cx: &mut Context<BrowserPane>,
) -> Option<impl IntoElement> {
    let label: SharedString = state::zoom_label(percent)?.into();
    Some(
        div()
            .id("browser-zoom")
            .flex_none()
            .h(px(18.0))
            .px(px(6.0))
            .flex()
            .items_center()
            .rounded(px(4.0))
            .cursor_pointer()
            .bg(theme.element_hover)
            .text_size(crate::typography::ui_rems(11.0))
            .text_color(theme.text_muted)
            .hover(|s| s.bg(theme.element_active))
            .child(label)
            .on_click(cx.listener(|this, _, _, cx| {
                this.step_zoom(state::ZOOM_DEFAULT, cx)
            })),
    )
}

fn button(
    id: &'static str,
    icon_path: &'static str,
    label: &'static str,
    enabled: bool,
    theme: &Theme,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let tint = if enabled { theme.text } else { theme.text_muted.opacity(0.5) };
    div()
        .id(id)
        .size(px(24.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .child(icon(icon_path).size(px(14.0)).text_color(tint))
        .when(enabled, |el| {
            el.cursor_pointer()
                .hover(|s| s.bg(theme.element_hover))
                .on_click(on_click)
                .tooltip(move |_, cx| cx.new(|_| super::tabs::PlainTooltip(label.into())).into())
                .tooltip_show_delay(std::time::Duration::from_millis(400))
        })
}

impl BrowserPane {
    /// Write the field from the page. Remembering the text is what lets the
    /// `Edited` subscription tell this apart from a person typing, however
    /// late that event arrives.
    pub(super) fn set_url_text(&mut self, text: String, cx: &mut Context<Self>) {
        self.last_written = text.clone();
        self.url.update(cx, |input, cx| input.set_text(text, cx));
    }

    pub(super) fn focus_url(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.url.focus_handle(cx), cx);
        // The bar has the keyboard now, not the page.
        super::backend::set_page_focus(false);
    }

    /// Enter in the address field. A scheme-less address gets `https://`
    /// and plain words go to a search; both happen inside the crate's
    /// `navigate_to`, which also refuses `file://` and `chrome://`.
    pub(super) fn submit_url(&mut self, cx: &mut Context<Self>) {
        let typed = self.url.read(cx).text().trim().to_string();
        if typed.is_empty() {
            return;
        }
        super::keys::typed();
        let resolved = super::backend::resolve(&typed);
        if resolved.is_empty() {
            // A scheme the pane refuses. The field keeps what was typed so
            // it can be corrected rather than retyped, and the reason is on
            // screen: a bar that silently did nothing reads as broken.
            self.refused = Some(state::refusal(&typed));
            super::keys::report(&format!("refused {typed:?}"));
            cx.notify();
            return;
        }
        self.refused = None;
        super::backend::navigate(&typed);
        super::keys::navigated();
        self.url_editing = false;
        super::keys::report(&format!("navigate {resolved}"));
        cx.notify();
    }

    /// Escape in the address field: the page's own address comes back.
    pub(super) fn restore_url(&mut self, cx: &mut Context<Self>) {
        self.url_editing = false;
        self.refused = None;
        let shown = super::backend::page().shown_address().to_string();
        self.set_url_text(shown, cx);
        cx.notify();
    }

    pub(super) fn go_back(&mut self, cx: &mut Context<Self>) {
        super::backend::back();
        self.url_editing = false;
        cx.notify();
    }

    pub(super) fn go_forward(&mut self, cx: &mut Context<Self>) {
        super::backend::forward();
        self.url_editing = false;
        cx.notify();
    }

    pub(super) fn reload_or_stop(&mut self, cx: &mut Context<Self>) {
        if super::backend::page().loading {
            super::backend::stop();
        } else {
            super::backend::reload();
        }
        cx.notify();
    }
}
