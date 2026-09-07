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
//! absent for reasons that have nothing to do with the user. Lighting the
//! entry then would promise a section nobody can see, the same trap
//! `needs_you_page_up` documents for the Needs you page.

use gpui::{AnyElement, Context, SharedString, div, prelude::*, px};

use super::Shell;
use crate::theme::Theme;

/// Motion/disclosure key for the section, shared by the header and chevron.
const KEY: &str = "agents-tree";

/// Is the agent tree on screen? This is both what the section renders on and
/// what the rail entry lights on, so the row can never disagree with what is
/// below it.
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
    fn the_entry_lights_while_the_tree_is_showing() {
        assert!(agents_tree_shown(true, false));
    }

    #[test]
    fn collapsing_the_tree_un_lights_the_entry() {
        assert!(!agents_tree_shown(true, true));
    }

    #[test]
    fn an_engine_less_window_never_lights_the_entry() {
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
    fn visible_agents_remain_distinct_from_home_or_files_selection() {
        for primary_selected in [true, false] {
            let indicator = entry_indicator(true);
            assert_eq!(indicator.status, Some("Shown"));
            assert!(!indicator.selected);
            assert!((primary_selected as usize + indicator.selected as usize) <= 1);
        }
        let hidden = entry_indicator(false);
        assert!(!hidden.selected);
        assert_eq!(hidden.status, None);
    }
}
