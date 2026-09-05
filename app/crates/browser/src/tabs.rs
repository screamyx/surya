//! The tab strip's model: which browsers exist, which one is on screen, and
//! each one's [`Page`]. One CEF browser per tab (haktui's shape, chrome.rs);
//! only the active tab paints, the others idle at one frame a second.
//!
//! The model works without CEF (`SURYA_NO_BROWSER`, or before `start`): a
//! tab then has no browser and its page never changes, but the strip can be
//! drawn and tested in a run that has no Chromium in it.
//!
//! This is the one source of "which browser is active": `client.rs` and
//! `render.rs` ask [`active_browser`]. `TABS` is never held while calling
//! into CEF or taking another lock; every `with` closure only touches the
//! model.

use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::Mutex;

use cef::ImplBrowserHost as _;

use crate::page::Page;

pub type TabId = u32;

/// What the strip draws for one tab.
#[derive(Clone, Debug)]
pub struct TabInfo {
    pub id: TabId,
    pub title: String,
    pub url: String,
    pub loading: bool,
    pub can_back: bool,
    pub can_forward: bool,
}

#[derive(Debug)]
struct Tab {
    id: TabId,
    /// CEF's browser identifier, 0 while the tab has no browser.
    browser: i32,
    page: Page,
    /// CEF zoom level (0.0 is 100 percent), as last set through [`set_zoom`].
    zoom: f64,
}

/// What `Tabs::close` took out.
#[derive(Debug, PartialEq, Eq)]
struct Closed {
    id: TabId,
    /// CEF's identifier, 0 for none.
    browser: i32,
    was_active: bool,
    /// The address it showed, kept for `reopen` when it was the last tab.
    url: String,
}

#[derive(Debug, Default)]
struct Tabs {
    tabs: Vec<Tab>,
    active: usize,
    next: TabId,
}

impl Tabs {
    fn index_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    fn open(&mut self, url: &str) -> TabId {
        self.next += 1;
        let id = self.next;
        let mut page = Page::default();
        page.begin_navigation(url);
        self.tabs.push(Tab { id, browser: 0, page, zoom: 0.0 });
        self.active = self.tabs.len() - 1;
        id
    }

    /// Remove a tab. The neighbour to the left becomes active when the
    /// active tab closes, as Chrome does when it was the last one.
    fn close(&mut self, id: TabId) -> Option<Closed> {
        let index = self.index_of(id)?;
        let removed = self.tabs.remove(index);
        let was_active = index == self.active;
        if self.tabs.is_empty() {
            self.active = 0;
        } else if index < self.active || (was_active && self.active == self.tabs.len()) {
            self.active -= 1;
        }
        Some(Closed { id: removed.id, browser: removed.browser, was_active, url: removed.page.url })
    }

    fn index_of_browser(&self, browser: i32) -> Option<usize> {
        if browser == 0 {
            return None;
        }
        self.tabs.iter().position(|t| t.browser == browser)
    }

    fn activate(&mut self, id: TabId) -> bool {
        match self.index_of(id) {
            Some(i) => {
                self.active = i;
                true
            }
            None => false,
        }
    }

    fn active(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    fn active_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active)
    }

    fn by_browser_mut(&mut self, browser: i32) -> Option<&mut Tab> {
        if browser == 0 {
            return None;
        }
        self.tabs.iter_mut().find(|t| t.browser == browser)
    }

    fn infos(&self) -> Vec<TabInfo> {
        self.tabs
            .iter()
            .map(|t| TabInfo {
                id: t.id,
                title: t.page.title.clone(),
                url: t.page.shown_address().to_string(),
                loading: t.page.loading,
                can_back: t.page.can_back,
                can_forward: t.page.can_forward,
            })
            .collect()
    }
}

