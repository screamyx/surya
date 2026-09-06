//! Yolo mode in the composer: the chip, its three states, and the line the
//! chat header shows while the mode is on.
//!
//! Yolo mode runs the agent's tools without asking. The flag itself is
//! `ChatConfig.auto_approve` on the chat row; everything here is about
//! telling the user which of three situations they are in, because only one
//! of them is a switch they own:
//!
//! - OFF and ON, for an agent that asks before it runs a tool. Today that is
//!   Claude Code (and the mock harness). The chip flips the chat's flag.
//! - ALWAYS, for an agent that never asks. Codex, Cursor, opencode and the
//!   ACP agents (Devin, Grok, Hermes, pi) are all launched in their own
//!   no-prompt modes, and the permission gate is not wired to them at all
//!   (`surya-harness`: cursor/mod.rs and opencode/mod.rs both say "The
//!   permission gate is wired for the claude driver only"). There is nothing
//!   to switch off, so the chip says so and does not pretend to be a toggle.
//!
//! Splitting the check out of `pickers.rs` keeps the decision testable and
//! both files under the 500-line build rule (decision 13).

use gpui::{SharedString, div, prelude::*, px};

use surya_proto::HarnessId;

use crate::motion;
use crate::theme::Theme;

/// What the chip shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YoloState {
    /// The agent asks, and the user has not turned prompts off.
    Off,
    /// The agent asks, and the engine answers for the user.
    On,
    /// The agent never asks. Not a toggle.
    Always,
}

impl YoloState {
    pub fn is_toggle(self) -> bool {
        self != YoloState::Always
    }

    /// The chip's first tone. The state is carried by [`Self::suffix`] beside
    /// it, the same two-tone shape the model chip uses for its traits, so the
    /// name itself never changes.
    pub fn name(self) -> &'static str {
        "Yolo"
    }

    /// The muted second tone on the chip, if any.
    pub fn suffix(self) -> Option<&'static str> {
        match self {
            YoloState::Off => None,
            YoloState::On => Some("on"),
            YoloState::Always => Some("always"),
        }
    }
}

/// Does this agent ask before it runs a tool?
///
/// Claude Code routes tool permissions over its stdio control channel, and
/// the mock harness scripts one for tests. Every other harness is launched
/// unattended by its own adapter.
pub fn harness_asks(harness: HarnessId) -> bool {
    match harness {
        HarnessId::ClaudeCode | HarnessId::Mock => true,
        HarnessId::Codex
        | HarnessId::Cursor
        | HarnessId::Devin
        | HarnessId::Grok
        | HarnessId::Hermes
        | HarnessId::Pi
        | HarnessId::Opencode => false,
    }
}

/// The chip's state. An unknown harness (catalog still loading) is treated as
/// one that asks: the toggle is the honest default, and it resolves to
/// `Always` by itself once the harness is known.
pub fn state_for(harness: Option<HarnessId>, on: bool) -> YoloState {
    match harness {
        Some(harness) if !harness_asks(harness) => YoloState::Always,
        _ if on => YoloState::On,
        _ => YoloState::Off,
    }
}

/// What the chat header says about yolo mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderNote {
    /// Nothing to say.
    None,
    /// The mode is on for this chat.
    On,
    /// The user switched it off while THE run that is still going was
    /// bypassing. That run was launched with the CLI's own bypass flag and
    /// goes on bypassing until it ends. There is no way to tell a running
    /// agent to start asking again, so the header says when prompts come back
    /// instead of claiming they already have.
    OffNextRun,
}

impl HeaderNote {
    pub fn text(self) -> Option<&'static str> {
        match self {
            HeaderNote::None => None,
            HeaderNote::On => Some("Yolo"),
            HeaderNote::OffNextRun => Some("Yolo off next run"),
        }
    }
}

