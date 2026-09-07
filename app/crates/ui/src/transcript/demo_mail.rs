//! `SURYA_DEMO_MAIL=<sender>|<body>`: start with a transcript that holds one
//! row the owner typed and one another agent mailed in, so the two shapes
//! can be photographed side by side.
//!
//! Display-only, the same shape as `SURYA_DEMO_CARDS` and
//! `SURYA_DEMO_TASK_ERROR`, and it exists for the same reason those do: the
//! state cannot be staged in front of a camera. A real mail row needs a
//! second agent running against the same engine, addressed at the right
//! moment, with the shot taken before its turn answers - which is not a
//! thing a screenshot rig can arrange.
//!
//! The knob builds doc entries and hands them to the real transcript, so
//! what it paints is the render path, not a mock of it.

use surya_doc::{MessagePart, MessageRole, MessageStatus, SessionMessageEntry};
use surya_proto::mail::MessageSource;

/// The fake chat the knob shows its rows in.
pub const DEMO_MAIL_CHAT: &str = "demo-mail";

/// The prompt the owner "typed", so the frame carries both row shapes. Fixed
/// rather than configurable: the knob is about the mail row, and the row it
/// is compared against only has to be an ordinary one.
const OWNER_PROMPT: &str = "ask the other agent where the build got to";

/// The delivery id the fake envelope carries. Any addressable string does;
/// it never leaves this process.
const DEMO_DELIVERY_ID: &str = "demo1";

/// The entries the knob seeds, or `None` when it is switched off.
///
/// Blank and whitespace-only text is `None`: exporting a variable empty is
/// how a shell leaves a knob switched off. A value with no `|` is taken as
/// the body alone, from a sender named `agent`, so the short form works.
pub fn entries(raw: Option<String>) -> Option<Vec<SessionMessageEntry>> {
    let raw = raw?;
    if raw.trim().is_empty() {
        return None;
    }
    let (from, body) = match raw.split_once('|') {
        Some((from, body)) => (from.trim(), body.trim()),
        None => ("agent", raw.trim()),
    };
    let from = if from.is_empty() { "agent" } else { from };
    let body = if body.is_empty() {
        "the build is green, all five commands"
    } else {
        body
    };
    let now = chrono::Utc::now().timestamp_millis();
    Some(vec![
        entry("demo-mail-owner", OWNER_PROMPT, now, None),
        entry(
            &format!("mailmsg-{DEMO_DELIVERY_ID}"),
            // The real thing: delivery stores the envelope text and the
            // source field beside it, and the row reads them exactly as it
            // would on a delivered message.
            &surya_proto::mail::envelope_block(DEMO_DELIVERY_ID, from, body),
            now + 1,
            Some(MessageSource::Mail {
                from: vec![from.to_string()],
            }),
        ),
    ])
}

fn entry(
    id: &str,
    text: &str,
    created_at: i64,
    source: Option<MessageSource>,
) -> SessionMessageEntry {
    SessionMessageEntry {
        id: id.into(),
        role: MessageRole::User,
        parts: vec![MessagePart::Text {
            id: "t0".into(),
            text: text.into(),
        }],
        created_at,
        device_id: "local".into(),
        status: Some(MessageStatus::Complete),
        continuation_of: None,
        source,
    }
}

/// Read the knob. The only reader of this variable in the process.
pub fn from_env() -> Option<String> {
    std::env::var("SURYA_DEMO_MAIL").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unset_or_blank_knob_seeds_nothing() {
        assert!(entries(None).is_none());
        assert!(entries(Some(String::new())).is_none());
        assert!(entries(Some("  \t ".into())).is_none());
    }

    #[test]
    fn the_knob_seeds_one_typed_row_and_one_mailed_row() {
        let seeded = entries(Some("finch|the build is green".into())).expect("entries");
        assert_eq!(seeded.len(), 2);
        assert_eq!(seeded[0].source, None, "the owner's row has no source");
        assert_eq!(
            seeded[1].source,
            Some(MessageSource::Mail {
                from: vec!["finch".into()]
            })
        );
    }

    /// The seeded row has to reach the transcript as mail, which means going
    /// through the same split the delivered thing goes through.
    #[test]
    fn the_seeded_row_reads_back_as_the_mail_it_claims_to_be() {
        let seeded = entries(Some("finch|the build is green".into())).expect("entries");
        let rows = crate::transcript::mail_row::rows(&seeded[1]).expect("a mail row");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].from, "finch");
        assert_eq!(rows[0].text, "the build is green");
        assert!(
            crate::transcript::mail_row::rows(&seeded[0]).is_none(),
            "the owner's row must not render as mail"
        );
    }

    #[test]
    fn the_short_form_names_a_sender_anyway() {
        let seeded = entries(Some("just a body".into())).expect("entries");
        let rows = crate::transcript::mail_row::rows(&seeded[1]).expect("a mail row");
        assert_eq!(rows[0].from, "agent");
        assert_eq!(rows[0].text, "just a body");
    }
}
