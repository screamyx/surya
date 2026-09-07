//! Which model a new chat opens on (owner ruling, 2026-09-07).
//!
//! Its own module because `pickers.rs` is nearly ten times the 500-line rule
//! (decision 13) and must not grow.

use surya_proto::Model;

/// The harness's default model: an Opus row when the catalog offers one,
/// otherwise the first row.
///
/// This was `models.first()`, on the reasoning that "both curated catalogs
/// lead with the flagship". The Claude catalog no longer does - it leads with
/// Fable 5.1 - so surya's `pickDefaultModel` Opus preference was quietly lost
/// and every new chat opened on Fable. Stating the preference here instead of
/// inferring it from row order means a catalog reorder cannot move the
/// default again without someone noticing (owner ruling, 2026-09-07).
///
/// The picker's LIST order is untouched: this changes which row is picked by
/// default, not how the catalog is displayed.
pub fn default_model(models: &[Model]) -> Option<&Model> {
    models.iter().find(|m| is_opus(m)).or_else(|| models.first())
}

/// Opus by model id, not by label. Ids are stable wire values; labels are
/// prose and get rewritten.
fn is_opus(model: &Model) -> bool {
    model.id.to_ascii_lowercase().contains("opus")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_prefers_opus_and_falls_back_to_the_first_row() {
        let models = vec![
            Model {
                id: "flagship".into(),
                label: "Flagship".into(),
                description: None,
                reasoning_levels: vec![],
                options: vec![],
            },
            Model {
                id: "fast".into(),
                label: "Fast".into(),
                description: None,
                reasoning_levels: vec![],
                options: vec![],
            },
        ];
        // No Opus in this catalog, so the first row still wins.
        assert_eq!(default_model(&models).map(|m| &*m.id), Some("flagship"));
        assert!(default_model(&[]).is_none());
    }

    /// The regression this exists for: the SHIPPED Claude catalog leads with
    /// Fable 5.1, so `models.first()` opened every new chat on Fable. The
    /// owner's ruling is that the app defaults to Opus.
    ///
    /// Asserted against the real catalog, not a fixture, because a fixture
    /// would have passed happily while the app did the wrong thing.
    #[test]
    fn the_shipped_claude_catalog_defaults_to_opus() {
        let models = surya_harness::claude::catalog::static_models();
        let picked = default_model(&models).expect("the catalog is not empty");
        assert!(
            picked.id.contains("opus"),
            "the app must open on Opus, got {:?} ({})",
            picked.id,
            picked.label
        );
        // And it is the CURRENT Opus, not an older one: the preference takes
        // the first Opus row, and the catalog lists newest first.
        assert_eq!(picked.id, "claude-opus-5");
    }

    #[test]
    fn an_opus_row_wins_wherever_it_sits_in_the_catalog() {
        let models = vec![
            Model {
                id: "claude-fable-5-1".into(),
                label: "Fable 5.1".into(),
                description: None,
                reasoning_levels: vec![],
                options: vec![],
            },
            Model {
                id: "claude-opus-5".into(),
                label: "Opus 5".into(),
                description: None,
                reasoning_levels: vec![],
                options: vec![],
            },
            Model {
                id: "claude-opus-4-8".into(),
                label: "Opus 4.8".into(),
                description: None,
                reasoning_levels: vec![],
                options: vec![],
            },
        ];
        assert_eq!(
            default_model(&models).map(|m| &*m.id),
            Some("claude-opus-5"),
            "row order must not decide this any more, and the newest Opus wins"
        );
    }
}
