//! The rail's Agents entry, and the section it names.
//!
//! Decision 15 puts the agent tree in the sidebar, above the chat list, so a
//! spawned agent is visible where its spawner is. That left the rail's own
//! Agents entry with no second surface to open, and it was wired to match:
//! it passed `on: false` so it could never light, and its click handler only
//! built the rail and asked for a redraw. Clicking it did nothing a user
//! could see (E2E-NAV-01). Confirmed on 2026-09-07: the frame before the
//! click and the frame after differ only by the hover wash under the resting
//! pointer.
//!
//! The entry now REVEALS that section. It does not toggle it, and the
//! difference is the whole of this module's reason to exist: the tree starts
//! expanded, so a toggle would make the first click on a row called Agents
//! HIDE the agents and un-light the row, while every sibling entry opens the
//! thing it names. Collapsing lives on the section's own header, which is
//! the affordance the sidebar's other collapsible sections already use.
//!
//! The second condition in [`agents_tree_shown`] is not an afterthought.
//! `agents_rail` returns None until an engine exists, so the section can be
//! absent for reasons that have nothing to do with the user. Labelling the
//! entry Shown then would promise a section nobody can see, the same trap
//! `needs_you_page_up` documents for the Needs you page.

use gpui::{AnyElement, Context, SharedString, div, prelude::*, px};

use super::{RightSurface, Shell};
use crate::theme::Theme;

/// Motion/disclosure key for the section, shared by the header and chevron.
const KEY: &str = "agents-tree";

/// Is the agent tree on screen? This is both what the section renders on and
/// what the rail entry's Shown label reads, so the row can never disagree
/// with what is below it. It is NOT a selected fill: an auxiliary surface
/// being visible is not the same as it being where you are (issue #212).
pub(super) fn agents_tree_shown(rail_built: bool, collapsed: bool) -> bool {
    rail_built && !collapsed
}

/// Where a click on the rail's Agents entry leaves the section.
///
/// Always shown, whatever it was before. The input is deliberately ignored:
/// that IS the rule. A rail entry reveals what it names and never hides it,
/// so this can only ever return "not collapsed".
pub(super) fn collapsed_after_rail_click(_was_collapsed: bool) -> bool {
    false
}

/// Which rail rows hold the selected fill.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct RailSelected {
    pub home: bool,
    pub needs_you: bool,
    pub agents: bool,
    pub tasks: bool,
    pub files: bool,
}

/// The whole rail-lit decision, in one place a test can call.
///
/// `agents` is deliberately always false: the tree's visibility is an
/// auxiliary cue with its own "Shown" label, not a destination, so the row
/// must never take the selected fill that Home, Needs you, Tasks and Files
/// use for "you are here" (issue #212). `agents_shown` is taken and ignored
/// so the signature says out loud that visibility is NOT an input to this.
/// Do not delete the parameter as unused: `tree_visibility_moves_no_rail_fill_at_all`
/// compares the two `agents_shown` values, so removing it turns that test
/// into a value compared with itself. Delete the test with it, or neither.
///
/// This lives here rather than inline in `render_rail_entries` for one
/// reason: inline, the only thing standing between a user and two identically
/// filled rows is an argument at a call site, and no test can read a call
/// site.
pub(super) fn rail_selected(
    active: Option<RightSurface>,
    inbox_on: bool,
    agents_shown: bool,
) -> RailSelected {
    let _ = agents_shown;
    RailSelected {
        home: active.is_none() && !inbox_on,
        needs_you: inbox_on,
        agents: false,
        tasks: active == Some(RightSurface::Tasks),
        files: active == Some(RightSurface::Files),
    }
}

/// Auxiliary visibility has its own cue; it is not the current destination.
pub(super) struct EntryIndicator {
    pub selected: bool,
    pub status: Option<&'static str>,
}

pub(super) fn entry_indicator(shown: bool) -> EntryIndicator {
    EntryIndicator {
        selected: false,
        status: shown.then_some("Shown"),
    }
}

