//! The Browser surface: a tab strip, an address bar, find in page and zoom
//! over haktui's offscreen Chromium.
//!
//! The pages are process state in `surya-browser` (one CEF process, one
//! browser per tab); this view owns only what the chrome is doing. It is
//! built from comet's own parts: the address and find fields are
//! [`ComposerInput`], the same input the composer and the palette searches
//! use, and every colour comes from [`Theme`].
//!
//! Layout, top to bottom:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │ tabs   [ Example Domain x ][ Docs x ] +   find field  │
//! ├──────────────────────────────────────────────────────┤
//! │ bar    <- -> reload [ https://example.com ] 125% slot │
//! ├──────────────────────────────────────────────────────┤
//! │ page                                                  │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! Each row's behaviour lives with its drawing: `tabs.rs` opens and closes
//! tabs, `bar.rs` owns the address, `find.rs` the search, `zoom.rs` the
//! size, `keys.rs` the chords, and `state.rs` the arithmetic behind all of
//! it. `backend.rs` is the only file that calls into `surya-browser`.

mod backend;
mod bar;
mod find;
mod keys;
pub mod state;
mod tabs;
mod zoom;

use gpui::{
    div, prelude::*, px, AnyElement, App, Context, Entity, FocusHandle, Render, SharedString,
    Subscription, Window,
};

use crate::composer::{ComposerInput, ComposerInputEvent};
use crate::theme::Theme;
use state::ZoomMemory;

pub use keys::{counters, init};

pub const TAB_STRIP_HEIGHT: f32 = 32.0;
pub const BAR_HEIGHT: f32 = 36.0;

/// What the device picker (surya-browser-cdp) draws at the end of the bar.
pub type RightSlot = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>;

pub struct BrowserPane {
    /// The page's keyboard focus: clicking the page takes it, and keys go to
    /// Chromium while it holds it.
    pub(super) page_focus: FocusHandle,
    /// The address field. comet's own input, not a hand-drawn one.
    pub(super) url: Entity<ComposerInput>,
    /// True while the person is typing an address the page has not loaded.
    /// The field follows the page's address whenever it is false.
    pub(super) url_editing: bool,
    /// Set while the pane writes the field itself, so the resulting `Edited`
    /// is not read back as typing.
    pub(super) syncing: bool,
    /// The find field, made once and shown from ctrl-f.
    pub(super) find_input: Entity<ComposerInput>,
    pub(super) find_open: bool,
    pub(super) find_focus_pending: bool,
    /// What the last find asked for, so Enter can step through the same run.
    pub(super) find_query: String,
    /// Per-host zoom, mirrored from `ui-settings.json`.
    pub(super) zoom: ZoomMemory,
    /// The active tab's zoom, in percent.
    pub(super) zoom_percent: u32,
    /// The address the zoom was last applied for, so a navigation to another
    /// host picks that host's zoom up.
    pub(super) zoom_url: String,
    pub(super) right_slot: Option<RightSlot>,
    _subs: Vec<Subscription>,
}

impl BrowserPane {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // "PaletteSearch" binds the text-editing keys only, so enter, escape
        // and shift-enter stay unbound and reach the pane's own key handler.
        let url = cx.new(|cx| {
            ComposerInput::with_context("Search or type an address", "PaletteSearch", cx)
        });
        let find_input =
            cx.new(|cx| ComposerInput::with_context("Find in page", "PaletteSearch", cx));
        let subs = vec![
            cx.subscribe(&url, |this: &mut Self, _, event, cx| {
                if matches!(event, ComposerInputEvent::Edited) && !this.syncing {
                    this.url_editing = true;
                    cx.notify();
                }
            }),
            cx.subscribe(&find_input, |this: &mut Self, _, event, cx| {
                if matches!(event, ComposerInputEvent::Edited) {
                    this.run_find(true, false, cx);
                }
            }),
        ];
        let saved = crate::settings::current(cx).browser_zoom;
        Self {
            page_focus: cx.focus_handle(),
            url,
            url_editing: false,
            syncing: false,
            find_input,
            find_open: false,
            find_focus_pending: false,
            find_query: String::new(),
            zoom: ZoomMemory::from_settings(&saved),
            zoom_percent: state::ZOOM_DEFAULT,
            zoom_url: String::new(),
            right_slot: None,
            _subs: subs,
        }
    }

    /// The device picker's element, drawn at the end of the bar row. Set
    /// from the shell; `None` leaves the row as it is.
    pub fn set_right_slot(&mut self, slot: Option<RightSlot>, cx: &mut Context<Self>) {
        self.right_slot = slot;
        cx.notify();
    }
}

impl Render for BrowserPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let page = backend::page();
        let open_tabs = backend::tabs();
        let active = backend::active_tab();
        self.follow_page_zoom(&page.url, cx);
        // The field follows the page while nobody is typing into it.
        if !self.url_editing {
            let shown = page.shown_address().to_string();
            if self.url.read(cx).text() != shown {
                self.set_url_text(shown, cx);
            }
        }
        if std::mem::take(&mut self.find_focus_pending) {
            window.focus(&self.find_input.focus_handle(cx), cx);
        }

        let mut tab_row = div()
            .flex_none()
            .h(px(TAB_STRIP_HEIGHT))
            .px(px(6.0))
            .gap(px(6.0))
            .flex()
            .flex_row()
            .items_center()
            .border_b_1()
            .border_color(theme.border)
            .child(tabs::strip(&open_tabs, active, &theme, cx));
        if self.find_open {
            tab_row = tab_row.child(find::bar(
                &self.find_input,
                backend::find_result(&page),
                &theme,
                window,
                cx,
            ));
        }
        let bar = bar::row(
            bar::BarProps {
                url: &self.url,
                can_back: page.can_back,
                can_forward: page.can_forward,
                loading: page.loading,
                zoom_percent: self.zoom_percent,
                right_slot: self.right_slot.as_ref(),
            },
            &theme,
            window,
            cx,
        );
        // The load's progress as a hairline under the bar, gone at 1.
        let hairline = div().flex_none().h(px(2.0)).w_full().child(
            div()
                .h_full()
                .w(gpui::relative(if page.loading { page.progress.max(0.05) } else { 0.0 }))
                .bg(theme.text_muted),
        );
        let error = page.error.clone().map(|e| {
            div()
                .flex_none()
                .px(px(10.0))
                .py(px(6.0))
                .text_size(crate::typography::ui_rems(12.0))
                .text_color(theme.text_muted)
                .child(SharedString::from(e))
        });

        keys::bind(div(), cx)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.bg)
            .child(tab_row)
            .child(bar)
            .child(hairline)
            .children(error)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .bg(gpui::white())
                    .child(backend::panel(&self.page_focus)),
            )
    }
}
