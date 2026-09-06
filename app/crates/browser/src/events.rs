//! gpui events into CEF: clicks, moves, the wheel and keys. Ported from
//! haktui's `osr_input.rs` (2026-09-05) without pins, phone emulation, the
//! context menu and tabs.
//!
//! An offscreen browser has no window of its own: nothing converts a click
//! into a page coordinate and nothing tracks which buttons are held. This
//! module is that bookkeeping, as integers, with `input` doing the
//! translation and the tests beside it.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};

use cef::{ImplBrowserHost as _, KeyEvent, KeyEventType, MouseButtonType, MouseEvent};
use gpui::{KeyDownEvent, KeyUpEvent, Modifiers, MouseDownEvent, MouseUpEvent, Pixels, Point,
    ScrollWheelEvent};

use crate::input::{self, Command};

// CEF's own numbers against ours. `input` cannot see `cef::sys`, so this is
// where the two are held together: a CEF bump that moves a bit fails the
// build instead of silently sending the wrong click.
const _: () = {
    use cef::sys::cef_event_flags_t as F;
    assert!(input::flags::SHIFT_DOWN == F::EVENTFLAG_SHIFT_DOWN.0 as u32);
    assert!(input::flags::CONTROL_DOWN == F::EVENTFLAG_CONTROL_DOWN.0 as u32);
    assert!(input::flags::ALT_DOWN == F::EVENTFLAG_ALT_DOWN.0 as u32);
    assert!(input::flags::LEFT_MOUSE_BUTTON == F::EVENTFLAG_LEFT_MOUSE_BUTTON.0 as u32);
    assert!(input::flags::MIDDLE_MOUSE_BUTTON == F::EVENTFLAG_MIDDLE_MOUSE_BUTTON.0 as u32);
    assert!(input::flags::RIGHT_MOUSE_BUTTON == F::EVENTFLAG_RIGHT_MOUSE_BUTTON.0 as u32);
    assert!(input::flags::COMMAND_DOWN == F::EVENTFLAG_COMMAND_DOWN.0 as u32);
    assert!(input::flags::CAPS_LOCK_ON == F::EVENTFLAG_CAPS_LOCK_ON.0 as u32);
    assert!(input::flags::NUM_LOCK_ON == F::EVENTFLAG_NUM_LOCK_ON.0 as u32);
    assert!(input::flags::IS_KEY_PAD == F::EVENTFLAG_IS_KEY_PAD.0 as u32);
    assert!(input::flags::IS_LEFT == F::EVENTFLAG_IS_LEFT.0 as u32);
    assert!(input::flags::IS_RIGHT == F::EVENTFLAG_IS_RIGHT.0 as u32);
    assert!(input::flags::ALTGR_DOWN == F::EVENTFLAG_ALTGR_DOWN.0 as u32);
    assert!(input::flags::IS_REPEAT == F::EVENTFLAG_IS_REPEAT.0 as u32);
    assert!(input::flags::PRECISION_SCROLLING_DELTA == F::EVENTFLAG_PRECISION_SCROLLING_DELTA.0 as u32);
};

/// Where the surface starts, in window DIPs. Written every frame by the
/// paint callback in `surface.rs`, which is the only code that knows.
static ORIGIN_X: AtomicI32 = AtomicI32::new(0);
static ORIGIN_Y: AtomicI32 = AtomicI32::new(0);
/// Which mouse buttons are held, as CEF modifier bits.
static BUTTONS: AtomicU32 = AtomicU32::new(0);
/// Whether the last move was over the surface, for the one leave event.
static INSIDE: AtomicBool = AtomicBool::new(false);

static MOUSE_SEEN: AtomicU64 = AtomicU64::new(0);
static MOUSE_SENT: AtomicU64 = AtomicU64::new(0);
static KEYS_SEEN: AtomicU64 = AtomicU64::new(0);
static KEYS_SENT: AtomicU64 = AtomicU64::new(0);
static WHEEL_SEEN: AtomicU64 = AtomicU64::new(0);
static WHEEL_SENT: AtomicU64 = AtomicU64::new(0);
static COMMANDS: AtomicU64 = AtomicU64::new(0);

fn tracing() -> bool {
    std::env::var_os("SURYA_TRACE_INPUT").is_some()
}

/// The input counters as pairs.
pub fn counters() -> String {
    format!(
        "mouse_seen={} mouse_sent={} keys_seen={} keys_sent={} wheel_seen={} wheel_sent={} commands={}",
        MOUSE_SEEN.load(Ordering::Relaxed),
        MOUSE_SENT.load(Ordering::Relaxed),
        KEYS_SEEN.load(Ordering::Relaxed),
        KEYS_SENT.load(Ordering::Relaxed),
        WHEEL_SEEN.load(Ordering::Relaxed),
        WHEEL_SENT.load(Ordering::Relaxed),
        COMMANDS.load(Ordering::Relaxed),
    )
}

