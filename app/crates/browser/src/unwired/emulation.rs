//! Mobile view mode, the half that talks to CEF. Windows only.
//!
//! `device.rs` decides the viewport and the CDP payloads. This file holds the
//! switch, sends the payloads through `execute_dev_tools_method` (in-process,
//! no websocket, same route as the pin), turns the owner's mouse into touches,
//! and remembers where the pointer is so the shell can draw the circle.
//!
//! **The mode is one switch for every tab.** DevTools emulates per target, so
//! a tab opened while the mode is on is put into it as it is created, and the
//! toggle walks every open browser. A mode that applied to one tab and not the
//! next would read as a bug in the page.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};

use cef::{ImplBrowser as _, ImplBrowserHost as _, ImplDictionaryValue as _};

use super::device;

static ON: AtomicBool = AtomicBool::new(false);

/// Emulation message ids start here so the pin's observer can tell them from
/// its own, which start at 1 and will never reach this.
pub(super) const ID_BASE: i32 = 1_000_000;
static NEXT_ID: AtomicI32 = AtomicI32::new(ID_BASE);

// Counters, always printed as pairs. `asked=4 answered=4 failed=0` is a mode
// that took; `asked=4 answered=0` is a message that never came back, which is
// what a wrong method name looks like from here.
static EMU_ASKED: AtomicU64 = AtomicU64::new(0);
static EMU_ANSWERED: AtomicU64 = AtomicU64::new(0);
static EMU_FAILED: AtomicU64 = AtomicU64::new(0);
/// Touches: what the mouse produced against what reached a browser.
static TOUCH_SEEN: AtomicU64 = AtomicU64::new(0);
static TOUCH_SENT: AtomicU64 = AtomicU64::new(0);

/// Where the pointer was last seen, in window DIPs, for the circle. `INSIDE`
/// is whether that was over the phone viewport; off it there is no circle.
static POINTER_X: AtomicI32 = AtomicI32::new(0);
static POINTER_Y: AtomicI32 = AtomicI32::new(0);
static POINTER_INSIDE: AtomicBool = AtomicBool::new(false);
/// A touch is down. Set on the press that became a touch and cleared on the
/// release, so a toggle in the middle of a press still releases the touch it
/// started rather than sending the page a mouse-up for a touch it never saw.
static TOUCH_HELD: AtomicBool = AtomicBool::new(false);

pub fn on() -> bool {
    ON.load(Ordering::Acquire)
}

pub fn counters() -> String {
    format!(
        "mobile={} emu_asked={} emu_answered={} emu_failed={} touch_seen={} touch_sent={}",
        u8::from(on()),
        EMU_ASKED.load(Ordering::Relaxed),
        EMU_ANSWERED.load(Ordering::Relaxed),
        EMU_FAILED.load(Ordering::Relaxed),
        TOUCH_SEEN.load(Ordering::Relaxed),
        TOUCH_SENT.load(Ordering::Relaxed),
    )
}

/// The viewport the mode gives CEF right now, from the column's live size.
pub(super) fn viewport() -> device::Fit {
    device::fit(
        super::osr::COLUMN_W.load(Ordering::Acquire),
        super::osr::COLUMN_H.load(Ordering::Acquire),
    )
}