static TABS: Mutex<Tabs> = Mutex::new(Tabs { tabs: Vec::new(), active: 0, next: 0 });
/// The tab whose browser CEF is creating right now: `on_after_created`
/// fires inside `create_browser_sync`, before `open` returns the id.
static PENDING: AtomicU32 = AtomicU32::new(0);
/// The last address shown before every tab closed, for [`crate::reopen`].
static LAST_URL: Mutex<String> = Mutex::new(String::new());
static OPENED: AtomicI32 = AtomicI32::new(0);

/// Run `f` on the model. A poisoned lock (a panic while it was held) is
/// taken anyway: the model is plain data and a tab strip that stops
/// answering is worse than one that shows the last state.
fn with<R>(f: impl FnOnce(&mut Tabs) -> R) -> R {
    let mut guard = TABS.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

/// The address a new tab opens on. Typed text goes through
/// [`crate::page::navigate_to`]; nothing typed, or something it refuses,
/// gives a blank page rather than no tab.
fn tab_url(typed: &str) -> String {
    let url = crate::page::navigate_to(typed);
    if url.is_empty() { "about:blank".to_string() } else { url }
}

/// Open a tab on `typed` and make it the active one. With CEF running the
/// tab gets its own browser at once; without, the tab exists and shows the
/// address it was asked for.
pub fn tab_open(typed: &str) -> TabId {
    let url = tab_url(typed);
    let id = with(|t| t.open(&url));
    OPENED.fetch_add(1, Ordering::Relaxed);
    if crate::has_cef() {
        PENDING.store(id, Ordering::Release);
        let browser = crate::client::open(&url);
        PENDING.store(0, Ordering::Release);
        if let Some(browser) = browser {
            with(|t| {
                if let Some(i) = t.index_of(id) {
                    t.tabs[i].browser = browser;
                }
            });
            crate::client::activate(browser);
        }
    }
    println!("browser: tab_open id={id} url={url} tabs={}", count());
    crate::pump::schedule_pump(0);
    id
}

/// CEF made the browser for the tab that is being opened. Called from
/// `on_after_created`, before `open` has returned.
pub(crate) fn attach_pending(browser: i32) {
    let id = PENDING.load(Ordering::Acquire);
    if id == 0 {
        return;
    }
    with(|t| {
        if let Some(i) = t.index_of(id) {
            t.tabs[i].browser = browser;
        }
    });
}

/// Close a tab. Its browser goes with it; the neighbour becomes active.
pub fn tab_close(id: TabId) {
    let Some(closed) = with(|t| t.close(id)) else { return };
    after_close(&closed);
    println!("browser: tab_close id={id} tabs={}", count());
    crate::pump::schedule_pump(0);
}

/// What every close has in common: the browser goes, the strip's new
/// active tab (if the closed one was it) comes on screen, and the last
/// tab's address is kept for [`crate::reopen`].
fn after_close(closed: &Closed) {
    if closed.browser != 0 {
        crate::client::close_browser(closed.browser);
    }
    if closed.was_active {
        show_active();
    }
    if count() == 0 {
        remember(&closed.url);
    }
}

pub fn tab_activate(id: TabId) {
    if !with(|t| t.activate(id)) {
        return;
    }
    show_active();
    crate::pump::schedule_pump(0);
}

/// Bring the active tab's browser on screen and park the others (all of
/// them, when no tab has a browser).
fn show_active() {
    crate::client::activate(active_browser());
}

/// Every tab in strip order, as the strip draws them.
pub fn tabs() -> Vec<TabInfo> {
    with(|t| t.infos())
}

/// The tab on screen, if any tab exists.
pub fn active_tab() -> Option<TabId> {
    with(|t| t.active().map(|t| t.id))
}

/// How many tabs exist.
pub fn count() -> usize {
    with(|t| t.tabs.len())
}

/// The active tab's page, as the address bar reads it.
pub fn page() -> Page {
    with(|t| t.active().map(|t| t.page.clone()).unwrap_or_default())
}

/// CEF's identifier for the active tab's browser, 0 for none. The one
/// answer to "which browser is on screen".
pub(crate) fn active_browser() -> i32 {
    with(|t| t.active().map(|t| t.browser).unwrap_or(0))
}

/// Whether `browser`'s main frame is loading right now (CEF's loading
/// state, `on_loading_state_change`). False for a browser no tab owns.
pub(crate) fn loading(browser: i32) -> bool {
    with(|t| t.by_browser_mut(browser).map(|t| t.page.loading).unwrap_or(false))
}

pub(crate) fn update_active(f: impl FnOnce(&mut Page)) {
    with(|t| {
        if let Some(tab) = t.active_mut() {
            f(&mut tab.page);
        }
    });
}

/// A CEF callback for `browser` updates that browser's tab, wherever it is
/// in the strip. A callback for a browser no tab owns is dropped.
pub(crate) fn update_by_browser(browser: i32, f: impl FnOnce(&mut Page)) {
    with(|t| {
        if let Some(tab) = t.by_browser_mut(browser) {
            f(&mut tab.page);
        }
    });
}

/// CEF closed `browser` on its own (the page called `window.close`, or the
/// app is shutting down, or `tab_close` asked): the tab goes too if it is
/// still here. `browser` 0 names nothing.
pub(crate) fn detach(browser: i32) {
    if browser == 0 {
        return;
    }
    let closed = with(|t| t.index_of_browser(browser).map(|i| t.tabs[i].id).and_then(|id| t.close(id)));
    let Some(mut closed) = closed else { return };
    // The browser is already gone; only the strip needs settling.
    closed.browser = 0;
    println!("browser: tab {} gone with its browser {browser}", closed.id);
    after_close(&closed);
}

fn remember(url: &str) {
    if !url.is_empty()
        && let Ok(mut last) = LAST_URL.lock()
    {
        *last = url.to_string();
    }
}

/// Close every tab (the Browser surface was closed). The active tab's
/// address is kept for [`crate::reopen`].
pub(crate) fn close_all() {
    remember(&page().url);
    let browsers: Vec<i32> = with(|t| {
        let ids: Vec<i32> = t.tabs.iter().map(|t| t.browser).filter(|b| *b != 0).collect();
        t.tabs.clear();
        t.active = 0;
        ids
    });
    for b in browsers {
        crate::client::close_browser(b);
    }
}

pub(crate) fn last_url() -> String {
    LAST_URL.lock().map(|l| l.clone()).unwrap_or_default()
}

/// CEF zoom level for the active tab. `0.0` is 100 percent; see
/// `input::commands::zoom` for Chrome's steps and limits.
pub fn set_zoom(level: f64) {
    with(|t| {
        if let Some(tab) = t.active_mut() {
            tab.zoom = level;
        }
    });
    if let Some(host) = crate::client::host() {
        host.set_zoom_level(level);
    }
}

pub fn zoom() -> f64 {
    with(|t| t.active().map(|t| t.zoom).unwrap_or(0.0))
}

/// `SURYA_SELFTEST_TABS=<secs>`: that long after start, switch to the first
/// tab once, from the pump (main thread), so a proof can grab the pane
/// before and after a tab switch with no strip to click.
pub(crate) fn selftest_tick() {
    use std::sync::OnceLock;
    static AT: OnceLock<Option<std::time::Instant>> = OnceLock::new();
    static DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let at = AT.get_or_init(|| {
        std::env::var("SURYA_SELFTEST_TABS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map(|secs| std::time::Instant::now() + std::time::Duration::from_secs(secs))
    });
    let Some(at) = at else { return };
    if std::time::Instant::now() < *at || DONE.swap(true, std::sync::atomic::Ordering::AcqRel) {
        return;
    }
    let first = with(|t| t.tabs.first().map(|t| t.id));
    if let Some(id) = first {
        tab_activate(id);
        println!("browser: selftest tabs: activated first tab {id}, active_browser={}", active_browser());
    }
}

pub(crate) fn counters() -> String {
    format!("tabs={} opened={} active={}", count(), OPENED.load(Ordering::Relaxed), active_tab().unwrap_or(0))
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tests;
