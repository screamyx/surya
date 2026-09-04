//! The Browser surface: a URL bar over haktui's offscreen Chromium.
//!
//! The page itself is process state in `surya-browser` (one browser per
//! process); this view owns only what the bar is doing. Pins, phone
//! emulation and agent CDP access are not wired here (2026-09-05 seat).

use gpui::{
    div, prelude::*, px, App, Context, FocusHandle, KeyDownEvent, Render, SharedString, Window,
};

use crate::icons::{self, icon};
use crate::theme::Theme;

pub const BAR_HEIGHT: f32 = 36.0;

pub struct BrowserPane {
    /// The URL field's keyboard focus.
    bar_focus: FocusHandle,
    /// The page's keyboard focus: clicking the page takes it, and keys go
    /// to Chromium while it holds it.
    page_focus: FocusHandle,
    /// What the person has typed so far, or `None` while the bar shows the
    /// page's own address.
    typed: Option<String>,
}

impl BrowserPane {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            bar_focus: cx.focus_handle(),
            page_focus: cx.focus_handle(),
            typed: None,
        }
    }

    fn on_bar_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ks = &event.keystroke;
        let page = surya_browser::page();
        match ks.key.as_str() {
            "enter" => {
                if let Some(typed) = self.typed.take()
                    && !typed.trim().is_empty()
                {
                    surya_browser::navigate(&typed);
                }
                cx.stop_propagation();
            }
            "escape" => {
                self.typed = None;
                cx.stop_propagation();
            }
            "backspace" => {
                let mut text = self.typed.take().unwrap_or_else(|| page.shown_address().to_string());
                text.pop();
                self.typed = Some(text);
                cx.stop_propagation();
            }
            _ => {
                if ks.modifiers.control || ks.modifiers.platform || ks.modifiers.alt {
                    return;
                }
                let Some(ch) = ks.key_char.as_deref() else { return };
                if ch.chars().any(char::is_control) {
                    return;
                }
                // Typing into a bar that still shows the page's address
                // replaces it, as a fresh select-all in Chrome would.
                let mut text = self.typed.take().unwrap_or_default();
                text.push_str(ch);
                self.typed = Some(text);
                cx.stop_propagation();
            }
        }
        cx.notify();
    }
}

fn bar_button(
    id: &'static str,
    icon_path: &'static str,
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
        .when(enabled, |el| el.cursor_pointer().hover(|s| s.bg(crate::theme::ink(0.06))))
        .child(icon(icon_path).size(px(14.0)).text_color(tint))
        .when(enabled, |el| el.on_click(on_click))
}

impl Render for BrowserPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let page = surya_browser::page();
        let editing = self.typed.is_some();
        let shown: SharedString = self
            .typed
            .clone()
            .unwrap_or_else(|| page.shown_address().to_string())
            .into();
        let focused = self.bar_focus.is_focused(window);
        let bar_focus = self.bar_focus.clone();
        let field = div()
            .id("browser-url")
            .flex_1()
            .min_w_0()
            .h(px(26.0))
            .px(px(10.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(if focused { theme.border_strong } else { theme.border })
            .bg(crate::theme::ink(0.03))
            .flex()
            .items_center()
            .cursor_text()
            .track_focus(&self.bar_focus)
            .on_click(move |_, window, cx| {
                window.focus(&bar_focus, cx);
                // The bar has the keyboard now, not the page.
                surya_browser::set_focus(false);
            })
            .on_key_down(cx.listener(Self::on_bar_key))
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(crate::typography::ui_rems(12.5))
                    .text_color(if editing { theme.text } else { theme.text_muted })
                    .child(if shown.is_empty() {
                        SharedString::from("Type an address")
                    } else {
                        shown
                    }),
            )
            .when(focused && editing, |el| {
                el.child(div().w(px(1.0)).h(px(14.0)).bg(theme.text))
            });
        let bar = div()
            .flex_none()
            .h(px(BAR_HEIGHT))
            .px(px(8.0))
            .gap(px(4.0))
            .flex()
            .flex_row()
            .items_center()
            .border_b_1()
            .border_color(theme.border)
            .child(bar_button("browser-back", icons::ARROW_LEFT, page.can_back, &theme, |_, _, _| {
                surya_browser::back()
            }))
            .child(bar_button(
                "browser-forward",
                icons::ARROW_RIGHT,
                page.can_forward,
                &theme,
                |_, _, _| surya_browser::forward(),
            ))
            .child(bar_button("browser-reload", icons::REFRESH, true, &theme, move |_, _, _| {
                if page.loading {
                    surya_browser::stop()
                } else {
                    surya_browser::reload()
                }
            }))
            .child(field);
        // The load's progress as a hairline under the bar, gone at 1.
        let progress = surya_browser::page();
        let hairline = div()
            .flex_none()
            .h(px(2.0))
            .w_full()
            .child(
                div()
                    .h_full()
                    .w(gpui::relative(if progress.loading { progress.progress.max(0.05) } else { 0.0 }))
                    .bg(theme.text_muted),
            );
        let error = progress.error.clone().map(|e| {
            div()
                .flex_none()
                .px(px(10.0))
                .py(px(6.0))
                .text_size(crate::typography::ui_rems(12.0))
                .text_color(theme.text_muted)
                .child(SharedString::from(e))
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(bar)
            .child(hairline)
            .children(error)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .bg(gpui::white())
                    .child(surya_browser::panel(&self.page_focus)),
            )
    }
}
