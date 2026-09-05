//! Which thread is CEF's UI thread, and how a call reaches it.
//!
//! Today (`external_message_pump`) CEF's UI thread is gpui's main thread:
//! every `BrowserHost` call from the shell runs inline. With
//! `SURYA_CEF_THREADED=1` (Windows only, off by default) CEF runs its own
//! UI thread (`multi_threaded_message_loop`), so the paint callbacks and
//! their GPU copy wait leave gpui's frame, and every `BrowserHost` call
//! from the shell must be posted there instead. [`on_ui`] is the one seam:
//! inline in the first mode, `post_task(TID_UI)` in the second. Callers
//! capture plain data and browser ids only (`Browser` and `BrowserHost`
//! are not `Send`) and look the host up again on the UI thread.
//!
//! This file is the stub the routing was written against; the threaded
//! internals (a Condvar for [`wait_until_closed`], the paint wake) are the
//! astra seat's.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use cef::rc::Rc as _;
use cef::*;

/// `SURYA_CEF_THREADED=1` on Windows: CEF owns its UI thread. Read once.
pub(crate) fn threaded() -> bool {
    static THREADED: OnceLock<bool> = OnceLock::new();
    *THREADED.get_or_init(|| {
        cfg!(windows) && std::env::var("SURYA_CEF_THREADED").map(|v| v.trim() == "1").unwrap_or(false)
    })
}

type Job = Box<dyn FnOnce() + Send>;

wrap_task! {
    struct UiTask {
        job: Arc<Mutex<Option<Job>>>,
    }
    impl Task {
        fn execute(&self) {
            let job = self.job.lock().ok().and_then(|mut j| j.take());
            if let Some(job) = job {
                job();
            }
        }
    }
}

static POSTED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Run `f` on CEF's UI thread: now, when that is this thread; otherwise
/// posted. A call that CEF refuses to post (before `initialize`, after
/// `shutdown`) is dropped, as the browser it named is gone anyway.
pub(crate) fn on_ui(f: impl FnOnce() + Send + 'static) {
    if !threaded() {
        f();
        return;
    }
    POSTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut task = UiTask::new(Arc::new(Mutex::new(Some(Box::new(f) as Job))));
    let _ = post_task(ThreadId::UI, Some(&mut task));
}

/// A browser answered its close (`on_before_close`, after the map entry is
/// gone and no lock is held). Stub: the threaded implementation wakes
/// [`wait_until_closed`] from here.
pub(crate) fn notify_closed() {}

/// Wait for every browser to answer its close, up to `timeout`. Inline
/// mode has to pump for the answers to arrive; threaded mode only waits.
/// Returns whether they are all gone.
pub(crate) fn wait_until_closed(timeout: Duration) -> bool {
    let started = Instant::now();
    while crate::client::is_open_or_pending() && started.elapsed() < timeout {
        if !threaded() {
            do_message_loop_work();
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    !crate::client::is_open_or_pending()
}

pub(crate) fn counters() -> String {
    format!("threaded={} ui_posted={}", u8::from(threaded()), POSTED.load(std::sync::atomic::Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_mode_runs_the_closure_now() {
        // No env, no CEF: not threaded, and the call runs before return.
        assert!(!threaded());
        let ran = Arc::new(Mutex::new(false));
        let seen = ran.clone();
        on_ui(move || *seen.lock().unwrap() = true);
        assert!(*ran.lock().unwrap());
    }
}
