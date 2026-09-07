//! The mark a stopped turn leaves in the transcript (E2E-CHAT-02).
//!
//! Pressing stop mid-stream cut the text mid-word and left nothing behind:
//! the turn ended at "25. Tw" under an ordinary timestamp, and the only cue
//! anywhere was the agent tree switching to "Waiting for you", which is a
//! different panel and reads like an inbox item. Later you cannot tell a
//! short answer from one you cut.
//!
//! The engine already records it - `DoneStatus::Interrupted` becomes
//! `MessageStatus::Aborted` (`surya-engine`'s `sessions.rs`), and the doc
//! carries it to us. Before this module the whole 8000-line `transcript.rs`
//! read `Aborted` in exactly one place, `entry_fingerprint`, which is a cache
//! key. The status never reached a pixel.
//!
//! Its own file because `transcript.rs` is far past the 500-line rule
//! (decision 13) and must not grow.

use gpui::prelude::*;
use gpui::{div, px, Div, SharedString};

use crate::theme::Theme;

/// What the mark says. One word, the same word decision 17 uses for the
/// agent state, so the transcript and the agent tree agree.
pub const STOPPED_LABEL: &str = "Stopped";

/// Does this entry's status mean the user cut it short?
///
/// Only `Aborted`. A `Complete` turn ended on its own and an entry with no
/// status at all is legacy or still arriving - neither is a stop, and
/// marking either would put the word on turns nobody touched.
pub fn is_stopped(status: Option<surya_doc::MessageStatus>) -> bool {
    matches!(status, Some(surya_doc::MessageStatus::Aborted))
}

/// The mark itself: a quiet label in the footer strip, beside the timestamp.
///
/// Deliberately NOT inside the hover fade the timestamp and copy button live
/// in. The whole point is that you can come back later and see why the answer
/// is short, and a cue you have to hunt for with the pointer is not that.
pub fn mark(theme: &Theme) -> Div {
    div()
        .flex_none()
        .px(px(6.0))
        .rounded(px(4.0))
        .text_size(crate::typography::ui_rems(11.5))
        .bg(theme.warning.opacity(0.12))
        .text_color(theme.warning)
        .child(SharedString::from(STOPPED_LABEL))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::rows_for_entry;
    use crate::markdown::parser::{parse_full, BlockTree};
    use std::sync::Arc;
    use surya_doc::{MessagePart, MessageRole, MessageStatus, SessionMessageEntry};

    fn parse(_: &str, text: &str) -> Arc<BlockTree> {
        Arc::new(parse_full(text))
    }

    fn assistant(id: &str, status: MessageStatus, text: &str) -> SessionMessageEntry {
        SessionMessageEntry {
            id: id.into(),
            role: MessageRole::Assistant,
            parts: vec![MessagePart::Text {
                id: "t0".into(),
                text: text.into(),
            }],
            created_at: 0,
            device_id: "dev".into(),
            status: Some(status),
            continuation_of: None,
        }
    }

    #[test]
    fn only_an_aborted_turn_is_marked_stopped() {
        assert!(is_stopped(Some(MessageStatus::Aborted)));
        assert!(!is_stopped(Some(MessageStatus::Complete)));
        assert!(
            !is_stopped(Some(MessageStatus::Streaming)),
            "a live turn has not been stopped yet"
        );
        assert!(
            !is_stopped(None),
            "a legacy or still-arriving entry is not a stop"
        );
    }

    #[test]
    fn the_mark_says_the_same_word_the_agent_tree_says() {
        // Decision 17 names the fourth agent state "Stopped". The transcript
        // must not invent a second word for the same thing.
        assert_eq!(STOPPED_LABEL, "Stopped");
    }

    /// E2E-CHAT-02: pressing stop cut the text mid-word and left nothing in
    /// the transcript saying so. The engine had already recorded it as
    /// `Aborted`; the rows threw it away.
    #[test]
    fn a_stopped_turn_marks_its_last_row_and_a_finished_one_does_not() {
        let cut = assistant("m1", MessageStatus::Aborted, "counting: 1. One 2. Two 25. Tw");
        let rows = rows_for_entry(&cut, false, &mut parse);
        let last = rows.last().expect("a stopped turn still renders its text");
        assert!(
            last.stopped,
            "the turn the user cut has to say so where they read the answer"
        );
        assert!(
            rows[..rows.len() - 1].iter().all(|r| !r.stopped),
            "the mark belongs to the turn, so only its last row carries it"
        );

        let finished = assistant("m2", MessageStatus::Complete, "counting: 1. One 2. Two");
        assert!(
            rows_for_entry(&finished, false, &mut parse)
                .iter()
                .all(|r| !r.stopped),
            "a turn that ended on its own must not be accused of being cut"
        );
    }
}
