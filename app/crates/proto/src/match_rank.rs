//! How a typed query ranks a candidate name.
//!
//! This lives in the shared crate because two sides now have to agree on it.
//! The pickers rank what is on screen; the engine ranks a directory before it
//! caps the listing it sends back. If those two rules drifted, the engine
//! would cut away exactly the rows the client was about to rank highest, and
//! the picker would show "No folders match" for a folder that exists - which
//! is the bug this was written for (E2E-PROJ-01).

/// Match rank of a label against a query: `0` prefix match, `1` substring,
/// `None` no match. Case-insensitive; an empty query matches everything at
/// rank 1 (input order preserved).
pub fn match_rank(query: &str, label: &str) -> Option<usize> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Some(1);
    }
    let label = label.to_lowercase();
    if label.starts_with(&query) {
        Some(0)
    } else if label.contains(&query) {
        Some(1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_beats_substring_and_a_miss_is_none() {
        assert_eq!(match_rank("ma", "main"), Some(0));
        assert_eq!(match_rank("ma", "the-main"), Some(1));
        assert_eq!(match_rank("zzz", "main"), None);
    }

    #[test]
    fn an_empty_query_keeps_everything_in_input_order() {
        assert_eq!(match_rank("", "anything"), Some(1));
        assert_eq!(match_rank("   ", "anything"), Some(1));
    }

    #[test]
    fn matching_ignores_case_on_both_sides() {
        assert_eq!(match_rank("MA", "main"), Some(0));
        assert_eq!(match_rank("ma", "MAIN"), Some(0));
    }
}
