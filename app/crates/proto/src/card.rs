//! A2UI cards on the wire: the agent-drawn card event and the typed action a
//! card button sends back (surya decision 4 and 14).
//!
//! The card JSON itself is opaque here — `surya-a2ui` parses it. The proto
//! only carries it and defines the v1 wire form of a card action as a user
//! turn: `[card:<id>] <action> <payload-json>`.

use serde::{Deserialize, Serialize};

/// A user's tap on a card button, resolved against the card's data model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardAction {
    /// The card the button lives in (`AgentEvent::Card::card_id`).
    pub card_id: String,
    /// The A2UI action's `event.name`.
    pub action: String,
    /// The action's resolved `event.context` (data paths already read).
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Prefix of the v1 wire form; a harness or agent can recognise a card
/// answer by it.
pub const CARD_ACTION_WIRE_PREFIX: &str = "[card:";

impl CardAction {
    /// The v1 wire form forwarded to the agent as a user turn:
    /// `[card:<id>] <action> <payload>`. The payload is compact JSON of the
    /// context; an empty context is `{}` so the shape is always three fields.
    pub fn to_wire(&self) -> String {
        let payload = match &self.payload {
            serde_json::Value::Null => "{}".to_owned(),
            other => serde_json::to_string(other).unwrap_or_else(|_| "{}".to_owned()),
        };
        format!("{CARD_ACTION_WIRE_PREFIX}{}] {} {payload}", self.card_id, self.action)
    }

    /// Parse the v1 wire form back. `None` for anything that is not a card
    /// action (ordinary prompts start with other text).
    pub fn parse_wire(text: &str) -> Option<Self> {
        let rest = text.trim().strip_prefix(CARD_ACTION_WIRE_PREFIX)?;
        let close = rest.find(']')?;
        let card_id = rest[..close].to_owned();
        if card_id.is_empty() {
            return None;
        }
        let rest = rest[close + 1..].trim_start();
        let (action, payload) = match rest.split_once(char::is_whitespace) {
            Some((action, payload)) => (action, payload.trim()),
            None => (rest, ""),
        };
        if action.is_empty() {
            return None;
        }
        let payload = if payload.is_empty() {
            serde_json::Value::Object(Default::default())
        } else {
            serde_json::from_str(payload).ok()?
        };
        Some(Self {
            card_id,
            action: action.to_owned(),
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_form_round_trips() {
        let action = CardAction {
            card_id: "leads_table".into(),
            action: "set_follow_ups".into(),
            payload: serde_json::json!({ "ids": [412, 418], "when": "2026-09-06" }),
        };
        let wire = action.to_wire();
        assert!(wire.starts_with("[card:leads_table] set_follow_ups {"), "{wire}");
        assert_eq!(CardAction::parse_wire(&wire), Some(action));
    }

    #[test]
    fn empty_context_is_an_empty_object() {
        let action = CardAction {
            card_id: "c".into(),
            action: "approve".into(),
            payload: serde_json::Value::Null,
        };
        assert_eq!(action.to_wire(), "[card:c] approve {}");
        let parsed = CardAction::parse_wire("[card:c] approve").unwrap();
        assert_eq!(parsed.payload, serde_json::json!({}));
    }

    #[test]
    fn ordinary_prompts_are_not_card_actions() {
        assert_eq!(CardAction::parse_wire("fix the tests"), None);
        assert_eq!(CardAction::parse_wire("[card:] approve"), None);
        assert_eq!(CardAction::parse_wire("[card:x]"), None);
        assert_eq!(CardAction::parse_wire("[card:x] approve not-json"), None);
    }

    #[test]
    fn serde_shape_is_camel_case() {
        let action = CardAction {
            card_id: "c".into(),
            action: "n".into(),
            payload: serde_json::json!({}),
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["cardId"], "c");
        let round: CardAction = serde_json::from_value(json).unwrap();
        assert_eq!(round, action);
    }
}
