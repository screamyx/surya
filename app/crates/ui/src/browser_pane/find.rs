//! Find in page: the field that sits at the end of the tab row, the match
//! count, and the two buttons that step through the run.
//!
//! CEF does the searching (`BrowserHost::find`); this draws the reading its
//! find handler publishes onto the page snapshot.

use gpui::{div, prelude::*, px, Context, Entity, Focusable as _, SharedString, Window};

use crate::composer::ComposerInput;
use crate::icons::{self, icon};
use crate::theme::Theme;

use super::tabs::PlainTooltip;
use super::{state, BrowserPane};

/// How wide the find field is allowed to get before the tabs stop shrinking
/// for it. Narrow, because it shares a 32px row with the strip.
const FIELD_WIDTH: f32 = 172.0;

pub fn bar(
    input: &Entity<ComposerInput>,
    result: Option<(i32, i32)>,
    theme: &Theme,
    window: &mut Window,
    cx: &mut Context<BrowserPane>,
) -> gpui::Div {
    let focused = input.focus_handle(cx).is_focused(window);
    let count: Option<SharedString> =
        result.and_then(|(current, total)| state::find_label(current, total)).map(Into::into);
    div()
        .flex_none()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .child(
            div()
                .id("browser-find")
                .w(px(FIELD_WIDTH))
                .flex_none()
                .h(px(24.0))
                .px(px(8.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(if focused { theme.border_strong } else { theme.border })
                .bg(theme.input_bg)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .cursor_text()
                .text_size(crate::typography::ui_rems(12.0))
                .child(icon(icons::MAGNIFER).size(px(11.0)).flex_none().text_color(theme.text_muted))
                .child(div().flex_1().min_w_0().child(input.clone()))
                .children(count.map(|count| {
                    div()
                        .flex_none()
                        .text_size(crate::typography::ui_rems(11.0))
                        .text_color(theme.text_muted)
                        .child(count)
                }))
                .on_click(cx.listener(|this, _, window: &mut Window, cx| {
                    this.focus_find(window, cx)
                })),
        )
        .child(step("browser-find-prev", icons::ARROW_UP, "Previous match", theme, cx, false))
        .child(step("browser-find-next", icons::ARROW_DOWN, "Next match", theme, cx, true))
        .child(
            div()
                .id("browser-find-close")
                .size(px(20.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_pointer()
                .hover(|s| s.bg(theme.element_hover))
                .child(icon(icons::CLOSE).size(px(10.0)).text_color(theme.text_muted))
                .tooltip(move |_, cx| cx.new(|_| PlainTooltip("Close find".into())).into())
                .tooltip_show_delay(std::time::Duration::from_millis(400))
                .on_click(cx.listener(|this, _, window: &mut Window, cx| {
                    this.close_find(window, cx)
                })),
        )
}

fn step(
    id: &'static str,
    icon_path: &'static str,
    label: &'static str,
    theme: &Theme,
    cx: &mut Context<BrowserPane>,
    forward: bool,
) -> impl IntoElement {
    div()
        .id(id)
        .size(px(20.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .cursor_pointer()
        .hover(|s| s.bg(theme.element_hover))
        .child(icon(icon_path).size(px(10.0)).text_color(theme.text_muted))
        .tooltip(move |_, cx| cx.new(|_| PlainTooltip(label.into())).into())
        .tooltip_show_delay(std::time::Duration::from_millis(400))
        .on_click(cx.listener(move |this, _, _, cx| this.step_find(forward, cx)))
}

impl BrowserPane {
    pub(super) fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find_open = true;
        self.find_focus_pending = true;
        self.focus_find(window, cx);
        super::keys::report("find open");
        cx.notify();
    }

    pub(super) fn focus_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.find_input.focus_handle(cx), cx);
        super::backend::set_page_focus(false);
    }

    /// The two arrows, and Enter or shift-Enter in the field.
    pub(super) fn step_find(&mut self, forward: bool, cx: &mut Context<Self>) {
        self.run_find(forward, true, cx);
    }

    /// Search, or step to the next or previous match of the same run. A
    /// changed query starts a new run; the same query steps through it,
    /// which is what CEF's `find_next` flag means.
    pub(super) fn run_find(&mut self, forward: bool, next: bool, cx: &mut Context<Self>) {
        let query = self.find_input.read(cx).text().trim().to_string();
        if query.is_empty() {
            super::backend::stop_find(true);
            self.find_query.clear();
            cx.notify();
            return;
        }
        let same = query == self.find_query;
        self.find_query = query.clone();
        super::backend::find(&query, forward, next && same);
        super::keys::found();
        cx.notify();
    }

    pub(super) fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find_open = false;
        self.find_query.clear();
        super::backend::stop_find(true);
        // The keyboard goes back to the page, as it does when Chrome's find
        // bar closes.
        window.focus(&self.page_focus, cx);
        super::backend::set_page_focus(true);
        super::keys::report("find close");
        cx.notify();
    }
}
