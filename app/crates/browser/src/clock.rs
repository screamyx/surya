//! The browser's own clock.
//!
//! gpui's timers on Windows are thread-pool timers and fire on the process
//! tick, 15.6 ms: haktui measured thirty 16 ms timers at a median of 30.9 ms
//! with `timeBeginPeriod(1)` accepted (spike-scroll-frame-rate-2026-08-28).
//! A std `Condvar::wait_timeout` on Windows is bound to the same tick. So
//! one thread owns a high-resolution waitable timer
//! (`CREATE_WAITABLE_TIMER_HIGH_RESOLUTION`, Windows 10 1803+), keeps a heap
//! of deadlines, earliest first, and is interrupted by an event when an
//! earlier deadline arrives. Elsewhere, and on a Windows that refuses the
//! kernel objects, the same shape waits on a condvar.
//!
//! Two shapes of wait: [`after`] hands back a oneshot the caller awaits;
//! [`wake_after`] sends on an unbounded channel, which is what CEF's
//! "pump me in N ms" wants. `SURYA_PUMP_TIMER=pool` is the old path, gpui's
//! timer and a condvar, kept as the control.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use futures::channel::mpsc::UnboundedSender;
use futures::channel::oneshot;

/// Who to tell when a deadline passes.
enum Done {
    Once(oneshot::Sender<()>),
    Wake(UnboundedSender<()>),
    /// Panics when fired: the test that proves the clock thread survives one.
    #[cfg(test)]
    Panic,
}

impl Done {
    fn fire(self) {
        match self {
            Done::Once(tx) => {
                let _ = tx.send(());
            }
            Done::Wake(tx) => {
                let _ = tx.unbounded_send(());
            }
            #[cfg(test)]
            Done::Panic => panic!("a deadline that panics when fired"),
        }
    }
}

struct Clock {
    /// Earliest deadline first. The sequence number breaks ties so two equal
    /// deadlines never compare their senders.
    heap: Mutex<BinaryHeap<Reverse<(Instant, u64)>>>,
    pending: Mutex<HashMap<u64, Done>>,
    seq: AtomicU64,
    wait: Wait,
}

static CLOCK: OnceLock<Arc<Clock>> = OnceLock::new();
static ASKED: AtomicU64 = AtomicU64::new(0);
static FIRED: AtomicU64 = AtomicU64::new(0);

/// Deadlines served so far; the pump's `timer_fired=`.
pub(crate) fn fired() -> u64 {
    FIRED.load(Ordering::Relaxed)
}

/// Deadlines asked for and served so far, as a pair.
pub(crate) fn counters() -> String {
    format!("clock asked={} fired={}", ASKED.load(Ordering::Relaxed), FIRED.load(Ordering::Relaxed))
}

/// The pump is on this clock: the idle chain waits here instead of on
/// gpui's timer, and on Windows the wait is the high-resolution waitable
/// timer. `SURYA_PUMP_TIMER=pool` is the old path, gpui's timer for the
/// idle chain and a condvar for CEF's delayed asks, kept as the control
/// (the baseline in docs/perf/browser-scroll-2026-09-05.md). Read once.
pub(crate) fn on_clock() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| from_env(std::env::var("SURYA_PUMP_TIMER").ok().as_deref()))
}

/// The flip itself, on the variable's value: only `pool` (whitespace
/// trimmed) leaves the clock. Unset, empty, or anything else is the clock.
fn from_env(value: Option<&str>) -> bool {
    !value.is_some_and(|v| v.trim() == "pool")
}

/// A poisoned lock is a thread that panicked while holding it; the data is a
/// heap of deadlines and a map of senders, sound whatever happened, so the
/// clock keeps serving rather than taking the idle chain down with it.
fn locked<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The pump timer's name for a log line: which path both waits are on.
pub(crate) fn label() -> &'static str {
    if on_clock() { "clock" } else { "pool" }
}

/// A oneshot that fires after `d`.
pub(crate) fn after(d: Duration) -> oneshot::Receiver<()> {
    let (tx, rx) = oneshot::channel();
    push(d, Done::Once(tx));
    rx
}

/// Send `()` on `tx` after `d`.
pub(crate) fn wake_after(d: Duration, tx: UnboundedSender<()>) {
    push(d, Done::Wake(tx));
}

fn push(d: Duration, done: Done) {
    let clock = CLOCK.get_or_init(start);
    ASKED.fetch_add(1, Ordering::Relaxed);
    let id = clock.seq.fetch_add(1, Ordering::Relaxed);
    locked(&clock.pending).insert(id, done);
    locked(&clock.heap).push(Reverse((Instant::now() + d, id)));
    clock.wait.interrupt();
}

