//! Find in page: the search the pane's ctrl-f field runs, and CEF's
//! running count of matches back into the tab's `Page::find`.

use cef::*;

use crate::client::{host_of, id_of};
use crate::tabs::update_by_browser;

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

/// The handler the client hands CEF for its find callbacks.
pub(crate) fn handler() -> FindHandler {
    Find::new()
}
