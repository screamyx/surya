//! The strip that NAMES every board column, above the scroller
//! (E2E-TASK-03).
//!
//! Four columns need about 960px. The right pane opens at 520, so the row
//! scrolls sideways and only Queued and Running fit. Done is cut to "Don" at
//! the pane edge and Blocked is not on screen at all - a user who never
//! presses the expand control does not know the column exists.
//!
//! Scrolling was never the missing piece; it was already there. What was
//! missing is any statement of what you are scrolling towards. This strip is
//! that statement: every column, named, with its count, at every pane width.
//! It reads left to right in the same order the columns do, so it doubles as
//! a map of the board.

use gpui::prelude::*;
use gpui::{div, px, SharedString};
use surya_proto::TaskStatus;

use super::board::{COLUMN_GAP, COLUMN_MIN_W};
use super::model::{COLUMNS, column_label};
use crate::theme::Theme;
use crate::typography::ui_rems;

/// One entry in the strip: the column's name, its count, and whether the
/// board is currently showing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StripEntry {
    pub label: &'static str,
    pub count: usize,
    /// The column is (at least partly) on screen right now. Off-screen
    /// columns are drawn quieter, so the strip says where you are as well as
    /// what exists.
    pub visible: bool,
}

/// Which columns the scroller is showing, given the geometry.
///
/// Pure, so the "is Blocked reachable" question can be answered in a test
/// rather than by eye on a screenshot.
pub fn entries(
    counts: impl Fn(TaskStatus) -> usize,
    scrolled: f32,
    viewport_w: f32,
    column_w: f32,
    gap: f32,
) -> Vec<StripEntry> {
    COLUMNS
        .iter()
        .enumerate()
        .map(|(ix, status)| {
            let left = ix as f32 * (column_w + gap);
            let right = left + column_w;
            StripEntry {
                label: column_label(*status),
                count: counts(*status),
                visible: right > scrolled && left < scrolled + viewport_w,
            }
        })
        .collect()
}

/// Draw the strip.
pub fn render(entries: &[StripEntry], theme: &Theme) -> gpui::Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .px(px(16.0))
        .pb(px(8.0))
        .children(entries.iter().enumerate().map(|(ix, entry)| {
            let dim = !entry.visible;
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .when(ix > 0, |el| {
                    el.child(
                        div()
                            .pr(px(5.0))
                            .text_size(ui_rems(12.0))
                            .text_color(theme.text_faint)
                            .child(SharedString::from("\u{00b7}")),
                    )
                })
                .child(
                    div()
                        .text_size(ui_rems(12.5))
                        .text_color(if dim { theme.text_faint } else { theme.text_muted })
                        .child(SharedString::from(entry.label)),
                )
                .child(
                    div()
                        .text_size(ui_rems(12.5))
                        .text_color(if dim { theme.text_faint } else { theme.text })
                        .child(SharedString::from(entry.count.to_string())),
                )
        }))
}

/// How far the column row can scroll, WITHOUT waiting for a measurement.
///
/// `max_offset` is zero until the scroll handle has measured, and nothing
/// re-runs the render on measure - so on the first paint the right-hand fade
/// was never drawn. The board opened with "Don" cut hard at the pane edge and
/// no hint that anything lay beyond it, and the fade only appeared after you
/// had already scrolled, which is after you already knew (E2E-TASK-03).
///
/// The content width needs no measuring: it is four columns at
/// [`COLUMN_MIN_W`] with a [`COLUMN_GAP`] between each. `measured` still wins
/// when it is larger, so once the real geometry is known it governs.
pub fn max_scroll(measured: f32, viewport: f32) -> f32 {
    let content =
        COLUMNS.len() as f32 * COLUMN_MIN_W + (COLUMNS.len() - 1) as f32 * COLUMN_GAP;
    measured.max((content - viewport).max(0.0))
}

