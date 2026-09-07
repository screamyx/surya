//! Re-running a turn that stopped or failed (E2E-CHAT-03).
//!
//! There was no way to run a turn again. Resending meant retyping the prompt
//! or copying it back into the composer by hand, and the Needs-you inbox -
//! which decision 17 says should carry "the reason, the detail and a Retry
//! button" for a stopped agent - offered only "Open chat". The `retryable`
//! flag was carried all the way from the engine into `InboxRow` and then
//! read by nobody.
//!
//! Retry re-sends the last user prompt as a NEW turn. It does not resume the
//! interrupted one: the stopped turn stays in the transcript as history, and
//! the existing send path already knows how to steer a live run or start an
//! idle one. The engine's own `startup_retry` is a different thing entirely -
//! an internal one-shot after a harness startup crash - and is not touched.
//!
//! Its own file because `transcript.rs` is far past the 500-line rule
//! (decision 13).

use gpui::prelude::*;
use gpui::{div, px, Div, SharedString};
use surya_doc::{MessagePart, MessageRole, MessageStatus, SessionMessageEntry};

use crate::theme::Theme;

/// The word on the control. Decision 17 names the action "Retry".
pub const RETRY_LABEL: &str = "Retry";

/// Did this turn end in a way a person would want to run again?
///
/// `Aborted` is the stop button. A turn with no status is legacy or still
/// arriving, and `Complete` finished on its own - offering to re-run either
/// would invite the user to spend a second run on an answer they already
/// have.
pub fn is_retryable(status: Option<MessageStatus>) -> bool {
    matches!(status, Some(MessageStatus::Aborted))
}

/// The prompt a Retry on `entry_id` would re-send: the text of the nearest
/// USER turn before it.
///
/// Walks backwards rather than taking the last user entry overall, so a
/// Retry on an older stopped turn re-sends the prompt that turn answered,
/// not whatever was typed most recently.
pub fn prompt_before(entries: &[SessionMessageEntry], entry_id: &str) -> Option<String> {
    let ix = entries.iter().position(|e| e.id == entry_id)?;
    entries[..ix]
        .iter()
        .rev()
        .find(|e| e.role == MessageRole::User)
        .and_then(user_text)
}

/// The prompt a Retry from the INBOX would re-send. The inbox row names a
/// chat, not a turn, so this is the last user prompt in it.
pub fn last_prompt(entries: &[SessionMessageEntry]) -> Option<String> {
    entries
        .iter()
        .rev()
        .find(|e| e.role == MessageRole::User)
        .and_then(user_text)
}

/// A user entry's text parts, joined the way the composer sent them.
fn user_text(entry: &SessionMessageEntry) -> Option<String> {
    let text = entry
        .parts
        .iter()
        .filter_map(|p| match p {
            MessagePart::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    (!text.trim().is_empty()).then_some(text)
}

/// What a parked inbox Retry should do on this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parked {
    /// The chat is not on screen yet. Keep the id and look again.
    Wait,
    /// The chat is up. Look for its last prompt and send.
    Arrived,
    /// The user selected a DIFFERENT chat. Drop the id.
    ///
    /// Without this the parked id survived indefinitely and fired a run
    /// nobody asked for the next time that chat was opened, which could be
    /// much later.
    Disarm,
}

/// Decide from the parked chat id and what is currently selected.
///
/// Pure so the three outcomes can be asserted without a window.
pub fn parked_state(parked: &str, selected: Option<&str>) -> Parked {
    match selected {
        Some(id) if id == parked => Parked::Arrived,
        // Still on the way: nothing is selected yet.
        None => Parked::Wait,
        Some(_) => Parked::Disarm,
    }
}

/// The control itself: a quiet text button in the footer strip.
pub fn control(theme: &Theme) -> Div {
    div()
        .flex_none()
        .px(px(6.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .text_size(crate::typography::ui_rems(11.5))
        .text_color(theme.text_muted)
        .hover(|s| s.bg(crate::theme::ink(0.08)).text_color(theme.text))
        .child(SharedString::from(RETRY_LABEL))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, role: MessageRole, status: Option<MessageStatus>, text: &str) -> SessionMessageEntry {
        SessionMessageEntry {
            id: id.into(),
            role,
            parts: vec![MessagePart::Text {
                id: format!("{id}-t").into(),
                text: text.into(),
            }],
            created_at: 0,
            device_id: "dev".into(),
            status,
            continuation_of: None,
            source: None,
        }
    }

    fn user(id: &str, text: &str) -> SessionMessageEntry {
        entry(id, MessageRole::User, None, text)
    }

    fn assistant(id: &str, status: MessageStatus, text: &str) -> SessionMessageEntry {
        entry(id, MessageRole::Assistant, Some(status), text)
    }

    #[test]
    fn only_a_stopped_turn_offers_a_retry() {
        assert!(is_retryable(Some(MessageStatus::Aborted)));
        assert!(
            !is_retryable(Some(MessageStatus::Complete)),
            "an answer that finished does not need running again"
        );
        assert!(!is_retryable(Some(MessageStatus::Streaming)));
        assert!(!is_retryable(None));
    }

    /// The finding: resending meant retyping the prompt. Retry has to find
    /// it without the user.
    #[test]
    fn retry_resends_the_prompt_that_turn_was_answering() {
        let entries = vec![
            user("u1", "count to a hundred"),
            assistant("a1", MessageStatus::Aborted, "1. One 2. Tw"),
        ];
        assert_eq!(
            prompt_before(&entries, "a1").as_deref(),
            Some("count to a hundred")
        );
    }

    /// A Retry on an OLD stopped turn must not re-send a newer prompt.
    #[test]
    fn an_older_stopped_turn_resends_its_own_prompt() {
        let entries = vec![
            user("u1", "count to a hundred"),
            assistant("a1", MessageStatus::Aborted, "1. One 2. Tw"),
            user("u2", "never mind, what is the weather"),
            assistant("a2", MessageStatus::Complete, "sunny"),
        ];
        assert_eq!(
            prompt_before(&entries, "a1").as_deref(),
            Some("count to a hundred"),
            "walking back from the turn, not taking the newest prompt"
        );
        assert_eq!(
            last_prompt(&entries).as_deref(),
            Some("never mind, what is the weather"),
            "the inbox names a chat, so it takes the latest"
        );
    }

    #[test]
    fn a_turn_with_no_prompt_behind_it_offers_nothing_to_resend() {
        let entries = vec![assistant("a1", MessageStatus::Aborted, "orphan")];
        assert_eq!(prompt_before(&entries, "a1"), None);
        assert_eq!(prompt_before(&entries, "nope"), None, "unknown id");
        assert_eq!(last_prompt(&[]), None);
    }

    #[test]
    fn an_empty_prompt_is_not_worth_resending() {
        let entries = vec![user("u1", "   "), assistant("a1", MessageStatus::Aborted, "x")];
        assert_eq!(
            prompt_before(&entries, "a1"),
            None,
            "re-sending whitespace would burn a run on nothing"
        );
    }

    #[test]
    fn a_parked_retry_waits_arrives_or_disarms() {
        assert_eq!(parked_state("c1", Some("c1")), Parked::Arrived);
        assert_eq!(parked_state("c1", None), Parked::Wait, "still on the way");
        assert_eq!(
            parked_state("c1", Some("c2")),
            Parked::Disarm,
            "the user went elsewhere; holding the id would fire an unasked run later"
        );
    }
}
