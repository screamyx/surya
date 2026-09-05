//! Paint to draw: how long a CEF frame waits for the draw that shows it.
//!
//! [`on_paint`] runs in CEF's `on_paint` for the active browser and stamps
//! the time. [`on_render`] runs from the root view's render; when a new
//! frame has landed since the last render, the gap between the two stamps is
//! one sample. haktui's spike measured this at 4.5 ms median on dtry: half
//! the 8 ms pump plus the wait for a frame slot. The queue after the draw is
//! DXGI's and not visible from here.
//!
//! The samples live in a fixed ring of atomics: a render that finds a new
//! frame does two atomic stores and never takes a lock or allocates.
//! `p2d n= median= p90= max=` on a counters line; the self-tests summarise
//! the samples taken during their window.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

static EPOCH: OnceLock<Instant> = OnceLock::new();
static LAST_PAINT_US: AtomicU64 = AtomicU64::new(0);
static SEEN_FRAMES: AtomicU64 = AtomicU64::new(0);
/// Renders of the root view, which is where a browser frame gets drawn.
static RENDERS: AtomicU64 = AtomicU64::new(0);
/// Renders that found a frame the previous render had not: the frames the
/// pane showed, one per CEF paint at best. A render that finds no new frame
/// draws the frame it already showed and does not count.
static SHOWN: AtomicU64 = AtomicU64::new(0);

/// A power of two, so `head % RING` is a mask. 1024 samples is eight
/// seconds of frames at 120 Hz, more than any self-test window.
const RING: usize = 1024;
static SAMPLES: [AtomicU64; RING] = [const { AtomicU64::new(0) }; RING];
/// Samples ever taken; the slot of sample `i` is `i % RING`.
static HEAD: AtomicU64 = AtomicU64::new(0);

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
    SHOWN.fetch_add(1, Ordering::Relaxed);
    let painted = LAST_PAINT_US.load(Ordering::Acquire);
    if painted == 0 {
        return;
    }
    push(now_us().saturating_sub(painted));
}

/// Frames shown so far (renders that drew a new CEF frame).
pub(crate) fn shown() -> u64 {
    SHOWN.load(Ordering::Relaxed)
}

fn push(gap_us: u64) {
    let i = HEAD.fetch_add(1, Ordering::AcqRel);
    SAMPLES[(i as usize) % RING].store(gap_us, Ordering::Release);
}

/// App renders so far, for a before/after pair around a test window.
pub(crate) fn renders() -> u64 {
    RENDERS.load(Ordering::Relaxed)
}

/// How many samples exist now, so a summary can start from here.
pub(crate) fn mark() -> u64 {
    HEAD.load(Ordering::Acquire)
}

/// `p2d n= median= p90= max=` over the samples taken since `since`; only
/// the last [`RING`] are still there.
pub(crate) fn summary(since: u64) -> String {
    let head = HEAD.load(Ordering::Acquire);
    let first = since.max(head.saturating_sub(RING as u64));
    if first >= head {
        return "p2d n=0".into();
    }
    let mut w: Vec<u64> = (first..head).map(|i| SAMPLES[(i as usize) % RING].load(Ordering::Acquire)).collect();
    w.sort_unstable();
    let at = |p: usize| w[(w.len() * p / 100).min(w.len() - 1)] as f64 / 1000.0;
    format!(
        "p2d n={} median={:.1}ms p90={:.1}ms p95={:.1}ms max={:.1}ms",
        w.len(),
        w[w.len() / 2] as f64 / 1000.0,
        at(90),
        at(95),
        w[w.len() - 1] as f64 / 1000.0
    )
}

/// For a counters line: the last ring of samples, plus the clock's pair.
pub(crate) fn counters() -> String {
    format!("app_renders={} shown={} {} {}", renders(), shown(), crate::clock::counters(), summary(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_reads_median_p90_max() {
        let since = mark();
        for v in [1000u64, 2000, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000] {
            push(v);
        }
        let s = summary(since);
        assert!(s.contains("n=10"), "{s}");
        assert!(s.contains("median=6.0ms"), "{s}");
        assert!(s.contains("p90=10.0ms"), "{s}");
        assert!(s.contains("p95=10.0ms"), "{s}");
        assert!(s.contains("max=10.0ms"), "{s}");
    }

    #[test]
    fn empty_window_says_so() {
        assert_eq!(summary(u64::MAX), "p2d n=0");
    }

    #[test]
    fn the_ring_keeps_only_the_last_samples() {
        let since = mark();
        for v in 0..(RING as u64 + 10) {
            push(v);
        }
        let s = summary(since);
        assert!(s.contains(&format!("n={RING}")), "{s}");
    }
}
