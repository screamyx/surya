//! `SURYA_SELFTEST_ANIM=<secs>`: a page that animates on its own, no input,
//! and how many frames CEF painted and the app drew in two seconds. The
//! case an input-driven frame loop can break: nothing asks, the page still
//! moves.

use std::time::Duration;

const PAGE: &str = "data:text/html,<style>@keyframes s{to{transform:rotate(360deg)}}div{width:200px;height:200px;margin:40px;background:%23c33;animation:s 1s linear infinite}</style><div></div>";
const WINDOW_MS: u64 = 2000;

pub(super) fn spawn(cx: &mut gpui::App, after: u64) {
    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
        super::sleep(cx, Duration::from_secs(after)).await;
        let loaded = super::load(PAGE);
        super::sleep(cx, Duration::from_millis(3000)).await;
        let before = super::counts();
        super::sleep(cx, Duration::from_millis(WINDOW_MS)).await;
        let (frames, p2d) = super::delta(before);
        println!(
            "selftest: ANIM loaded={} over {WINDOW_MS}ms {frames} pump_timer={} pump_ms={} {p2d}",
            u8::from(loaded),
            super::pump_timer(),
            crate::pump::base_ms(),
        );
    })
    .detach();
}
