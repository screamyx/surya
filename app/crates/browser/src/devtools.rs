//! Chrome DevTools Protocol, in process.
//!
//! CEF exposes DevTools to its embedder through
//! `BrowserHost::send_dev_tools_message` and a `DevToolsMessageObserver`, so
//! nothing here opens a port, holds a websocket, or needs a token: the
//! message goes straight to the page's DevTools agent and the reply comes
//! back on the CEF UI thread, which is the app's main thread. This is the
//! route haktui's pin and phone mode already take ("No websocket", pin.rs).
//!
//! The remote debugging port (`SURYA_CDP_PORT`, lib.rs) stays what it was:
//! off by default, loopback when set, for an out-of-process client during a
//! diagnosis. Chromium has no per-run secret for that port, only
//! `--remote-allow-origins`, so its protection is being off and being
//! loopback; nothing in the app depends on it.
//!
//! One call is one `{id, method, params}` JSON message. The reply for that
//! id resolves the caller's future. Every caller sits on the main thread and
//! awaits while the pump keeps CEF turning, so a reply is never waited for
//! synchronously; a call that never answers is failed by [`sweep`] after
//! [`CALL_DEADLINE`].

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cef::rc::Rc as _;
use cef::*;
use futures::channel::oneshot;
use serde_json::{Value, json};

/// A reply that has not come after this long is an error, not a wait.
pub const CALL_DEADLINE: Duration = Duration::from_secs(30);

/// One reply handler, kept until its id answers or its deadline passes.
struct Pending {
    since: Instant,
    method: &'static str,
    done: Box<dyn FnOnce(Result<Value, String>) + Send>,
}

static PENDING: Mutex<Option<HashMap<i32, Pending>>> = Mutex::new(None);
/// Message ids start at 1 and only grow; CEF wants a positive `int`.
static NEXT_ID: AtomicI32 = AtomicI32::new(1);
static ASKED: AtomicU64 = AtomicU64::new(0);
static ANSWERED: AtomicU64 = AtomicU64::new(0);
static FAILED: AtomicU64 = AtomicU64::new(0);
static EXPIRED: AtomicU64 = AtomicU64::new(0);
/// Browser ids whose observer is registered, so a browser is observed once.
static OBSERVED: Mutex<Vec<i32>> = Mutex::new(Vec::new());

