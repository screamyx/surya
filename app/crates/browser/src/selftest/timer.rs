//! `SURYA_SELFTEST_TIMER=<secs>`: thirty 16 ms timers back to back, gpui's
//! thread-pool timer and then the browser's own clock, and how long each
//! really took. The instrument for the process timer tick: on Windows a
//! 16 ms gpui timer took 31 ms in haktui's measurement.

use std::time::{Duration, Instant};

const ASK: Duration = Duration::from_millis(16);
const ROUNDS: usize = 30;

pub(super) fn spawn(cx: &mut gpui::App, after: u64) {
    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
        super::sleep(cx, Duration::from_secs(after)).await;
        let mut gpui_us: Vec<u128> = Vec::with_capacity(ROUNDS);
        for _ in 0..ROUNDS {
            let t = Instant::now();
            cx.background_executor().timer(ASK).await;
            gpui_us.push(t.elapsed().as_micros());
        }
        let mut ours_us: Vec<u128> = Vec::with_capacity(ROUNDS);
        for _ in 0..ROUNDS {
            let t = Instant::now();
            let _ = crate::clock::after(ASK).await;
            ours_us.push(t.elapsed().as_micros());
        }
        println!(
            "selftest: TIMER asked={}ms x{ROUNDS} gpui {} | ours {}",
            ASK.as_millis(),
            stats(&mut gpui_us),
            stats(&mut ours_us)
        );
    })
    .detach();
}

fn stats(v: &mut Vec<u128>) -> String {
    v.sort_unstable();
    let ms = |x: u128| x as f64 / 1000.0;
    format!("median={:.1}ms min={:.1}ms max={:.1}ms", ms(v[v.len() / 2]), ms(v[0]), ms(v[v.len() - 1]))
}
