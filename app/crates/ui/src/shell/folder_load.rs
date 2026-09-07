//! Why the folder picker is asking the device for a listing.
//!
//! Two callers with different needs. Moving to another folder should say so:
//! clear the list and show it loading. Re-asking for the SAME folder because
//! the filter changed should not, because a spinner on every character typed
//! is worse than one stale frame.
//!
//! The filter has to reach the device at all because the listing is capped:
//! the engine sends at most 500 entries, so on a large directory the client
//! was filtering a page it had already lost the answer from, and a folder
//! that existed came back as "No folders match" (E2E-PROJ-01).
//!
//! Be precise about what "one stale frame" means, because it is not always a
//! harmless one. While a refilter is in flight the pane keeps filtering the
//! PREVIOUS listing, and for the case this exists to fix - a folder past the
//! cap - that page holds no match. So the pane can read "No folders match"
//! for one debounce before the engine's answer lands and the row appears.
//! That is briefly the same wrong answer the bug gave permanently.
//!
//! It is still the right trade: 150 ms of a stale answer beats a list that
//! blinks empty on every keystroke, and the alternative - blanking on each
//! character - shows nothing at all for just as long. Worth knowing before
//! anyone reads that flash as a regression.

use std::time::Duration;

/// Wait before a filter reaches the device. Long enough that typing a folder
/// name is one call rather than ten, short enough not to feel held up.
const REFILTER_DEBOUNCE: Duration = Duration::from_millis(150);

/// What prompted a folder listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FolderLoad {
    /// The user moved: a rail click, a descent, a breadcrumb, going up.
    Browse,
    /// The user typed: same folder, new filter.
    Refilter,
}

impl FolderLoad {
    /// How long to wait before calling the device.
    ///
    /// A browse is a deliberate act and answers at once. A refilter waits,
    /// and because each keystroke replaces the in-flight task - dropping a
    /// gpui `Task` cancels it - only the last keystroke of a burst is ever
    /// sent.
    pub(super) fn debounce(self) -> Option<Duration> {
        match self {
            Self::Browse => None,
            Self::Refilter => Some(REFILTER_DEBOUNCE),
        }
    }

    /// Does this load blank the list while it waits?
    ///
    /// Only about whether there is a list to look at. Where the list is
    /// LOOKING is a separate question, and both kinds answer it the same way:
    /// see the scroll reset in `load_space_folders_for`.
    pub(super) fn clears_the_list(self) -> bool {
        matches!(self, Self::Browse)
    }

    /// Should this load ask for the search input back?
    ///
    /// A browse follows a click on a rail row, a breadcrumb or a folder row,
    /// all of which sit outside the input and take focus with them, so it
    /// has to (E2E-PROJ-03). A refilter is caused by typing INTO the input,
    /// which therefore already has focus - asking again mid-keystroke could
    /// only disturb a caret that is already where the user put it.
    pub(super) fn wants_the_search_box(self) -> bool {
        matches!(self, Self::Browse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_answers_at_once_and_says_it_is_loading() {
        assert_eq!(FolderLoad::Browse.debounce(), None);
        assert!(FolderLoad::Browse.clears_the_list());
    }

    #[test]
    fn only_a_browse_blanks_the_list() {
        // Blanking is the ONLY thing this decides. The scroll used to ride
        // along with it, which left a refilter's highlight at the top of the
        // list and the viewport somewhere else.
        assert!(FolderLoad::Browse.clears_the_list());
        assert!(!FolderLoad::Refilter.clears_the_list());
    }

    #[test]
    fn a_browse_takes_the_search_box_back_and_typing_does_not() {
        // The first half is E2E-PROJ-03's rule and must not weaken: every
        // path that click-navigates asks for the input back.
        assert!(FolderLoad::Browse.wants_the_search_box());
        // The second half is new with the device-side filter. A refilter
        // only happens because the user typed into the box, so it already
        // holds focus and re-asking mid-keystroke risks the caret.
        assert!(!FolderLoad::Refilter.wants_the_search_box());
    }

    #[test]
    fn typing_waits_and_leaves_the_rows_alone() {
        // The two halves of "do not flash on every character": wait before
        // asking, and keep showing what is already there while you do.
        assert_eq!(
            FolderLoad::Refilter.debounce(),
            Some(Duration::from_millis(150))
        );
        assert!(!FolderLoad::Refilter.clears_the_list());
    }
}
