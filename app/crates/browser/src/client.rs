//! The one browser, and the CEF client that owns its callbacks: lifetime,
//! loads, and what the address bar shows.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use cef::rc::Rc as _;
use cef::*;

use crate::page::update_page;

static BROWSER: Mutex<Option<Browser>> = Mutex::new(None);
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

/// Whether the one browser exists right now.
pub(crate) fn is_open() -> bool {
    BROWSER.lock().map(|b| b.is_some()).unwrap_or(false)
}

/// The pane went on or off screen. A hidden browser is told so, which
/// stops the compositor, and its frame cap drops to one a second so a page
/// that animates costs nothing while nobody looks. Idempotent.
pub(crate) fn set_visible(on: bool) {
    if VISIBLE.swap(on, Ordering::AcqRel) == on {
        return;
    }
    let Some(host) = host() else { return };
    if on {
        host.set_windowless_frame_rate(60);
        host.was_hidden(0);
        // A resize may have happened while hidden; CEF only paints damage.
        host.was_resized();
    } else {
        host.was_hidden(1);
        host.set_windowless_frame_rate(1);
    }
    println!("browser: visible={}", u8::from(on));
}

/// Close the one browser. CEF answers with `on_before_close`, which clears
/// the slot; the pane's frame stays until a new browser paints.
pub(crate) fn close() {
    if let Some(host) = host() {
        host.close_browser(1);
    }
}
static LOAD_END_OK: AtomicU64 = AtomicU64::new(0);
static LOAD_END_OTHER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn browser() -> Option<Browser> {
    BROWSER.lock().ok()?.clone()
}

pub(crate) fn host() -> Option<BrowserHost> {
    browser().and_then(|b| b.host())
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
            // One pane, one browser: a target=_blank link or window.open
            // opens in this pane instead of a second browser that would
            // take the frame slot (haktui gives it a tab; there are none
            // here yet). Cancelled, then the opener navigates.
            // The same allowlist as the address bar: `window.open("file:///..")`
            // gets nothing, not the disk.
            let asked = target_url.map(|u| u.to_string()).unwrap_or_default();
            let url = crate::page::navigate_to(&asked);
            POPUPS_REFUSED.fetch_add(1, Ordering::Relaxed);
            println!("browser: popup refused, opening in the pane: {url:?} (asked {asked:?})");
            if let (Some(browser), false) = (browser, url.is_empty())
                && let Some(frame) = browser.main_frame()
            {
                update_page(|p| p.begin_navigation(&url));
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
            if let Ok(mut slot) = BROWSER.lock() {
                *slot = Some(browser.clone());
            }
            let n = CREATED.fetch_add(1, Ordering::Relaxed) + 1;
            println!("browser: created id={id} (#{n})");
            // Being created is not damage: a brand new browser has nothing to
            // repaint until it is told its size and shown (haktui, 2026-08-26).
            if let Some(host) = browser.host() {
                host.was_resized();
                host.was_hidden(1);
                host.was_hidden(0);
                host.set_focus(1);
            }
        }

        fn on_before_close(&self, browser: Option<&mut Browser>) {
            let id = browser.map(|b| b.identifier()).unwrap_or(0);
            if let Ok(mut slot) = BROWSER.lock()
                && slot.as_ref().map(|b| b.identifier()) == Some(id)
            {
                *slot = None;
                CLOSED.fetch_add(1, Ordering::Relaxed);
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
            _browser: Option<&mut Browser>,
            is_loading: ::std::os::raw::c_int,
            can_go_back: ::std::os::raw::c_int,
            can_go_forward: ::std::os::raw::c_int,
        ) {
            update_page(|p| {
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
            _browser: Option<&mut Browser>,
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
                update_page(|p| p.failed(&failed, &text));
            }
        }
    }
}

wrap_display_handler! {
    struct Display;
    impl DisplayHandler {
        fn on_address_change(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            url: Option<&CefString>,
        ) {
            if frame.map(|f| f.is_main() == 0).unwrap_or(false) {
                return;
            }
            let url = url.map(|u| u.to_string()).unwrap_or_default();
            println!("browser: address {url}");
            update_page(|p| p.committed(url));
        }

        fn on_loading_progress_change(&self, _browser: Option<&mut Browser>, progress: f64) {
            update_page(|p| p.progressed(progress));
        }

        fn on_title_change(&self, _browser: Option<&mut Browser>, title: Option<&CefString>) {
            let title = title.map(|t| t.to_string()).unwrap_or_default();
            update_page(|p| p.title = title);
        }
    }
}

wrap_client! {
    struct BrowserClient;
    impl Client {
        fn render_handler(&self) -> Option<RenderHandler> {
            Some(crate::render::render_handler())
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

/// Make the offscreen browser. Main thread, after `initialize`.
pub(crate) fn open(url: &str) -> bool {
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
    .is_some()
}
