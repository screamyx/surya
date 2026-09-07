//! The lines on a permission card, and what each one commits the user to.
//!
//! Three of them existed before yolo mode: allow this once, write a rule that
//! allows EXACTLY THIS command forever in this project, deny. The fourth
//! answers this request and switches the CHAT into yolo mode, which is the
//! same thing the composer chip does and is revertible in one click.
//!
//! Its own module so the table and the key mapping can be read and tested
//! without going through the composer (`composer.rs` is far past the 500-line
//! rule already, decision 13).

use surya_proto::PermissionDecision;

/// What one line on the card does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionAnswer {
    /// This request, this once.
    Allow,
    /// This request, and an always-allow RULE for the project. The rule is
    /// pinned to THIS exact command: the engine stores it with `exact` set
    /// (`AllowRules::from_remember`), so the next command, or the next file,
    /// asks again. The label says so - it used to read "Always allow in this
    /// project", which promised a project-wide grant the rule never gave
    /// (E2E-PERM-02). Widening happens on the approval-policy page, in the
    /// open. Permanent, and it outlives the chat.
    AllowAlways,
    /// [`Self::AllowAlways`], but on every checkout on this device rather
    /// than only this project. Same rule, same one-command narrowness, wider
    /// scope - the choice the Needs you inbox has always offered and the
    /// card did not (PERM-03).
    AllowAlwaysEverywhere,
    /// This request, and the chat stops asking: yolo mode on. A mode, not a
    /// rule - nothing is written to the always-allow table and the chip turns
    /// it back off.
    AllowAndStopAsking,
    /// Not this one.
    Deny,
}

impl PermissionAnswer {
    pub fn label(self) -> &'static str {
        match self {
            PermissionAnswer::Allow => "Allow",
            PermissionAnswer::AllowAlways => "Always allow exactly this in this project",
            PermissionAnswer::AllowAlwaysEverywhere => "Always allow exactly this everywhere",
            PermissionAnswer::Deny => "Deny",
            PermissionAnswer::AllowAndStopAsking => "Allow, and stop asking in this chat",
        }
    }

    /// The decision sent to the engine, for the lines that send one.
    /// `AllowAndStopAsking` sends none: switching yolo on IS the answer, and
    /// the engine allows what the chat has parked as part of the flip. A
    /// `RespondPermission` behind it would arrive to find nothing pending and
    /// report a failure for an answer that worked.
    pub fn decision(self) -> Option<PermissionDecision> {
        match self {
            PermissionAnswer::Allow
            | PermissionAnswer::AllowAlways
            | PermissionAnswer::AllowAlwaysEverywhere => Some(PermissionDecision::Allow),
            PermissionAnswer::Deny => Some(PermissionDecision::Deny),
            PermissionAnswer::AllowAndStopAsking => None,
        }
    }

    /// Does this line write an always-allow rule?
    pub fn remembers(self) -> bool {
        self.rule_scope().is_some()
    }

    /// How far this line's rule reaches, for the lines that write one.
    ///
    /// The card and the Needs you inbox now offer the same two reaches and
    /// call them the same two things - "this project" and "everywhere". They
    /// used to disagree: the card said "project" and had no wider option at
    /// all, while the inbox said "workspace" and did (PERM-03).
    pub fn rule_scope(self) -> Option<surya_proto::RuleScope> {
        match self {
            PermissionAnswer::AllowAlways => Some(surya_proto::RuleScope::Workspace),
            PermissionAnswer::AllowAlwaysEverywhere => Some(surya_proto::RuleScope::Global),
            _ => None,
        }
    }

    /// Does this line switch the chat into yolo mode?
    pub fn stops_asking(self) -> bool {
        self == PermissionAnswer::AllowAndStopAsking
    }
}

/// The lines, in the order they are drawn and numbered.
///
/// The new line is LAST rather than grouped with the other allows on purpose.
/// 1, 2 and 3 are muscle memory, and a card is answered by typing a digit;
/// renumbering them so that an old 2 or 3 lands on "stop asking" would turn a
/// mis-key into "this chat runs tools without asking from now on", which is
/// the one outcome on this card nobody should reach by accident.
pub const ROWS: [PermissionAnswer; 5] = [
    PermissionAnswer::Allow,
    PermissionAnswer::AllowAlways,
    PermissionAnswer::Deny,
    PermissionAnswer::AllowAndStopAsking,
    PermissionAnswer::AllowAlwaysEverywhere,
];

