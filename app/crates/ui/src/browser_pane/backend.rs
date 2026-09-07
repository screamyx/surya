//! Every call the pane makes into `surya-browser`, in one file.
//!
//! The crate is surya-cef2's. Keeping the calls here means a change to its
//! signatures is a one-file fix on this side, and the rest of the pane reads
//! as plain UI code. The shapes were agreed over agb on 2026-09-05:
//! `tab_open/tab_close/tab_activate/tabs/active_tab`, `find/stop_find`,
//! `set_zoom/zoom`, and `page()` as the active tab's snapshot.

pub type TabId = surya_browser::TabId;

/// One row in the strip.
pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub url: String,
    pub loading: bool,
}

pub fn tabs() -> Vec<Tab> {
    surya_browser::tabs()
        .into_iter()
        .map(|t| Tab { id: t.id, title: t.title, url: t.url, loading: t.loading })
        .collect()
}

pub fn active_tab() -> Option<TabId> {
    surya_browser::active_tab()
}

pub fn tab_open(url: &str) -> TabId {
    surya_browser::tab_open(url)
}

pub fn tab_close(id: TabId) {
    surya_browser::tab_close(id);
}

pub fn tab_activate(id: TabId) {
    surya_browser::tab_activate(id);
}

/// The active tab: address, load state, and the last find result.
pub fn page() -> surya_browser::Page {
    surya_browser::page()
}

/// The active tab's find reading: which match, of how many. `None` until
/// CEF's find handler has answered. The crate publishes `current` and
/// `total` on the page snapshot; the cast keeps this side working whether
/// they are signed or unsigned.
pub fn find_result(page: &surya_browser::Page) -> Option<(i32, i32)> {
    page.find.as_ref().map(|f| (f.current as i32, f.total as i32))
}

/// What a typed address means, before it is loaded. Scheme-less input gets
/// `https://`, plain words go to DuckDuckGo, and `file://` gets nothing.
pub fn resolve(typed: &str) -> String {
    surya_browser::navigate_to(typed)
}

pub fn navigate(typed: &str) {
    surya_browser::navigate(typed);
}

pub fn back() {
    surya_browser::back();
}

pub fn forward() {
    surya_browser::forward();
}

pub fn reload() {
    surya_browser::reload();
}

pub fn stop() {
    surya_browser::stop();
}

pub fn find(text: &str, forward: bool, find_next: bool) {
    surya_browser::find(text, forward, find_next);
}

pub fn stop_find(clear_selection: bool) {
    surya_browser::stop_find(clear_selection);
}

pub fn set_zoom(level: f64) {
    surya_browser::set_zoom(level);
}

/// The keyboard is in the page, or in one of the pane's own fields.
pub fn set_page_focus(on: bool) {
    surya_browser::set_focus(on);
}

/// The page's pixels plus its input listeners.
pub fn panel(focus: &gpui::FocusHandle) -> gpui::AnyElement {
    surya_browser::panel(focus)
}

/// What to tell the person when the pane has no page, or `None` while it has
/// one. The crate owns the words; this side paints them (see
/// `mod.rs::note_colors`).
pub fn off_note() -> Option<surya_browser::OffNote> {
    surya_browser::off_note()
}
