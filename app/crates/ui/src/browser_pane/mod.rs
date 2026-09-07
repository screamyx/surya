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
    div, prelude::*, px, AnyElement, App, Context, Entity, FocusHandle, Focusable as _, Hsla,
    Render, SharedString, Subscription, Window,
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
    /// The last address the pane itself wrote into the field.
    ///
    /// `Edited` arrives late: `Context::emit` pushes onto `pending_effects`
    /// and subscribers run when those flush, so a flag set around `set_text`
    /// is already false by the time the event lands and the pane's own write
    /// looked exactly like a person typing. Comparing the text has no such
    /// race: if the field still holds what the pane put there, nobody typed.
    pub(super) last_written: String,
    /// The find field, made once and shown from ctrl-f.
    pub(super) find_input: Entity<ComposerInput>,
    pub(super) find_open: bool,
    pub(super) find_focus_pending: bool,
    /// What the last find asked for, so Enter can step through the same run.
    pub(super) find_query: String,
    /// Why the last typed address went nowhere. The crate refuses anything
    /// that is not http or https, and a refusal that only reached stdout
    /// looked to the person like a bar that had stopped working.
    pub(super) refused: Option<String>,
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
            let mut input =
                ComposerInput::with_context("Search or type an address", "PaletteSearch", cx);
            // A click into the bar takes the whole address, so typing
            // replaces it (E2E-BROWSER-01); a second click places the caret.
            input.set_select_all_on_focus(true);
            input
        });
        let find_input =
            cx.new(|cx| ComposerInput::with_context("Find in page", "PaletteSearch", cx));
        let subs = vec![
            cx.subscribe(&url, |this: &mut Self, input, event, cx| {
                if !matches!(event, ComposerInputEvent::Edited) {
                    return;
                }
                // The pane's own write, arriving late. Not typing.
                if !state::is_user_edit(input.read(cx).text(), &this.last_written) {
                    return;
                }
                this.url_editing = true;
                cx.notify();
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
            last_written: String::new(),
            find_input,
            find_open: false,
            find_focus_pending: false,
            find_query: String::new(),
            refused: None,
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
            let stale = self.url.read(cx).text() != shown.as_str();
            if stale {
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
        // A refusal is the pane's own answer and outlives no navigation, so
        // it wins over the page's last load error while it is showing.
        let error = self.refused.clone().or_else(|| page.error.clone()).map(|e| {
            div()
                .flex_none()
                .px(px(10.0))
                .py(px(6.0))
                .text_size(crate::typography::ui_rems(12.0))
                .text_color(theme.danger)
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
                    .map(|page| match backend::off_note() {
                        // No page, so no white plane: the note sits on the
                        // shell's own surface in the shell's own tones.
                        Some(note) => page.bg(theme.surface).child(off_pane(&note, &theme)),
                        // A page that paints no background of its own should
                        // be white, the way it is in a browser.
                        None => {
                            page.bg(gpui::white()).child(backend::panel(&self.page_focus))
                        }
                    }),
            )
    }
}

/// The plane the "no page" note sits on and the three tones on it.
///
/// One function so the test measures exactly what the renderer paints. The
/// note used to have no colours at all, so it inherited the shell's text onto
/// the white page surface: cef3 measured the darkest pixel of it at
/// (232,232,234) on (255,255,255), about 1.2:1, on Windows dark.
///
/// `text_faint` is deliberately not used for the hint. The palette holds it to
/// a 4.1:1 floor as placeholder and disabled-control copy, which WCAG 1.4.3
/// exempts (theme.rs `text_tones_clear_wcag_aa`). This note is not incidental:
/// it is the only thing telling a person why the browser is empty and how to
/// get it back, so every line of it clears AA.
fn note_colors(theme: &Theme) -> NoteColors {
    NoteColors {
        plane: theme.surface,
        label: theme.text,
        line: theme.text_muted,
        hint: theme.text_dim,
    }
}

struct NoteColors {
    plane: Hsla,
    label: Hsla,
    line: Hsla,
    hint: Hsla,
}

/// The note itself, painted on [`note_colors`].
fn off_pane(note: &surya_browser::OffNote, theme: &Theme) -> gpui::Div {
    let colors = note_colors(theme);
    let mut pane = div()
        .size_full()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(14.0))
        .text_size(crate::typography::ui_rems(12.0))
        .child(div().text_color(colors.label).child(note.label))
        .child(div().text_color(colors.line).child(note.line));
    // The way out sits under the sentence, quieter but still readable, so the
    // eye reads what happened before it reads what to do about it.
    if let Some(hint) = note.hint {
        pane = pane.child(div().text_color(colors.hint).child(hint));
    }
    pane
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::contrast_ratio;

    /// Every line of the note clears WCAG AA against the plane it is drawn on,
    /// in both appearances. Fails if the note goes back onto the white page
    /// surface: the shell's dark-mode text measures about 1.2:1 there.
    #[test]
    fn every_line_of_the_note_clears_aa_on_its_own_plane() {
        for theme in [Theme::dark(), Theme::light()] {
            let colors = note_colors(&theme);
            for (name, tone) in
                [("label", colors.label), ("line", colors.line), ("hint", colors.hint)]
            {
                let ratio = contrast_ratio(tone, colors.plane);
                assert!(
                    ratio >= 4.5,
                    "{:?} {name} is {ratio:.2}:1 on its plane, below AA",
                    theme.appearance
                );
            }
        }
    }

    /// What the fix is for, kept as a measurement rather than a memory.
    ///
    /// The label is the tone that was photographed on Windows dark: its pixel
    /// read (232,232,234) on a (255,255,255) page surface, about 1.2:1. No
    /// tone the note uses is readable on that plane, so the plane is never
    /// white.
    #[test]
    fn the_white_page_surface_is_not_readable_for_the_note() {
        let dark = Theme::dark();
        let label_on_white = contrast_ratio(dark.text, gpui::white());
        assert!(label_on_white < 1.5, "the photographed pair, got {label_on_white:.2}:1");
        for (name, tone) in
            [("label", dark.text), ("line", dark.text_muted), ("hint", dark.text_dim)]
        {
            let ratio = contrast_ratio(tone, gpui::white());
            assert!(ratio < 4.5, "{name} reads {ratio:.2}:1 on white, so it needs no fix");
        }
        assert_ne!(note_colors(&dark).plane, gpui::white());
    }
}