/// Flip the mode and tell every open browser.
pub fn toggle() {
    let now = !ON.fetch_xor(true, Ordering::AcqRel);
    let fit = viewport();
    println!("mobile: {} viewport {}x{}", if now { "on" } else { "off" }, fit.w, fit.h);
    // The page under the pointer was hovering. A phone does not hover, and
    // the desktop page that comes back should not think the pointer is
    // still where it was.
    if let Some(host) = super::osr::active_browser().and_then(|b| b.host()) {
        let leave = cef::MouseEvent { x: -1, y: -1, modifiers: 0 };
        host.send_mouse_move_event(Some(&leave), 1);
    }
    if TOUCH_HELD.swap(false, Ordering::AcqRel) {
        send_touch(cef::TouchEventType::CANCELLED, 0.0, 0.0);
    }
    // Apply, then RELOAD. Measured 2026-08-27 on the probe page: after the
    // override took, `pointer: coarse` and `maxTouchPoints` changed at once,
    // but `'ontouchstart' in window` stayed false until a reload, because a
    // document decides that once when it is created. A library that picks
    // its input handlers from that would keep the mouse ones. DevTools puts
    // up a "reload to apply" banner for the same reason. Ours reloads.
    if let Ok(browsers) = super::osr::BROWSERS.lock() {
        if let Ok(mut seen) = APPLIED_ON_LOAD.lock() {
            seen.clear();
            if now {
                seen.extend(browsers.iter().map(|b| b.identifier()));
            }
        }
        for b in browsers.iter() {
            if let Some(host) = b.host() {
                apply(&host, now, fit.w, fit.h);
            }
            b.reload();
        }
    }
    // The surface re-measures on the next frame; ask for one now rather than
    // waiting for whatever renders next.
    super::pin::NEEDS_FRAME.store(true, Ordering::Relaxed);
}

/// Put one browser into, or out of, the mode. Called from the toggle and from
/// `on_after_created`, so a tab opened in mobile mode starts in it.
pub(super) fn apply(host: &cef::BrowserHost, on: bool, w: i32, h: i32) {
    for (method, params) in device::cdp_calls(on, w, h) {
        let Some(mut dict) = cef::dictionary_value_create() else {
            println!("mobile: could not build params for {method}");
            continue;
        };
        if let Some(obj) = params.as_object() {
            for (k, v) in obj {
                let key = cef::CefString::from(k.as_str());
                match v {
                    serde_json::Value::Bool(b) => {
                        dict.set_bool(Some(&key), i32::from(*b));
                    }
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            dict.set_int(Some(&key), i as i32);
                        } else if let Some(f) = n.as_f64() {
                            dict.set_double(Some(&key), f);
                        }
                    }
                    serde_json::Value::String(s) => {
                        dict.set_string(Some(&key), Some(&cef::CefString::from(s.as_str())));
                    }
                    _ => println!("mobile: {method}.{k} is a shape this does not send"),
                }
            }
        }
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        EMU_ASKED.fetch_add(1, Ordering::Relaxed);
        host.execute_dev_tools_method(id, Some(&cef::CefString::from(method)), Some(&mut dict));
    }
}

/// Browsers that got the mode on their first load. One reload each, never a
/// loop.
static APPLIED_ON_LOAD: std::sync::Mutex<Vec<i32>> = std::sync::Mutex::new(Vec::new());

/// `HAKTUI_MOBILE_APPLY_EARLY=1` restores the 2026-08-27 order: the DevTools
/// call inside `on_after_created`. That order killed the process, silently,
/// on the owner's desk and in `HAKTUI_SELFTEST_MOBILE` + `HAKTUI_SELFTEST_NEWTAB`
/// (0 processes left, no panic text). Kept as the control for the A/B.
fn apply_early() -> bool {
    std::env::var("HAKTUI_MOBILE_APPLY_EARLY").is_ok_and(|v| v == "1")
}

/// A browser just appeared. With the switch set, apply here; otherwise wait
/// for `on_first_load_end`, when the browser can take a DevTools call.
pub(super) fn apply_current(host: &cef::BrowserHost) {
    if on() && apply_early() {
        println!("mobile: applying inside on_after_created (HAKTUI_MOBILE_APPLY_EARLY)");
        let fit = viewport();
        apply(host, true, fit.w, fit.h);
    }
}

/// A browser finished its first load while the mode is on: apply, then
/// reload once, for the same reason `toggle` reloads. Returns whether it did.
pub(super) fn on_load_end(browser: &cef::Browser) -> bool {
    if !on() || apply_early() {
        return false;
    }
    let id = browser.identifier();
    let Ok(mut seen) = APPLIED_ON_LOAD.lock() else { return false };
    if seen.contains(&id) {
        return false;
    }
    seen.push(id);
    drop(seen);
    let Some(host) = browser.host() else { return false };
    let fit = viewport();
    println!("mobile: tab {id} loaded, applying {}x{} then reloading once", fit.w, fit.h);
    apply(&host, true, fit.w, fit.h);
    browser.reload();
    true
}

