//! The browsers (one per tab) and the CEF client that owns their callbacks:
//! lifetime, loads, find results, and what each tab's address bar shows.
//! Every callback names its browser; `tabs.rs` maps that to a tab.
//!
//! Which browser is "the active one" has one source: the tab model
//! (`tabs::active_browser`). This module never keeps its own pointer.
//!
//! Locks: `BROWSERS` is taken only to read or change the map and is never
//! held across a CEF call (hosts are cloned out first); `tabs::TABS` is
//! never taken while `BROWSERS` is held, and no CEF callback takes both at
//! once. `render::FRAMES` is independent of either.
//!
//! Threads: every `BrowserHost` call that the shell starts goes through
//! `cef_thread::on_ui` with a browser id, and looks the host up again there
//! ([`host_of`]); the `*_now` functions are the halves that run on CEF's UI
//! thread and may also be called from a callback. See cef_thread.rs.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use cef::rc::Rc as _;
use cef::*;

use crate::tabs::update_by_browser;

/// Every live browser by CEF identifier.
static BROWSERS: Mutex<Option<HashMap<i32, Browser>>> = Mutex::new(None);
static CREATED: AtomicU64 = AtomicU64::new(0);
/// Asynchronous creates (threaded mode) that have not reached
/// `on_after_created` yet. Shutdown waits for them: an empty map with a
/// create in flight is not "closed".
static PENDING_CREATES: AtomicU64 = AtomicU64::new(0);
static CLOSED: AtomicU64 = AtomicU64::new(0);
static POPUPS_REFUSED: AtomicU64 = AtomicU64::new(0);
/// Whether the pane is on screen. Off, CEF stops compositing and paints
/// once a second at most; on, it paints at 60.
static VISIBLE: AtomicBool = AtomicBool::new(true);

pub(crate) fn lifecycle_counters() -> String {
    format!(
        "created={} closed={} popups_refused={} visible={}",
        CREATED.load(Ordering::Relaxed),
        CLOSED.load(Ordering::Relaxed),
        POPUPS_REFUSED.load(Ordering::Relaxed),
        u8::from(VISIBLE.load(Ordering::Relaxed)),
    )
}

/// Whether any browser exists right now.
pub(crate) fn is_open() -> bool {
    BROWSERS.lock().map(|b| b.as_ref().is_some_and(|m| !m.is_empty())).unwrap_or(false)
}

/// [`is_open`], or a create still in flight: what shutdown must wait for.
pub(crate) fn is_open_or_pending() -> bool {
    is_open() || PENDING_CREATES.load(Ordering::Acquire) > 0
}

/// Show a browser: paint at the display's rate, tell it the size it may have missed, and
/// give it the keyboard.
fn show(host: &BrowserHost) {
    host.set_windowless_frame_rate(crate::display::frame_rate());
    host.was_hidden(0);
    // A resize may have happened while hidden; CEF only paints damage.
    host.was_resized();
    host.set_focus(1);
}

/// Park a browser: the compositor stops and its frame cap drops to one a
/// second, so a page that animates costs nothing while nobody looks.
fn park(host: &BrowserHost) {
    host.was_hidden(1);
    host.set_windowless_frame_rate(1);
}

/// The pane went on or off screen. Only the active browser follows; the
/// others are parked already. Idempotent.
pub(crate) fn set_visible(on: bool) {
    if VISIBLE.swap(on, Ordering::AcqRel) == on {
        return;
    }
    let id = crate::tabs::active_browser();
    crate::cef_thread::on_ui(move || {
        if let Some(host) = host_of(id) {
            if on { show(&host) } else { park(&host) }
        }
    });
    println!("browser: visible={}", u8::from(on));
}

/// Every live host, with its identifier. The browsers are cloned out from
/// under the lock first; `host()` is a CEF call and runs after it is dropped.
fn hosts() -> Vec<(i32, BrowserHost)> {
    let browsers: Vec<(i32, Browser)> = {
        let Ok(guard) = BROWSERS.lock() else { return Vec::new() };
        let Some(map) = guard.as_ref() else { return Vec::new() };
        map.iter().map(|(id, b)| (*id, b.clone())).collect()
    };
    browsers.into_iter().filter_map(|(id, b)| b.host().map(|h| (id, h))).collect()
}

