//! The chords the pane owns, and the counters that prove they ran.
//!
//! Getting a chord to the pane at all took three tries, so the rule is
//! written down here rather than rediscovered. Read from gpui's source, not
//! from memory:
//!
//! * `Window::dispatch_key_event` resolves keymap **bindings first**, and
//!   only calls `finish_dispatch_key_event` - which runs capture-phase and
//!   then bubble-phase key listeners - if no binding consumed the event. So
//!   a listener of any phase cannot beat a binding. A capture listener here
//!   was silently skipped for ctrl-tab, and only for ctrl-tab, because that
//!   is the one chord comet already binds.
//! * `Keymap::binding_enabled` gives a binding with **no context** the depth
//!   `contexts.len()`, the deepest there is, while a binding with a context
//!   gets the depth its context matched at. `bindings_for_input` sorts on
//!   depth first, so a `BrowserPane`-scoped binding loses to comet's
//!   context-less `ctrl-tab` (NextSession) and `cmd-w` (CloseWindow)
//!   whenever the pane's context is not last on the stack - which is exactly
//!   the case while the keyboard is in the address field.
//! * Ties at equal depth break by insertion order, later wins
//!   (`ix_b.cmp(ix_a)`).
//!
//! So the pane's chords are bound with **no context, after comet's**, from
//! the end of `shell::apply_keymap`. Equal depth, later insertion, so they
//! sort first. Scoping then comes from the handlers, not the binding:
//! `dispatch_action_on_node_inner` sets `propagate_event = true` before
//! walking the focused path, so when the pane is not focused there is no
//! `on_action` for these on that path, propagation survives, and comet's own
//! binding runs next. The pane takes ctrl-tab only when the keyboard is
//! inside it.

use std::sync::atomic::{AtomicU64, Ordering};

use gpui::{actions, prelude::*, App, Context, Focusable as _, KeyBinding, KeyDownEvent, Window};

use super::{backend, state, BrowserPane};

actions!(
    browser_pane,
    [NewTab, CloseTab, NextTab, PreviousTab, FindInPage, ZoomIn, ZoomOut, ZoomReset]
);

/// Bind the pane's chords. Called from the END of `shell::apply_keymap`,
/// which clears the whole keymap and rebuilds it whenever the shortcuts
/// change; being last is what makes these sort ahead of comet's own
/// context-less bindings. See the module header for why.
pub fn init(cx: &mut App) {
    // ctrl on Windows and Linux, cmd on macOS. Both, because the owner uses
    // both machines and a browser that ignores cmd-t on a Mac is not a
    // browser (haktui's `input/commands.rs` takes the same view).
    let mut bindings = Vec::new();
    for prefix in ["cmd", "ctrl"] {
        bindings.push(KeyBinding::new(&format!("{prefix}-t"), NewTab, None));
        bindings.push(KeyBinding::new(&format!("{prefix}-w"), CloseTab, None));
        bindings.push(KeyBinding::new(&format!("{prefix}-f"), FindInPage, None));
        bindings.push(KeyBinding::new(&format!("{prefix}-tab"), NextTab, None));
        bindings.push(KeyBinding::new(&format!("shift-{prefix}-tab"), PreviousTab, None));
        // gpui resolves ctrl-shift-= to "+" and clears shift, so both
        // spellings of zoom-in arrive and both are Chrome's.
        bindings.push(KeyBinding::new(&format!("{prefix}-="), ZoomIn, None));
        bindings.push(KeyBinding::new(&format!("{prefix}-+"), ZoomIn, None));
        bindings.push(KeyBinding::new(&format!("{prefix}--"), ZoomOut, None));
        bindings.push(KeyBinding::new(&format!("{prefix}-0"), ZoomReset, None));
    }
    let n = bindings.len();
    cx.bind_keys(bindings);
    tracing::info!(target: "surya_browser_ui", asked = n, bound = n, "keymap applied");
}

/// The pane's root: the action handlers, whose presence on the focused
/// dispatch path is what scopes the chords to this pane, and the enter and
/// escape the two fields deliberately leave unbound.
pub fn bind(root: gpui::Div, cx: &mut Context<BrowserPane>) -> gpui::Div {
    root.on_action(cx.listener(BrowserPane::act_new_tab))
        .on_action(cx.listener(BrowserPane::act_close_tab))
        .on_action(cx.listener(BrowserPane::act_next_tab))
        .on_action(cx.listener(BrowserPane::act_previous_tab))
        .on_action(cx.listener(BrowserPane::act_find))
        .on_action(cx.listener(BrowserPane::act_zoom_in))
        .on_action(cx.listener(BrowserPane::act_zoom_out))
        .on_action(cx.listener(BrowserPane::act_zoom_reset))
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

/// The counters, as one string, logged after every action the pane takes so
/// a proof run can be read from the log without a screenshot.
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

/// One line per thing the pane did, with the counters beside it. `info`, not
/// stdout: the ui crate logs through tracing, and a shipped build should not
/// print on every chord.
pub(super) fn report(what: &str) {
    tracing::info!(target: "surya_browser_ui", action = what, counters = %counters(), "pane acted");
}

impl BrowserPane {
    fn act_new_tab(&mut self, _: &NewTab, window: &mut Window, cx: &mut Context<Self>) {
        self.chord("NewTab", cx);
        self.open_tab(window, cx);
    }

    fn act_close_tab(&mut self, _: &CloseTab, _: &mut Window, cx: &mut Context<Self>) {
        self.chord("CloseTab", cx);
        let Some(active) = backend::active_tab() else { return };
        self.close(active, cx);
    }

    fn act_next_tab(&mut self, _: &NextTab, _: &mut Window, cx: &mut Context<Self>) {
        self.chord("NextTab", cx);
        self.step_tab(1, cx);
    }

    fn act_previous_tab(&mut self, _: &PreviousTab, _: &mut Window, cx: &mut Context<Self>) {
        self.chord("PreviousTab", cx);
        self.step_tab(-1, cx);
    }

    fn act_find(&mut self, _: &FindInPage, window: &mut Window, cx: &mut Context<Self>) {
        self.chord("FindInPage", cx);
        self.open_find(window, cx);
    }

    fn act_zoom_in(&mut self, _: &ZoomIn, _: &mut Window, cx: &mut Context<Self>) {
        self.chord("ZoomIn", cx);
        self.step_zoom(state::zoom_in(self.zoom_percent), cx);
    }

    fn act_zoom_out(&mut self, _: &ZoomOut, _: &mut Window, cx: &mut Context<Self>) {
        self.chord("ZoomOut", cx);
        self.step_zoom(state::zoom_out(self.zoom_percent), cx);
    }

    fn act_zoom_reset(&mut self, _: &ZoomReset, _: &mut Window, cx: &mut Context<Self>) {
        self.chord("ZoomReset", cx);
        self.step_zoom(state::ZOOM_DEFAULT, cx);
    }

    /// Every chord handler starts here: count it, trace it, and take it off
    /// the event so comet's own binding for the same keys does not also run.
    fn chord(&mut self, name: &str, cx: &mut Context<Self>) {
        key_seen();
        key_handled();
        // `debug`, because a chord that fires is only interesting while
        // something is wrong; RUST_LOG turns it on instead of an env var of
        // this module's own.
        tracing::debug!(target: "surya_browser_ui", chord = name, "chord taken");
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
            ("enter", false) => self.submit_url(window, cx),
            ("escape", true) => self.close_find(window, cx),
            ("escape", false) => self.restore_url(cx),
            _ => return,
        }
        key_handled();
        cx.stop_propagation();
    }
}
