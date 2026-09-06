//! Turning the live signals into rows: the inbox, the five states, and the
//! roll-up a parent shows for its descendants (surya decisions 15, 17, 20).
//!
//! Split out of `agent_states.rs` to keep both files under the 500-line rule
//! (decision 13). Everything here is a pure function of the store's state —
//! `AgentStates` owns the mutation, this owns the derivation.

use std::collections::HashMap;

use chrono::Utc;
use surya_proto::{AgentState, AgentStateRow, NeedsYouItem, NeedsYouKind, SessionStatus};

use super::{AgentStates, Node, lock};

impl AgentStates {
    pub(super) fn build_needs_you(&self) -> Vec<NeedsYouItem> {
        let mut items: Vec<NeedsYouItem> = Vec::new();
        for pending in lock(&self.inner.permissions).values() {
            items.push(NeedsYouItem {
                id: pending.request.request_id.clone(),
                chat_id: pending.chat_id.clone(),
                agent_id: pending.agent_id.clone(),
                kind: NeedsYouKind::Permission,
                title: format!("Allow {}?", pending.request.tool_name),
                prompt: pending.request.command.clone(),
                options: Vec::new(),
                multi_select: false,
                tool_name: Some(pending.request.tool_name.clone()),
                tool_command: Some(pending.request.command.clone()),
                retryable: false,
                created_at: pending.created_at,
            });
        }
        for (request_id, pending) in lock(&self.inner.questions).iter() {
            for question in &pending.questions {
                items.push(NeedsYouItem {
                    // The question id keys the answer; the request id keys the
                    // parked resolver. Both are needed, so the item carries
                    // the pair.
                    id: format!("{request_id}:{}", question.id),
                    chat_id: pending.chat_id.clone(),
                    agent_id: pending.agent_id.clone(),
                    kind: NeedsYouKind::Question,
                    title: question.header.clone(),
                    prompt: question.question.clone(),
                    options: question.options.clone(),
                    multi_select: question.multi_select,
                    tool_name: None,
                    tool_command: None,
                    retryable: false,
                    created_at: pending.created_at,
                });
            }
        }
        // Decision 17: a crashed run AND a run you stopped both sit in the
        // inbox with a reason and a Retry. They differ only in the row state
        // the rail draws, which `derive_state` decides.
        for (id, node) in lock(&self.inner.nodes).iter() {
            let Some(failure) = &node.failure else {
                continue;
            };
            items.push(NeedsYouItem {
                id: format!("{id}:failure"),
                chat_id: node.chat_id.clone(),
                agent_id: id.clone(),
                kind: NeedsYouKind::Failed,
                title: failure.reason.clone(),
                prompt: failure.detail.clone(),
                options: Vec::new(),
                multi_select: false,
                tool_name: None,
                tool_command: None,
                retryable: failure.retryable,
                created_at: failure.at,
            });
        }
        // Newest first, with the id as the tiebreak so the order is stable
        // across recomputes that land in the same millisecond.
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));
        items
    }

    pub(super) fn build_states(&self, needs_you: &[NeedsYouItem]) -> Vec<AgentStateRow> {
        let nodes = lock(&self.inner.nodes);
        let mut own: HashMap<String, AgentState> = HashMap::new();
        for (id, node) in nodes.iter() {
            let kind = needs_you
                .iter()
                .filter(|item| &item.agent_id == id)
                .map(|item| item.kind)
                .min_by_key(|kind| match kind {
                    // A blocked tool is the most urgent of the three: the run
                    // is frozen mid-turn until it is answered.
                    NeedsYouKind::Permission => 0,
                    NeedsYouKind::Question => 1,
                    NeedsYouKind::Failed => 2,
                });
            own.insert(id.clone(), derive_state(node, kind));
        }
        // Roll a child's state up every ancestor (decision 15).
        let mut rows: Vec<AgentStateRow> = Vec::with_capacity(nodes.len());
        for (id, node) in nodes.iter() {
            let mine = own.get(id).copied().unwrap_or(AgentState::Idle);
            let mut rolled = mine;
            let mut needs_children = 0u32;
            for (other_id, other) in nodes.iter() {
                if other_id == id || !is_descendant(&nodes, other, id) {
                    continue;
                }
                let child_state = own.get(other_id).copied().unwrap_or(AgentState::Idle);
                rolled = rolled.worse(child_state);
                if child_state.needs_you() {
                    needs_children += 1;
                }
            }
            rows.push(AgentStateRow {
                id: id.clone(),
                parent_id: node.parent_id.clone(),
                chat_id: node.chat_id.clone(),
                state: mine,
                rolled_up: rolled,
                needs_you_children: needs_children,
                label: node.label.clone(),
                updated_at: node.updated_at.unwrap_or_else(Utc::now),
            });
        }
        // Rank first (decision 15: needs you, working, done, idle), then
        // most recent, then id so the order never flickers between
        // recomputes. A parent and its child share a rank once the child
        // rolls up, so this is a rank HINT for top-level rows — the tree
        // shape lives in `parent_id`, and the rail nests from that.
        rows.sort_by(|a, b| {
            a.rolled_up
                .sort_rank()
                .cmp(&b.rolled_up.sort_rank())
                .then(b.updated_at.cmp(&a.updated_at))
                .then(a.id.cmp(&b.id))
        });
        rows
    }
}

/// One node's own state, before any child rolls into it. `needs` is the most
/// urgent kind this node has in the inbox, if any.
fn derive_state(node: &Node, needs: Option<NeedsYouKind>) -> AgentState {
    match needs {
        Some(NeedsYouKind::Permission) => {
            return AgentState::NeedsYou {
                kind: NeedsYouKind::Permission,
            };
        }
        Some(NeedsYouKind::Question) => {
            return AgentState::NeedsYou {
                kind: NeedsYouKind::Question,
            };
        }
        Some(NeedsYouKind::Failed) => {
            return AgentState::NeedsYou {
                kind: NeedsYouKind::Failed,
            };
        }
        None => {}
    }
    // A run the user interrupted wants Retry or Give up, not an answer — so
    // it reads Stopped rather than NeedsYou, and rolls up all the same.
    if node.stopped {
        return AgentState::Stopped;
    }
    match node.session {
        Some(SessionStatus::Working) => AgentState::Working,
        Some(SessionStatus::AwaitingInput) => AgentState::NeedsYou {
            // AwaitingInput with nothing parked means the ask was resolved a
            // beat before the status caught up. Question is the honest label.
            kind: NeedsYouKind::Question,
        },
        _ if node.finished_unseen => AgentState::Done,
        _ => AgentState::Idle,
    }
}

/// Is `node` a descendant of `ancestor_id`? Walks parents with a depth cap so
/// a cycle from a malformed frame can never hang the recompute.
fn is_descendant(nodes: &HashMap<String, Node>, node: &Node, ancestor_id: &str) -> bool {
    let mut parent = node.parent_id.as_deref();
    for _ in 0..64 {
        match parent {
            None => return false,
            Some(id) if id == ancestor_id => return true,
            Some(id) => parent = nodes.get(id).and_then(|n| n.parent_id.as_deref()),
        }
    }
    false
}
