//! Whether the Agents rail entry has anything to show, and to say.
//!
//! Decision 15 puts the agent tree in the sidebar, above the chat list, so a
//! spawned agent is visible where its spawner is. That left the rail's own
//! Agents entry with no second surface to open, and it was wired to match:
//! it passed `on: false` so it could never light, and its click handler only
//! built the rail and asked for a redraw. Clicking it did nothing a user
//! could see - the page did not change and the row never took the active
//! colour (E2E-NAV-01). Confirmed on 2026-09-07: the frame before the click
//! and the frame after differ only by the hover wash under the resting
//! pointer.
//!
//! The entry now collapses and expands that tree, which is the one thing in
//! the sidebar it actually owns. The tree stays where decision 15 put it.
//!
//! The second condition below is not an afterthought. `agents_rail` returns
//! None until an engine exists, so the section can be absent for reasons
//! that have nothing to do with the user. Lighting the entry then would
//! promise a section nobody can see - the same trap `needs_you_page_up`
//! documents for the Needs you page.

/// Is the agent tree on screen? This is both what the section renders on and
/// what the rail entry lights on, so the row can never disagree with what is
/// below it.
pub(super) fn agents_tree_shown(rail_built: bool, collapsed: bool) -> bool {
    rail_built && !collapsed
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
        // The click's whole visible effect: the section goes, the plate goes
        // with it. E2E-NAV-01 was that neither ever happened.
        assert!(!agents_tree_shown(true, true));
    }

    #[test]
    fn an_engine_less_window_never_lights_the_entry() {
        // No rail to show, so nothing to promise - whatever the user last
        // asked for.
        assert!(!agents_tree_shown(false, false));
        assert!(!agents_tree_shown(false, true));
    }
}
