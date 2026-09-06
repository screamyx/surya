//! Press-and-hold arming for the right-pane surface tab strip.
//!
//! gpui arms a drag on the mouse-down plus two pixels of travel
//! (`DRAG_THRESHOLD` in gpui `elements/div.rs`). On a real mouse an ordinary
//! click clears two pixels, so every click on a tab started a reorder. The
//! owner asked for the other rule: "clicking any tabs (browser, files, tasks)
//! moves me into it, not reorder it. to reorder it, hold-click the tab."
//!
//! So the strip does not hand gpui a drag listener at all until the button has
//! been held for [`TAB_HOLD`]. Travel never arms on its own - that is the
//! whole point, and `travel_alone_never_arms` pins it.
//!
//! The shell calls [`TabPress::arm`] from two mouse-move handlers, the strip's
//! and the shell root's, not from a timer. Two because the strip occludes: it
//! ends gpui's hit-test walk, so the root never sees a move made over the
//! strip, which is exactly where a hold-drag starts. A timer has to be trusted or re-checked against the clock, and a
//! re-check against a timer that fires a millisecond early arms nothing and
//! never retries - the reorder simply stops working, which is what the first
//! build of this fix did on Windows.

use std::time::{Duration, Instant};

/// How long the button must stay down before a reorder drag can start.
/// 250 ms sits between a deliberate hold and a slow click; Windows' own
/// double-click window is 500 ms, so a hold is never mistaken for one.
pub(crate) const TAB_HOLD: Duration = Duration::from_millis(250);

/// What a finished press on a surface tab meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PressOutcome {
    /// Released before a drag took over: activate that tab.
    Click,
    /// The hold elapsed and gpui took the press: the reorder owns it, and the
    /// drop (or the release outside the strip) decides the order.
    DragArmed,
    /// The press ended on something else, or on a tab that is no longer the
    /// one that went down. Nothing happens.
    Cancelled,
}

/// A live left-button press on one tab chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TabPress {
    /// Index of the chip the button went down on.
    tab: usize,
    down_at: Instant,
    /// The hold elapsed, so the chip now carries a gpui drag listener.
    armed: bool,
    /// gpui built the ghost: the drag, not the click, owns this press.
    dragging: bool,
}

impl TabPress {
    pub(crate) fn new(tab: usize, down_at: Instant) -> Self {
        Self {
            tab,
            down_at,
            armed: false,
            dragging: false,
        }
    }

    pub(crate) fn tab(&self) -> usize {
        self.tab
    }

    /// Does this chip carry a drag listener this frame?
    pub(crate) fn armed_for(&self, tab: usize) -> bool {
        self.armed && self.tab == tab
    }

    /// The hold timer fired. Re-checks the clock rather than trusting the
    /// timer, so a late or coalesced wake-up cannot arm a press that has
    /// already been replaced. Returns true when this call armed it.
    pub(crate) fn arm(&mut self, now: Instant) -> bool {
        if self.armed || now.duration_since(self.down_at) < TAB_HOLD {
            return false;
        }
        self.armed = true;
        true
    }

    /// gpui created the ghost for this press.
    pub(crate) fn note_dragging(&mut self) {
        self.dragging = true;
    }

    /// Did this press get as far as a live reorder drag?
    pub(crate) fn dragged(&self) -> bool {
        self.dragging
    }

    /// The button came up over `tab`.
    pub(crate) fn release(&self, tab: usize) -> PressOutcome {
        if self.dragging {
            PressOutcome::DragArmed
        } else if self.tab == tab {
            PressOutcome::Click
        } else {
            PressOutcome::Cancelled
        }
    }
}

