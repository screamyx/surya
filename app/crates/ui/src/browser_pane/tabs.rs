//! The tab strip: one row of tabs and the button that adds another.
//!
//! Shape ported from haktui's `chrome.rs` tab list, drawn with comet's
//! tokens: the active tab is the surface the page sits on, the rest are the
//! pane's background, and the row is separated by one hairline border.

use gpui::{div, prelude::*, px, Context, MouseButton, Render, SharedString, Window};

use crate::icons::{self, icon};
use crate::theme::Theme;

use super::backend::{Tab, TabId};
use super::{state, BrowserPane};

/// Room for the words on a tab. Narrower than Chrome's, because the pane is
/// a column and not a window.
const TAB_MAX_WIDTH: f32 = 168.0;
const TAB_MIN_WIDTH: f32 = 56.0;

pub fn strip(
    tabs: &[Tab],
    active: Option<TabId>,
    theme: &Theme,
    cx: &mut Context<BrowserPane>,
) -> gpui::Div {
    let mut row = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .overflow_hidden();
    for tab in tabs {
        row = row.child(one(tab, active == Some(tab.id), theme, cx));
    }
    row.child(add_button(theme, cx))
}

fn one(
    tab: &Tab,
    is_active: bool,
    theme: &Theme,
    cx: &mut Context<BrowserPane>,
) -> impl IntoElement {
    let id = tab.id;
    let title: SharedString = state::tab_title(&tab.title, &tab.url).into();
    let close_id = gpui::ElementId::Name(format!("browser-tab-close-{id}").into());
    div()
        .id(gpui::ElementId::Name(format!("browser-tab-{id}").into()))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .h(px(24.0))
        .px(px(8.0))
        .min_w(px(TAB_MIN_WIDTH))
        .max_w(px(TAB_MAX_WIDTH))
        .rounded(px(6.0))
        .cursor_pointer()
        .when(is_active, |el| {
            el.bg(theme.surface_raised).border_1().border_color(theme.border)
        })
        .when(!is_active, |el| el.border_1().border_color(gpui::transparent_black()))
        .when(!is_active, |el| el.hover(|s| s.bg(theme.element_hover)))
        // A tab that is still loading says so, the way a spinner does in
        // Chrome, without a spinner's cost on an offscreen browser.
        .when(tab.loading, |el| {
            el.child(div().size(px(6.0)).flex_none().rounded_full().bg(theme.busy))
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                // An ellipsis, not a hard clip: a strip full of titles cut
                // mid-word reads as a rendering fault.
                .truncate()
                .text_size(crate::typography::ui_rems(12.0))
                .text_color(if is_active { theme.text } else { theme.text_muted })
                .child(title.clone()),
        )
        .child(
            div()
                .id(close_id)
                .size(px(16.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_pointer()
                .hover(|s| s.bg(theme.element_active))
                .child(icon(icons::CLOSE).size(px(10.0)).text_color(theme.text_muted))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.close(id, cx);
                    cx.stop_propagation();
                })),
        )
        .on_click(cx.listener(move |this, _, _, cx| this.activate(id, cx)))
        // A middle click closes a tab, as it does in every browser.
        .on_mouse_down(
            MouseButton::Middle,
            cx.listener(move |this, _, _, cx| {
                this.close(id, cx);
                cx.stop_propagation();
            }),
        )
        // The strip truncates; the whole title is one hover away.
        .tooltip(move |_, cx| cx.new(|_| PlainTooltip(title.clone())).into())
        .tooltip_show_delay(std::time::Duration::from_millis(400))
}

fn add_button(theme: &Theme, cx: &mut Context<BrowserPane>) -> impl IntoElement {
    div()
        .id("browser-tab-add")
        .size(px(22.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(|s| s.bg(theme.element_hover))
        .child(icon(icons::PLUS).size(px(12.0)).text_color(theme.text_muted))
        .on_click(cx.listener(|this, _, window: &mut Window, cx| this.open_tab(window, cx)))
}

/// A few words on hover: a tab's whole title where the strip cut it short,
/// or the name of a button that shows only an icon.
pub struct PlainTooltip(pub SharedString);

impl Render for PlainTooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx);
        div()
            .px(px(8.0))
            .py(px(6.0))
            .max_w(px(320.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.surface_raised)
            .shadow_md()
            .text_size(crate::typography::ui_rems(11.0))
            .text_color(theme.text)
            .child(self.0.clone())
    }
}

impl BrowserPane {
    /// A new tab, from ctrl-t or the strip's plus. It opens empty and the
    /// keyboard goes to the address field, as it does in Chrome.
    pub(super) fn open_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        super::backend::tab_open("");
        super::keys::tab_opened();
        self.url_editing = true;
        self.set_url_text(String::new(), cx);
        self.focus_url(window, cx);
        super::keys::report("new tab");
        cx.notify();
    }

    pub(super) fn close(&mut self, id: TabId, cx: &mut Context<Self>) {
        super::backend::tab_close(id);
        super::keys::tab_closed();
        self.url_editing = false;
        // The next tab may be another host, so its zoom is picked up on the
        // next frame rather than kept from the one that just closed.
        self.zoom_url.clear();
        super::keys::report("close tab");
        cx.notify();
    }

    pub(super) fn activate(&mut self, id: TabId, cx: &mut Context<Self>) {
        super::backend::tab_activate(id);
        self.url_editing = false;
        self.zoom_url.clear();
        cx.notify();
    }

    /// ctrl-tab and shift-ctrl-tab. The strip wraps, as a tab strip does.
    pub(super) fn step_tab(&mut self, delta: isize, cx: &mut Context<Self>) {
        super::keys::key_seen();
        let ids: Vec<TabId> = super::backend::tabs().into_iter().map(|t| t.id).collect();
        let Some(active) = super::backend::active_tab() else { return };
        let Some(at) = ids.iter().position(|id| *id == active) else { return };
        if ids.len() < 2 {
            return;
        }
        let next = (at as isize + delta).rem_euclid(ids.len() as isize) as usize;
        super::keys::key_handled();
        self.activate(ids[next], cx);
    }
}