fn start() -> Arc<Clock> {
    fine_timer();
    let clock = Arc::new(Clock {
        heap: Mutex::new(BinaryHeap::new()),
        pending: Mutex::new(HashMap::new()),
        seq: AtomicU64::new(0),
        wait: Wait::new(),
    });
    let worker = Arc::clone(&clock);
    std::thread::Builder::new()
        .name("surya-browser-clock".into())
        .spawn(move || run(worker))
        .expect("clock thread");
    clock
}

static PASS_PANICS: AtomicU64 = AtomicU64::new(0);

/// The clock thread never ends. One pass collects what is due and fires it;
/// a panic inside a pass is caught and counted, and the next pass runs at
/// once, firing whatever the panicking pass had collected but not yet fired
/// (the queue keeps them; nothing is dropped on the way out). The thread is
/// load-bearing for the pump once the idle chain waits here, so it does not
/// get to die.
fn run(clock: Arc<Clock>) {
    let mut due_now: VecDeque<Done> = VecDeque::new();
    loop {
        let pass = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| one_pass(&clock, &mut due_now)));
        match pass {
            Ok(Some(due)) => clock.wait.until(due),
            Ok(None) => clock.wait.forever(),
            Err(_) => {
                let n = PASS_PANICS.fetch_add(1, Ordering::Relaxed) + 1;
                if n == 1 {
                    println!("browser: clock pass panicked; the clock keeps running");
                }
            }
        }
    }
}

/// Everything due is collected under the locks, then fired with both
/// released: a waker that asks for another deadline re-enters push(). Each
/// deadline leaves the queue only as it fires, so an unwind mid-way leaves
/// the rest for the next pass. Returns the next deadline, or `None` when
/// the heap is empty.
fn one_pass(clock: &Clock, due_now: &mut VecDeque<Done>) -> Option<Instant> {
    let next = {
        let mut heap = locked(&clock.heap);
        let now = Instant::now();
        while let Some(Reverse((due, id))) = heap.peek().copied() {
            if due > now {
                break;
            }
            heap.pop();
            if let Some(done) = locked(&clock.pending).remove(&id) {
                due_now.push_back(done);
            }
        }
        heap.peek().map(|Reverse((due, _))| *due)
    };
    while let Some(done) = due_now.pop_front() {
        FIRED.fetch_add(1, Ordering::Relaxed);
        done.fire();
    }
    next
}

/// Passes that panicked and were caught; zero in a healthy session.
#[cfg(test)]
fn pass_panics() -> u64 {
    PASS_PANICS.load(Ordering::Relaxed)
}

/// Ask Windows for 1 ms timer resolution for the life of the process. This
/// is a declared default on both pump paths, decided on purpose:
///
/// - The clock's own wait is a high-resolution waitable timer and does not
///   need it. Everything else in the process that sleeps on the tick does:
///   the pool-timer control path, `Condvar::wait_timeout`, gpui's timers.
///   The dtry numbers in docs/perf/browser-scroll-2026-09-05.md were taken
///   with it on, so it stays on rather than moving the measured default.
/// - It is never released: since Windows 10 2004 the request is per process,
///   and Windows itself drops the process back to the default tick while its
///   window is minimised or fully behind another. The earlier build also
///   opted out of that throttling (`PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION`),
///   which kept the 1 ms tick alive for a hidden surya; that opt-out is gone,
///   so a backgrounded session costs what Windows says it should.
///
/// haktui measured 2026-08-28 that this is not what fixes the 31 ms timer
/// (the clock is); it is cheap and correct. `SURYA_COARSE_TIMER=1` skips it.
#[cfg(windows)]
fn fine_timer() {
    if std::env::var_os("SURYA_COARSE_TIMER").is_some() {
        println!("browser: timer coarse (SURYA_COARSE_TIMER)");
        return;
    }
    // SAFETY: a plain Win32 call with a constant argument; timeEndPeriod is
    // never called on purpose, the process wants this until it exits.
    let r = unsafe { windows::Win32::Media::timeBeginPeriod(1) };
    println!("browser: timer resolution 1ms asked, timeBeginPeriod={r}; the OS default returns while the window is in the background");
}

#[cfg(not(windows))]
fn fine_timer() {}

