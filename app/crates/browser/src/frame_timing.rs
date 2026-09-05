//! Browser frames actually submitted by the surface, and the app-thread
//! work that feeds them. CEF callback counts alone are not shown frames:
//! callbacks can overtake the app or arrive while a root render is running.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static EPOCH: OnceLock<Instant> = OnceLock::new();
static SHOWN: AtomicU64 = AtomicU64::new(0);
static LAST_SEQ: AtomicU64 = AtomicU64::new(0);
static MAIN_PUMP_US: AtomicU64 = AtomicU64::new(0);
static MAIN_HANDOFF_US: AtomicU64 = AtomicU64::new(0);
static MAIN_SURFACE_US: AtomicU64 = AtomicU64::new(0);
const RING: usize = 1024;
static GAPS: [AtomicU64; RING] = [const { AtomicU64::new(0) }; RING];

pub(crate) fn now_us() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_micros() as u64
}

pub(crate) fn main_pump(took: Duration) {
    MAIN_PUMP_US.fetch_add(took.as_micros() as u64, Ordering::Relaxed);
}

pub(crate) fn main_handoff(took: Duration) {
    MAIN_HANDOFF_US.fetch_add(took.as_micros() as u64, Ordering::Relaxed);
}

/// Main thread, after the surface submits the image/texture. `seq` is
/// unique across browsers and both render paths. Repainting an old frame
/// does not count; an image upload error must not call this function.
pub(crate) fn on_draw(seq: u64, arrived_us: u64, started: Instant) {
    MAIN_SURFACE_US.fetch_add(started.elapsed().as_micros() as u64, Ordering::Relaxed);
    if LAST_SEQ.swap(seq, Ordering::Relaxed) == seq {
        return;
    }
    let shown = SHOWN.load(Ordering::Relaxed);
    GAPS[shown as usize % RING].store(now_us().saturating_sub(arrived_us), Ordering::Release);
    // There is one writer (gpui); publish only after the sample is ready.
    SHOWN.store(shown + 1, Ordering::Release);
}

#[derive(Clone, Copy)]
pub(crate) struct Mark {
    shown: u64,
    pump_us: u64,
    handoff_us: u64,
    surface_us: u64,
}

pub(crate) fn mark() -> Mark {
    Mark {
        shown: SHOWN.load(Ordering::Acquire),
        pump_us: MAIN_PUMP_US.load(Ordering::Relaxed),
        handoff_us: MAIN_HANDOFF_US.load(Ordering::Relaxed),
        surface_us: MAIN_SURFACE_US.load(Ordering::Relaxed),
    }
}

pub(crate) fn summary(since: Mark) -> String {
    let now = mark();
    let shown = now.shown.saturating_sub(since.shown);
    let pump = now.pump_us.saturating_sub(since.pump_us);
    let handoff = now.handoff_us.saturating_sub(since.handoff_us);
    let surface = now.surface_us.saturating_sub(since.surface_us);
    let samples = since.shown.max(now.shown.saturating_sub(RING as u64))..now.shown;
    let gaps = samples.map(|i| GAPS[i as usize % RING].load(Ordering::Acquire)).collect();
    summarize(shown, pump, handoff, surface, gaps)
}

fn summarize(shown: u64, pump: u64, handoff: u64, surface: u64, mut gaps: Vec<u64>) -> String {
    gaps.sort_unstable();
    let rank = (gaps.len() * 95).div_ceil(100);
    let p95 = gaps.get(rank.saturating_sub(1)).copied().unwrap_or(0);
    let per_frame = if shown == 0 { 0.0 } else { (pump + handoff + surface) as f64 / shown as f64 / 1000.0 };
    format!(
        "surface_shown={shown} main_pump_us={pump} main_handoff_us={handoff} \
         main_surface_us={surface} main_ms_per_shown={per_frame:.3} surface_p2d_n={} surface_p2d_p95_ms={:.3}",
        gaps.len(), p95 as f64 / 1000.0,
    )
}

pub(crate) fn counters() -> String {
    summary(Mark { shown: 0, pump_us: 0, handoff_us: 0, surface_us: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn averages_use_shown_frames_and_p95_sorts_the_measured_gaps() {
        let summary = summarize(4, 1000, 500, 500, vec![9000, 1000, 7000, 3000]);
        assert!(summary.contains("surface_shown=4"));
        assert!(summary.contains("main_ms_per_shown=0.500"));
        assert!(summary.contains("surface_p2d_p95_ms=9.000"));
    }

    #[test]
    fn empty_windows_report_no_samples_without_dividing_by_zero() {
        let summary = summarize(0, 0, 0, 0, Vec::new());
        assert!(summary.contains("surface_p2d_n=0"));
        assert!(summary.contains("main_ms_per_shown=0.000"));
    }

    #[test]
    fn p95_uses_the_nearest_rank_when_the_sample_count_is_a_multiple_of_twenty() {
        let summary = summarize(20, 0, 0, 0, (1..=20).map(|n| n * 1000).collect());
        assert!(summary.contains("surface_p2d_p95_ms=19.000"));
    }
}
