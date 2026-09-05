//! The browser's own clock.
//!
//! gpui's timers on Windows are thread-pool timers and fire on the process
//! tick, 15.6 ms: haktui measured thirty 16 ms timers at a median of 30.9 ms
//! with `timeBeginPeriod(1)` accepted (spike-scroll-frame-rate-2026-08-28).
//! A std `Condvar::wait_timeout` on Windows is bound to the same tick. So
//! one thread owns a high-resolution waitable timer
//! (`CREATE_WAITABLE_TIMER_HIGH_RESOLUTION`, Windows 10 1803+), keeps a heap
//! of deadlines, earliest first, and is interrupted by an event when an
//! earlier deadline arrives. Elsewhere the same shape waits on a condvar,
//! which is exact enough on Linux and the Mac.
//!
//! Two shapes of wait: [`after`] hands back a oneshot the caller awaits;
//! [`wake_after`] sends on an unbounded channel, which is what CEF's
//! "pump me in N ms" wants. `SURYA_PUMP_TIMER=pool` keeps gpui's timer for
//! the idle pump, as the control.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures::channel::mpsc::UnboundedSender;
use futures::channel::oneshot;

/// Who to tell when a deadline passes.
enum Done {
    Once(oneshot::Sender<()>),
    Wake(UnboundedSender<()>),
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
        }
    }
}

struct Clock {
    /// Earliest deadline first. The sequence number breaks ties so two equal
    /// deadlines never compare their senders.
    heap: Mutex<BinaryHeap<Reverse<(Instant, u64)>>>,
    pending: Mutex<std::collections::HashMap<u64, Done>>,
    seq: AtomicU64,
    wait: PlatformWait,
}

static CLOCK: OnceLock<Arc<Clock>> = OnceLock::new();
static FIRED: AtomicU64 = AtomicU64::new(0);

/// Deadlines served so far.
pub(crate) fn fired() -> u64 {
    FIRED.load(Ordering::Relaxed)
}

/// `SURYA_PUMP_TIMER=pool`: the idle pump waits on gpui's timer, the old
/// path, kept as the control for the measurements.
pub(crate) fn pool_timer() -> bool {
    std::env::var("SURYA_PUMP_TIMER").is_ok_and(|v| v.trim() == "pool")
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
    let id = clock.seq.fetch_add(1, Ordering::Relaxed);
    clock.pending.lock().unwrap().insert(id, done);
    clock.heap.lock().unwrap().push(Reverse((Instant::now() + d, id)));
    clock.wait.interrupt();
}

fn start() -> Arc<Clock> {
    let clock = Arc::new(Clock {
        heap: Mutex::new(BinaryHeap::new()),
        pending: Mutex::new(Default::default()),
        seq: AtomicU64::new(0),
        wait: PlatformWait::new(),
    });
    let worker = Arc::clone(&clock);
    std::thread::Builder::new()
        .name("surya-browser-clock".into())
        .spawn(move || run(worker))
        .expect("clock thread");
    clock
}

fn run(clock: Arc<Clock>) {
    loop {
        // Fire everything due, then find the next deadline.
        let next = {
            let mut heap = clock.heap.lock().unwrap();
            let now = Instant::now();
            while let Some(Reverse((due, id))) = heap.peek().copied() {
                if due > now {
                    break;
                }
                heap.pop();
                if let Some(done) = clock.pending.lock().unwrap().remove(&id) {
                    FIRED.fetch_add(1, Ordering::Relaxed);
                    done.fire();
                }
            }
            heap.peek().map(|Reverse((due, _))| *due)
        };
        match next {
            Some(due) => clock.wait.until(due),
            None => clock.wait.forever(),
        }
    }
}

/// The wait itself: Windows' high-resolution waitable timer plus an event
/// to cut it short; a condvar everywhere else.
#[cfg(windows)]
struct PlatformWait {
    /// An auto-reset event, as an `isize` so the struct is `Send + Sync`.
    event: isize,
    timer: isize,
}

#[cfg(windows)]
impl PlatformWait {
    fn new() -> Self {
        use windows::Win32::System::Threading::{
            CreateEventW, CreateWaitableTimerExW, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS,
        };
        // SAFETY: plain creation of two kernel objects owned by the process;
        // a zero handle means creation failed and the waits fall back to sleeps.
        let event = unsafe { CreateEventW(None, false, false, None) }.map(|h| h.0 as isize).unwrap_or(0);
        let timer =
            unsafe { CreateWaitableTimerExW(None, None, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS.0) }
                .map(|h| h.0 as isize)
                .unwrap_or(0);
        println!("browser: clock event={} hires_timer={}", u8::from(event != 0), u8::from(timer != 0));
        Self { event, timer }
    }

    fn handle(raw: isize) -> windows::Win32::Foundation::HANDLE {
        windows::Win32::Foundation::HANDLE(raw as *mut std::ffi::c_void)
    }

    fn interrupt(&self) {
        if self.event != 0 {
            // SAFETY: signalling our own event.
            unsafe {
                let _ = windows::Win32::System::Threading::SetEvent(Self::handle(self.event));
            }
        }
    }

    fn until(&self, due: Instant) {
        use windows::Win32::System::Threading::{SetWaitableTimer, WaitForMultipleObjects, INFINITE};
        let d = due.saturating_duration_since(Instant::now());
        if self.timer == 0 || self.event == 0 {
            std::thread::sleep(d.min(Duration::from_millis(1)));
            return;
        }
        // Relative time in 100 ns units, negative; never zero.
        let due: i64 = -((d.as_nanos() / 100) as i64).max(1);
        // SAFETY: our own handles; no APC routine; the wait returns on either.
        unsafe {
            if SetWaitableTimer(Self::handle(self.timer), &due, 0, None, None, false).is_ok() {
                WaitForMultipleObjects(&[Self::handle(self.timer), Self::handle(self.event)], false, INFINITE);
            } else {
                std::thread::sleep(d.min(Duration::from_millis(1)));
            }
        }
    }

    fn forever(&self) {
        use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE};
        if self.event == 0 {
            std::thread::sleep(Duration::from_millis(1));
            return;
        }
        // SAFETY: waiting on our own event.
        unsafe {
            WaitForSingleObject(Self::handle(self.event), INFINITE);
        }
    }
}

#[cfg(not(windows))]
struct PlatformWait {
    /// The condvar pairs with this flag rather than the heap lock, so the
    /// worker never holds the heap while it sleeps.
    poked: Mutex<bool>,
    cv: std::sync::Condvar,
}

#[cfg(not(windows))]
impl PlatformWait {
    fn new() -> Self {
        Self { poked: Mutex::new(false), cv: std::sync::Condvar::new() }
    }

    fn interrupt(&self) {
        *self.poked.lock().unwrap() = true;
        self.cv.notify_one();
    }

    fn until(&self, due: Instant) {
        let mut poked = self.poked.lock().unwrap();
        while !*poked {
            let d = due.saturating_duration_since(Instant::now());
            if d.is_zero() {
                break;
            }
            poked = self.cv.wait_timeout(poked, d).unwrap().0;
        }
        *poked = false;
    }

    fn forever(&self) {
        let mut poked = self.poked.lock().unwrap();
        while !*poked {
            poked = self.cv.wait(poked).unwrap();
        }
        *poked = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(fired() >= 2);
    }

    #[test]
    fn wake_sends_on_the_channel() {
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<()>();
        wake_after(Duration::from_millis(5), tx);
        futures::executor::block_on(futures::StreamExt::next(&mut rx)).unwrap();
    }
}
