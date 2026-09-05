//! Offscreen Chromium in a gpui pane.
//!
//! One browser, one window. CEF is a process singleton, so the whole module
//! is process state behind statics rather than a gpui entity: a second
//! `start` in the same process would panic inside CEF.
//!
//! Call order, from the binary:
//!
//! 1. [`preflight`] as the first line of `main`. CEF re-executes the main
//!    binary for its helper processes when no helper binary sits beside it,
//!    and those must return before anything else runs.
//! 2. [`start`] inside `Application::run`, before the window opens.
//! 3. [`pump`] from the root view's `render`, and [`surface`] wherever the
//!    page should be drawn.
//!
//! Everything else here is the address bar's read side ([`page`]) and the
//! commands behind its buttons.

mod cef_app;
mod client;
mod events;
pub mod input;
mod page;
mod pump;
mod render;
mod surface;
pub mod tabs;

pub use events::counters as input_counters;
pub use page::{navigate_to, FindState, Page};
pub use tabs::{active_tab, page, set_zoom, tab_activate, tab_close, tab_open, tabs, zoom, TabId, TabInfo};
pub use pump::{counters, pump};
pub use surface::{panel, surface, surface_origin};

use std::sync::atomic::{AtomicBool, Ordering};

use cef::args::Args;
// The glob brings the `Impl*` traits that carry every method on `Browser`
// and `BrowserHost`; `cef::App` is a CEF type here, gpui's is spelled out.
use cef::*;

/// Set by [`start`] when CEF refused to initialize (another instance owns the
/// cache dir, or the runtime files are missing). The rest of the module then
/// behaves as `SURYA_NO_BROWSER`.
static DISABLED_AT_RUNTIME: AtomicBool = AtomicBool::new(false);
static STARTED: AtomicBool = AtomicBool::new(false);

/// `SURYA_NO_BROWSER=1` leaves CEF out of the process entirely: no helper
/// subprocesses, no Chromium start-up, and the pane shows a placeholder.
pub fn disabled() -> bool {
    std::env::var_os("SURYA_NO_BROWSER").is_some() || DISABLED_AT_RUNTIME.load(Ordering::Acquire)
}

/// What sits behind a page that paints no background of its own. Opaque
/// white, because a zero here is fully transparent and a plain page's black
/// text arrived black on the shell's dark canvas (haktui, 2026-08-26).
const OPAQUE_WHITE: u32 = 0xFFFF_FFFF;

/// Lock the CEF API version and let a helper process exit before the app
/// runs. Returns only in the browser process.
pub fn preflight() {
    if disabled() {
        return;
    }
    let _ = api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    let args = Args::new();
    let code = execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());
    if code >= 0 {
        std::process::exit(code);
    }
}

/// The body of the helper binary: hand the process to CEF and exit with its
/// code. Never returns.
pub fn helper_main() -> ! {
    let _ = api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    let args = Args::new();
    let code = execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());
    std::process::exit(code);
}

fn start_url() -> String {
    std::env::var("SURYA_BROWSER_URL").unwrap_or_else(|_| "https://example.com".to_string())
}

fn cache_dir() -> String {
    if let Ok(dir) = std::env::var("SURYA_CEF_CACHE") {
        return dir;
    }
    let base = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOME").ok().map(|h| format!("{h}/.local/share")))
        .unwrap_or_else(|| ".".to_string());
    format!("{base}/surya/cef")
}

/// CEF's subprocess binary, when it sits beside the main one. Without it
/// CEF re-executes the main binary, which `preflight` makes safe.
fn helper_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let name = if cfg!(windows) { "zeron-browser-helper.exe" } else { "zeron-browser-helper" };
    let path = dir.join(name);
    path.is_file().then_some(path)
}

/// The appearance the page should see through `prefers-color-scheme`.
/// The shell resolves its theme first and hands the answer to [`start`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    Light,
    Dark,
}

