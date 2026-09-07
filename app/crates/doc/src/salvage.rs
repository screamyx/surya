//! Field-level recovery for transcript entries the strict shape rejects.
//!
//! 2026-08-10 incident rule: a missing field must cost AT MOST what the field
//! carried - never the entry, never the transcript. Rooms merge writes from
//! every device and app version, and one bad writer blanking whole sessions
//! for every reader is what that night looked like.
//!
//! Its own file because `schema.rs` is past the 500-line rule (decision 13)
//! and must not grow.

use crate::parts::MessagePart;
use crate::schema::{DocError, DocPartJson, MessageRole, SessionMessageEntry, from_doc_part};

/// Field-level salvage for entries the strict shape rejects. Missing
/// identity/attribution fields get deterministic stand-ins (content-hashed
/// id, so repeated reads and continuation joins stay stable); parts are
/// salvaged individually - a part missing `kind` is inferred from its
/// content shape, and only truly contentless parts are dropped.
pub(crate) fn salvage_entry(
    v: serde_json::Value,
    strict_err: serde_json::Error,
) -> Result<SessionMessageEntry, DocError> {
    let Some(obj) = v.as_object() else {
        return Err(DocError::Json(strict_err));
    };
    let stable_hash = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        v.to_string().hash(&mut hasher);
        hasher.finish()
    };
    let str_field = |key: &str| obj.get(key).and_then(|x| x.as_str()).map(str::to_owned);
    let id = str_field("id").unwrap_or_else(|| format!("recovered-{stable_hash:016x}"));
    let role = obj
        .get("role")
        .and_then(|r| serde_json::from_value::<MessageRole>(r.clone()).ok())
        .unwrap_or(MessageRole::Assistant);
    let mut parts = Vec::new();
    let mut dropped_parts = 0usize;
    if let Some(raw_parts) = obj.get("parts").and_then(|p| p.as_array()) {
        for (ix, part) in raw_parts.iter().enumerate() {
            match serde_json::from_value::<DocPartJson>(part.clone()) {
                Ok(p) => parts.push(from_doc_part(p)),
                Err(_) => match salvage_part(part, &id, ix) {
                    Some(p) => parts.push(p),
                    None => dropped_parts += 1,
                },
            }
        }
    }
    tracing::warn!(
        entry = %id,
        error = %strict_err,
        salvaged_parts = parts.len(),
        dropped_parts,
        "transcript entry failed strict parse; salvaged"
    );
    Ok(SessionMessageEntry {
        id,
        role,
        parts,
        created_at: obj.get("createdAt").and_then(|x| x.as_i64()).unwrap_or(0),
        device_id: str_field("deviceId").unwrap_or_default(),
        status: obj
            .get("status")
            .and_then(|s| serde_json::from_value(s.clone()).ok()),
        continuation_of: str_field("continuationOf"),
        // A mail row that failed the strict parse still has to READ as mail:
        // dropping the source here hands it back to the owner's own bubble,
        // which is the defect this field exists to fix.
        source: obj
            .get("source")
            .and_then(|s| serde_json::from_value(s.clone()).ok()),
    })
}

/// Salvage one part whose strict `DocPartJson` parse failed: infer the kind
/// from the content shape (`text` → text part, parseable `call` → tool
/// part). `None` only when nothing renderable survives.
fn salvage_part(part: &serde_json::Value, entry_id: &str, ix: usize) -> Option<MessagePart> {
    let obj = part.as_object()?;
    let id = obj
        .get("id")
        .and_then(|x| x.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{entry_id}#recovered-{ix}"));
    if let Some(reasoning) = obj.get("reasoning").and_then(|x| x.as_str()) {
        return Some(MessagePart::Reasoning {
            id,
            text: reasoning.to_owned(),
        });
    }
    if let Some(text) = obj.get("text").and_then(|x| x.as_str()) {
        return Some(MessagePart::Text {
            id,
            text: text.to_owned(),
        });
    }
    if let Some(call) = obj
        .get("call")
        .and_then(|c| serde_json::from_value(c.clone()).ok())
    {
        return Some(MessagePart::Tool {
            id,
            call,
            is_error: obj
                .get("isError")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            resolved: obj
                .get("resolved")
                .and_then(|x| x.as_bool())
                .unwrap_or(true),
            output: obj
                .get("output")
                .and_then(|x| x.as_str())
                .map(str::to_owned),
            diff: None,
            output_ref: None,
            output_bytes: None,
            diff_ref: None,
            diff_stats: None,
            subagent_ref: None,
            subagent_status: None,
            subagent_tail: None,
        });
    }
    if let Some(message) = obj.get("message").and_then(|x| x.as_str()) {
        return Some(MessagePart::Error {
            id,
            message: message.to_owned(),
        });
    }
    None
}