/// `bypassing_run_live`: the run that was in flight when the user switched
/// yolo off is STILL the live run. Not "a run is working": a later run
/// launched with prompts on is prompting normally, and telling that user
/// prompts come back next run would be the honesty rule stood on its head.
pub fn header_note(on: bool, bypassing_run_live: bool) -> HeaderNote {
    if on {
        return HeaderNote::On;
    }
    if bypassing_run_live {
        return HeaderNote::OffNextRun;
    }
    HeaderNote::None
}

/// The composer chip. The caller owns the click (it is the one holding the
/// chat id and the engine handle) and attaches it only when
/// [`YoloState::is_toggle`].
pub fn chip(state: YoloState, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    // Same ghost pill as the model chip (surya composer/styles.tsx `pill`):
    // h-8, rounded-lg, 12px medium, hover wash, no border. On reads in the
    // warning family: amber is the app's "this needs your attention" tone
    // and running tools unasked is exactly that, without being an error.
    let id = "picker-yolo";
    let (text, wash) = match state {
        YoloState::On => (theme.warning, Some(theme.warning_wash)),
        YoloState::Off => (theme.text_muted, None),
        YoloState::Always => (theme.text_faint, None),
    };
    div()
        .id(id)
        .h(px(32.0))
        .flex_none()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(10.0))
        .rounded(px(8.0))
        .text_size(crate::typography::ui_rems(12.0))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(match state {
            // Only a real toggle brightens under the pointer.
            YoloState::Always => text,
            _ => motion::hover_blend(id, text, theme.text),
        })
        .when_some(wash, |el, wash| el.bg(wash))
        .when(wash.is_none() && state.is_toggle(), |el| {
            el.bg(motion::hover_blend(
                id,
                gpui::transparent_black(),
                theme.element_hover,
            ))
            .on_hover(motion::hover_listener(id))
            .cursor_pointer()
        })
        .child(
            crate::icons::icon(crate::icons::DANGER_TRIANGLE)
                .size(px(16.0))
                .text_color(text),
        )
        .child(div().child(SharedString::from(state.name())))
        .when_some(state.suffix(), |el, suffix| {
            el.child(
                div()
                    .text_color(theme.text_muted.opacity(0.7))
                    .child(SharedString::from(suffix)),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_and_the_mock_are_the_agents_that_ask() {
        assert!(harness_asks(HarnessId::ClaudeCode));
        assert!(harness_asks(HarnessId::Mock));
        for harness in [
            HarnessId::Codex,
            HarnessId::Cursor,
            HarnessId::Devin,
            HarnessId::Grok,
            HarnessId::Hermes,
            HarnessId::Pi,
            HarnessId::Opencode,
        ] {
            assert!(!harness_asks(harness), "{harness:?} does not ask");
        }
    }

    #[test]
    fn an_agent_that_never_asks_is_never_a_toggle() {
        assert_eq!(
            state_for(Some(HarnessId::Codex), false),
            YoloState::Always,
            "off is not a state codex can be in"
        );
        assert_eq!(state_for(Some(HarnessId::Codex), true), YoloState::Always);
        assert!(!YoloState::Always.is_toggle());
    }

    #[test]
    fn an_agent_that_asks_follows_the_flag() {
        assert_eq!(state_for(Some(HarnessId::ClaudeCode), false), YoloState::Off);
        assert_eq!(state_for(Some(HarnessId::ClaudeCode), true), YoloState::On);
        assert!(YoloState::On.is_toggle());
    }

    #[test]
    fn an_unknown_harness_reads_as_a_toggle() {
        assert_eq!(state_for(None, false), YoloState::Off);
        assert_eq!(state_for(None, true), YoloState::On);
    }

    #[test]
    fn the_header_never_claims_prompts_are_back_before_they_are() {
        assert_eq!(header_note(true, false), HeaderNote::On);
        assert_eq!(header_note(true, true), HeaderNote::On);
        assert_eq!(header_note(false, true), HeaderNote::OffNextRun);
        assert_eq!(
            header_note(false, false),
            HeaderNote::None,
            "no bypassing run left: prompts really are back"
        );
    }
}
