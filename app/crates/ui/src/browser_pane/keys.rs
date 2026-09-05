//! The chords the pane owns, and the counters that prove they ran.
//!
//! These are a **capture-phase key listener**, not a keymap binding, and the
//! reason is a rule in gpui's keymap. `Keymap::binding_enabled`
//! (`keymap.rs`) gives a binding with no context predicate
//! `Some(contexts.len())`, the deepest depth there is, while a binding with
//! a predicate gets the depth at which its context matched.
//! `bindings_for_input` sorts on depth first and only breaks ties by
//! insertion order. So a context-less `ctrl-tab` bound anywhere in the app
//! beats a `BrowserPane` binding whenever the pane's context is not the last
//! one on the stack - which is exactly the case when the keyboard sits in
//! the address field, where the stack ends with the input's own context.
//! comet binds `ctrl-tab` to NextSession and `cmd-w` to CloseWindow without
//! a context (`shell.rs`, `app_menus.rs`), so a bound pane would have cycled
//! sessions instead of tabs and no amount of ordering would fix it.
//!
//! `Window::dispatch_key_event` runs capture-phase key listeners **before**
//! it resolves any binding, so a capture listener on the pane's root wins
//! whenever the keyboard is inside the pane, and touches nothing when it is
//! not. One mechanism for every chord, and no dependence on what the rest of
//! the app binds.

use std::sync::atomic::{AtomicU64, Ordering};

use gpui::{prelude::*, Context, Focusable as _, KeyDownEvent, Window};

use super::{backend, state, BrowserPane};

/// What a chord means to the pane. `ctrl` on Windows and Linux, `cmd` on
/// macOS: the owner uses both machines, and a browser that ignores cmd-t on
/// a Mac is not a browser (haktui's `input/commands.rs` takes the same view).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Chord {
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    FindInPage,
    ZoomIn,
    ZoomOut,
    ZoomReset,
}

fn chord(event: &KeyDownEvent) -> Option<Chord> {
    let ks = &event.keystroke;
    if !(ks.modifiers.control || ks.modifiers.platform) || ks.modifiers.alt {
        return None;
    }
    Some(match ks.key.as_str() {
        "t" => Chord::NewTab,
        "w" => Chord::CloseTab,
        "f" => Chord::FindInPage,
        "tab" if ks.modifiers.shift => Chord::PreviousTab,
        "tab" => Chord::NextTab,
        // gpui resolves ctrl-shift-= to "+" and clears shift, so both
        // spellings of zoom-in arrive here and both are Chrome's.
        "=" | "+" => Chord::ZoomIn,
        "-" | "_" => Chord::ZoomOut,
        "0" => Chord::ZoomReset,
        _ => return None,
    })
}

/// `SURYA_TRACE_BROWSER_UI=1` prints every key the pane's root is handed,
/// which is how a proof run tells a chord that never matched from one that
/// matched and did nothing.
fn tracing() -> bool {
    std::env::var_os("SURYA_TRACE_BROWSER_UI").is_some()
}

/// The pane's root: the chords in the capture phase, and the enter and
/// escape the two fields deliberately leave unbound, in the bubble phase.
pub fn bind(root: gpui::Div, cx: &mut Context<BrowserPane>) -> gpui::Div {
    root.capture_key_down(cx.listener(BrowserPane::on_chord))
        .on_key_down(cx.listener(BrowserPane::on_key))
}

/// Proof counters. `keys` is every chord and field key the pane was handed,
/// `handled` the ones it acted on; `typed` is every address submitted from
/// the bar and `navigated` the ones that became a load.
static KEYS: AtomicU64 = AtomicU64::new(0);
static KEYS_HANDLED: AtomicU64 = AtomicU64::new(0);
static TYPED: AtomicU64 = AtomicU64::new(0);
static NAVIGATED: AtomicU64 = AtomicU64::new(0);
static TABS_OPENED: AtomicU64 = AtomicU64::new(0);
static TABS_CLOSED: AtomicU64 = AtomicU64::new(0);
static FINDS: AtomicU64 = AtomicU64::new(0);
static ZOOMS: AtomicU64 = AtomicU64::new(0);

pub(super) fn key_seen() {
    KEYS.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn key_handled() {
    KEYS_HANDLED.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn typed() {
    TYPED.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn navigated() {
    NAVIGATED.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn tab_opened() {
    TABS_OPENED.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn tab_closed() {
    TABS_CLOSED.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn found() {
    FINDS.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn zoomed() {
    ZOOMS.fetch_add(1, Ordering::Relaxed);
}

/// One line, printed after every action the pane takes, so a proof run can
/// be read off stdout without a screenshot.
pub fn counters() -> String {
    format!(
        "keys={} handled={} typed={} navigated={} tabs_opened={} tabs_closed={} finds={} zooms={}",
        KEYS.load(Ordering::Relaxed),
        KEYS_HANDLED.load(Ordering::Relaxed),
        TYPED.load(Ordering::Relaxed),
        NAVIGATED.load(Ordering::Relaxed),
        TABS_OPENED.load(Ordering::Relaxed),
        TABS_CLOSED.load(Ordering::Relaxed),
        FINDS.load(Ordering::Relaxed),
        ZOOMS.load(Ordering::Relaxed),
    )
}

pub(super) fn report(what: &str) {
    println!("browser-ui: {what}; {}", counters());
}

impl BrowserPane {
    /// Capture phase: the pane's own chords, before any keymap binding and
    /// before the page's key path in `surya_browser::panel`.
    fn on_chord(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(chord) = chord(event) else { return };
        key_seen();
        if tracing() {
            println!("browser-ui: chord {chord:?}");
        }
        match chord {
            Chord::NewTab => self.open_tab(window, cx),
            Chord::CloseTab => {
                let Some(active) = backend::active_tab() else { return };
                self.close(active, cx);
            }
            Chord::NextTab => {
                if !self.step_tab(1, cx) {
                    return;
                }
            }
            Chord::PreviousTab => {
                if !self.step_tab(-1, cx) {
                    return;
                }
            }
            Chord::FindInPage => self.open_find(window, cx),
            Chord::ZoomIn => self.step_zoom(state::zoom_in(self.zoom_percent), cx),
            Chord::ZoomOut => self.step_zoom(state::zoom_out(self.zoom_percent), cx),
            Chord::ZoomReset => self.step_zoom(state::ZOOM_DEFAULT, cx),
        }
        key_handled();
        // The chord was the pane's, so nothing else sees it: not the page,
        // and not comet's own context-less bindings for ctrl-tab and cmd-w.
        cx.stop_propagation();
    }

    /// Enter, shift-enter and escape are unbound in the "PaletteSearch"
    /// context the two fields use, so they bubble to here.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if !matches!(key, "enter" | "escape") {
            return;
        }
        let url_focused = self.url.focus_handle(cx).is_focused(window);
        let find_focused = self.find_input.focus_handle(cx).is_focused(window);
        if !url_focused && !find_focused {
            return;
        }
        key_seen();
        match (key, find_focused) {
            ("enter", true) => self.step_find(!event.keystroke.modifiers.shift, cx),
            ("enter", false) => self.submit_url(cx),
            ("escape", true) => self.close_find(window, cx),
            ("escape", false) => self.restore_url(cx),
            _ => return,
        }
        key_handled();
        cx.stop_propagation();
    }
}