impl Shell {
    /// The Agents section: a collapsible header, and the tree under it while
    /// it is expanded. `None` when there is no rail to show at all.
    ///
    /// The header renders whether or not the tree does, because it is the
    /// only way back once the section is collapsed.
    pub(super) fn render_agents_section(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let rail = self.agents_rail(cx)?;
        let collapsed = self.agents_collapsed;
        let chevron = self.sidebar_disclosure_chevron(KEY, !collapsed, theme);
        let header =
            super::spaces::sidebar_disclosure_header(theme, SharedString::from("Agents"), chevron)
                .id("sidebar-agents-header")
                .on_click(cx.listener(|this, _, _, cx| {
                    // The header is where collapsing lives, so this one toggles.
                    this.agents_collapsed = !this.agents_collapsed;
                    cx.notify();
                }));
        Some(
            div()
                .flex_none()
                .border_b_1()
                .border_color(theme.border)
                .flex()
                .flex_col()
                .child(header)
                .when(!collapsed, |el| {
                    el.child(div().flex_none().max_h(px(220.0)).child(rail))
                })
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tree_is_shown_when_built_and_expanded() {
        assert!(agents_tree_shown(true, false));
    }

    #[test]
    fn collapsing_the_tree_takes_it_off_screen() {
        assert!(!agents_tree_shown(true, true));
    }

    #[test]
    fn an_engine_less_window_never_shows_the_tree() {
        // No rail to show, so nothing to promise, whatever the user last
        // asked for.
        assert!(!agents_tree_shown(false, false));
        assert!(!agents_tree_shown(false, true));
    }

    #[test]
    fn the_rail_entry_reveals_from_either_state_and_never_hides() {
        // The review point on the first version of this change: the tree
        // starts expanded, so a toggle here would make the very first click
        // on a row called Agents hide the agents. From both starting states
        // the section must end up shown.
        for was_collapsed in [true, false] {
            let after = collapsed_after_rail_click(was_collapsed);
            assert!(
                agents_tree_shown(true, after),
                "a rail click must reveal, not hide (was_collapsed={was_collapsed})"
            );
        }
    }
    #[test]
    fn only_visible_agents_get_the_shown_label() {
        // Input to output: this is the whole of `entry_indicator`'s job now
        // that the fill decision lives in `rail_selected`.
        assert_eq!(entry_indicator(true).status, Some("Shown"));
        assert_eq!(entry_indicator(false).status, None);
    }

    #[test]
    fn the_agents_row_never_takes_the_selected_fill() {
        // The bug in #212 was the Agents row wearing the same selected fill
        // as a destination. This is the guard: change agents: false to
        // agents: agents_shown in the body above and both this and
        // tree_visibility_moves_no_rail_fill_at_all fail. What it does NOT
        // cover is the call site - render_rail_entries still has
        // agents_shown in scope for the Shown label, so passing it to
        // entry("rail-agents", ..) instead of lit.agents compiles clean and
        // stays green. Shell cannot be constructed in a test, so no unit
        // test can read a call site; keeping every row on lit.* is the
        // convention that closes that gap, not this assertion.
        for active in [None, Some(RightSurface::Tasks), Some(RightSurface::Files)] {
            for inbox_on in [true, false] {
                for agents_shown in [true, false] {
                    let lit = rail_selected(active, inbox_on, agents_shown);
                    assert!(
                        !lit.agents,
                        "Agents took the fill (active={active:?} inbox_on={inbox_on} \
                         agents_shown={agents_shown})"
                    );
                }
            }
        }
    }

    #[test]
    fn tree_visibility_moves_no_rail_fill_at_all() {
        // The other half of #212: `agents_shown` is an auxiliary cue, so
        // flipping it must leave every one of the five fills where it was.
        // Compare whole structs, so a future row added to `RailSelected`
        // that reads `agents_shown` is caught here too.
        for active in [None, Some(RightSurface::Tasks), Some(RightSurface::Files)] {
            for inbox_on in [true, false] {
                assert_eq!(
                    rail_selected(active, inbox_on, true),
                    rail_selected(active, inbox_on, false),
                    "showing the tree moved a fill (active={active:?} inbox_on={inbox_on})"
                );
            }
        }
    }

    // Not asserted here: that at most one row is lit at a time. It is not
    // true yet. Needs you and Files both keep the fill when the inbox is on
    // and the file pane is open, which is #258. That belongs to the PR that
    // fixes it, not to a test that would have to be written around the bug.
}