thread_local! {
    /// CEF drops an observer the moment its registration is dropped, so the
    /// registrations live as long as the process (haktui, pin.rs).
    static REGISTRATIONS: std::cell::RefCell<Vec<Registration>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Counters as pairs: `asked=4 answered=4 failed=0 expired=0` is a page that
/// answers; `asked=4 answered=0` is a wrong method name, or a browser that
/// was never observed.
pub fn counters() -> String {
    format!(
        "devtools asked={} answered={} failed={} expired={}",
        ASKED.load(Ordering::Relaxed),
        ANSWERED.load(Ordering::Relaxed),
        FAILED.load(Ordering::Relaxed),
        EXPIRED.load(Ordering::Relaxed),
    )
}

/// Register the observer on a browser once, keyed by its CEF identifier.
/// From `on_after_created`; a DevTools *call* there killed haktui's process
/// silently, so this only listens and the first call waits for a load.
pub(crate) fn observe(browser: &Browser) {
    let id = browser.identifier();
    let Ok(mut seen) = OBSERVED.lock() else { return };
    if seen.contains(&id) {
        return;
    }
    let Some(host) = browser.host() else { return };
    let mut obs = Observer::new();
    if let Some(reg) = host.add_dev_tools_message_observer(Some(&mut obs)) {
        REGISTRATIONS.with(|r| r.borrow_mut().push(reg));
        seen.push(id);
        println!("devtools: observing browser id={id}");
    } else {
        println!("devtools: could not observe browser id={id}");
    }
}

/// The browser is closing: forget its id everywhere, so a later browser
/// with the same id is observed and set up again.
pub(crate) fn on_before_close(id: i32) {
    if let Ok(mut seen) = OBSERVED.lock() {
        seen.retain(|b| *b != id);
    }
    crate::emulation::forget(id);
    crate::scheme::forget(id);
}

/// The main frame of browser `id` finished loading: the first moment a
/// browser takes a DevTools call safely. Wakes `agent::open`, then gives a
/// new browser the current device preset and colour scheme.
pub(crate) fn on_load_end(id: i32) {
    crate::agent::on_load_end();
    crate::scheme::on_load_end(id);
    crate::emulation::on_load_end(id);
}

/// Send one method to the current browser and hand its reply to `done`.
/// Returns the message id, or `None` when there is no browser or CEF
/// refused the message (then `done` was called with the error already).
pub(crate) fn send(
    method: &'static str,
    params: Value,
    done: Box<dyn FnOnce(Result<Value, String>) + Send>,
) -> Option<i32> {
    sweep();
    let Some(host) = crate::client::host() else {
        done(Err("no browser is open in the pane".into()));
        return None;
    };
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let message = json!({ "id": id, "method": method, "params": params }).to_string();
    if let Ok(mut guard) = PENDING.lock() {
        guard
            .get_or_insert_with(HashMap::new)
            .insert(id, Pending { since: Instant::now(), method, done });
    }
    ASKED.fetch_add(1, Ordering::Relaxed);
    if host.send_dev_tools_message(Some(message.as_bytes())) == 0 {
        // CEF took nothing; answer now rather than at the deadline.
        if let Some(p) = take(id) {
            FAILED.fetch_add(1, Ordering::Relaxed);
            (p.done)(Err(format!("{method}: CEF refused the DevTools message")));
        }
        return None;
    }
    Some(id)
}

/// Send one method and await its `result` object.
pub(crate) fn call(method: &'static str, params: Value) -> impl std::future::Future<Output = Result<Value, String>> {
    let (tx, rx) = oneshot::channel();
    send(
        method,
        params,
        Box::new(move |r| {
            let _ = tx.send(r);
        }),
    );
    async move {
        rx.await
            .unwrap_or_else(|_| Err(format!("{method}: the reply handler was dropped")))
    }
}

/// Fire a method and only count its reply (emulation, media).
pub(crate) fn fire(method: &'static str, params: Value) {
    send(
        method,
        params,
        Box::new(move |r| {
            if let Err(e) = r {
                println!("devtools: {method} failed: {e}");
            }
        }),
    );
}

fn take(id: i32) -> Option<Pending> {
    PENDING.lock().ok()?.as_mut()?.remove(&id)
}

/// Fail every call older than [`CALL_DEADLINE`]. Runs on every send, so a
/// quiet period cannot let a stale wait live forever either: the next call
/// clears it.
fn sweep() {
    let expired: Vec<Pending> = {
        let Ok(mut guard) = PENDING.lock() else { return };
        let Some(map) = guard.as_mut() else { return };
        let old: Vec<i32> = map
            .iter()
            .filter(|(_, p)| p.since.elapsed() > CALL_DEADLINE)
            .map(|(id, _)| *id)
            .collect();
        old.into_iter().filter_map(|id| map.remove(&id)).collect()
    };
    for p in expired {
        EXPIRED.fetch_add(1, Ordering::Relaxed);
        (p.done)(Err(format!(
            "{}: no DevTools reply within {}s",
            p.method,
            CALL_DEADLINE.as_secs()
        )));
    }
}

/// One reply arrived. `result` is the method's result object on success
/// and `{code, message}` on failure.
fn on_result(message_id: i32, success: bool, result: Option<&[u8]>) {
    let Some(p) = take(message_id) else {
        // Not ours: an id from before a restart, or a swept call.
        return;
    };
    let text = result.and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
    let value: Value = serde_json::from_str(text).unwrap_or(Value::Null);
    if success {
        ANSWERED.fetch_add(1, Ordering::Relaxed);
        (p.done)(Ok(value));
    } else {
        FAILED.fetch_add(1, Ordering::Relaxed);
        let message = value["message"].as_str().unwrap_or(text).to_string();
        (p.done)(Err(format!("{}: {message}", p.method)));
    }
}

wrap_dev_tools_message_observer! {
    struct Observer;
    impl DevToolsMessageObserver {
        fn on_dev_tools_method_result(
            &self,
            _browser: Option<&mut Browser>,
            message_id: ::std::os::raw::c_int,
            success: ::std::os::raw::c_int,
            result: Option<&[u8]>,
        ) {
            on_result(message_id, success != 0, result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_reaches_its_handler_and_a_stranger_is_ignored() {
        let (tx, rx) = std::sync::mpsc::channel();
        PENDING.lock().unwrap().get_or_insert_with(HashMap::new).insert(
            7,
            Pending {
                since: Instant::now(),
                method: "Runtime.evaluate",
                done: Box::new(move |r| tx.send(r).unwrap()),
            },
        );
        on_result(8, true, Some(br#"{"x":1}"#));
        assert!(rx.try_recv().is_err(), "a stranger's id must not answer ours");
        on_result(7, true, Some(br#"{"result":{"value":2}}"#));
        assert_eq!(rx.recv().unwrap().unwrap()["result"]["value"], 2);
    }

    #[test]
    fn a_failed_reply_carries_the_method_and_the_message() {
        let (tx, rx) = std::sync::mpsc::channel();
        PENDING.lock().unwrap().get_or_insert_with(HashMap::new).insert(
            9,
            Pending {
                since: Instant::now(),
                method: "Page.captureScreenshot",
                done: Box::new(move |r| tx.send(r).unwrap()),
            },
        );
        on_result(9, false, Some(br#"{"code":-32601,"message":"'Page.x' wasn't found"}"#));
        let err = rx.recv().unwrap().unwrap_err();
        assert_eq!(err, "Page.captureScreenshot: 'Page.x' wasn't found");
    }

    #[test]
    fn a_stale_call_is_swept_with_the_deadline_in_the_error() {
        let (tx, rx) = std::sync::mpsc::channel();
        PENDING.lock().unwrap().get_or_insert_with(HashMap::new).insert(
            11,
            Pending {
                since: Instant::now() - CALL_DEADLINE - Duration::from_secs(1),
                method: "DOM.getDocument",
                done: Box::new(move |r| tx.send(r).unwrap()),
            },
        );
        sweep();
        let err = rx.recv().unwrap().unwrap_err();
        assert!(err.contains("DOM.getDocument") && err.contains("30s"), "{err}");
    }
}
