//! Mail envelope: the message row, its address forms, and the exact text the
//! recipient's turn carries.
//!
//! Decision 19 keeps agb's shape: an agent id or a `#workspace` address, one
//! delivery id per message, automatic ack when the carrying turn completes.
//! `from_device` / `to_device` ride every row from day one so cross-server
//! forwarding is an added call, not a schema change.

use serde::{Deserialize, Serialize};

/// Where a message is in its life. Derived from the timestamps, never stored
/// twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MailState {
    /// Written, not yet in front of the recipient.
    Queued,
    /// Injected into a turn; that turn has not finished.
    Delivered,
    /// The carrying turn completed, or a reader marked it seen.
    Acked,
}

/// One mail row. `id` is the delivery id every sender gets back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailMessage {
    pub id: String,
    /// Sender address as written (agent id, or a human alias).
    pub from: String,
    /// Recipient address as written: an agent id, an alias, or `#workspace`.
    pub to: String,
    /// The agent this row is actually for. A `#workspace` send fans out into
    /// one row per live agent, each with its own delivery id and this field
    /// resolved to a chat id.
    pub to_agent: String,
    pub body: String,
    pub created_at: i64,
    pub delivered_at: Option<i64>,
    pub acked_at: Option<i64>,
    /// Device that accepted the send. Always this engine's id today.
    pub from_device: String,
    /// Device that must deliver. Only honoured when it equals the local
    /// device; a row addressed elsewhere waits for the forwarding call.
    pub to_device: String,
    /// Run that carries the message, once one does.
    pub run_id: Option<String>,
}

impl MailMessage {
    pub fn state(&self) -> MailState {
        if self.acked_at.is_some() {
            MailState::Acked
        } else if self.delivered_at.is_some() {
            MailState::Delivered
        } else {
            MailState::Queued
        }
    }

    /// The line the recipient reads. One line per message, in send order, so a
    /// batch of queued mail folds into a single turn without ambiguity.
    pub fn envelope_line(&self) -> String {
        format!("[MAIL {} from {}] {}", self.id, self.from, self.body)
    }
}

/// A parsed recipient address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailAddress {
    /// One agent: a chat id, or a chat title used as a human alias.
    Agent(String),
    /// Every live agent in one workspace: a space id, or a space name.
    Workspace(String),
}

impl MailAddress {
    /// `#name` is a workspace fan-out; anything else names one agent.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("empty address".into());
        }
        match trimmed.strip_prefix('#') {
            Some("") => Err("empty workspace address".into()),
            Some(name) => Ok(Self::Workspace(name.to_string())),
            None => Ok(Self::Agent(trimmed.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_address_forms() {
        assert_eq!(
            MailAddress::parse("chat-a").unwrap(),
            MailAddress::Agent("chat-a".into())
        );
        assert_eq!(
            MailAddress::parse("#surya").unwrap(),
            MailAddress::Workspace("surya".into())
        );
        assert!(MailAddress::parse("  ").is_err());
        assert!(MailAddress::parse("#").is_err());
    }

    #[test]
    fn state_follows_the_timestamps() {
        let mut m = MailMessage {
            id: "m1".into(),
            from: "a".into(),
            to: "b".into(),
            to_agent: "b".into(),
            body: "hi".into(),
            created_at: 1,
            delivered_at: None,
            acked_at: None,
            from_device: "d".into(),
            to_device: "d".into(),
            run_id: None,
        };
        assert_eq!(m.state(), MailState::Queued);
        m.delivered_at = Some(2);
        assert_eq!(m.state(), MailState::Delivered);
        m.acked_at = Some(3);
        assert_eq!(m.state(), MailState::Acked);
        assert_eq!(m.envelope_line(), "[MAIL m1 from a] hi");
    }
}
