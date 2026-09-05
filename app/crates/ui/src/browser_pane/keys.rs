//! The chords the pane owns, and the counters that prove they ran.
//!
//! gpui dispatches matched key bindings **before** raw key listeners
//! (`window.rs` `dispatch_key_event`), so a binding here wins over both the
//! page's own key path in `surya_browser::panel` and the input's editing
//! keys. That is why ctrl-t works while the keyboard is in the address
//! field, and why ctrl-f is not swallowed by the page.

use std::sync::atomic::{AtomicU64, Ordering};

use gpui::{actions, prelude::*, App, Context, Focusable as _, KeyBinding, KeyDownEvent, Window};

use super::{backend, BrowserPane};

/// The key context the chords are bound in. It sits on the pane's root, so
/// a chord fires whether the keyboard is in the page, the address field or
/// the find field.
const KEY_CONTEXT: &str = "BrowserPane";

actions!(
    browser_pane,
    [NewTab, CloseTab, NextTab, PreviousTab, FindInPage, ZoomIn, ZoomOut, ZoomReset]
);

/// Bind the pane's keymap. Called once at app boot, beside `composer::init`.
pub fn init(cx: &mut App) {
    let ctx = Some(KEY_CONTEXT);
    let mut bindings = Vec::new();
    // ctrl on Windows and Linux, cmd on macOS. Both are bound: the owner
    // uses both machines, and a browser that ignores cmd-t on a Mac is not
    // a browser (haktui `input/commands.rs` takes the same view).
    for prefix in ["cmd", "ctrl"] {
        bindings.push(KeyBinding::new(&format!("{prefix}-t"), NewTab, ctx));
        bindings.push(KeyBinding::new(&format!("{prefix}-w"), CloseTab, ctx));
        bindings.push(KeyBinding::new(&format!("{prefix}-f"), FindInPage, ctx));
        bindings.push(KeyBinding::new(&format!("{prefix}-tab"), NextTab, ctx));
        bindings.push(KeyBinding::new(&format!("shift-{prefix}-tab"), PreviousTab, ctx));
        // gpui resolves ctrl-shift-= to "+" and clears shift, so both
        // spellings of zoom-in arrive and both are Chrome's.
        bindings.push(KeyBinding::new(&format!("{prefix}-="), ZoomIn, ctx));
        bindings.push(KeyBinding::new(&format!("{prefix}-+"), ZoomIn, ctx));
        bindings.push(KeyBinding::new(&format!("{prefix}--"), ZoomOut, ctx));
        bindings.push(KeyBinding::new(&format!("{prefix}-0"), ZoomReset, ctx));
    }
    println!("browser-ui: keymap asked={n} bound={n}", n = bindings.len());
    cx.bind_keys(bindings);
}

/// `SURYA_TRACE_BROWSER_UI=1` prints every key the pane's root is handed,
/// which is how a proof run tells a chord that never matched from one that
/// matched and did nothing.
fn tracing() -> bool {
    std::env::var_os("SURYA_TRACE_BROWSER_UI").is_some()
}

/// The pane's root: its key context, its action handlers, and the enter and
/// escape the two fields deliberately leave unbound.
pub fn bind(root: gpui::Div, cx: &mut Context<BrowserPane>) -> gpui::Div {
    root.key_context(KEY_CONTEXT)
        .on_action(cx.listener(BrowserPane::new_tab))
        .on_action(cx.listener(BrowserPane::close_tab))
        .on_action(cx.listener(BrowserPane::next_tab))
        .on_action(cx.listener(BrowserPane::previous_tab))
        .on_action(cx.listener(BrowserPane::find_in_page))
        .on_action(cx.listener(BrowserPane::zoom_in))
        .on_action(cx.listener(BrowserPane::zoom_out))
        .on_action(cx.listener(BrowserPane::zoom_reset))
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
    fn new_tab(&mut self, _: &NewTab, window: &mut Window, cx: &mut Context<Self>) {
        key_seen();
        key_handled();
        self.open_tab(window, cx);
    }

    fn close_tab(&mut self, _: &CloseTab, _: &mut Window, cx: &mut Context<Self>) {
        key_seen();
        let Some(active) = backend::active_tab() else { return };
        key_handled();
        self.close(active, cx);
    }

    fn next_tab(&mut self, _: &NextTab, _: &mut Window, cx: &mut Context<Self>) {
        self.step_tab(1, cx);
    }

    fn previous_tab(&mut self, _: &PreviousTab, _: &mut Window, cx: &mut Context<Self>) {
        self.step_tab(-1, cx);
    }

    fn find_in_page(&mut self, _: &FindInPage, window: &mut Window, cx: &mut Context<Self>) {
        key_seen();
        key_handled();
        self.open_find(window, cx);
    }

    fn zoom_in(&mut self, _: &ZoomIn, _: &mut Window, cx: &mut Context<Self>) {
        key_seen();
        key_handled();
        self.step_zoom(super::state::zoom_in(self.zoom_percent), cx);
    }

    fn zoom_out(&mut self, _: &ZoomOut, _: &mut Window, cx: &mut Context<Self>) {
        key_seen();
        key_handled();
        self.step_zoom(super::state::zoom_out(self.zoom_percent), cx);
    }

    pub(super) fn zoom_reset(&mut self, _: &ZoomReset, _: &mut Window, cx: &mut Context<Self>) {
        key_seen();
        key_handled();
        self.step_zoom(super::state::ZOOM_DEFAULT, cx);
    }

    /// Enter, shift-enter and escape are unbound in the "PaletteSearch"
    /// context the two fields use, so they bubble to here.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if tracing() {
            println!("browser-ui: key {:?} mods={:?}", key, event.keystroke.modifiers);
        }
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
