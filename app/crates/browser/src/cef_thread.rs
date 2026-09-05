//! Route browser operations to CEF's UI thread. The external-pump mode
//! keeps its existing synchronous calls; the Windows experiment owns a
//! separate CEF UI thread and never pumps CEF from gpui.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use cef::rc::Rc as _;
use cef::*;

pub(crate) fn threaded() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        cfg!(windows) && std::env::var("SURYA_CEF_THREADED").is_ok_and(|v| v == "1")
    })
}

type Callback = Box<dyn FnOnce() + Send + 'static>;

/// CEF's wrapper is cloneable; every clone shares one consumable callback.
/// Take the callback out before calling it so nested UI operations cannot
/// re-enter while this mutex is held.
#[derive(Clone)]
struct UiJob(Arc<Mutex<Option<Callback>>>);

impl UiJob {
    fn new(f: impl FnOnce() + Send + 'static) -> Self {
        Self(Arc::new(Mutex::new(Some(Box::new(f)))))
    }

    fn run(&self) {
        let callback = self.0.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(callback) = callback {
            callback();
        }
    }
}

wrap_task! {
    struct UiTask { job: UiJob }
    impl Task {
        fn execute(&self) {
            RAN.fetch_add(1, Ordering::Relaxed);
            self.job.run();
        }
    }
}

static ASKED: AtomicU64 = AtomicU64::new(0);
static POSTED: AtomicU64 = AtomicU64::new(0);
static RAN: AtomicU64 = AtomicU64::new(0);
static REJECTED: AtomicU64 = AtomicU64::new(0);

/// Capture plain data/browser ids, then look up CEF objects inside `f`.
/// Browser and BrowserHost wrappers must not be carried across threads.
/// Nested calls already on TID_UI run inline to preserve lifecycle order.
pub(crate) fn on_ui(f: impl FnOnce() + Send + 'static) {
    ASKED.fetch_add(1, Ordering::Relaxed);
    if !threaded() || currently_on(ThreadId::UI) == 1 {
        RAN.fetch_add(1, Ordering::Relaxed);
        f();
        return;
    }
    let mut task = UiTask::new(UiJob::new(f));
    if post_task(ThreadId::UI, Some(&mut task)) == 1 {
        POSTED.fetch_add(1, Ordering::Relaxed);
    } else {
        let rejected = REJECTED.fetch_add(1, Ordering::Relaxed) + 1;
        eprintln!("browser: CEF UI task rejected rejected={rejected}");
    }
}

pub(crate) fn counters() -> String {
    format!(
        "cef_threaded={} ui_asked={} ui_done={} ui_posted={} ui_rejected={}",
        threaded(),
        ASKED.load(Ordering::Relaxed),
        RAN.load(Ordering::Relaxed),
        POSTED.load(Ordering::Relaxed),
        REJECTED.load(Ordering::Relaxed),
    )
}

#[derive(Default)]
struct CloseSignal {
    generation: Mutex<u64>,
    changed: Condvar,
}

impl CloseSignal {
    fn notify(&self) {
        let mut generation = self.generation.lock().unwrap_or_else(|e| e.into_inner());
        *generation = generation.wrapping_add(1);
        self.changed.notify_all();
    }

    fn wait(&self, timeout: Duration, is_open: impl Fn() -> bool) -> bool {
        let started = Instant::now();
        let mut generation = self.generation.lock().unwrap_or_else(|e| e.into_inner());
        while is_open() {
            let Some(left) = timeout.checked_sub(started.elapsed()) else { return false };
            let (next, _) = self.changed.wait_timeout(generation, left)
                .unwrap_or_else(|e| e.into_inner());
            generation = next;
        }
        true
    }
}

static CLOSED: OnceLock<CloseSignal> = OnceLock::new();

/// Signal after releasing the browser/tab maps. The waiter holds only the
/// signal mutex while checking the lifecycle predicate, never across CEF.
pub(crate) fn notify_closed() {
    CLOSED.get_or_init(CloseSignal::default).notify();
}

/// `client::is_open_or_pending` must include pending async creation requests, so an
/// empty browser map before on_after_created cannot finish shutdown early.
pub(crate) fn wait_until_closed(timeout: Duration) -> bool {
    if threaded() {
        return CLOSED.get_or_init(CloseSignal::default).wait(timeout, crate::client::is_open_or_pending);
    }
    let started = Instant::now();
    while crate::client::is_open_or_pending() {
        if started.elapsed() >= timeout {
            return false;
        }
        crate::pump::work_now();
        std::thread::sleep(Duration::from_millis(5));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn cloned_task_runs_once_on_its_executor_thread() {
        let called = Arc::new(AtomicU64::new(0));
        let count = called.clone();
        let caller = std::thread::current().id();
        let job = UiJob::new(move || {
            assert_ne!(std::thread::current().id(), caller);
            count.fetch_add(1, Ordering::Relaxed);
        });
        let clone = job.clone();
        std::thread::spawn(move || { job.run(); clone.run(); }).join().unwrap();
        assert_eq!(called.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn job_releases_its_lock_before_invoking_callback() {
        let job = UiJob::new(|| {});
        let clone = job.clone();
        *job.0.lock().unwrap() = Some(Box::new(move || {
            assert!(clone.0.try_lock().is_ok());
            clone.run();
        }));
        job.run();
    }

    #[test]
    fn close_notification_before_wait_is_not_lost() {
        let signal = CloseSignal::default();
        signal.notify();
        assert!(signal.wait(Duration::ZERO, || false));
        assert!(!signal.wait(Duration::ZERO, || true));
    }

    #[test]
    fn close_wait_survives_unrelated_wakes_and_exits_when_closed() {
        let signal = Arc::new(CloseSignal::default());
        let open = Arc::new(AtomicBool::new(true));
        let (worker_signal, worker_open) = (signal.clone(), open.clone());
        let worker = std::thread::spawn(move || {
            worker_signal.notify();
            worker_open.store(false, Ordering::Release);
            worker_signal.notify();
        });
        assert!(signal.wait(Duration::from_secs(2), || open.load(Ordering::Acquire)));
        worker.join().unwrap();
    }
}
