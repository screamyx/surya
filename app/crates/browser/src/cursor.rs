//! The pointer the page asks for: a hand over a link, an I-beam over text,
//! an arrow elsewhere, as in Chrome.
//!
//! Chromium tells the client the cursor type from its own thread
//! (`DisplayHandler::on_cursor_change`, client.rs) whenever the pointer
//! moves onto something that wants a different one. This module keeps the
//! last answer per browser, maps it onto gpui's [`CursorStyle`], and hands
//! the active tab's value to the surface, which applies it to the page
//! hitbox alone (surface.rs). gpui applies a hitbox cursor only while that
//! hitbox is hovered, so the pointer goes back to the shell's own the
//! moment it leaves the page area; and a tab switch reads the new browser's
//! value, arrow until its page has said otherwise.
//!
//! What has no gpui counterpart maps to the nearest shape, or the arrow:
//! custom bitmap cursors, wait, help, zoom, and "none" (gpui cannot hide
//! the pointer). gpui's Windows platform draws the arrow for the hand-grab
//! and drag-copy shapes too; that is gpui-surya's table, not this one.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use cef::sys::cef_cursor_type_t as Ct;
use cef::CursorType;
use gpui::CursorStyle;

/// The last cursor each browser asked for, by CEF identifier.
static CURSORS: Mutex<Option<HashMap<i32, CursorStyle>>> = Mutex::new(None);
/// Bumped on every change, so the idle pump knows to repaint (pump.rs).
static SEQ: AtomicU64 = AtomicU64::new(0);

/// gpui's shape for a CEF cursor type. `custom` is a bitmap cursor the
/// page supplied; gpui has no way to show one, so it gets the arrow.
pub(crate) fn map(t: CursorType, custom: bool) -> CursorStyle {
    if custom {
        return CursorStyle::Arrow;
    }
    match *t.as_ref() {
        Ct::CT_HAND => CursorStyle::PointingHand,
        Ct::CT_IBEAM => CursorStyle::IBeam,
        Ct::CT_VERTICALTEXT => CursorStyle::IBeamCursorForVerticalLayout,
        Ct::CT_CROSS | Ct::CT_CELL => CursorStyle::Crosshair,
        Ct::CT_EASTRESIZE => CursorStyle::ResizeRight,
        Ct::CT_WESTRESIZE => CursorStyle::ResizeLeft,
        Ct::CT_EASTWESTRESIZE => CursorStyle::ResizeLeftRight,
        Ct::CT_NORTHRESIZE => CursorStyle::ResizeUp,
        Ct::CT_SOUTHRESIZE => CursorStyle::ResizeDown,
        Ct::CT_NORTHSOUTHRESIZE => CursorStyle::ResizeUpDown,
        Ct::CT_NORTHWESTRESIZE | Ct::CT_SOUTHEASTRESIZE | Ct::CT_NORTHWESTSOUTHEASTRESIZE => {
            CursorStyle::ResizeUpLeftDownRight
        }
        Ct::CT_NORTHEASTRESIZE | Ct::CT_SOUTHWESTRESIZE | Ct::CT_NORTHEASTSOUTHWESTRESIZE => {
            CursorStyle::ResizeUpRightDownLeft
        }
        Ct::CT_COLUMNRESIZE => CursorStyle::ResizeColumn,
        Ct::CT_ROWRESIZE => CursorStyle::ResizeRow,
        Ct::CT_GRAB
        | Ct::CT_MIDDLEPANNING
        | Ct::CT_MIDDLE_PANNING_VERTICAL
        | Ct::CT_EASTPANNING
        | Ct::CT_NORTHPANNING
        | Ct::CT_NORTHEASTPANNING
        | Ct::CT_NORTHWESTPANNING
        | Ct::CT_SOUTHPANNING
        | Ct::CT_SOUTHEASTPANNING
        | Ct::CT_SOUTHWESTPANNING
        | Ct::CT_WESTPANNING => CursorStyle::OpenHand,
        Ct::CT_GRABBING | Ct::CT_MOVE | Ct::CT_DND_MOVE => CursorStyle::ClosedHand,
        Ct::CT_NOTALLOWED | Ct::CT_NODROP => CursorStyle::OperationNotAllowed,
        Ct::CT_ALIAS | Ct::CT_DND_LINK => CursorStyle::DragLink,
        Ct::CT_COPY | Ct::CT_DND_COPY => CursorStyle::DragCopy,
        Ct::CT_CONTEXTMENU => CursorStyle::ContextualMenu,
        // Pointer, wait, progress, help, zoom, none, custom, dnd-none, and
        // anything a newer CEF adds.
        _ => CursorStyle::Arrow,
    }
}

/// The pointer the page area should show: the active browser's last
/// request while the pointer is inside it, the arrow otherwise or until
/// that browser has asked for anything.
pub(crate) fn resolve(inside: bool, stored: Option<CursorStyle>) -> CursorStyle {
    if inside { stored.unwrap_or(CursorStyle::Arrow) } else { CursorStyle::Arrow }
}

