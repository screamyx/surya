//! `RegistryDoc::chat_state` and the two booleans built on it.
//!
//! Live / Tombstoned / Unknown is three answers because the chat-lifetime
//! guards added in #150, #151 and #152 read the last two opposite ways: a
//! tombstone is a refusal, absence is how a brand-new chat starts. Every
//! case here also asserts both booleans, because those are the callers and a
//! refactor of the primitive must not move either.

use chrono::{DateTime, Utc};
use serde_json::json;
use surya_doc::{ChatState, RegistryDoc, RegistryRow};

fn ts(ms: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(ms).unwrap_or(DateTime::UNIX_EPOCH)
}

#[test]
fn a_chat_nobody_has_written_is_unknown() {
    let doc = RegistryDoc::new("dev-a");

    assert_eq!(doc.chat_state("chat-1"), ChatState::Unknown);
    assert!(!doc.chat_exists("chat-1"));
    assert!(!doc.chat_tombstoned("chat-1"));
}

#[test]
fn a_claimed_chat_is_live() {
    let mut doc = RegistryDoc::new("dev-a");
    doc.claim_chat("chat-1", Some("/tmp/w"), Some("sp-1"), ts(1_000));

    assert_eq!(doc.chat_state("chat-1"), ChatState::Live);
    assert!(doc.chat_exists("chat-1"));
    assert!(!doc.chat_tombstoned("chat-1"));
}

#[test]
fn a_deleted_chat_is_tombstoned() {
    let mut doc = RegistryDoc::new("dev-a");
    doc.claim_chat("chat-1", Some("/tmp/w"), Some("sp-1"), ts(1_000));
    assert!(doc.delete_chat("chat-1").unwrap(), "the row was there");

    assert_eq!(doc.chat_state("chat-1"), ChatState::Tombstoned);
    assert!(!doc.chat_exists("chat-1"));
    assert!(doc.chat_tombstoned("chat-1"));
}

/// The distinction the guards are built on. Both non-live states answer
/// `chat_exists` false, so that boolean alone cannot tell a deleted chat from
/// one whose debounced registry write lost a race with a crash.
#[test]
fn a_tombstone_and_an_absence_differ_only_in_chat_tombstoned() {
    let mut doc = RegistryDoc::new("dev-a");
    doc.claim_chat("deleted", Some("/tmp/w"), Some("sp-1"), ts(1_000));
    doc.delete_chat("deleted").unwrap();

    assert!(!doc.chat_exists("deleted"));
    assert!(!doc.chat_exists("never-written"));

    assert!(doc.chat_tombstoned("deleted"));
    assert!(
        !doc.chat_tombstoned("never-written"),
        "absence is not deletion: a crash can predate the debounced write"
    );
}

/// Presence, not parse. A row with one unreadable field still reads Live, so
/// a live chat with bad data does not lose every write the guards cover.
/// `chat` returns `None` for the same row, which is why it cannot be the
/// primitive.
#[test]
fn a_malformed_row_is_still_live() {
    let mut doc = RegistryDoc::new("dev-a");
    let row: RegistryRow = serde_json::from_value(json!({
        "kind": "chats",
        "id": "chat-1",
        "seq": 1,
        "deleted": false,
        "fields": {"id": 5},
        "clocks": {}
    }))
    .unwrap();
    assert!(doc.apply_rows(1, vec![row]), "seq 1 is in order");

    assert!(
        doc.chat("chat-1").unwrap().is_none(),
        "the row does not parse"
    );
    assert_eq!(doc.chat_state("chat-1"), ChatState::Live);
    assert!(doc.chat_exists("chat-1"));
    assert!(!doc.chat_tombstoned("chat-1"));
}

/// A tombstone survives the overlay: the pending delete is enough, no server
/// round trip. The three guards run on the deleting device, right after the
/// cascade, so they read this state and not the acked one.
#[test]
fn the_tombstone_reads_from_pending_ops_alone() {
    let mut doc = RegistryDoc::new("dev-a");
    doc.claim_chat("chat-1", Some("/tmp/w"), Some("sp-1"), ts(1_000));
    doc.delete_chat("chat-1").unwrap();

    assert!(
        doc.pending_len() > 0,
        "nothing was acked; this is the overlay talking"
    );
    assert_eq!(doc.chat_state("chat-1"), ChatState::Tombstoned);
}