/// A browser went away; forget it so its id can be reused cleanly.
pub(super) fn forget(id: i32) {
    if let Ok(mut seen) = APPLIED_ON_LOAD.lock() {
        seen.retain(|b| *b != id);
    }
}

/// A DevTools reply with one of our ids. The pin's observer routes it here.
pub(super) fn on_result(message_id: i32, success: bool) {
    if success {
        EMU_ANSWERED.fetch_add(1, Ordering::Relaxed);
    } else {
        EMU_FAILED.fetch_add(1, Ordering::Relaxed);
        println!("mobile: devtools id={message_id} failed");
    }
}

fn send_touch(type_: cef::TouchEventType, x: f32, y: f32) -> bool {
    let event = cef::TouchEvent {
        id: 0,
        x,
        y,
        radius_x: 0.0,
        radius_y: 0.0,
        rotation_angle: 0.0,
        pressure: 1.0,
        type_,
        modifiers: 0,
        pointer_type: cef::PointerType::TOUCH,
    };
    let Some(host) = super::osr::active_browser().and_then(|b| b.host()) else { return false };
    super::osr::mark_input();
    host.send_touch_event(Some(&event));
    true
}

/// The left button went down over the viewport. Returns true when it became
/// a touch, so the caller sends no mouse event for it.
pub(super) fn press(x: i32, y: i32) -> bool {
    if !on() {
        return false;
    }
    TOUCH_SEEN.fetch_add(1, Ordering::Relaxed);
    TOUCH_HELD.store(true, Ordering::Release);
    if send_touch(cef::TouchEventType::PRESSED, x as f32, y as f32) {
        TOUCH_SENT.fetch_add(1, Ordering::Relaxed);
    }
    true
}

/// The left button came up. Returns true when it released a touch.
pub(super) fn release(x: i32, y: i32) -> bool {
    if !TOUCH_HELD.swap(false, Ordering::AcqRel) {
        return false;
    }
    TOUCH_SEEN.fetch_add(1, Ordering::Relaxed);
    if send_touch(cef::TouchEventType::RELEASED, x as f32, y as f32) {
        TOUCH_SENT.fetch_add(1, Ordering::Relaxed);
    }
    true
}

/// The pointer moved. In mobile mode a page never sees a move unless a touch
/// is down: a phone has no hover. Returns true when the mode consumed it.
pub(super) fn moved(window_x: i32, window_y: i32, view_x: i32, view_y: i32, inside: bool) -> bool {
    if !on() {
        return false;
    }
    POINTER_X.store(window_x, Ordering::Relaxed);
    POINTER_Y.store(window_y, Ordering::Relaxed);
    POINTER_INSIDE.store(inside, Ordering::Relaxed);
    if TOUCH_HELD.load(Ordering::Acquire) {
        TOUCH_SEEN.fetch_add(1, Ordering::Relaxed);
        if send_touch(cef::TouchEventType::MOVED, view_x as f32, view_y as f32) {
            TOUCH_SENT.fetch_add(1, Ordering::Relaxed);
        }
    }
    true
}

/// Column 3 lost the keyboard, or the window. A touch that is down cannot be
/// left down.
pub(super) fn cancel() {
    if TOUCH_HELD.swap(false, Ordering::AcqRel) {
        send_touch(cef::TouchEventType::CANCELLED, 0.0, 0.0);
    }
}

/// Where to draw the circle, in window DIPs, and whether a touch is down.
/// `None` when the mode is off or the pointer is not over the viewport.
pub fn touch_cursor() -> Option<(f32, f32, bool)> {
    if !on() || !POINTER_INSIDE.load(Ordering::Relaxed) {
        return None;
    }
    Some((
        POINTER_X.load(Ordering::Relaxed) as f32,
        POINTER_Y.load(Ordering::Relaxed) as f32,
        TOUCH_HELD.load(Ordering::Acquire),
    ))
}