/// `browser` asked for `t`. CEF's UI thread, from the display handler.
/// True when that is news: a page repeats its cursor on every move.
pub(crate) fn changed(browser: i32, t: CursorType, custom: bool) -> bool {
    if browser == 0 {
        return false;
    }
    let style = map(t, custom);
    let same = CURSORS
        .lock()
        .map(|mut g| g.get_or_insert_with(HashMap::new).insert(browser, style) == Some(style))
        .unwrap_or(true);
    if same {
        return false;
    }
    SEQ.fetch_add(1, Ordering::Release);
    // gpui re-reads the cursor only from a fresh paint; wake the pump so
    // the surface paints and asks for the new one.
    crate::pump::schedule_pump(0);
    true
}

/// The browser closed; its cursor goes with it.
pub(crate) fn forget(browser: i32) {
    if let Ok(mut g) = CURSORS.lock()
        && let Some(map) = g.as_mut()
    {
        map.remove(&browser);
    }
}

/// What `browser` last asked for, if anything.
pub(crate) fn of(browser: i32) -> Option<CursorStyle> {
    CURSORS.lock().ok()?.as_ref()?.get(&browser).copied()
}

/// The active tab's pointer, for the page hitbox. Arrow with no tab, and
/// for a tab whose page has not asked for anything yet.
pub fn active() -> CursorStyle {
    resolve(crate::events::inside(), of(crate::tabs::active_browser()))
}

/// Changes so far, for the idle pump's "anything new?" check.
pub(crate) fn seq() -> u64 {
    SEQ.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chrome_shapes_map_one_to_one() {
        assert_eq!(map(CursorType::POINTER, false), CursorStyle::Arrow);
        assert_eq!(map(CursorType::HAND, false), CursorStyle::PointingHand);
        assert_eq!(map(CursorType::IBEAM, false), CursorStyle::IBeam);
        assert_eq!(map(CursorType::CROSS, false), CursorStyle::Crosshair);
        assert_eq!(map(CursorType::WESTRESIZE, false), CursorStyle::ResizeLeft);
        assert_eq!(map(CursorType::NORTHSOUTHRESIZE, false), CursorStyle::ResizeUpDown);
        assert_eq!(map(CursorType::NORTHWESTSOUTHEASTRESIZE, false), CursorStyle::ResizeUpLeftDownRight);
        assert_eq!(map(CursorType::NORTHEASTRESIZE, false), CursorStyle::ResizeUpRightDownLeft);
        assert_eq!(map(CursorType::COLUMNRESIZE, false), CursorStyle::ResizeColumn);
        assert_eq!(map(CursorType::GRAB, false), CursorStyle::OpenHand);
        assert_eq!(map(CursorType::GRABBING, false), CursorStyle::ClosedHand);
        assert_eq!(map(CursorType::NOTALLOWED, false), CursorStyle::OperationNotAllowed);
        assert_eq!(map(CursorType::ALIAS, false), CursorStyle::DragLink);
        assert_eq!(map(CursorType::COPY, false), CursorStyle::DragCopy);
        assert_eq!(map(CursorType::CONTEXTMENU, false), CursorStyle::ContextualMenu);
    }

    #[test]
    fn what_gpui_cannot_draw_is_the_arrow() {
        assert_eq!(map(CursorType::WAIT, false), CursorStyle::Arrow);
        assert_eq!(map(CursorType::PROGRESS, false), CursorStyle::Arrow);
        assert_eq!(map(CursorType::HELP, false), CursorStyle::Arrow);
        assert_eq!(map(CursorType::ZOOMIN, false), CursorStyle::Arrow);
        assert_eq!(map(CursorType::NONE, false), CursorStyle::Arrow);
        assert_eq!(map(CursorType::CUSTOM, false), CursorStyle::Arrow);
        // A bitmap cursor is the arrow whatever type CEF sends with it.
        assert_eq!(map(CursorType::HAND, true), CursorStyle::Arrow);
    }

    #[test]
    fn each_browser_keeps_its_own_and_a_switch_reads_the_other() {
        forget(901);
        forget(902);
        changed(901, CursorType::HAND, false);
        assert_eq!(of(901), Some(CursorStyle::PointingHand));
        // The other tab's page has not asked for anything: arrow.
        assert_eq!(resolve(true, of(902)), CursorStyle::Arrow);
        changed(902, CursorType::IBEAM, false);
        assert_eq!(resolve(true, of(902)), CursorStyle::IBeam);
        assert_eq!(resolve(true, of(901)), CursorStyle::PointingHand);
        forget(901);
        assert_eq!(of(901), None);
        forget(902);
    }

    #[test]
    fn leaving_the_page_area_is_the_arrow() {
        assert_eq!(resolve(false, Some(CursorStyle::PointingHand)), CursorStyle::Arrow);
        assert_eq!(resolve(false, None), CursorStyle::Arrow);
        assert_eq!(resolve(true, Some(CursorStyle::PointingHand)), CursorStyle::PointingHand);
    }

    #[test]
    fn a_repeat_does_not_count_as_a_change() {
        forget(903);
        assert!(changed(903, CursorType::IBEAM, false));
        assert!(!changed(903, CursorType::IBEAM, false));
        assert!(changed(903, CursorType::HAND, false));
        // No browser, nothing to remember.
        assert!(!changed(0, CursorType::HAND, false));
        forget(903);
    }
}
