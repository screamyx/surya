//! How CEF gets serviced from gpui's main thread.
//!
//! CEF's message loop must run on the thread that called `initialize`, which
//! is gpui's main thread, and CEF asks to be pumped (`schedule_pump`) from
//! whatever thread it likes. Two rules from haktui, each of which cost a
//! session: a purely callback-driven pump deadlocks (CEF only asks while it
//! is already running), and an idle gpui stops draining its main-thread
//! queue, so nothing may wait on a *frame*.
//!
//! The answer is one foreground task that sleeps on a timer OR a wake from
//! CEF, pumps, and backs off from 8ms to 100ms while nothing happens. A wake
//! from CEF's thread is a channel send; gpui reschedules the task on the main
//! thread through its own dispatcher, so this module never touches a
//! runnable itself. `pump()` from a render is a free extra pump.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use cef::do_message_loop_work;
use futures::channel::mpsc::{self, UnboundedSender};
use futures::{FutureExt as _, StreamExt as _};

static PUMP_ASKS: AtomicU64 = AtomicU64::new(0);
static WORK_DID: AtomicU64 = AtomicU64::new(0);
static RENDERS: AtomicU64 = AtomicU64::new(0);
static IDLE_ARMED: AtomicU64 = AtomicU64::new(0);
static IDLE_RAN: AtomicU64 = AtomicU64::new(0);
static IDLE_REFRESH: AtomicU64 = AtomicU64::new(0);
static TIMER_ARMED: AtomicU64 = AtomicU64::new(0);
static TIMER_FIRED: AtomicU64 = AtomicU64::new(0);
static INPUT_SEQ: AtomicU64 = AtomicU64::new(0);

static WAKE: OnceLock<UnboundedSender<()>> = OnceLock::new();

/// One `do_message_loop_work()`, counted. Main thread only.
pub(crate) fn work_now() {
    if crate::disabled() {
        return;
    }
    WORK_DID.fetch_add(1, Ordering::Relaxed);
    do_message_loop_work();
}

/// CEF's `on_schedule_message_pump_work`: pump now, or in `delay_ms`.
pub(crate) fn schedule_pump(delay_ms: i64) {
    PUMP_ASKS.fetch_add(1, Ordering::Relaxed);
    let Some(tx) = WAKE.get() else { return };
    if delay_ms <= 0 {
        let _ = tx.unbounded_send(());
        return;
    }
    TIMER_ARMED.fetch_add(1, Ordering::Relaxed);
    clock_after(Duration::from_millis(delay_ms.min(100) as u64), tx.clone());
}

/// Called the moment an input event is handed to CEF, so the idle chain
/// snaps back from its back-off. The input path is the next seat's.
#[allow(dead_code)]
pub(crate) fn mark_input() {
    INPUT_SEQ.fetch_add(1, Ordering::Relaxed);
    if let Some(tx) = WAKE.get() {
        let _ = tx.unbounded_send(());
    }
}

pub(crate) fn base_ms() -> u64 {
    std::env::var("SURYA_PUMP_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(8)
        .clamp(1, 100)
}

/// Start the idle pump. Call once from `start`, on the main thread.
pub(crate) fn install(cx: &mut gpui::App) {
    let (tx, mut rx) = mpsc::unbounded::<()>();
    if WAKE.set(tx).is_err() {
        return;
    }
    let base = Duration::from_millis(base_ms());
    let max = Duration::from_millis(100);
    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
        let mut wait = base;
        let mut seen_frames = crate::render::frames();
        let mut seen = (INPUT_SEQ.load(Ordering::Relaxed), PUMP_ASKS.load(Ordering::Relaxed));
        loop {
            IDLE_ARMED.fetch_add(1, Ordering::Relaxed);
            let timer = cx.background_executor().timer(wait).fuse();
            futures::pin_mut!(timer);
            futures::select! {
                _ = timer => {}
                _ = rx.next() => {}
            }
            // A burst of wakes is one pump.
            while let Ok(()) = rx.try_recv() {}
            IDLE_RAN.fetch_add(1, Ordering::Relaxed);
            work_now();
            let now = (INPUT_SEQ.load(Ordering::Relaxed), PUMP_ASKS.load(Ordering::Relaxed));
            let frames = crate::render::frames();
            let quiet = now == seen && frames == seen_frames;
            seen = now;
            wait = if quiet { (wait * 2).min(max) } else { base };
            if frames != seen_frames {
                seen_frames = frames;
                IDLE_REFRESH.fetch_add(1, Ordering::Relaxed);
                cx.refresh();
            }
        }
    })
    .detach();
}