/// The answer a bare digit picks, 1-based as the card numbers them.
pub fn for_digit(digit: usize) -> Option<PermissionAnswer> {
    ROWS.get(digit.checked_sub(1)?).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_old_three_keys_still_mean_what_they_meant() {
        assert_eq!(for_digit(1), Some(PermissionAnswer::Allow));
        assert_eq!(for_digit(2), Some(PermissionAnswer::AllowAlways));
        assert_eq!(for_digit(3), Some(PermissionAnswer::Deny));
        assert_eq!(for_digit(4), Some(PermissionAnswer::AllowAndStopAsking));
        assert_eq!(
            for_digit(5),
            Some(PermissionAnswer::AllowAlwaysEverywhere),
            "the wider reach is appended, so 1-4 keep their meaning"
        );
        assert_eq!(for_digit(6), None);
        assert_eq!(for_digit(0), None);
    }

    #[test]
    fn stop_asking_is_a_mode_not_a_rule_and_sends_no_decision() {
        let a = PermissionAnswer::AllowAndStopAsking;
        assert!(a.stops_asking());
        assert!(!a.remembers(), "yolo writes nothing to the rules table");
        assert_eq!(
            a.decision(),
            None,
            "the flip answers the parked request; a second answer would fail"
        );
    }

    /// E2E-PERM-02: the rule the engine writes for this line is pinned to
    /// the one command the card showed (`AllowRules::from_remember` stores it
    /// with `exact`), so the label must not promise the project. "Always
    /// allow in this project" did, and the next file prompted again.
    #[test]
    fn always_allow_says_it_covers_only_this_command() {
        let label = PermissionAnswer::AllowAlways.label();
        assert!(
            label.contains("exactly this"),
            "the line that writes a rule must say the rule is one command: {label:?}"
        );
        assert_ne!(
            label, "Always allow in this project",
            "the old label promised a project-wide grant the rule never gave"
        );
    }

    /// PERM-03: the card and the Needs you inbox describe the same two
    /// reaches, so they have to use the same two words. The card used to say
    /// "project" and offer no wider choice at all; the inbox said
    /// "workspace" and did.
    #[test]
    fn the_card_and_the_inbox_name_the_same_two_reaches() {
        use crate::inbox::model::AlwaysAllowScope;
        let here = PermissionAnswer::AllowAlways.label();
        let anywhere = PermissionAnswer::AllowAlwaysEverywhere.label();

        assert!(here.contains("this project"), "{here:?}");
        assert!(
            !here.contains("workspace"),
            "the user-facing word is project everywhere: {here:?}"
        );
        assert!(anywhere.contains("everywhere"), "{anywhere:?}");

        // The inbox chip renders "scope: <label>" for the same two reaches.
        assert_eq!(AlwaysAllowScope::ThisWorkspace.label(), "this project");
        assert_eq!(AlwaysAllowScope::Everywhere.label(), "everywhere");
        assert!(
            here.contains(AlwaysAllowScope::ThisWorkspace.label()),
            "card {here:?} and inbox {:?} must not drift apart",
            AlwaysAllowScope::ThisWorkspace.label()
        );
        assert!(anywhere.contains(AlwaysAllowScope::Everywhere.label()));
    }

    /// Both remembering lines pin ONE command; only their reach differs.
    #[test]
    fn both_remembering_lines_stay_narrow_and_differ_only_in_reach() {
        use surya_proto::RuleScope;
        assert_eq!(
            PermissionAnswer::AllowAlways.rule_scope(),
            Some(RuleScope::Workspace)
        );
        assert_eq!(
            PermissionAnswer::AllowAlwaysEverywhere.rule_scope(),
            Some(RuleScope::Global)
        );
        for line in [
            PermissionAnswer::AllowAlways,
            PermissionAnswer::AllowAlwaysEverywhere,
        ] {
            assert!(line.remembers());
            assert!(
                line.label().contains("exactly this"),
                "widening the reach must not widen what is pinned: {:?}",
                line.label()
            );
        }
    }

    /// Renamed with PERM-03: there are two remembering lines now, so the old
    /// name (`always_allow_is_the_only_line_that_writes_a_rule`) said
    /// something that had stopped being true.
    #[test]
    fn only_the_remembering_lines_write_a_rule() {
        for row in ROWS {
            let expected = matches!(
                row,
                PermissionAnswer::AllowAlways | PermissionAnswer::AllowAlwaysEverywhere
            );
            assert_eq!(row.remembers(), expected, "{:?}", row.label());
        }
    }

    #[test]
    fn every_line_either_decides_or_switches_the_mode() {
        for row in ROWS {
            assert!(
                row.decision().is_some() || row.stops_asking(),
                "{row:?} would leave the tool blocked with nothing sent"
            );
        }
    }
}
