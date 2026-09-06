//! Yolo mode: the set of chats whose permission requests the engine answers
//! itself.
//!
//! The durable copy of this flag is `ChatConfig.auto_approve` on the chat row
//! (persisted, synced, readable by every device). This module holds the LIVE
//! copy on the device that runs the agents, because two things need it inside
//! a single run and neither can wait for a doc round trip:
//!
//! - the permission gate, which has a blocked tool and a responder in hand
//!   ([`crate::agent_states::AgentStates::open_permission`]);
//! - the flip itself, which has to reach a session that is already running
//!   (the owner's request: "on new and running sessions").
//!
//! Nothing here is persisted. The row is the truth; this set is re-seeded
//! from it at dispatch and by the engine's chat watch, so a restarted engine
//! never has to remember anything.
//!
//! Scope is one chat, not a rule: turning yolo on answers this chat's
//! requests and writes nothing into the always-allow table
//! ([`crate::rules`]). A rule outlives the chat and applies everywhere it
//! matches; yolo is a mode the user can take back in one click.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use tokio::sync::watch;

use surya_proto::Chat;

use crate::agent_states::lock;
use crate::sessions::SessionsEngine;

/// The chats currently running without permission prompts.
#[derive(Default)]
pub struct Yolo {
    chats: Mutex<HashSet<String>>,
}

impl Yolo {
    pub fn new() -> Self {
        Self::default()
    }

    /// Turn yolo on or off for one chat. Returns whether this CHANGED it, so
    /// callers that fan out (the RPC, the chat watch) can skip the work when
    /// two feeds report the same flip.
    pub fn set(&self, chat_id: &str, on: bool) -> bool {
        let mut chats = lock(&self.chats);
        if on {
            chats.insert(chat_id.to_string())
        } else {
            chats.remove(chat_id)
        }
    }

    pub fn is_on(&self, chat_id: &str) -> bool {
        lock(&self.chats).contains(chat_id)
    }

    /// The chat is gone. Forgetting is not the same as switching off: there
    /// is nothing left to answer for, and a chat id could be reused by an
    /// import.
    pub fn forget(&self, chat_id: &str) {
        lock(&self.chats).remove(chat_id);
    }
}

/// Follow the chat rows and keep the live set equal to what they say.
///
/// This is the path that does not need an RPC: a flip made on another device
/// arrives through the workspace doc, and an engine that just started reads
/// the current rows on its first pass. The RPC
/// ([`surya_rpc::methods::SET_AUTO_APPROVE`]) exists beside it only to make
/// the local flip immediate.
pub fn spawn_row_watch(
    sessions: SessionsEngine,
    mut chats: watch::Receiver<Vec<Chat>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut seen: HashMap<String, bool> = HashMap::new();
        apply_rows(&sessions, &chats.borrow_and_update().clone(), &mut seen);
        while chats.changed().await.is_ok() {
            let rows = chats.borrow_and_update().clone();
            apply_rows(&sessions, &rows, &mut seen);
        }
    })
}

/// Apply the rows whose flag CHANGED since the last pass.
///
/// Changed, not "differs from the live set": the live set is also written by
/// [`surya_rpc::methods::SET_AUTO_APPROVE`], which is the same flip arriving
/// a moment earlier. Reconciling against the live set on every unrelated row
/// change would let a stale row undo a flip the user just made.
fn apply_rows(sessions: &SessionsEngine, chats: &[Chat], seen: &mut HashMap<String, bool>) {
    for chat in chats {
        let wanted = chat
            .config
            .as_ref()
            .is_some_and(|config| config.auto_approve);
        let first_sighting = match seen.insert(chat.id.clone(), wanted) {
            Some(before) if before == wanted => continue, // nothing changed
            Some(_) => false,
            None => true,
        };
        // A row seen for the FIRST time saying "off" is not an instruction:
        // off is what the live set already is, and a row that has not caught
        // up with a flip the user just made would otherwise undo it. Only an
        // "on", or a row that CHANGED to off, is news.
        if first_sighting && !wanted {
            continue;
        }
        if wanted != sessions.auto_approve(&chat.id) {
            sessions.set_auto_approve(&chat.id, wanted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Yolo;

    #[test]
    fn set_reports_only_real_changes() {
        let yolo = Yolo::new();
        assert!(!yolo.is_on("a"));
        assert!(yolo.set("a", true));
        assert!(!yolo.set("a", true), "already on is not a change");
        assert!(yolo.is_on("a"));
        assert!(yolo.set("a", false));
        assert!(!yolo.set("a", false), "already off is not a change");
        assert!(!yolo.is_on("a"));
    }

    #[test]
    fn chats_do_not_share_the_flag() {
        let yolo = Yolo::new();
        yolo.set("a", true);
        assert!(yolo.is_on("a"));
        assert!(!yolo.is_on("b"));
    }

    #[test]
    fn forget_clears_the_chat() {
        let yolo = Yolo::new();
        yolo.set("a", true);
        yolo.forget("a");
        assert!(!yolo.is_on("a"));
    }
}
