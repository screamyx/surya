//! Which harnesses a model prefetch asks the device for.
//!
//! Asking for a harness's models makes the engine resolve that harness, and
//! resolving an ACP harness SPAWNS its CLI to read the model list off the
//! wire. So this list is not a cache-warming detail: it decides which command
//! line tools run on the user's machine, and when.
//!
//! The picker used to prefetch every offered harness as soon as the catalog
//! loaded and on render, before any popover opened. That started every
//! installed agent CLI merely because a window existed. It was noticed as a
//! Codex home appearing under an engine whose default harness was the mock
//! one: nothing had picked Codex, and the prefetch had started it anyway
//! (surya#195).
//!
//! Two scopes now. The chip needs one harness's models to name a concrete
//! pick, so that is all a load or a render asks for. Opening the picker is
//! the moment the other lists are about to be looked at, so that is where
//! they are fetched.

use surya_proto::HarnessId;

/// How much of the catalog a prefetch covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPrefetch {
    /// Only the harness whose models are about to be shown on the chip. The
    /// scope for a catalog load and for render.
    Effective,
    /// Every offered harness, so switching rails inside an open picker is
    /// instant. The scope for opening the picker, and only then.
    AllOffered,
}

/// The harnesses to request, in request order.
///
/// `effective` is included under both scopes, and it is included even when it
/// is not in `offered`: a chat keeps its harness after that harness has been
/// disabled, and its models still have to load or the chip cannot name the
/// pick the chat is actually using.
pub fn prefetch_targets(
    scope: ModelPrefetch,
    offered: &[HarnessId],
    effective: Option<HarnessId>,
) -> Vec<HarnessId> {
    let mut targets: Vec<HarnessId> = match scope {
        ModelPrefetch::Effective => Vec::new(),
        ModelPrefetch::AllOffered => offered.to_vec(),
    };
    if let Some(effective) = effective
        && !targets.contains(&effective)
    {
        targets.push(effective);
    }
    targets
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFFERED: [HarnessId; 3] = [HarnessId::ClaudeCode, HarnessId::Codex, HarnessId::Mock];

    #[test]
    fn a_load_asks_for_the_effective_harness_and_nothing_else() {
        // surya#195: this is the case that was starting every installed CLI.
        // Codex is offered and must NOT be requested here.
        assert_eq!(
            prefetch_targets(ModelPrefetch::Effective, &OFFERED, Some(HarnessId::Mock)),
            vec![HarnessId::Mock]
        );
    }

    #[test]
    fn opening_the_picker_asks_for_every_offered_harness() {
        // The other lists are about to be looked at, so this is where the
        // cost belongs.
        let targets = prefetch_targets(
            ModelPrefetch::AllOffered,
            &OFFERED,
            Some(HarnessId::ClaudeCode),
        );
        assert_eq!(targets, OFFERED.to_vec());
    }

    #[test]
    fn a_harness_outside_the_offered_set_is_still_asked_for() {
        // A chat keeps its harness after that harness is disabled. Without
        // this the chip cannot name the pick the chat is actually using.
        assert_eq!(
            prefetch_targets(ModelPrefetch::Effective, &[], Some(HarnessId::Codex)),
            vec![HarnessId::Codex]
        );
        assert_eq!(
            prefetch_targets(
                ModelPrefetch::AllOffered,
                &[HarnessId::Mock],
                Some(HarnessId::Codex)
            ),
            vec![HarnessId::Mock, HarnessId::Codex]
        );
    }

    #[test]
    fn nothing_is_asked_for_when_there_is_nothing_to_show() {
        // No catalog yet and no chat: a render must not reach the device.
        assert!(prefetch_targets(ModelPrefetch::Effective, &[], None).is_empty());
        assert!(prefetch_targets(ModelPrefetch::AllOffered, &[], None).is_empty());
    }

    #[test]
    fn the_effective_harness_is_not_requested_twice() {
        assert_eq!(
            prefetch_targets(ModelPrefetch::AllOffered, &OFFERED, Some(HarnessId::Codex)),
            OFFERED.to_vec()
        );
    }
}