/// The strip's own source, checked below. gpui hands the app no way to read a
/// window-control hit test back, so the guard against the Windows regression
/// is structural: the strip element must keep `occlude()`, and nothing inside
/// it may go back to `block_mouse_except_scroll`.
#[cfg(test)]
fn strip_source() -> &'static str {
    const SHELL: &str = include_str!("../shell.rs");
    const START: &str = "fn render_right_tab_strip";
    let from = SHELL.find(START).expect("render_right_tab_strip moved");
    let rest = &SHELL[from + START.len()..];
    // The body runs to the next method at the same indent.
    let to = rest.find("\n    fn ").unwrap_or(rest.len());
    &rest[..to]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quick_release_is_a_click() {
        let press = TabPress::new(1, Instant::now());
        assert_eq!(press.release(1), PressOutcome::Click);
    }

    #[test]
    fn a_release_short_of_the_hold_is_still_a_click() {
        let start = Instant::now();
        let mut press = TabPress::new(1, start);
        assert!(!press.arm(start + TAB_HOLD - Duration::from_millis(1)));
        assert!(!press.armed_for(1));
        assert_eq!(press.release(1), PressOutcome::Click);
    }

    #[test]
    fn the_hold_arms_the_drag() {
        let start = Instant::now();
        let mut press = TabPress::new(2, start);
        assert!(press.arm(start + TAB_HOLD));
        assert!(press.armed_for(2));
        // Only the chip that went down carries the listener.
        assert!(!press.armed_for(0));
    }

    #[test]
    fn arming_twice_reports_the_change_once() {
        let start = Instant::now();
        let mut press = TabPress::new(0, start);
        assert!(press.arm(start + TAB_HOLD));
        assert!(!press.arm(start + TAB_HOLD * 2));
    }

    #[test]
    fn an_armed_press_that_never_dragged_still_clicks() {
        // Hold still and let go: nothing moved, so the tab activates. Chrome
        // and Zed do the same; only an actual drag suppresses the click.
        let start = Instant::now();
        let mut press = TabPress::new(1, start);
        press.arm(start + TAB_HOLD);
        assert_eq!(press.release(1), PressOutcome::Click);
    }

    #[test]
    fn a_press_that_dragged_is_not_a_click() {
        let start = Instant::now();
        let mut press = TabPress::new(1, start);
        press.arm(start + TAB_HOLD);
        press.note_dragging();
        assert_eq!(press.release(1), PressOutcome::DragArmed);
    }

    #[test]
    fn releasing_over_another_tab_cancels() {
        let press = TabPress::new(1, Instant::now());
        assert_eq!(press.release(2), PressOutcome::Cancelled);
    }

    #[test]
    fn travel_alone_never_arms() {
        // The bug this file exists for: gpui arms on two pixels of travel.
        // Nothing here takes a position, so no amount of movement can arm a
        // press before the clock says so.
        let start = Instant::now();
        let mut press = TabPress::new(1, start);
        for _ in 0..100 {
            assert!(!press.arm(start));
        }
        assert_eq!(press.release(1), PressOutcome::Click);
    }

    #[test]
    fn a_dragged_press_reports_itself_for_the_drop_clear() {
        // The shell drops the press in `on_drop` and, as a second net, in the
        // render heal once no drag is active; the heal keys off `dragged()`.
        // Nothing else would clear it: the source chip renders as a
        // placeholder for the whole drag, so its `on_mouse_up_out` is never
        // painted. A press surviving a drop stays armed, and the slot it names
        // gets an `on_drag` with no hold gate. This side of it is what a unit
        // test can reach; the two clears themselves are shell code.
        let start = Instant::now();
        let mut press = TabPress::new(1, start);
        press.arm(start + TAB_HOLD);
        press.note_dragging();
        assert!(press.dragged());
        assert_eq!(press.release(1), PressOutcome::DragArmed);
    }

    #[test]
    fn a_press_that_never_dragged_is_not_swept_by_the_heal() {
        // The heal runs on every frame with no active drag, which is most of
        // them. It must not eat a live press that simply has not moved yet.
        let start = Instant::now();
        let mut press = TabPress::new(1, start);
        press.arm(start + TAB_HOLD);
        assert!(!press.dragged());
    }

    #[test]
    fn the_first_move_past_the_hold_arms_it() {
        // How the shell drives this: `arm` on every move with the button down.
        // Moves before the hold do nothing, the first one after it arms, and
        // gpui starts the drag on the move after that.
        let start = Instant::now();
        let mut press = TabPress::new(0, start);
        for ms in [10, 60, 120, 200, 249] {
            assert!(!press.arm(start + Duration::from_millis(ms)), "armed at {ms}ms");
        }
        assert!(press.arm(start + Duration::from_millis(260)));
        assert!(press.armed_for(0));
    }

    #[test]
    fn the_tab_strip_stays_out_of_the_window_drag_region() {
        // shell/tabs.rs wraps the whole titlebar in `titlebar_drag_region`, a
        // `WindowControlArea::Drag`. On Windows gpui answers WM_NCHITTEST with
        // HTCAPTION whenever that hitbox id is still in the hit-test ids, and
        // the OS then swallows the press: the tab never activates and a few
        // pixels of travel move the window. `block_mouse_except_scroll` does
        // not remove it (gpui `hit_test` only breaks on `BlockMouse`);
        // `occlude()` does.
        let src = strip_source();
        assert!(
            src.contains(".occlude()"),
            "the surface-tab strip lost occlude(): every tab press goes back \
             to the Windows window-move"
        );
        assert!(
            !src.contains(".block_mouse_except_scroll()"),
            "block_mouse_except_scroll leaves the titlebar drag hitbox in the \
             hit-test ids - that is the bug, use occlude() on the strip"
        );
    }
}
