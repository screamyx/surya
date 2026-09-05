//! The tab strip's model: which browsers exist, which one is on screen, and
//! each one's [`Page`]. One CEF browser per tab (haktui's shape, chrome.rs);
//! only the active tab paints, the others idle at one frame a second.
//!
//! The model works without CEF (`SURYA_NO_BROWSER`, or before `start`): a
//! tab then has no browser and its page never changes, but the strip can be
//! drawn and tested in a run that has no Chromium in it.

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

    /// Remove a tab. Returns its browser id (0 for none) and whether the
    /// active tab changed. The neighbour to the left becomes active when the
    /// active tab closes, as Chrome does when it was the last one.
    fn close(&mut self, id: TabId) -> Option<(i32, bool)> {
        let index = self.index_of(id)?;
        let removed = self.tabs.remove(index);
        let was_active = index == self.active;
        if self.tabs.is_empty() {
            self.active = 0;
        } else if index < self.active || (was_active && self.active == self.tabs.len()) {
            self.active -= 1;
        }
        Some((removed.browser, was_active))
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

fn with<R>(f: impl FnOnce(&mut Tabs) -> R) -> Option<R> {
    TABS.lock().ok().map(|mut t| f(&mut t))
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
    let id = with(|t| t.open(&url)).unwrap_or(0);
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
    let Some(Some((browser, was_active))) = with(|t| t.close(id)) else { return };
    if browser != 0 {
        crate::client::close_browser(browser);
    }
    if was_active {
        show_active();
    }
    remember_last();
    println!("browser: tab_close id={id} tabs={}", count());
    crate::pump::schedule_pump(0);
}

pub fn tab_activate(id: TabId) {
    if with(|t| t.activate(id)) != Some(true) {
        return;
    }
    show_active();
    crate::pump::schedule_pump(0);
}

/// Bring the active tab's browser on screen and park the others.
fn show_active() {
    let browser = active_browser();
    if browser != 0 {
        crate::client::activate(browser);
    }
}

pub fn tabs() -> Vec<TabInfo> {
    with(|t| t.infos()).unwrap_or_default()
}

pub fn active_tab() -> Option<TabId> {
    with(|t| t.active().map(|t| t.id)).flatten()
}

pub fn count() -> usize {
    with(|t| t.tabs.len()).unwrap_or(0)
}

/// The active tab's page, as the address bar reads it.
pub fn page() -> Page {
    with(|t| t.active().map(|t| t.page.clone()).unwrap_or_default()).unwrap_or_default()
}

/// CEF's identifier for the active tab's browser, 0 for none.
pub(crate) fn active_browser() -> i32 {
    with(|t| t.active().map(|t| t.browser).unwrap_or(0)).unwrap_or(0)
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
/// app is shutting down): the tab goes too.
pub(crate) fn detach(browser: i32) {
    let closed = with(|t| {
        let id = t.tabs.iter().find(|tab| tab.browser == browser).map(|tab| tab.id);
        id.and_then(|id| t.close(id).map(|(_, was_active)| (id, was_active)))
    })
    .flatten();
    if let Some((id, was_active)) = closed {
        println!("browser: tab {id} gone with its browser");
        if was_active {
            show_active();
        }
    }
    remember_last();
}

fn remember_last() {
    if count() == 0 {
        return;
    }
    let url = page().url;
    if !url.is_empty()
        && let Ok(mut last) = LAST_URL.lock()
    {
        *last = url;
    }
}

/// Close every tab (the Browser surface was closed). The last address is
/// kept for [`crate::reopen`].
pub(crate) fn close_all() {
    remember_last();
    let browsers: Vec<i32> = with(|t| {
        let ids: Vec<i32> = t.tabs.iter().map(|t| t.browser).filter(|b| *b != 0).collect();
        t.tabs.clear();
        t.active = 0;
        ids
    })
    .unwrap_or_default();
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
    with(|t| t.active().map(|t| t.zoom).unwrap_or(0.0)).unwrap_or(0.0)
}

pub(crate) fn counters() -> String {
    format!("tabs={} opened={} active={}", count(), OPENED.load(Ordering::Relaxed), active_tab().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_makes_the_new_tab_active_and_ids_never_repeat() {
        let mut t = Tabs::default();
        let a = t.open("https://a");
        let b = t.open("https://b");
        assert_ne!(a, b);
        assert_eq!(t.active().map(|t| t.id), Some(b));
        assert_eq!(t.infos().len(), 2);
        assert_eq!(t.infos()[1].url, "https://b");
        assert!(t.infos()[1].loading);
    }

    #[test]
    fn closing_the_active_tab_activates_its_left_neighbour() {
        let mut t = Tabs::default();
        let a = t.open("https://a");
        let b = t.open("https://b");
        let c = t.open("https://c");
        t.activate(c);
        assert_eq!(t.close(c), Some((0, true)));
        assert_eq!(t.active().map(|t| t.id), Some(b));
        t.activate(a);
        assert_eq!(t.close(a), Some((0, true)));
        assert_eq!(t.active().map(|t| t.id), Some(b));
        assert_eq!(t.close(b), Some((0, true)));
        assert!(t.active().is_none());
        assert_eq!(t.close(b), None);
    }

    #[test]
    fn closing_a_tab_left_of_the_active_one_keeps_the_active_tab() {
        let mut t = Tabs::default();
        let a = t.open("https://a");
        let b = t.open("https://b");
        let c = t.open("https://c");
        t.activate(b);
        assert_eq!(t.close(a), Some((0, false)));
        assert_eq!(t.active().map(|t| t.id), Some(b));
        assert_eq!(t.close(c), Some((0, false)));
        assert_eq!(t.active().map(|t| t.id), Some(b));
    }

    #[test]
    fn callbacks_land_on_the_tab_that_owns_the_browser() {
        let mut t = Tabs::default();
        let a = t.open("https://a");
        let b = t.open("https://b");
        t.tabs[0].browser = 7;
        t.tabs[1].browser = 9;
        t.by_browser_mut(7).unwrap().page.title = "A".into();
        t.by_browser_mut(9).unwrap().page.committed("https://b/".into());
        assert!(t.by_browser_mut(0).is_none());
        assert!(t.by_browser_mut(8).is_none());
        let infos = t.infos();
        assert_eq!((infos[0].id, infos[0].title.as_str()), (a, "A"));
        assert_eq!((infos[1].id, infos[1].url.as_str()), (b, "https://b/"));
    }

    #[test]
    fn a_new_tab_never_opens_what_the_bar_refuses() {
        assert_eq!(tab_url(""), "about:blank");
        assert_eq!(tab_url("file:///etc/passwd"), "about:blank");
        assert_eq!(tab_url("example.com"), "https://example.com");
    }
}
