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
    let Some(host) = crate::client::host() else { return false };
    let x = crate::render::VIEW_W.load(Ordering::Acquire) / 2;
    let y = crate::render::VIEW_H.load(Ordering::Acquire) / 2;
    let ev = MouseEvent { x, y, modifiers: 0 };
    // The same marker a real wheel sets on its way to CEF.
    crate::pump::mark_input();
    host.send_mouse_wheel_event(Some(&ev), 0, dy);
    true
}

/// Frames and renders at one instant, for a before/after pair.
#[derive(Clone, Copy)]
struct Counts {
    cef_frames: u64,
    app_frames: u64,
    p2d_mark: u64,
}

fn counts() -> Counts {
    Counts { cef_frames: crate::render::frames(), app_frames: crate::perf::renders(), p2d_mark: crate::perf::mark() }
}

fn delta(before: Counts) -> (u64, u64, String) {
    let now = counts();
    (
        now.cef_frames - before.cef_frames,
        now.app_frames - before.app_frames,
        crate::perf::summary(before.p2d_mark),
    )
}

/// Which timer the idle pump is on, for the line.
fn pump_timer() -> &'static str {
    if crate::clock::pool_timer() { "pool" } else { "clock" }
}
