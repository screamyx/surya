//! Which folder the add-space palette's confirm chord adds.
//!
//! The chord used to always add the folder open in the breadcrumbs, and the
//! reason was sound as far as it went: the highlight auto-rests on the first
//! row, so a chord that took the highlight would add an arbitrary subfolder
//! of wherever you happened to be standing, and the usual target - a repo
//! root full of subfolders - is the folder you are standing in.
//!
//! That reasoning holds for a resting highlight. It does not hold for a
//! chosen one. Type a filter and the highlight stops being a default: it is
//! the row you narrowed the list down to, drawn selected, with its name
//! completed into the search box. Confirming there added the PARENT instead,
//! with no dialog and nothing on screen to say what had happened except the
//! sidebar quietly gaining the wrong project (E2E-PROJ-02). Reproduced on
//! 2026-09-07: the list filtered to one row, that row highlighted, and the
//! chord added the breadcrumb folder.
//!
//! So the query is what separates the two cases, and it is the only thing
//! that has to.

use surya_proto::FolderEntry;

/// The folder a confirm chord acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfirmTarget {
    /// The folder open in the breadcrumbs - the one you are standing in.
    Browsed,
    /// The highlighted row of the filtered list - the one you picked.
    Highlighted,
}

/// Pick the target. An empty query means the highlight is only resting, so
/// the browsed folder wins; a query means the user narrowed the list and the
/// highlight is a choice.
///
/// `has_highlighted_row` is false when the filter matched nothing. The
/// browsed folder is the fallback there, which is what the chord did before
/// this change: there is no picked row to prefer, and doing nothing at all
/// would leave the chord silently dead.
pub(super) fn confirm_target(query: &str, has_highlighted_row: bool) -> ConfirmTarget {
    if query.trim().is_empty() || !has_highlighted_row {
        ConfirmTarget::Browsed
    } else {
        ConfirmTarget::Highlighted
    }
}

/// Resolve the chord to a concrete folder: its path, and whether it is
/// already a git repo.
///
/// `browsed` is the listing's own path and repo seed, `highlighted` the
/// filtered list's current row.
pub(super) fn confirm_path(
    query: &str,
    browsed: (&str, bool),
    highlighted: Option<&FolderEntry>,
) -> (String, bool) {
    match confirm_target(query, highlighted.is_some()) {
        ConfirmTarget::Highlighted => {
            let entry = highlighted.expect("Highlighted is only returned for Some");
            (
                crate::pickers::child_path(browsed.0, &entry.name),
                entry.is_repo,
            )
        }
        ConfirmTarget::Browsed => (browsed.0.to_string(), browsed.1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filtered_and_highlighted_row_is_what_gets_added() {
        // E2E-PROJ-02. This is the case that added the parent.
        assert_eq!(
            confirm_target("check", true),
            ConfirmTarget::Highlighted
        );
    }

    #[test]
    fn a_resting_highlight_does_not_beat_the_folder_you_stand_in() {
        // No query, so the highlight is just the first row sitting there.
        // Taking it would add an arbitrary subfolder.
        assert_eq!(confirm_target("", true), ConfirmTarget::Browsed);
        // Whitespace is not a filter either.
        assert_eq!(confirm_target("   ", true), ConfirmTarget::Browsed);
    }

    fn entry(name: &str, is_repo: bool) -> FolderEntry {
        FolderEntry {
            name: name.into(),
            is_dir: true,
            is_repo,
        }
    }

    #[test]
    fn the_resolved_path_is_the_highlighted_row_under_the_browsed_folder() {
        // The exact shape of E2E-PROJ-02: standing in one folder, filtered
        // to a child, confirming. The child is what must be added.
        assert_eq!(
            confirm_path("che", ("/projects", false), Some(&entry("checkout", true))),
            ("/projects/checkout".to_string(), true),
            "the filtered row is the project, not its parent"
        );
    }

    #[test]
    fn an_empty_query_resolves_to_the_folder_you_stand_in() {
        assert_eq!(
            confirm_path("", ("/projects", true), Some(&entry("checkout", false))),
            ("/projects".to_string(), true),
            "no filter, so the resting highlight is ignored and the repo seed is the browsed one"
        );
    }

    #[test]
    fn a_query_that_matches_nothing_falls_back_to_the_browsed_folder() {
        // Unchanged behaviour: there is no picked row to prefer, and a chord
        // that did nothing would look broken.
        assert_eq!(confirm_target("zzz", false), ConfirmTarget::Browsed);
        assert_eq!(confirm_target("", false), ConfirmTarget::Browsed);
    }
}
