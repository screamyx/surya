//! The pin: point at something on the page, and hand an agent enough to find it
//! again.
//!
//! D24 Level 1, which works with no framework support at all: the page URL, a
//! selector, the element's box, the viewport and DPR, and the owner's words.
//!
//! **No screenshot.** The picture would be written on the machine running CEF,
//! and the agent receiving the pin is on the other end of an SSH connection, so
//! a path in the pin is a dead reference. It is not needed either: the agent
//! already reaches this same browser over CDP and can take its own picture, of
//! any region, at any moment. A pin carries one stale frame; a url and a
//! selector carry the live page. Owner's ruling, 2026-08-26.
//!
//! **No websocket.** CEF exposes DevTools in-process through
//! `execute_dev_tools_method`, so the pin does not need the remote debugging
//! port open, or a client, or a tunnel. The port stays on for agents; the pin
//! does not use it.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;

use cef::rc::Rc as _;
use cef::*;

/// The pin he is writing about, if the sheet is open.
///
/// One at a time. There is no queue: the prompt in column 2 IS the queue, and
/// enter is the send. `lavish-live` has to queue because its send is a POST -
/// once it fires it is gone - and ours cannot fire until he presses enter. So
/// a pin lands in the prompt as it is made, five pins make five lines in one
/// prompt, and he sends the lot with one keystroke.
///
/// Do not rebuild the draft store without reading that: a queue file that
/// outlives the browser it describes has its own failure mode, pins pointing
/// at a page he is no longer on.
static COMPOSER: Mutex<Option<super::chrome::Composer>> = Mutex::new(None);
/// Message ids we issued, so a reply from anything else is ignored.
static NEXT_ID: AtomicI32 = AtomicI32::new(1);
static ASKED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static ANSWERED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Counters as a pair, so `answered=0` can be read against how many were asked.
pub fn counters() -> String {
    format!(
        "pins asked={} answered={} sent={}",
        ASKED.load(Ordering::Relaxed),
        ANSWERED.load(Ordering::Relaxed),
        SENT.load(Ordering::Relaxed)
    )
}