/// Put `browser` on screen and park every other one. A tab switch, or a
/// tab close (then `browser` is the new active tab's, or 0 for none).
pub(crate) fn activate(browser: i32) {
    crate::cef_thread::on_ui(move || activate_now(browser));
}

/// [`activate`] on CEF's UI thread.
fn activate_now(browser: i32) {
    let mut shown = 0;
    for (id, host) in hosts() {
        if id == browser && VISIBLE.load(Ordering::Acquire) {
            show(&host);
            shown += 1;
        } else {
            park(&host);
        }
    }
    println!("browser: activate browser={browser} shown={shown}");
}

/// Close one browser. CEF answers with `on_before_close`, which drops it
/// from the map and its frame; the pane shows the next active tab's frame.
pub(crate) fn close_browser(browser: i32) {
    crate::cef_thread::on_ui(move || {
        if let Some(host) = host_of(browser) {
            host.close_browser(1);
        }
    });
}

static LOAD_END_OK: AtomicU64 = AtomicU64::new(0);
static LOAD_END_OTHER: AtomicU64 = AtomicU64::new(0);

/// The active tab's browser, as the tab model says. For code that already
/// runs on CEF's UI thread (a callback, or the inline mode's main thread);
/// a call from the shell goes through `cef_thread::on_ui` with
/// [`browser_of`] instead. Kept for the devtools and self-test modules.
#[allow(dead_code)]
pub(crate) fn browser() -> Option<Browser> {
    browser_of(crate::tabs::active_browser())
}

#[allow(dead_code)]
pub(crate) fn host() -> Option<BrowserHost> {
    browser().and_then(|b| b.host())
}

/// The browser with CEF identifier `id`, if it still exists.
pub(crate) fn browser_of(id: i32) -> Option<Browser> {
    if id == 0 {
        return None;
    }
    BROWSERS.lock().ok()?.as_ref()?.get(&id).cloned()
}

/// Whether `id` names a live browser, without touching its refcount.
pub(crate) fn has_browser(id: i32) -> bool {
    id != 0 && BROWSERS.lock().ok().and_then(|g| g.as_ref().map(|m| m.contains_key(&id))).unwrap_or(false)
}

pub(crate) fn host_of(id: i32) -> Option<BrowserHost> {
    browser_of(id).and_then(|b| b.host())
}

/// CEF's identifier for `browser`, 0 for none.
fn id_of(browser: Option<&mut Browser>) -> i32 {
    browser.map(|b| b.identifier()).unwrap_or(0)
}

