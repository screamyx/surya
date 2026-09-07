//! What a dispatched turn writes into the transcript as its user row.
//!
//! Dispatch used to take a bare `Option<String>` message id. Mail needs one
//! more thing on that row - who sent it - and the two travel together: the
//! id names the entry, the source says who wrote it. Keeping them in one
//! value is what stops a caller passing the id and forgetting the source.

use surya_proto::mail::MessageSource;

/// The user row a dispatch writes.
///
/// `None` at a call site means "an ordinary turn, mint an id" and is the
/// common case; every existing caller reads that way unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageOrigin {
    /// The doc entry id to write under. Idempotent: a re-dispatch under the
    /// same id finds the entry and leaves it alone.
    pub message_id: String,
    /// Who authored the prompt, when that was not the person at the
    /// keyboard. `None` for a typed turn.
    pub source: Option<MessageSource>,
}

impl MessageOrigin {
    /// A turn the user typed, landing under a client-minted id.
    pub fn typed(message_id: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            source: None,
        }
    }

    /// A mail delivery. `from` names the senders whose envelopes the turn's
    /// prompt carries, in delivery order.
    pub fn mail(message_id: impl Into<String>, from: Vec<String>) -> Self {
        Self {
            message_id: message_id.into(),
            source: Some(MessageSource::Mail { from }),
        }
    }
}

impl From<String> for MessageOrigin {
    fn from(message_id: String) -> Self {
        Self::typed(message_id)
    }
}

impl From<&str> for MessageOrigin {
    fn from(message_id: &str) -> Self {
        Self::typed(message_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_turn_carries_no_source() {
        let origin: MessageOrigin = "msg-1".to_string().into();
        assert_eq!(origin.message_id, "msg-1");
        assert_eq!(origin.source, None);
    }

    #[test]
    fn a_mail_turn_names_every_sender_it_carries() {
        let origin = MessageOrigin::mail("mailmsg-7", vec!["finch".into(), "osprey".into()]);
        assert_eq!(origin.message_id, "mailmsg-7");
        assert_eq!(
            origin.source.as_ref().map(MessageSource::mail_senders),
            Some(&["finch".to_string(), "osprey".to_string()][..])
        );
    }
}
