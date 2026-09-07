//! Where the keyboard lives on the Chat route.
//!
//! gpui dispatches a matched key binding along the focus chain: from the
//! focused element up through its ancestors to the root. Every one of the
//! shell's shortcuts is an `on_action` on the shell's root div, so the chain
//! has to reach that div for any of them to fire. An element that is not
//! mounted has no ancestors, so focus parked on one is a dead end - not a
//! quiet one, a total one. `Shell::render` says it plainly: "Keyboard
//! shortcuts (mod-s/b/j) dispatch through the window focus chain - with
//! nothing focused they go dead."
//!
//! The composer used to be the only thing the Chat route ever focused, and
//! the Needs you page unmounts it (`render_main` mounts the composer only
//! `when(has_spaces && !inbox_page_up)`). Opening the page therefore left
//! focus on an unmounted composer and killed every shortcut until a click on
//! Home remounted it - E2E-KEY-01, "every keyboard shortcut dies while the
//! Needs you page is open".
//!
//! So the rule is one function: name the element that is BOTH mounted and
//! the right place to type, and let the render land focus there.

use gpui::Focusable;

use super::{Route, Shell};

/// The element that should hold keyboard focus this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyboardHome {
    /// The chat composer, the ordinary case.
    Composer,
    /// The Needs you page. It takes the main column and unmounts the
    /// composer, so it has to hold the keyboard itself.
    NeedsYou,
}

/// Which element `render` should focus, or `None` when this frame has no
/// mounted home for the keyboard (Settings, and the onboarding canvas that
/// has no composer yet).
///
/// `inbox_page_up` and `has_spaces` are exactly the two conditions
/// `render_main` gates the composer mount on, read the same way round: what
/// is on screen decides where focus goes, never the other way about.
pub(super) fn keyboard_home(
    is_chat_route: bool,
    inbox_page_up: bool,
    has_spaces: bool,
) -> Option<KeyboardHome> {
    if !is_chat_route {
        return None;
    }
    if inbox_page_up {
        return Some(KeyboardHome::NeedsYou);
    }
    has_spaces.then_some(KeyboardHome::Composer)
}

impl Shell {
    /// This frame's [`keyboard_home`], read off the live shell.
    pub(super) fn current_keyboard_home(
        &mut self,
        cx: &mut gpui::Context<Self>,
    ) -> Option<KeyboardHome> {
        let has_spaces = !self.state.read(cx).spaces.is_empty();
        keyboard_home(
            matches!(self.route, Route::Chat),
            self.inbox_visible(cx),
            has_spaces,
        )
    }

    /// Follow the keyboard home when it changes, and re-land focus when
    /// nothing holds it.
    ///
    /// Only a CHANGE re-lands. Focus lives inside the home most of the time -
    /// the composer's own input, a picker's search box, the terminal - and
    /// re-landing every frame would yank it out of all three. But when the
    /// home itself changes the old one is being unmounted this very frame, so
    /// the keyboard has to move with it or it moves nowhere (E2E-KEY-01).
    pub(super) fn sync_keyboard_focus(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let home = self.current_keyboard_home(cx);
        let moved = home != self.keyboard_home;
        self.keyboard_home = home;
        if moved || window.focused(cx).is_none() {
            self.land_keyboard_focus(window, cx);
        }
    }

    /// Put focus on this frame's keyboard home. Called both when focus is
    /// lost with no successor and when the frame renders with nothing
    /// focused, so a page that unmounts the focused element hands the
    /// keyboard on rather than dropping it.
    pub(super) fn land_keyboard_focus(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        match self.current_keyboard_home(cx) {
            Some(KeyboardHome::Composer) => {
                let handle = self.composer.focus_handle(cx);
                window.focus(&handle, cx);
            }
            Some(KeyboardHome::NeedsYou) => {
                // `inbox_visible` is only true once the pane exists, so this
                // never builds one just to focus it.
                if let Some(pane) = self.inbox_pane(cx) {
                    let handle = pane.read(cx).focus_handle.clone();
                    window.focus(&handle, cx);
                }
            }
            // Nothing mounted to type into. Clear the stale handle so
            // `window.focused()` reads None: a lingering unmounted handle
            // would otherwise dead-end keyboard dispatch for good, and the
            // render hook re-lands focus the moment a home appears.
            None => window.blur(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_needs_you_page_takes_the_keyboard_from_the_unmounted_composer() {
        // E2E-KEY-01. The page unmounts the composer, so focus cannot stay
        // on it: with the page up the keyboard belongs to the page.
        assert_eq!(
            keyboard_home(true, true, true),
            Some(KeyboardHome::NeedsYou)
        );
        // ...and comes back the moment the page goes down.
        assert_eq!(
            keyboard_home(true, false, true),
            Some(KeyboardHome::Composer)
        );
    }

    #[test]
    fn a_route_with_no_composer_mounted_gets_no_focus() {
        // Settings has no composer at all, whatever the other two say.
        assert_eq!(keyboard_home(false, false, true), None);
        assert_eq!(keyboard_home(false, true, true), None);
        // The onboarding canvas: `render_main` gates the composer on
        // `has_spaces`, so there is nothing to focus until a project exists.
        assert_eq!(keyboard_home(true, false, false), None);
    }

    #[test]
    fn the_page_beats_an_empty_project_list() {
        // The page mounts on its own and does not need a project, so it is
        // still the keyboard's home with no spaces.
        assert_eq!(
            keyboard_home(true, true, false),
            Some(KeyboardHome::NeedsYou)
        );
    }
}