/// What runs in the page. `document.elementFromPoint` rather than CDP's
/// `DOM.getNodeForLocation`, which needs three round trips and a node handle;
/// this is one call and returns the same element.
///
/// **Two elements come back, not one.** `elementFromPoint` returns the
/// *topmost* element at the point, which for a click in open space is whatever
/// layout box happens to be under the pointer. Measured on the probe page: a
/// click in the middle of the pad returned a 1158x3000 scroller with no text in
/// it. So the pin reports what was literally under the pointer **and** the
/// nearest ancestor that carries text or is interactive, and says which it
/// used. Never silently substitute one for the other.
const PIN_JS: &str = r#"(function (x, y) {
  const el = document.elementFromPoint(x, y);
  if (!el) return JSON.stringify({ error: 'nothing at ' + x + ',' + y });
  // CLASS-FREE, structural.
  //
  // The obvious selector uses class names, and on a Tailwind app the class
  // names ARE the styling. Measured on the real DMS: the h1 came out as
  // `... > h1.text-2xl.font-semibold`. He pins that h1 to say "make this
  // bigger", the fix changes text-2xl, and **the selector breaks on exactly
  // the edit the pin asked for.** A reference that cannot survive being acted
  // on is not a reference.
  //
  // Structural breaks if a wrapper is added. It survives text-2xl becoming
  // text-3xl, which is the edit he will actually ask for.
  const sel = (n0) => {
    const parts = [];
    for (let n = n0; n && n.nodeType === 1 && n !== document.documentElement; n = n.parentElement) {
      // An id is only a shortcut if it is actually unique. Duplicate ids are
      // invalid HTML and common in real apps, and stopping at one that is not
      // unique produces a selector that finds several elements while looking
      // like it found one. Measured on the repeated fixture: three cards given
      // the same id turned `#dup > div` into a locator for three things.
      if (n.id) {
        const esc = '#' + CSS.escape(n.id);
        let only = false;
        try { only = document.querySelectorAll(esc).length === 1; } catch (e) { only = false; }
        if (only) { parts.unshift(esc); break; }
      }
      let s = n.localName;
      const sibs = n.parentElement ? [...n.parentElement.children].filter((k) => k.localName === n.localName) : [];
      if (sibs.length > 1) s += ':nth-of-type(' + (sibs.indexOf(n) + 1) + ')';
      parts.unshift(s);
    }
    return parts.join(' > ');
  };
  const meaningful = (n) => {
    if (n.matches('a,button,input,select,textarea,img,svg,[role],[onclick],label,summary')) return true;
    return [...n.childNodes].some((c) => c.nodeType === 3 && c.textContent.trim());
  };
  let t = el;
  while (t && t.nodeType === 1 && t !== document.body && !meaningful(t)) t = t.parentElement;
  if (!t || t.nodeType !== 1 || t === document.body) t = el;
  // OWN text, not innerText.
  //
  // Measured on the real DMS: a click in open space landed on a full-viewport
  // <main>, whose innerText is every word on the page. Reporting that as the
  // pinned element's text tells an agent the owner pinned the whole screen and
  // complained about all of it. Direct text children are what this element
  // actually says.
  const own = (n) =>
    [...n.childNodes].filter((c) => c.nodeType === 3).map((c) => c.textContent).join(' ')
      .trim().replace(/\s+/g, ' ').slice(0, 80);
  const desc = (n) => {
    const r = n.getBoundingClientRect();
    const q = sel(n);
    // Does the class-free path actually find exactly this one element? The
    // class was sometimes doing the disambiguating, so dropping it can make a
    // path ambiguous. Reported rather than assumed.
    let matches = 0;
    try { matches = document.querySelectorAll(q).length; } catch (e) { matches = -1; }
    return {
      sel: q, matches, tag: n.localName, text: own(n),
      x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height),
    };
  };
  // The jsxDEV stamp, if the page happens to carry one. D24's Level 2 for
  // free: the pin does not depend on the proxy, it just reads an attribute
  // when one is there.
  let stamp = null;
  for (let n = t; n && n.nodeType === 1 && !stamp; n = n.parentElement) {
    for (const a of n.attributes) if (/^data-(hl|source|inspector)/.test(a.name)) { stamp = a.value; break; }
  }
  return JSON.stringify({
    hit: desc(el), target: desc(t), climbed: t !== el, stamp,
    vw: innerWidth, vh: innerHeight, dpr: devicePixelRatio, url: location.href,
  });
})"#;

/// Ask the page what is at a point. The answer arrives on the observer.
pub fn take(x: i32, y: i32) {
    let Some(host) = super::osr::active_browser().and_then(|b| b.host()) else {
        println!("pin: no browser to pin");
        return;
    };
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    ASKED.fetch_add(1, Ordering::Relaxed);
    let expr = format!("({PIN_JS})({x}, {y})");
    let Some(mut params) = dictionary_value_create() else {
        println!("pin: could not build params");
        return;
    };
    params.set_string(Some(&CefString::from("expression")), Some(&CefString::from(expr.as_str())));
    params.set_bool(Some(&CefString::from("returnByValue")), 1);
    host.execute_dev_tools_method(id, Some(&CefString::from("Runtime.evaluate")), Some(&mut params));
    println!("pin: asked at {x},{y} id={id}");
}

/// Build the tested line from what the page returned.
///
/// The formatting lives in `chrome::Pin` because that is the half this project
/// can test on Linux, and the format is the part an agent has to parse.
fn to_pin(v: &serde_json::Value) -> super::chrome::Pin {
    let g = |o: &str, k: &str| v[o][k].as_str().unwrap_or_default().to_string();
    let n = |o: &str, k: &str| v[o][k].as_i64().unwrap_or(0);
    let climbed = v["climbed"].as_bool().unwrap_or(false);
    super::chrome::Pin {
        url: v["url"].as_str().unwrap_or_default().to_string(),
        selector: g("target", "sel"),
        matches: n("target", "matches"),
        tag: g("target", "tag"),
        text: g("target", "text"),
        x: n("target", "x"),
        y: n("target", "y"),
        w: n("target", "w"),
        h: n("target", "h"),
        vw: v["vw"].as_i64().unwrap_or(0),
        vh: v["vh"].as_i64().unwrap_or(0),
        dpr: v["dpr"].as_f64().unwrap_or(1.0),
        stamp: v["stamp"].as_str().map(|s| s.to_string()),
        under_pointer: climbed.then(|| g("hit", "sel")),
    }
}

