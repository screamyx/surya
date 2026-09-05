//! `SURYA_SELFTEST_SCROLL=<secs>`: load a tall page and send one second of
//! wheel events at 60 a second, then count what CEF painted and what the app
//! drew. haktui's owner, 2026-08-28: "using the browser feels very
//! sluggish"; his recording showed CEF delivering 8 to 25 frames a second
//! while he scrolled. This is the instrument for that number.

use std::time::{Duration, Instant};

const PAGE: &str = "data:text/html,<body style='margin:0'><div style='height:40000px;background:repeating-linear-gradient(%23fff 0 20px,%23888 20px 40px)'></div></body>";
const WHEEL_EVENTS: u32 = 60;

pub(super) fn spawn(cx: &mut gpui::App, after: u64) {
    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
        super::sleep(cx, Duration::from_secs(after)).await;
        let loaded = super::load(PAGE);
        super::sleep(cx, Duration::from_millis(2500)).await;
        let before = super::counts();
        let t0 = Instant::now();
        let mut sent = 0u32;
        for _ in 0..WHEEL_EVENTS {
            if super::wheel(-120) {
                sent += 1;
            }
            super::sleep(cx, Duration::from_millis(16)).await;
        }
        let sent_ms = t0.elapsed().as_millis();
        // Let the last frames land.
        super::sleep(cx, Duration::from_millis(500)).await;
        let (frames, p2d) = super::delta(before);
        println!(
            "selftest: SCROLL loaded={} wheel asked={WHEEL_EVENTS} sent={sent} over {sent_ms}ms \
             {frames} pump_timer={} pump_ms={} {p2d}",
            u8::from(loaded),
            super::pump_timer(),
            crate::pump::base_ms(),
        );
    })
    .detach();
}
