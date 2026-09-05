//! Paint to draw: how long a CEF frame waits for the draw that shows it.
//!
//! [`on_paint`] runs in CEF's `on_paint` for the active browser and stamps
//! the time. [`on_render`] runs from the root view's render; when a new
//! frame has landed since the last render, the gap between the two stamps is
//! one sample. haktui's spike measured this at 4.5 ms median on dtry: half
//! the 8 ms pump plus the wait for a frame slot. The queue after the draw is
//! DXGI's and not visible from here.
//!
//! `p2d n= median= p90= max=` on a counters line; the self-tests summarise
//! the samples taken during their window.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static EPOCH: OnceLock<Instant> = OnceLock::new();
static LAST_PAINT_US: AtomicU64 = AtomicU64::new(0);
static SEEN_FRAMES: AtomicU64 = AtomicU64::new(0);
/// Renders of the root view, which is where a browser frame gets drawn.
static RENDERS: AtomicU64 = AtomicU64::new(0);
static SAMPLES: Mutex<Vec<u64>> = Mutex::new(Vec::new());
const KEEP: usize = 100_000;

/// Microseconds since the first call.
pub(crate) fn now_us() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_micros() as u64
}

/// CEF handed over a frame of the active browser. CEF's thread.
pub(crate) fn on_paint() {
    LAST_PAINT_US.store(now_us(), Ordering::Release);
}

/// The root view is rendering. Main thread.
pub(crate) fn on_render() {
    RENDERS.fetch_add(1, Ordering::Relaxed);
    let frames = crate::render::frames();
    if frames == SEEN_FRAMES.swap(frames, Ordering::AcqRel) {
        return;
    }
    let painted = LAST_PAINT_US.load(Ordering::Acquire);
    if painted == 0 {
        return;
    }
    let gap = now_us().saturating_sub(painted);
    if let Ok(mut v) = SAMPLES.lock()
        && v.len() < KEEP
    {
        v.push(gap);
    }
}

/// App renders so far, for a before/after pair around a test window.
pub(crate) fn renders() -> u64 {
    RENDERS.load(Ordering::Relaxed)
}

/// How many samples exist now, so a summary can start from here.
pub(crate) fn mark() -> usize {
    SAMPLES.lock().map(|v| v.len()).unwrap_or(0)
}

/// `p2d n= median= p90= max=` over the samples taken since `since`.
pub(crate) fn summary(since: usize) -> String {
    let Ok(v) = SAMPLES.lock() else { return "p2d=?".into() };
    let mut w: Vec<u64> = v[since.min(v.len())..].to_vec();
    if w.is_empty() {
        return "p2d n=0".into();
    }
    w.sort_unstable();
    format!(
        "p2d n={} median={:.1}ms p90={:.1}ms max={:.1}ms",
        w.len(),
        w[w.len() / 2] as f64 / 1000.0,
        w[(w.len() * 9 / 10).min(w.len() - 1)] as f64 / 1000.0,
        w[w.len() - 1] as f64 / 1000.0
    )
}

/// For a counters line: every sample so far, plus the clock's count.
pub(crate) fn counters() -> String {
    format!("app_renders={} clock_fired={} {}", renders(), crate::clock::fired(), summary(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_reads_median_p90_max() {
        let since = mark();
        if let Ok(mut v) = SAMPLES.lock() {
            v.extend([1000u64, 2000, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000]);
        }
        let s = summary(since);
        assert!(s.contains("n=10"), "{s}");
        assert!(s.contains("median=6.0ms"), "{s}");
        assert!(s.contains("p90=10.0ms"), "{s}");
        assert!(s.contains("max=10.0ms"), "{s}");
    }

    #[test]
    fn empty_window_says_so() {
        assert_eq!(summary(usize::MAX), "p2d n=0");
    }
}