thread_local! {
    /// CEF drops an observer the moment its registration is dropped, so the
    /// registrations are parked here for the life of the process. A
    /// thread_local rather than a static because `Registration` is a CEF
    /// ref-counted handle and is not `Send`; every one of these is made on the
    /// main thread, which is the only thread allowed to make one.
    static REGISTRATIONS: std::cell::RefCell<Vec<Registration>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Where a finished pin goes: the PTY in column 2.
///
/// Set by `main` when the live session starts. In `-Frame` there is no PTY at
/// all, so this stays empty and a pin only reaches the log - which is why step
/// 5 cannot be proven in frame mode.
static SINK: Mutex<Option<std::sync::mpsc::Sender<Vec<u8>>>> = Mutex::new(None);
static SENT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn set_sink(tx: std::sync::mpsc::Sender<Vec<u8>>) {
    if let Ok(mut s) = SINK.lock() {
        *s = Some(tx);
    }
}

/// Type the pin into the terminal, and do **not** press enter.
///
/// The owner still has to say what is wrong with the thing he pinned, and the
/// agent should not receive a pin with no complaint attached. Landing it in the
/// prompt leaves both to him, and it means a pin never submits anything on its
/// own.
/// How consecutive pins are separated in his prompt.
///
/// The separator is load-bearing, not a nicety: five pins run together on one
/// line is the feature failing at the last inch. It cannot be a bare newline,
/// because a newline submits the prompt.
///
/// A bracketed-paste span carries one. MEASURED 2026-08-27 through haktui, the
/// keystroke channel, SSH, herdr and the PTY: 35 bytes of span went in and the
/// far process received exactly `PINLINE-ONE\nPINLINE-TWO`, markers consumed
/// and the 0x0a intact. That settles the TRANSPORT. Whether an agent's own
/// prompt keeps the span whole is NOT settled - it needs an agent at the far
/// end - so `HAKTUI_PIN_JOIN=line` falls back to one line and a visible
/// separator, which is guaranteed and ugly.
fn send_line(line: &str) {
    let bytes = match std::env::var("HAKTUI_PIN_JOIN").as_deref() {
        Ok("line") => format!("{line}   //   ").into_bytes(),
        _ => format!("\x1b[200~{line}\n\x1b[201~").into_bytes(),
    };
    send_to_pty_bytes(bytes, line.len());
}

fn send_to_pty_bytes(bytes: Vec<u8>, line_len: usize) {
    let Ok(guard) = SINK.lock() else { return };
    let Some(tx) = guard.as_ref() else {
        println!("pin: no PTY to send to, this is -Frame");
        return;
    };
    let n = bytes.len();
    if tx.send(bytes).is_ok() {
        SENT.fetch_add(1, Ordering::Relaxed);
        // D43: the page tab carries an amber count of its pins.
        if let Some(id) = super::active_tab_id() {
            super::note_pin(id);
        }
        println!("pin: sent {n} bytes to the PTY, {line_len} of them the pin");
    }
}

/// Put raw bytes down the same channel a pin uses.
///
/// Only for measuring the transport. Nothing in the pin path calls this.
pub(super) fn send_raw(bytes: Vec<u8>) {
    let Ok(guard) = SINK.lock() else { return };
    match guard.as_ref() {
        Some(tx) => {
            let _ = tx.send(bytes);
        }
        None => println!("paste: no PTY, this is -Frame"),
    }
}

/// Is the sheet open? The shell asks every render.
pub fn composing() -> bool {
    COMPOSER.lock().map(|c| c.is_some()).unwrap_or(false)
}

/// Read the sheet's contents for drawing.
pub fn with_composer<R>(f: impl FnOnce(&super::chrome::Composer) -> R) -> Option<R> {
    COMPOSER.lock().ok()?.as_ref().map(f)
}

/// Feed a key to the sheet. Returns true when the sheet consumed it, which it
/// does for everything while open - the page must not also receive what he is
/// typing about the page.
/// When a key was consumed by the sheet, so the next render can say how long
/// it waited. Measures the GPUI-side edit path, which nothing measured before:
/// the input-to-paint probe timed a click reaching CEF and CEF painting back,
/// and there was no interactive GPUI surface in this app until the sheet.
pub(super) static KEY_AT_US: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub(super) static KEY_RENDERS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Set by every consumed key, cleared by the idle chain when it refreshes.
///
/// A backstop, not the main path. The real fix is `window.refresh()` in the
/// GPUI key handler, which turns a keystroke into a frame in about a
/// millisecond - but that handler is only reached by a real key event, so
/// anything driving `key()` directly does not benefit and cannot measure it.
/// This bounds the wait at one tick either way.
pub(super) static NEEDS_FRAME: AtomicBool = AtomicBool::new(false);

pub fn key(ks: &gpui::Keystroke) -> bool {
    if COMPOSER.lock().map(|c| c.is_some()).unwrap_or(false) {
        KEY_AT_US.store(super::osr::now_us().max(1), Ordering::Relaxed);
        NEEDS_FRAME.store(true, Ordering::Relaxed);
    }
    let Ok(mut guard) = COMPOSER.lock() else { return false };
    let Some(c) = guard.as_mut() else { return false };
    match c.key(ks) {
        super::chrome::ComposeAction::Add(line) => {
            *guard = None;
            drop(guard);
            send_line(&line);
            true
        }
        super::chrome::ComposeAction::Cancel => {
            *guard = None;
            println!("pin: cancelled, nothing sent");
            true
        }
        _ => true,
    }
}

/// Type into the open sheet, for the self-test. His keyboard is not always
/// available and the gesture has to be exercisable without it.
pub(super) fn type_for_test(s: &str) {
    if let Ok(mut g) = COMPOSER.lock()
        && let Some(c) = g.as_mut()
    {
        c.insert(s);
    }
}

/// Add the pin, as ctrl+enter would.
pub(super) fn add_for_test() {
    let line = {
        let Ok(mut g) = COMPOSER.lock() else { return };
        let Some(c) = g.as_ref() else {
            println!("selftest: no sheet open");
            return;
        };
        match c.line() {
            Some(l) => {
                *g = None;
                l
            }
            None => {
                println!("selftest: empty, adding nothing");
                return;
            }
        }
    };
    send_line(&line);
}

pub fn cancel() {
    if let Ok(mut c) = COMPOSER.lock() {
        *c = None;
    }
}

/// Register the observer on a browser and park the registration.
///
/// `win.rs` never names the type: CEF drops an observer the moment its
/// registration is dropped, so the two have to be done together or not at all.
pub(super) fn observe(host: &BrowserHost) {
    let mut obs = PinObserver::new();
    if let Some(reg) = host.add_dev_tools_message_observer(Some(&mut obs)) {
        REGISTRATIONS.with(|r| r.borrow_mut().push(reg));
    }
}

wrap_dev_tools_message_observer! {
    struct PinObserver;
    impl DevToolsMessageObserver {
        fn on_dev_tools_method_result(
            &self,
            _browser: Option<&mut Browser>,
            message_id: ::std::os::raw::c_int,
            success: ::std::os::raw::c_int,
            result: Option<&[u8]>,
        ) {
            // One observer per browser sees every reply. Mobile view mode's
            // ids start at a base the pin never reaches.
            if message_id >= super::emulation::ID_BASE {
                super::emulation::on_result(message_id, success != 0);
                return;
            }
            if success == 0 {
                println!("pin: id={message_id} failed");
                return;
            }
            let Some(bytes) = result else { return };
            let Ok(text) = std::str::from_utf8(bytes) else { return };
            let Ok(outer) = serde_json::from_str::<serde_json::Value>(text) else {
                println!("pin: id={message_id} unparseable result");
                return;
            };
            // Runtime.evaluate wraps the return value; ours is a JSON string
            // inside it, so it is parsed twice on purpose.
            let Some(inner) = outer["result"]["value"].as_str() else {
                println!("pin: id={message_id} no value in result");
                return;
            };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(inner) else { return };
            if let Some(e) = v["error"].as_str() {
                println!("pin: {e}");
                return;
            }
            ANSWERED.fetch_add(1, Ordering::Relaxed);
            let pin = to_pin(&v);
            println!("pin: caught {} - {}", pin.tag, pin.selector);
            // Do NOT send yet. He has not said what is wrong with it, and an
            // agent should never receive a pin with an empty complaint on it.
            if let Ok(mut c) = COMPOSER.lock() {
                *c = Some(super::chrome::Composer::new(pin));
            }
        }
    }
}