wrap_life_span_handler! {
    struct LifeSpan;
    impl LifeSpanHandler {
        fn on_before_popup(
            &self,
            browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _popup_id: ::std::os::raw::c_int,
            target_url: Option<&CefString>,
            _target_frame_name: Option<&CefString>,
            _target_disposition: WindowOpenDisposition,
            _user_gesture: ::std::os::raw::c_int,
            _popup_features: Option<&PopupFeatures>,
            _window_info: Option<&mut WindowInfo>,
            _client: Option<&mut Option<Client>>,
            _settings: Option<&mut BrowserSettings>,
            _extra_info: Option<&mut Option<DictionaryValue>>,
            _no_javascript_access: Option<&mut ::std::os::raw::c_int>,
        ) -> ::std::os::raw::c_int {
            // A target=_blank link or window.open opens in the opener's own
            // tab: a browser may not be created from inside this callback,
            // and a deferred tab is the ui seat's follow-up. Cancelled, then
            // the opener navigates.
            // The same allowlist as the address bar: `window.open("file:///..")`
            // gets nothing, not the disk.
            let asked = target_url.map(|u| u.to_string()).unwrap_or_default();
            let url = crate::page::navigate_to(&asked);
            POPUPS_REFUSED.fetch_add(1, Ordering::Relaxed);
            println!("browser: popup refused, opening in the tab: {url:?} (asked {asked:?})");
            if let (Some(browser), false) = (browser, url.is_empty())
                && let Some(frame) = browser.main_frame()
            {
                let id = browser.identifier();
                update_by_browser(id, |p| p.begin_navigation(&url));
                frame.load_url(Some(&CefString::from(url.as_str())));
            }
            1
        }

        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let Some(browser) = browser else { return };
            let id = browser.identifier();
            // Belt and braces with `on_before_popup`: a browser that is not
            // the pane's never takes the slot.
            if browser.is_popup() != 0 {
                println!("browser: stray popup browser id={id} ignored");
                if let Some(host) = browser.host() {
                    host.close_browser(1);
                }
                return;
            }
            if let Ok(mut guard) = BROWSERS.lock() {
                guard.get_or_insert_with(HashMap::new).insert(id, browser.clone());
            }
            let _ = PENDING_CREATES.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1));
            crate::tabs::attach_pending(id);
            let n = CREATED.fetch_add(1, Ordering::Relaxed) + 1;
            println!("browser: created id={id} (#{n})");
            // Being created is not damage: a brand new browser has nothing to
            // repaint until it is told its size and shown (haktui, 2026-08-26).
            // It starts parked; if its tab is the active one it comes on
            // screen here, on CEF's UI thread, whichever thread that is.
            if let Some(host) = browser.host() {
                host.was_resized();
                host.was_hidden(1);
            }
            crate::devtools::observe(browser);
            if crate::tabs::active_browser() == id {
                activate_now(id);
            }
        }

        fn on_before_close(&self, browser: Option<&mut Browser>) {
            let id = id_of(browser);
            let removed = BROWSERS
                .lock()
                .ok()
                .and_then(|mut g| g.as_mut().and_then(|m| m.remove(&id)))
                .is_some();
            if removed {
                CLOSED.fetch_add(1, Ordering::Relaxed);
                crate::render::forget(id);
                crate::tabs::detach(id);
            }
            crate::devtools::on_before_close(id);
            crate::cef_thread::notify_closed();
            println!("browser: closed id={id}");
        }
    }
}

wrap_load_handler! {
    struct Load;
    impl LoadHandler {
        fn on_loading_state_change(
            &self,
            browser: Option<&mut Browser>,
            is_loading: ::std::os::raw::c_int,
            can_go_back: ::std::os::raw::c_int,
            can_go_forward: ::std::os::raw::c_int,
        ) {
            update_by_browser(id_of(browser), |p| {
                p.loading = is_loading != 0;
                p.can_back = can_go_back != 0;
                p.can_forward = can_go_forward != 0;
                if !p.loading {
                    p.progress = 1.0;
                }
            });
        }

        fn on_load_end(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            status: ::std::os::raw::c_int,
        ) {
            let main = frame.map(|f| f.is_main() != 0).unwrap_or(false);
            if main {
                println!("browser: load_end status={status}");
                crate::devtools::on_load_end(browser.map(|b| b.identifier()).unwrap_or(0));
            }
            if status == 200 {
                LOAD_END_OK.fetch_add(1, Ordering::Relaxed);
            } else {
                LOAD_END_OTHER.fetch_add(1, Ordering::Relaxed);
            }
        }

        fn on_load_error(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            error: Errorcode,
            error_text: Option<&CefString>,
            failed_url: Option<&CefString>,
        ) {
            let text = error_text.map(|t| t.to_string()).unwrap_or_default();
            let failed = failed_url.map(|u| u.to_string()).unwrap_or_default();
            println!("browser: LOAD ERROR {error:?} {text:?} {failed:?}");
            // ERR_ABORTED is a navigation replaced by another one, not a
            // failure to read about.
            let aborted = error == Errorcode::ABORTED;
            let main = frame.map(|f| f.is_main() != 0).unwrap_or(false);
            if main && !aborted {
                update_by_browser(id_of(browser), |p| p.failed(&failed, &text));
            }
        }
    }
}

wrap_display_handler! {
    struct Display;
    impl DisplayHandler {
        fn on_address_change(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            url: Option<&CefString>,
        ) {
            if frame.map(|f| f.is_main() == 0).unwrap_or(false) {
                return;
            }
            let url = url.map(|u| u.to_string()).unwrap_or_default();
            println!("browser: address {url}");
            update_by_browser(id_of(browser), |p| p.committed(url));
        }

        fn on_loading_progress_change(&self, browser: Option<&mut Browser>, progress: f64) {
            update_by_browser(id_of(browser), |p| p.progressed(progress));
        }

        fn on_title_change(&self, browser: Option<&mut Browser>, title: Option<&CefString>) {
            let title = title.map(|t| t.to_string()).unwrap_or_default();
            update_by_browser(id_of(browser), |p| p.title = title);
        }
    }
}

