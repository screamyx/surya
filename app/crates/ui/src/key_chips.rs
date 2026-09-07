//! The key-cap labels the add-space palette shows (E2E-UI-04).
//!
//! Both chips used to draw `icons::COMMAND` unconditionally, so a Windows
//! user opening New project read "⌘K" and "⌘ Enter" - the Mac Command
//! symbol, on the product platform (decision 28). The Shortcuts settings
//! page named the same keys correctly all along, which is what made the
//! glyphs a defect rather than a house style.
//!
//! `settings::badge_combo_on` is the ONE place the primary modifier is
//! chosen, and it was already right: the glyph on macOS, the word "Ctrl"
//! everywhere else. The picker simply never called it. These two functions
//! are the picker's labels, named so they can be asserted per platform
//! without standing up a window.

use crate::settings::badge_combo_on;

/// The chip on the search bar: the chord that summons the palette.
pub fn summon_label(mac: bool) -> String {
    badge_combo_on(mac, "mod-k")
}

/// The primary chip: the chord that adds the previewed folder.
///
/// The key stays the WORD "Enter" rather than the bare return arrow (user
/// request - the arrow read as noise). `badge_combo_on` spells it that way.
pub fn submit_label(mac: bool) -> String {
    badge_combo_on(mac, "mod-enter")
}

/// The model picker's jump chip: the chord that activates the nth model.
///
/// Same story as the picker chips - it built its label with a hardcoded
/// Command glyph, so Windows read the Mac symbol here too.
pub fn jump_label(mac: bool, slot: usize) -> String {
    badge_combo_on(mac, &format!("mod-{slot}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// U+2318 PLACE OF INTEREST SIGN, the Mac Command key.
    const COMMAND_GLYPH: char = '⌘';

    /// The finding: this is what Windows saw.
    #[test]
    fn no_mac_glyph_reaches_a_windows_or_linux_chip() {
        for label in [summon_label(false), submit_label(false)] {
            assert!(
                !label.contains(COMMAND_GLYPH),
                "a Mac modifier glyph off macOS: {label:?}"
            );
        }
        assert_eq!(summon_label(false), "Ctrl+K");
        assert_eq!(submit_label(false), "Ctrl+Enter");
    }

    /// The model picker's jump chip had the same hardcoded glyph, found in
    /// review after the first two were fixed.
    #[test]
    fn the_model_jump_chip_names_its_key_per_platform() {
        for slot in 1..=9 {
            let off_mac = jump_label(false, slot);
            assert!(
                !off_mac.contains(COMMAND_GLYPH),
                "a Mac modifier glyph off macOS: {off_mac:?}"
            );
            assert_eq!(off_mac, format!("Ctrl+{slot}"));
            assert_eq!(jump_label(true, slot), format!("⌘{slot}"));
        }
    }

    #[test]
    fn macos_still_gets_its_glyph() {
        assert_eq!(summon_label(true), "⌘K");
        assert_eq!(submit_label(true), "⌘Enter");
    }

    /// The add chip says the key in words, not as a return arrow.
    #[test]
    fn the_add_chip_spells_enter_out() {
        assert!(submit_label(true).contains("Enter"));
        assert!(submit_label(false).contains("Enter"));
    }
}
