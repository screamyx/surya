//! Tests for `tabs.rs`, in their own file so the module stays under the
//! 500-line rule (decision 13).

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
    assert_eq!(t.close(c).map(|c| (c.id, c.was_active)), Some((c, true)));
    assert_eq!(t.active().map(|t| t.id), Some(b));
    t.activate(a);
    assert_eq!(t.close(a).map(|c| (c.id, c.was_active)), Some((a, true)));
    assert_eq!(t.active().map(|t| t.id), Some(b));
    assert_eq!(t.close(b).map(|c| c.was_active), Some(true));
    assert!(t.active().is_none());
    assert!(t.close(b).is_none());
}

#[test]
fn closing_a_tab_left_of_the_active_one_keeps_the_active_tab() {
    let mut t = Tabs::default();
    let a = t.open("https://a");
    let b = t.open("https://b");
    let c = t.open("https://c");
    t.activate(b);
    assert_eq!(t.close(a).map(|c| c.was_active), Some(false));
    assert_eq!(t.active().map(|t| t.id), Some(b));
    assert_eq!(t.close(c).map(|c| c.was_active), Some(false));
    assert_eq!(t.active().map(|t| t.id), Some(b));
}

#[test]
fn a_browser_that_closes_itself_takes_its_tab_and_keeps_its_address() {
    let mut t = Tabs::default();
    let a = t.open("https://a");
    let b = t.open("https://b");
    t.tabs[0].browser = 7;
    t.tabs[1].browser = 9;
    t.by_browser_mut(9).unwrap().page.committed("https://b/".into());
    // CEF says browser 9 is gone (window.close): tab b goes with it.
    let id = t.index_of_browser(9).map(|i| t.tabs[i].id);
    assert_eq!(id, Some(b));
    let closed = t.close(b).unwrap();
    assert_eq!((closed.browser, closed.was_active, closed.url.as_str()), (9, true, "https://b/"));
    assert_eq!(t.active().map(|t| t.id), Some(a));
    // Nothing owns 9 any more, and 0 never names a browser.
    assert_eq!(t.index_of_browser(9), None);
    assert_eq!(t.index_of_browser(0), None);
}

#[test]
fn the_last_tab_to_close_reports_its_address_for_reopen() {
    let mut t = Tabs::default();
    let a = t.open("https://a");
    t.tabs[0].page.committed("https://a/page".into());
    let closed = t.close(a).unwrap();
    assert_eq!(closed.url, "https://a/page");
    assert_eq!(t.tabs.len(), 0);
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

/// Through the statics, the way the strip and the CEF callbacks use
/// them, with no CEF in the process (`has_cef()` is false in a test):
/// open, close, detach and close_all in one sequence, since the other
/// tests drive a private `Tabs`. The lock only keeps two copies of THIS
/// test apart; the others never touch the statics.
#[test]
fn the_public_api_orders_open_close_detach_and_close_all() {
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
    let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    close_all();
    assert_eq!(count(), 0);
    assert_eq!(active_tab(), None);
    let a = tab_open("example.com");
    let b = tab_open("");
    assert_eq!(count(), 2);
    assert_eq!(active_tab(), Some(b));
    assert_eq!(page().shown_address(), "about:blank");
    assert_eq!(tabs()[0].url, "https://example.com");
    tab_activate(a);
    assert_eq!(active_tab(), Some(a));
    // A CEF callback for a browser no tab owns changes nothing.
    update_by_browser(0, |p| p.title = "never".into());
    update_by_browser(42, |p| p.title = "never".into());
    assert!(tabs().iter().all(|t| t.title.is_empty()));
    // Give tab b a browser id, then let "CEF" close it: detach takes
    // the tab, and a second detach for the same id is a no-op.
    with(|t| t.tabs[1].browser = 42);
    update_by_browser(42, |p| p.title = "B".into());
    assert_eq!(tabs()[1].title, "B");
    detach(42);
    assert_eq!(count(), 1);
    assert_eq!(active_tab(), Some(a));
    detach(42);
    detach(0);
    assert_eq!(count(), 1);
    // Closing the last tab keeps its address for reopen.
    update_active(|p| p.committed("https://example.com/last".into()));
    tab_close(a);
    assert_eq!(count(), 0);
    assert_eq!(active_tab(), None);
    assert_eq!(last_url(), "https://example.com/last");
    // close_all on two tabs empties the strip and remembers the active one.
    let c = tab_open("https://c.example");
    let _d = tab_open("https://d.example");
    tab_activate(c);
    update_active(|p| p.committed("https://c.example/".into()));
    close_all();
    assert_eq!(count(), 0);
    assert_eq!(last_url(), "https://c.example/");
    // tab_close of an unknown id is a no-op, not a panic.
    tab_close(9999);
}

#[test]
fn creates_that_land_out_of_order_bind_to_their_own_tabs() {
    let mut t = Tabs::default();
    let a = t.open("https://a");
    let b = t.open("https://b");
    let c = t.open("https://c");
    // CEF answers c first, then a, then b.
    assert!(t.attach(c, 30));
    assert!(t.attach(a, 10));
    assert!(t.attach(b, 20));
    assert_eq!(t.tabs.iter().map(|t| (t.id, t.browser)).collect::<Vec<_>>(), vec![(a, 10), (b, 20), (c, 30)]);
    assert_eq!(t.index_of_browser(20), Some(1));
}

#[test]
fn a_create_for_a_tab_closed_meanwhile_is_refused() {
    let mut t = Tabs::default();
    let a = t.open("https://a");
    let b = t.open("https://b");
    assert!(t.close(b).is_some());
    // b's browser lands after b closed: nobody owns it.
    assert!(!t.attach(b, 20));
    assert!(t.attach(a, 10));
    assert_eq!(t.index_of_browser(20), None);
    assert_eq!(t.tabs.len(), 1);
}

#[test]
fn a_new_tab_never_opens_what_the_bar_refuses() {
    assert_eq!(tab_url(""), "about:blank");
    assert_eq!(tab_url("file:///etc/passwd"), "about:blank");
    assert_eq!(tab_url("example.com"), "https://example.com");
}