wrap_find_handler! {
    struct Find;
    impl FindHandler {
        fn on_find_result(
            &self,
            browser: Option<&mut Browser>,
            _identifier: ::std::os::raw::c_int,
            count: ::std::os::raw::c_int,
            _selection_rect: Option<&Rect>,
            active_match_ordinal: ::std::os::raw::c_int,
            final_update: ::std::os::raw::c_int,
        ) {
            update_by_browser(id_of(browser), |p| {
                p.find = Some(crate::page::FindState {
                    current: active_match_ordinal.max(0) as u32,
                    total: count.max(0) as u32,
                    final_update: final_update != 0,
                });
            });
        }
    }
}

/// Find in the active tab. `find_next` continues the current search;
/// otherwise a new one starts. Results arrive in `Page::find`.
pub(crate) fn find(text: &str, forward: bool, find_next: bool) {
    let id = crate::tabs::active_browser();
    let text = text.to_string();
    crate::cef_thread::on_ui(move || {
        if let Some(host) = host_of(id) {
            host.find(Some(&CefString::from(text.as_str())), i32::from(forward), 0, i32::from(find_next));
        }
    });
}

pub(crate) fn stop_find(clear_selection: bool) {
    let id = crate::tabs::active_browser();
    crate::cef_thread::on_ui(move || {
        if let Some(host) = host_of(id) {
            host.stop_finding(i32::from(clear_selection));
        }
    });
    crate::tabs::update_active(|p| p.find = None);
}

wrap_client! {
    struct BrowserClient;
    impl Client {
        fn render_handler(&self) -> Option<RenderHandler> {
            Some(crate::render::render_handler())
        }
        fn find_handler(&self) -> Option<FindHandler> {
            Some(Find::new())
        }
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(Display::new())
        }
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(LifeSpan::new())
        }
        fn load_handler(&self) -> Option<LoadHandler> {
            Some(Load::new())
        }
    }
}

/// No parent window: the browser is offscreen.
fn no_parent() -> cef::sys::cef_window_handle_t {
    #[cfg(windows)]
    {
        cef::sys::HWND(std::ptr::null_mut())
    }
    #[cfg(target_os = "macos")]
    {
        std::ptr::null_mut()
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        0
    }
}

/// Make an offscreen browser on `url`, parked until `activate`. Main
/// thread, after `initialize`. Inline mode creates it synchronously and
/// returns CEF's identifier; threaded mode posts the create and returns
/// `None`, and `on_after_created` binds the browser to its tab.
pub(crate) fn open(url: &str) -> Option<i32> {
    let mut window_info = WindowInfo::default().set_as_windowless(no_parent());
    // Windows, `SURYA_BROWSER_ZERO_COPY=1` and the D3D11 device could be
    // made: CEF paints into a shared texture and calls `on_accelerated_paint`
    // instead of `on_paint`; see `zero_copy`. Otherwise the CPU path.
    window_info.shared_texture_enabled = i32::from(crate::zero_copy::ready());
    let browser_settings = BrowserSettings {
        windowless_frame_rate: crate::display::frame_rate(),
        background_color: crate::OPAQUE_WHITE,
        ..Default::default()
    };
    let mut client = BrowserClient::new();
    if crate::cef_thread::threaded() {
        PENDING_CREATES.fetch_add(1, Ordering::AcqRel);
        let asked = browser_host_create_browser(
            Some(&window_info),
            Some(&mut client),
            Some(&CefString::from(url)),
            Some(&browser_settings),
            None,
            None,
        );
        if asked == 0 {
            let _ = PENDING_CREATES.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1));
        }
        return None;
    }
    browser_host_create_browser_sync(
        Some(&window_info),
        Some(&mut client),
        Some(&CefString::from(url)),
        Some(&browser_settings),
        None,
        None,
    )
    .map(|b| b.identifier())
}
