//! Self-tests for the frame path, each behind an environment variable with
//! a start delay in seconds, so a run on a box with no one at the keyboard
//! still measures the real pipeline. Same shapes and the same pages as
//! haktui's (spike-scroll-frame-rate-2026-08-28), so the tables compare.
//!
//! | variable | what |
//! | --- | --- |
//! | `SURYA_SELFTEST_SCROLL=<secs>` | a 40,000 px page, sixty wheel events at 60 a second |
//! | `SURYA_SELFTEST_ANIM=<secs>` | a page that animates on its own, no input, two seconds |
//! | `SURYA_SELFTEST_TIMER=<secs>` | thirty 16 ms timers back to back, gpui's and ours |
//!
//! Every line starts with `selftest:` and carries counted pairs.

mod anim;
mod scroll;
mod timer;

use std::time::Duration;

use cef::*;

/// Read the hooks once; call from `start`, after the pump is installed.
pub(crate) fn install(cx: &mut gpui::App) {
    if let Some(after) = delay("SURYA_SELFTEST_SCROLL") {
        scroll::spawn(cx, after);
    }
    if let Some(after) = delay("SURYA_SELFTEST_ANIM") {
        anim::spawn(cx, after);
    }
    if let Some(after) = delay("SURYA_SELFTEST_TIMER") {
        timer::spawn(cx, after);
    }
}

fn delay(var: &str) -> Option<u64> {
    std::env::var(var).ok().and_then(|v| v.trim().parse::<u64>().ok())
}

async fn sleep(cx: &mut gpui::AsyncApp, d: Duration) {
    cx.background_executor().timer(d).await;
}

/// Load a page straight into the active browser. The address bar refuses
/// `data:` addresses on purpose, so this does not go through [`crate::navigate`].
fn load(url: &str) -> bool {
    let Some(frame) = crate::client::browser().and_then(|b| b.main_frame()) else { return false };
    frame.load_url(Some(&CefString::from(url)));
    crate::pump::schedule_pump(0);
    true
}

/// One wheel notch at the middle of the view, the way a mouse sends it.
fn wheel(dy: i32) -> bool {
    use std::sync::atomic::Ordering;
    let id = crate::tabs::active_browser();
    if crate::client::browser_of(id).is_none() {
        return false;
    }
    let x = crate::render::VIEW_W.load(Ordering::Acquire) / 2;
    let y = crate::render::VIEW_H.load(Ordering::Acquire) / 2;
    // The same marker a real wheel sets on its way to CEF.
    crate::pump::mark_input();
    // On CEF's UI thread, like every shell-side host call (cef_thread.rs).
    crate::cef_thread::on_ui(move || {
        if let Some(host) = crate::client::host_of(id) {
            let ev = MouseEvent { x, y, modifiers: 0 };
            host.send_mouse_wheel_event(Some(&ev), 0, dy);
        }
    });
    true
}

/// Frames, renders and shown frames at one instant, for a before/after pair.
#[derive(Clone, Copy)]
struct Counts {
    cef_frames: u64,
    app_frames: u64,
    shown: u64,
    p2d_mark: u64,
}

fn counts() -> Counts {
    Counts {
        cef_frames: crate::render::frames(),
        app_frames: crate::perf::renders(),
        shown: crate::perf::shown(),
        p2d_mark: crate::perf::mark(),
    }
}

/// `cef_frames= app_frames= shown=` over the window, and the p2d summary.
/// `shown` is the renders that drew a frame the previous render had not:
/// the pair the tables want is `cef_frames` painted, `shown` shown.
fn delta(before: Counts) -> (String, String) {
    let now = counts();
    (
        format!(
            "cef_frames={} app_frames={} shown={}",
            now.cef_frames - before.cef_frames,
            now.app_frames - before.app_frames,
            now.shown - before.shown
        ),
        crate::perf::summary(before.p2d_mark),
    )
}

/// Which path the pump's waits are on, for the line: `clock` is the
/// browser's clock for both the idle chain and CEF's delayed asks, `pool`
/// is gpui's timer and a condvar.
fn pump_timer() -> &'static str {
    crate::clock::label()
}