/// Pump CEF from whatever render gpui was already doing. Call from the root
/// view's `render`; it is on the main thread, so the pump is free.
pub fn pump(_window: &mut gpui::Window, _cx: &mut gpui::App) {
    if crate::disabled() {
        return;
    }
    RENDERS.fetch_add(1, Ordering::Relaxed);
    crate::perf::on_render();
    work_now();
}

/// The counters as pairs, for a log line or a status bar.
pub fn counters() -> String {
    let (n, last, avg, max) = crate::surface::upload_stats();
    let (cn, clast, cavg, cmax) = crate::render::copy_stats();
    let (w, h) = crate::render::last_size();
    format!(
        "renders={} work_did={} pump_asks={} timer_armed={} timer_fired={} idle_armed={} idle_ran={} \
         idle_refresh={} inputs={} frames={} size={w}x{h} {} \
         copy_ms n={cn} last={clast:.2} avg={cavg:.2} max={cmax:.2} \
         upload_ms n={n} last={last:.2} avg={avg:.2} max={max:.2}",
        RENDERS.load(Ordering::Relaxed),
        WORK_DID.load(Ordering::Relaxed),
        PUMP_ASKS.load(Ordering::Relaxed),
        TIMER_ARMED.load(Ordering::Relaxed),
        TIMER_FIRED.load(Ordering::Relaxed),
        IDLE_ARMED.load(Ordering::Relaxed),
        IDLE_RAN.load(Ordering::Relaxed),
        IDLE_REFRESH.load(Ordering::Relaxed),
        INPUT_SEQ.load(Ordering::Relaxed),
        crate::render::frames(),
        crate::client::lifecycle_counters(),
    ) + &format!(" {}", crate::perf::counters())
}

/// Print the counters every 5 seconds from a plain thread that reads atomics
/// and touches nothing else, so a stopped main thread is a finding rather
/// than a silence.
pub(crate) fn start_heartbeat() {
    std::thread::Builder::new()
        .name("surya-browser-heartbeat".into())
        .spawn(|| {
            let started = Instant::now();
            let mut last_work = 0u64;
            loop {
                std::thread::sleep(Duration::from_secs(5));
                let work = WORK_DID.load(Ordering::Relaxed);
                let stalled = work == last_work;
                last_work = work;
                println!(
                    "browser: t={:.0}s {}{}",
                    started.elapsed().as_secs_f64(),
                    if stalled { "MAIN THREAD QUIET " } else { "" },
                    counters()
                );
            }
        })
        .expect("heartbeat thread");
}

/// A small clock thread: `clock_after(d, tx)` sends on `tx` after `d`. One
/// thread serves every wait, earliest first, so a burst of CEF asks does not
/// spawn a thread each.
struct Clock {
    heap: Mutex<std::collections::BinaryHeap<std::cmp::Reverse<(Instant, u64)>>>,
    pending: Mutex<std::collections::HashMap<u64, UnboundedSender<()>>>,
    seq: AtomicU64,
    wake: std::sync::Condvar,
}

static CLOCK: OnceLock<std::sync::Arc<Clock>> = OnceLock::new();

fn clock_after(d: Duration, tx: UnboundedSender<()>) {
    let clock = CLOCK.get_or_init(|| {
        let clock = std::sync::Arc::new(Clock {
            heap: Mutex::new(Default::default()),
            pending: Mutex::new(Default::default()),
            seq: AtomicU64::new(0),
            wake: std::sync::Condvar::new(),
        });
        let worker = clock.clone();
        std::thread::Builder::new()
            .name("surya-browser-clock".into())
            .spawn(move || loop {
                let mut heap = worker.heap.lock().unwrap();
                let now = Instant::now();
                match heap.peek().copied() {
                    None => {
                        drop(worker.wake.wait(heap).unwrap());
                    }
                    Some(std::cmp::Reverse((due, id))) if due <= now => {
                        heap.pop();
                        drop(heap);
                        if let Some(tx) = worker.pending.lock().unwrap().remove(&id) {
                            TIMER_FIRED.fetch_add(1, Ordering::Relaxed);
                            let _ = tx.unbounded_send(());
                        }
                    }
                    Some(std::cmp::Reverse((due, _))) => {
                        drop(worker.wake.wait_timeout(heap, due - now).unwrap());
                    }
                }
            })
            .expect("clock thread");
        clock
    });
    let id = clock.seq.fetch_add(1, Ordering::Relaxed);
    clock.pending.lock().unwrap().insert(id, tx);
    clock.heap.lock().unwrap().push(std::cmp::Reverse((Instant::now() + d, id)));
    clock.wake.notify_one();
}