/// Start CEF and open the one offscreen browser. Call once, inside
/// `Application::run`, before the window opens. `scheme` is the app's
/// resolved appearance; the page's `prefers-color-scheme` follows it.
pub fn start(cx: &mut gpui::App, scheme: ColorScheme) {
    if disabled() {
        println!("browser: SURYA_NO_BROWSER is set, the pane is a placeholder");
        return;
    }
    if STARTED.swap(true, Ordering::AcqRel) {
        panic!("surya_browser::start called twice");
    }
    cef_app::set_color_scheme(scheme);
    let _ = api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    let args = Args::new();
    let cache = cache_dir();
    let _ = std::fs::create_dir_all(&cache);
    let helper = helper_path();
    println!(
        "browser: cache={cache} helper={}",
        helper.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "(re-exec self)".into())
    );
    let settings = Settings {
        windowless_rendering_enabled: 1,
        external_message_pump: 1,
        no_sandbox: 1,
        root_cache_path: CefString::from(cache.as_str()),
        persist_session_cookies: 1,
        background_color: OPAQUE_WHITE,
        remote_debugging_port: std::env::var("SURYA_CDP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        browser_subprocess_path: helper
            .map(|p| CefString::from(p.to_string_lossy().as_ref()))
            .unwrap_or_default(),
        // `SURYA_CEF_LOG=<file>`: Chromium's own log, verbose, for a
        // diagnosis. Off by default (CEF then logs errors to stderr).
        log_file: std::env::var("SURYA_CEF_LOG")
            .map(|f| CefString::from(f.as_str()))
            .unwrap_or_default(),
        log_severity: if std::env::var_os("SURYA_CEF_LOG").is_some() {
            LogSeverity::VERBOSE
        } else {
            LogSeverity::default()
        },
        ..Default::default()
    };
    let mut app = cef_app::SuryaApp::new();
    let ok = initialize(Some(args.as_main_args()), Some(&settings), Some(&mut app), std::ptr::null_mut());
    if ok != 1 {
        println!("browser: cef initialize failed (another instance on {cache}?); pane disabled");
        DISABLED_AT_RUNTIME.store(true, Ordering::Release);
        return;
    }
    pump::install(cx);
    let url = start_url();
    let id = tabs::tab_open(&url);
    let made = tabs::active_browser() != 0;
    println!("browser: create asked=1 made={} url={url} tab={id}", u8::from(made));
    // Kick the loop once so CEF gets going before its first callback.
    pump::schedule_pump(0);
    pump::start_heartbeat();
}

/// Whether CEF is running in this process, so a tab can have a browser.
pub(crate) fn has_cef() -> bool {
    !disabled() && STARTED.load(Ordering::Acquire)
}

/// Load a typed address in the active tab. `typed` is what the person
/// wrote; see [`navigate_to`] for how it becomes a URL, and which it
/// refuses. With no tab open, one opens.
pub fn navigate(typed: &str) {
    let url = navigate_to(typed);
    if url.is_empty() {
        return;
    }
    if tabs::count() == 0 {
        tabs::tab_open(&url);
        return;
    }
    tabs::update_active(|p| p.begin_navigation(&url));
    if let Some(frame) = client::browser().and_then(|b| b.main_frame()) {
        frame.load_url(Some(&CefString::from(url.as_str())));
    }
    pump::schedule_pump(0);
}

/// Find in the active tab; the running count lands in [`Page::find`].
/// `find_next` steps the current search, otherwise a new one starts.
pub fn find(text: &str, forward: bool, find_next: bool) {
    if text.is_empty() {
        stop_find(true);
        return;
    }
    client::find(text, forward, find_next);
    pump::schedule_pump(0);
}

/// End the search; `clear_selection` also drops the highlight.
pub fn stop_find(clear_selection: bool) {
    client::stop_find(clear_selection);
    pump::schedule_pump(0);
}

pub fn back() {
    if let Some(b) = client::browser() {
        b.go_back();
    }
}

pub fn forward() {
    if let Some(b) = client::browser() {
        b.go_forward();
    }
}

pub fn reload() {
    if let Some(b) = client::browser() {
        b.reload();
    }
}

pub fn reload_ignoring_cache() {
    if let Some(b) = client::browser() {
        b.reload_ignore_cache();
    }
}

pub fn stop() {
    if let Some(b) = client::browser() {
        b.stop_load();
    }
}

/// Keyboard focus into or out of the page.
pub fn set_focus(on: bool) {
    events::set_focus(on);
}

/// The pane is on screen, or not. Call every frame; it only acts on a change.
pub fn set_visible(on: bool) {
    if !disabled() {
        client::set_visible(on);
    }
}

/// Close every tab: the Browser surface was closed. Chromium's renderers
/// go away; the CEF process stays for a later [`reopen`].
pub fn close() {
    if !disabled() {
        tabs::close_all();
    }
}

/// Open a tab again after a [`close`], on the address last shown (or the
/// start page). A no-op while a tab exists.
pub fn reopen() {
    if disabled() || !STARTED.load(Ordering::Acquire) || tabs::count() > 0 {
        return;
    }
    let last = tabs::last_url();
    let url = if last.is_empty() { start_url() } else { last };
    let id = tabs::tab_open(&url);
    println!("browser: reopen asked=1 made={} url={url} tab={id}", u8::from(tabs::active_browser() != 0));
}

/// Bring CEF down with the app: close the browser, pump until it is gone,
/// then `cef_shutdown`. Main thread, from `on_app_quit`. Without this the
/// helper processes outlive a clean quit until their parent dies.
pub fn shutdown() {
    if disabled() || !STARTED.load(Ordering::Acquire) {
        return;
    }
    tabs::close_all();
    let started = std::time::Instant::now();
    let mut pumps = 0u32;
    while client::is_open() && started.elapsed() < std::time::Duration::from_secs(3) {
        do_message_loop_work();
        pumps += 1;
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    // A few more turns so CEF finishes its own teardown before shutdown.
    for _ in 0..20 {
        do_message_loop_work();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    println!(
        "browser: shutdown closed={} pumps={pumps} {}",
        u8::from(!client::is_open()),
        client::lifecycle_counters()
    );
    // A browser that did not answer in time still had a frame slot.
    render::forget_all();
    cef::shutdown();
}