/// Four columns at [`COLUMN_MIN_W`] each need ~960px; the right pane opens at
/// 520. Below that the row is the scroller (the right-surface strip's proven
/// shape: id + overflow_x_scroll + track_scroll) and a painted fade on
/// whichever edge hides a column says the board continues (critique round 3,
/// N3: "the third column is cut at 'Don' with no scroll hint").
pub fn render_scroller(
scroll: &gpui::ScrollHandle,
columns: Vec<gpui::Stateful<gpui::Div>>,
theme: &Theme,
) -> gpui::AnyElement {
    const FADE_WIDTH: f32 = 36.0;
    let scrolled = -f32::from(scroll.offset().x);
    // `max_offset` is ZERO until the scroll handle has measured, and
    // nothing re-runs this function on measure - so on the first paint
    // fade_right was false and no fade appeared at all. The board opened
    // with "Don" cut hard at the pane edge and no hint that anything lay
    // beyond it, and the fade only ever showed up after you had already
    // scrolled, which is after you already knew (E2E-TASK-03).
    //
    // The content width is known WITHOUT measuring: four columns at
    // COLUMN_MIN_W with a gap between each. Compare that against the
    // measured viewport and fall back to max_offset once it exists.
    let max_scroll = max_scroll(
        f32::from(scroll.max_offset().x),
        f32::from(scroll.bounds().size.width),
    );
    let fade_left = scrolled > 1.0;
    let fade_right = scrolled < max_scroll - 1.0;
    let page_bg = theme.bg;
    let fade = |angle: f32| {
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .w(px(FADE_WIDTH))
            .bg(gpui::linear_gradient(
                angle,
                gpui::linear_color_stop(page_bg, 0.0),
                gpui::linear_color_stop(page_bg.opacity(0.0), 1.0),
            ))
    };
    div()
        .relative()
        .flex_1()
        .min_w(px(0.0))
        .min_h(px(0.0))
        .flex()
        .child(
            div()
                .id("task-columns")
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_row()
                .gap(px(COLUMN_GAP))
                .px(px(16.0))
                .pb(px(16.0))
                .overflow_x_scroll()
                .track_scroll(&scroll)
                .children(columns),
        )
        .when(fade_left, |el| el.child(fade(90.0).left_0()))
        .when(fade_right, |el| el.child(fade(270.0).right_0()))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: TaskStatus) -> usize {
        0
    }

    /// The finding: at the default 520px pane, Blocked is never on screen.
    /// The strip has to name it anyway - that is the whole point of it.
    #[test]
    fn every_column_is_named_even_when_it_is_off_screen() {
        // 220px columns, 16px gaps, the docked pane's usable width.
        let strip = entries(none, 0.0, 488.0, 220.0, 16.0);
        assert_eq!(strip.len(), 4, "all four columns, always");
        let names: Vec<&str> = strip.iter().map(|e| e.label).collect();
        assert_eq!(names, ["Queued", "Running", "Done", "Blocked"]);
        assert!(
            strip.iter().any(|e| e.label == "Blocked" && !e.visible),
            "Blocked is off screen at this width - the strip is how you learn it exists"
        );
    }

    #[test]
    fn the_strip_says_which_columns_are_actually_showing() {
        let strip = entries(none, 0.0, 488.0, 220.0, 16.0);
        let visible: Vec<&str> = strip
            .iter()
            .filter(|e| e.visible)
            .map(|e| e.label)
            .collect();
        // Queued at 0..220, Running at 236..456 both fit; Done starts at 472,
        // so only a sliver of it shows - the "Don" in the frame.
        assert_eq!(visible, ["Queued", "Running", "Done"]);
    }

    #[test]
    fn scrolling_to_the_end_brings_blocked_into_view() {
        // Full content is 4*220 + 3*16 = 928; scrolled to the end of 488.
        let strip = entries(none, 928.0 - 488.0, 488.0, 220.0, 16.0);
        assert!(
            strip.iter().find(|e| e.label == "Blocked").unwrap().visible,
            "the column is reachable, and the strip should say so once you are there"
        );
    }

    #[test]
    fn a_wide_pane_shows_everything() {
        let strip = entries(none, 0.0, 1400.0, 220.0, 16.0);
        assert!(strip.iter().all(|e| e.visible));
    }

    #[test]
    fn counts_come_from_the_board_not_from_the_strip() {
        let strip = entries(
            |s| if s == TaskStatus::Queued { 3 } else { 0 },
            0.0,
            488.0,
            220.0,
            16.0,
        );
        assert_eq!(strip[0].count, 3);
        assert_eq!(strip[3].count, 0);
    }

    /// The fade half of the finding. On the FIRST paint `max_offset` is 0,
    /// so anything derived from it alone says "nothing to scroll to" and no
    /// fade is drawn - at the exact moment the user most needs telling.
    #[test]
    fn the_docked_pane_knows_it_can_scroll_before_it_has_measured() {
        // 520px pane less its 16px padding either side.
        let unmeasured = max_scroll(0.0, 488.0);
        assert!(
            unmeasured > 1.0,
            "with max_offset still 0 the board must already know it overflows, got {unmeasured}"
        );
    }

    #[test]
    fn a_pane_wide_enough_for_four_columns_has_nothing_to_scroll() {
        assert_eq!(max_scroll(0.0, 1400.0), 0.0);
    }

    #[test]
    fn a_real_measurement_wins_once_it_arrives() {
        assert_eq!(max_scroll(999.0, 488.0), 999.0);
    }
}
