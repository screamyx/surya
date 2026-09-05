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

pub use events::counters as input_counters;
pub use page::{navigate_to, page, Page};
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

/// Start CEF and open the one offscreen browser. Call once, inside
/// `Application::run`, before the window opens.
pub fn start(cx: &mut gpui::App) {
    if disabled() {
        println!("browser: SURYA_NO_BROWSER is set, the pane is a placeholder");
        return;
    }
    if STARTED.swap(true, Ordering::AcqRel) {
        panic!("surya_browser::start called twice");
    }
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
    let made = client::open(&url);
    println!("browser: create asked=1 made={} url={url}", u8::from(made));
    // Kick the loop once so CEF gets going before its first callback.
    pump::schedule_pump(0);
    pump::start_heartbeat();
}

/// Load a typed address in the page. `typed` is what the person wrote; see
/// [`navigate_to`] for how it becomes a URL.
pub fn navigate(typed: &str) {
    let url = navigate_to(typed);
    page::update_page(|p| p.begin_navigation(&url));
    if let Some(frame) = client::browser().and_then(|b| b.main_frame()) {
        frame.load_url(Some(&CefString::from(url.as_str())));
    }
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