/// The wait itself: Windows' high-resolution waitable timer plus an event
/// to cut it short when the pump is on the clock, or a condvar: the old
/// path, and the fallback when the kernel objects are not to be had.
enum Wait {
    #[cfg(windows)]
    HiRes(HiRes),
    Condvar(CondvarWait),
}

impl Wait {
    fn new() -> Self {
        #[cfg(windows)]
        if on_clock() {
            if let Some(h) = HiRes::new() {
                println!("browser: pump timer=clock hires=1");
                return Wait::HiRes(h);
            }
            println!("browser: pump timer=clock hires=0 (kernel objects refused, condvar)");
            return Wait::Condvar(CondvarWait::new());
        }
        println!("browser: pump timer={} hires=0 (condvar)", label());
        Wait::Condvar(CondvarWait::new())
    }

    fn interrupt(&self) {
        match self {
            #[cfg(windows)]
            Wait::HiRes(h) => h.interrupt(),
            Wait::Condvar(c) => c.interrupt(),
        }
    }

    fn until(&self, due: Instant) {
        match self {
            #[cfg(windows)]
            Wait::HiRes(h) => h.until(due),
            Wait::Condvar(c) => c.until(due),
        }
    }

    fn forever(&self) {
        match self {
            #[cfg(windows)]
            Wait::HiRes(h) => h.forever(),
            Wait::Condvar(c) => c.forever(),
        }
    }
}

/// The condvar pairs with a flag rather than the heap lock, so the worker
/// never holds the heap while it sleeps and a push is never lost.
struct CondvarWait {
    poked: Mutex<bool>,
    cv: Condvar,
}

impl CondvarWait {
    fn new() -> Self {
        Self { poked: Mutex::new(false), cv: Condvar::new() }
    }

    fn interrupt(&self) {
        *locked(&self.poked) = true;
        self.cv.notify_one();
    }

    fn until(&self, due: Instant) {
        let mut poked = locked(&self.poked);
        while !*poked {
            let d = due.saturating_duration_since(Instant::now());
            if d.is_zero() {
                break;
            }
            poked = self.cv.wait_timeout(poked, d).unwrap_or_else(|e| e.into_inner()).0;
        }
        *poked = false;
    }

    fn forever(&self) {
        let mut poked = locked(&self.poked);
        while !*poked {
            poked = self.cv.wait(poked).unwrap_or_else(|e| e.into_inner());
        }
        *poked = false;
    }
}

/// Raw handles as `isize` so the struct is `Send + Sync`.
#[cfg(windows)]
struct HiRes {
    event: isize,
    timer: isize,
}

#[cfg(windows)]
impl HiRes {
    /// `None` when either kernel object is refused; the caller then waits
    /// on a condvar instead.
    fn new() -> Option<Self> {
        use windows::Win32::System::Threading::{
            CreateEventW, CreateWaitableTimerExW, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS,
        };
        // SAFETY: plain creation of two kernel objects owned by the process.
        let event = unsafe { CreateEventW(None, false, false, None) }.ok()?.0 as isize;
        let timer =
            unsafe { CreateWaitableTimerExW(None, None, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS.0) }
                .ok()?
                .0 as isize;
        Some(Self { event, timer })
    }

    fn handle(raw: isize) -> windows::Win32::Foundation::HANDLE {
        windows::Win32::Foundation::HANDLE(raw as *mut std::ffi::c_void)
    }

    fn interrupt(&self) {
        // SAFETY: signalling our own event.
        unsafe {
            let _ = windows::Win32::System::Threading::SetEvent(Self::handle(self.event));
        }
    }

    fn until(&self, due: Instant) {
        use windows::Win32::System::Threading::{SetWaitableTimer, WaitForMultipleObjects, WaitForSingleObject, INFINITE};
        let d = due.saturating_duration_since(Instant::now());
        // Relative time in 100 ns units, negative; never zero.
        let rel: i64 = -((d.as_nanos() / 100) as i64).max(1);
        // SAFETY: our own handles; no APC routine; the wait returns on either.
        unsafe {
            if SetWaitableTimer(Self::handle(self.timer), &rel, 0, None, None, false).is_ok() {
                WaitForMultipleObjects(&[Self::handle(self.timer), Self::handle(self.event)], false, INFINITE);
            } else {
                // The timer refused this deadline: a bounded wait on the
                // event alone, at the tick's resolution, never a spin.
                WaitForSingleObject(Self::handle(self.event), d.as_millis().clamp(1, u32::MAX as u128) as u32);
            }
        }
    }