pub(crate) fn set_origin(x: i32, y: i32) {
    ORIGIN_X.store(x, Ordering::Release);
    ORIGIN_Y.store(y, Ordering::Release);
}

fn origin() -> (i32, i32) {
    (ORIGIN_X.load(Ordering::Acquire), ORIGIN_Y.load(Ordering::Acquire))
}

/// Run `f` against the active tab's host on CEF's UI thread. Returns
/// whether there was a browser to send to. Callers are on gpui's main
/// thread; `f` captures the event by value and runs inline or posted
/// (cef_thread.rs).
fn with_host(f: impl FnOnce(&cef::BrowserHost) + Send + 'static) -> bool {
    crate::pump::mark_input();
    let id = crate::tabs::active_browser();
    if crate::client::browser_of(id).is_none() {
        return false;
    }
    crate::cef_thread::on_ui(move || {
        if let Some(host) = crate::client::host_of(id) {
            f(&host);
        }
    });
    true
}

fn event_at(pos: Point<Pixels>, modifiers: u32) -> MouseEvent {
    let (x, y) = input::view_point(pos, origin());
    MouseEvent { x, y, modifiers }
}

fn cef_button(b: input::Button) -> MouseButtonType {
    match b {
        input::Button::Left => MouseButtonType::LEFT,
        input::Button::Middle => MouseButtonType::MIDDLE,
        input::Button::Right => MouseButtonType::RIGHT,
    }
}

/// The page has the keyboard, or does not. CEF needs telling: without it a
/// text box shows no caret and the page never fires a focus event.
pub fn set_focus(on: bool) {
    if !on {
        // A held button plus a lost window is how a page gets stuck mid-drag.
        BUTTONS.store(0, Ordering::Release);
        with_host(|h| h.send_capture_lost_event());
    }
    with_host(move |h| h.set_focus(i32::from(on)));
}

