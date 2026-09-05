//! The browsers (one per tab) and the CEF client that owns their callbacks:
//! lifetime, loads, find results, and what each tab's address bar shows.
//! Every callback names its browser; `tabs.rs` maps that to a tab.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::Mutex;

use cef::rc::Rc as _;
use cef::*;

use crate::tabs::update_by_browser;

/// Every live browser by CEF identifier.
static BROWSERS: Mutex<Option<HashMap<i32, Browser>>> = Mutex::new(None);
/// The browser on screen (the active tab's), 0 for none.
static ACTIVE: AtomicI32 = AtomicI32::new(0);
static CREATED: AtomicU64 = AtomicU64::new(0);
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

/// Show a browser: paint at 60, tell it the size it may have missed, and
/// give it the keyboard.
fn show(host: &BrowserHost) {
    host.set_windowless_frame_rate(60);
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
    if let Some(host) = host() {
        if on { show(&host) } else { park(&host) }
    }
    println!("browser: visible={}", u8::from(on));
}

/// Make `browser` the one on screen and park every other. A tab switch.
pub(crate) fn activate(browser: i32) {
    let previous = ACTIVE.swap(browser, Ordering::AcqRel);
    if previous == browser {
        return;
    }
    let Ok(guard) = BROWSERS.lock() else { return };
    let Some(map) = guard.as_ref() else { return };
    if let Some(host) = map.get(&previous).and_then(|b| b.host()) {
        park(&host);
    }
    if let Some(host) = map.get(&browser).and_then(|b| b.host()) {
        if VISIBLE.load(Ordering::Acquire) {
            show(&host);
        } else {
            park(&host);
        }
    }
    println!("browser: activate browser={browser} (was {previous})");
}

/// Close one browser. CEF answers with `on_before_close`, which drops it
/// from the map and its frame; the pane shows the next active tab's frame.
pub(crate) fn close_browser(browser: i32) {
    let host = BROWSERS
        .lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(|m| m.get(&browser).cloned()))
        .and_then(|b| b.host());
    if let Some(host) = host {
        host.close_browser(1);
    }
}

/// Close every browser (the Browser surface was closed, or the app quits).
pub(crate) fn close_all() {
    crate::tabs::close_all();
    let hosts: Vec<BrowserHost> = BROWSERS
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|m| m.values().filter_map(|b| b.host()).collect()))
        .unwrap_or_default();
    for host in hosts {
        host.close_browser(1);
    }
}
static LOAD_END_OK: AtomicU64 = AtomicU64::new(0);
static LOAD_END_OTHER: AtomicU64 = AtomicU64::new(0);

/// The active tab's browser.
pub(crate) fn browser() -> Option<Browser> {
    let id = ACTIVE.load(Ordering::Acquire);
    BROWSERS.lock().ok()?.as_ref()?.get(&id).cloned()
}

pub(crate) fn host() -> Option<BrowserHost> {
    browser().and_then(|b| b.host())
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
            crate::tabs::attach_pending(id);
            let n = CREATED.fetch_add(1, Ordering::Relaxed) + 1;
            println!("browser: created id={id} (#{n})");
            // Being created is not damage: a brand new browser has nothing to
            // repaint until it is told its size and shown (haktui, 2026-08-26).
            // It starts parked; `activate` shows the one the tab strip picked.
            if let Some(host) = browser.host() {
                host.was_resized();
                host.was_hidden(1);
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
                let _ = ACTIVE.compare_exchange(id, 0, Ordering::AcqRel, Ordering::Acquire);
                crate::render::forget(id);
                crate::tabs::detach(id);
            }
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
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            status: ::std::os::raw::c_int,
        ) {
            let main = frame.map(|f| f.is_main() != 0).unwrap_or(false);
            if main {
                println!("browser: load_end status={status}");
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
    if let Some(host) = host() {
        host.find(Some(&CefString::from(text)), i32::from(forward), 0, i32::from(find_next));
    }
}

pub(crate) fn stop_find(clear_selection: bool) {
    if let Some(host) = host() {
        host.stop_finding(i32::from(clear_selection));
    }
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
/// thread, after `initialize`. Returns CEF's identifier for it.
pub(crate) fn open(url: &str) -> Option<i32> {
    let window_info = WindowInfo::default().set_as_windowless(no_parent());
    let browser_settings = BrowserSettings {
        windowless_frame_rate: 60,
        background_color: crate::OPAQUE_WHITE,
        ..Default::default()
    };
    let mut client = BrowserClient::new();
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