    fn forever(&self) {
        use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE};
        // SAFETY: waiting on our own event.
        unsafe {
            WaitForSingleObject(Self::handle(self.event), INFINITE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_pool_leaves_the_clock() {
        assert!(from_env(None));
        assert!(from_env(Some("")));
        assert!(from_env(Some("clock")));
        assert!(from_env(Some("Pool")), "the value is case-sensitive, like every other switch");
        assert!(!from_env(Some("pool")));
        assert!(!from_env(Some("  pool \n")));
    }

    #[test]
    fn a_poisoned_lock_is_recovered() {
        let m = Mutex::new(7);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = m.lock().unwrap();
            panic!("poison it");
        }));
        assert!(m.is_poisoned());
        assert_eq!(*locked(&m), 7);
    }

    /// A deadline that panics when fired, then one that must still fire:
    /// the clock thread caught the first and served the second.
    #[test]
    fn the_clock_outlives_a_panicking_pass() {
        let before = pass_panics();
        // Same deadline as the panic, pushed after it: collected in the same
        // pass, behind the one that panics. It must still fire.
        push(Duration::from_millis(3), Done::Panic);
        let same_pass = after(Duration::from_millis(3));
        let later = after(Duration::from_millis(30));
        futures::executor::block_on(same_pass).expect("the deadline behind the panic was dropped");
        futures::executor::block_on(later).unwrap();
        assert!(pass_panics() > before, "the panicking pass was not observed");
    }

    #[test]
    fn oneshot_fires_close_to_its_deadline() {
        let t = Instant::now();
        futures::executor::block_on(after(Duration::from_millis(20))).unwrap();
        let took = t.elapsed();
        assert!(took >= Duration::from_millis(20), "early: {took:?}");
        assert!(took < Duration::from_millis(200), "late: {took:?}");
    }

    #[test]
    fn an_earlier_deadline_interrupts_a_later_wait() {
        let long = after(Duration::from_millis(400));
        let t = Instant::now();
        let short = after(Duration::from_millis(10));
        futures::executor::block_on(short).unwrap();
        assert!(t.elapsed() < Duration::from_millis(200), "the 10 ms wait sat behind the 400 ms one");
        futures::executor::block_on(long).unwrap();
        assert!(counters().contains("fired="));
    }

    #[test]
    fn wake_sends_on_the_channel() {
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<()>();
        wake_after(Duration::from_millis(5), tx);
        futures::executor::block_on(futures::StreamExt::next(&mut rx)).unwrap();
    }

    #[test]
    fn a_waker_may_ask_again_from_inside_the_wait() {
        // The first deadline's waker pushes a second one while the clock
        // thread is between its heap pass and the fire.
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<()>();
        let again = tx.clone();
        std::thread::spawn(move || {
            futures::executor::block_on(after(Duration::from_millis(3))).unwrap();
            wake_after(Duration::from_millis(1), again);
        });
        wake_after(Duration::from_millis(3), tx);
        futures::executor::block_on(futures::StreamExt::next(&mut rx)).unwrap();
        futures::executor::block_on(futures::StreamExt::next(&mut rx)).unwrap();
    }

    /// Fifty deadlines 4 ms apart, scheduled at once: the p95 error must be
    /// under 2 ms. On a 15.6 ms tick this fails; it is the test that tells
    /// the two clocks apart. Behind `SURYA_CLOCK_JITTER=1`, because a loaded
    /// box fails it for reasons that are not the clock's; on Windows run it
    /// with `SURYA_PUMP_TIMER=clock` too, or it measures the condvar path.
    #[test]
    fn fifty_deadlines_land_within_two_milliseconds_at_p95() {
        if std::env::var_os("SURYA_CLOCK_JITTER").is_none() {
            println!("jitter: skipped, set SURYA_CLOCK_JITTER=1 (and SURYA_PUMP_TIMER=clock on Windows)");
            return;
        }
        const N: u32 = 50;
        const STEP: Duration = Duration::from_millis(4);
        let t0 = Instant::now();
        let waits: Vec<_> = (1..=N).map(|i| (STEP * i, after(STEP * i))).collect();
        let mut errors_us: Vec<u128> = Vec::with_capacity(N as usize);
        for (asked, rx) in waits {
            futures::executor::block_on(rx).unwrap();
            let took = t0.elapsed();
            assert!(took >= asked, "early: asked {asked:?} took {took:?}");
            errors_us.push((took - asked).as_micros());
        }
        errors_us.sort_unstable();
        let p95 = errors_us[(N as usize * 95 / 100).min(N as usize - 1)];
        assert!(p95 < 2000, "p95 error {p95} us, all: {errors_us:?}");
    }
}