pub fn mouse_down(e: &MouseDownEvent) {
    MOUSE_SEEN.fetch_add(1, Ordering::Relaxed);
    if let Some(command) = input::mouse_shortcut(e.button) {
        run(command);
        return;
    }
    let Some(button) = input::button_type(e.button) else { return };
    let bit = input::button_bit(e.button).unwrap_or(0);
    let held = BUTTONS.fetch_or(bit, Ordering::AcqRel) | bit;
    let event = event_at(e.position, input::mouse_flags(&e.modifiers, held));
    if tracing() {
        println!("input: down {:?} at {},{} click_count={}", e.button, event.x, event.y, e.click_count);
    }
    // `click_count` is gpui's, and it is what makes a double click select a
    // word and a triple click select a line.
    let (btn, clicks) = (cef_button(button), e.click_count as i32);
    if with_host(move |h| h.send_mouse_click_event(Some(&event), btn, 0, clicks)) {
        MOUSE_SENT.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn mouse_up(e: &MouseUpEvent) {
    MOUSE_SEEN.fetch_add(1, Ordering::Relaxed);
    let Some(button) = input::button_type(e.button) else { return };
    let bit = input::button_bit(e.button).unwrap_or(0);
    // Clear before reading: a mouse-up that still claims the button is down
    // leaves a page mid-drag.
    let before = BUTTONS.fetch_and(!bit, Ordering::AcqRel);
    // Only a press that came through here gets its release. The element
    // listens with `on_mouse_up_out` so a drag that leaves the pane still
    // ends, and gpui fires that for ANY release outside, whoever saw the
    // press (haktui, 2026-08-27: a stray right-up opened Chromium's menu).
    if before & bit == 0 {
        if tracing() {
            println!("input: up {:?} with no press seen, not forwarded", e.button);
        }
        return;
    }
    let held = before & !bit;
    let event = event_at(e.position, input::mouse_flags(&e.modifiers, held));
    let (btn, clicks) = (cef_button(button), e.click_count as i32);
    if with_host(move |h| h.send_mouse_click_event(Some(&event), btn, 1, clicks)) {
        MOUSE_SENT.fetch_add(1, Ordering::Relaxed);
    }
}

/// Whether the last move left the pointer inside the page area.
pub(crate) fn inside() -> bool {
    INSIDE.load(Ordering::Acquire)
}

/// A pointer move. Delivered while inside the surface or while a button is
/// held (a drag that leaves the pane keeps selecting); one move with
/// `mouse_leave` set when the pointer leaves.
pub fn mouse_moved(pos: Point<Pixels>, modifiers: &Modifiers) {
    let held = BUTTONS.load(Ordering::Acquire);
    let (x, y) = input::view_point(pos, origin());
    let width = crate::render::VIEW_W.load(Ordering::Acquire);
    let height = crate::render::VIEW_H.load(Ordering::Acquire);
    let inside = x >= 0 && y >= 0 && x < width && y < height;
    let was_inside = INSIDE.swap(inside, Ordering::AcqRel);
    let leave = if held != 0 || inside {
        0
    } else if was_inside {
        1
    } else {
        return;
    };
    MOUSE_SEEN.fetch_add(1, Ordering::Relaxed);
    let event = MouseEvent { x, y, modifiers: input::mouse_flags(modifiers, held) };
    if with_host(move |h| h.send_mouse_move_event(Some(&event), leave)) {
        MOUSE_SENT.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn scroll(e: &ScrollWheelEvent) {
    WHEEL_SEEN.fetch_add(1, Ordering::Relaxed);
    let (dx, dy, modifiers) = input::wheel(e.delta, &e.modifiers);
    let event = event_at(e.position, modifiers | BUTTONS.load(Ordering::Acquire));
    if tracing() {
        println!("input: wheel {dx},{dy} modifiers={modifiers:#x}");
    }
    if with_host(move |h| h.send_mouse_wheel_event(Some(&event), dx, dy)) {
        WHEEL_SENT.fetch_add(1, Ordering::Relaxed);
    }
}

/// Returns true when the pane acted on the key itself and the page must not
/// also see it. F5 that both reloads and fires a keydown is not Chrome.
pub fn key_down(e: &KeyDownEvent) -> bool {
    KEYS_SEEN.fetch_add(1, Ordering::Relaxed);
    if let Some(command) = input::shortcut(&e.keystroke) {
        run(command);
        return true;
    }
    let ks = &e.keystroke;
    let Some(code) = input::key_code(&ks.key) else {
        if tracing() {
            println!("input: no virtual key for {:?}", ks.key);
        }
        return false;
    };
    let mut modifiers = input::key_flags(&ks.modifiers, &ks.key);
    if e.is_held {
        modifiers |= input::flags::IS_REPEAT;
    }
    // RAWKEYDOWN drives shortcuts and the non-character keys. CHAR is what
    // types. Both, in this order, is what a key press looks like to
    // Chromium; either alone leaves half the keyboard dead.
    let Some(native) = native_key(ks) else { return false };
    let unit = input::char_unit(ks);
    let down = KeyEvent {
        type_: KeyEventType::RAWKEYDOWN,
        modifiers,
        windows_key_code: code,
        native_key_code: native.code,
        character: native.character,
        unmodified_character: native.unmodified,
        ..Default::default()
    };
    let sent = with_host(move |h| h.send_key_event(Some(&down)));
    if let Some(unit) = unit {
        let typed = KeyEvent {
            type_: KeyEventType::CHAR,
            modifiers,
            // For a CHAR event Chromium reads the character out of
            // `windows_key_code`, not the virtual key.
            windows_key_code: i32::from(unit),
            native_key_code: native.code,
            character: unit,
            unmodified_character: unit,
            ..Default::default()
        };
        with_host(move |h| h.send_key_event(Some(&typed)));
    }
    if sent {
        KEYS_SENT.fetch_add(1, Ordering::Relaxed);
    }
    false
}

pub fn key_up(e: &KeyUpEvent) {
    let ks = &e.keystroke;
    if input::shortcut(ks).is_some() {
        return;
    }
    let Some(code) = input::key_code(&ks.key) else { return };
    let Some(native) = native_key(ks) else { return };
    let event = KeyEvent {
        type_: KeyEventType::KEYUP,
        modifiers: input::key_flags(&ks.modifiers, &ks.key),
        windows_key_code: code,
        native_key_code: native.code,
        character: native.character,
        unmodified_character: native.unmodified,
        ..Default::default()
    };
    with_host(move |h| h.send_key_event(Some(&event)));
}

/// The platform's half of a key event. Windows and Linux send none; the Mac
/// sends `input::mac_key`'s, and `None` for a key it has no code for.
struct NativeKey {
    code: i32,
    character: u16,
    unmodified: u16,
}

#[cfg(target_os = "macos")]
fn native_key(ks: &gpui::Keystroke) -> Option<NativeKey> {
    let k = input::mac_key(ks)?;
    Some(NativeKey { code: i32::from(k.code), character: k.character, unmodified: k.unmodified })
}

#[cfg(not(target_os = "macos"))]
fn native_key(_: &gpui::Keystroke) -> Option<NativeKey> {
    Some(NativeKey { code: 0, character: 0, unmodified: 0 })
}

/// A shortcut the pane owns. Zoom, find and tabs are the next seat's.
fn run(command: Command) {
    COMMANDS.fetch_add(1, Ordering::Relaxed);
    if tracing() {
        println!("input: command {command:?}");
    }
    match command {
        Command::Back => crate::back(),
        Command::Forward => crate::forward(),
        Command::Reload => crate::reload(),
        Command::ReloadIgnoringCache => crate::reload_ignoring_cache(),
        Command::ZoomIn
        | Command::ZoomOut
        | Command::ZoomReset
        | Command::NewTab
        | Command::CloseTab
        | Command::NextTab
        | Command::PreviousTab
        | Command::ToggleMobile => {}
    }
}
