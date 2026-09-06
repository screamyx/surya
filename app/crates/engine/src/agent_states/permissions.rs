//! The permission half of [`super::AgentStates`]: the gate one request
//! passes through, the answer a person gives it, and the flush yolo mode
//! performs on a running session.
//!
//! Split out of `agent_states.rs` to keep both files under the 500-line
//! build rule (decision 13); the state itself still lives in
//! [`super::Inner`], which this module reaches as a child of its owner.

use chrono::Utc;
use tokio::sync::oneshot;

use surya_proto::{PermissionDecision, PermissionRequest, RememberRule};

use super::{AgentStates, PendingPermission, PermissionOpen, PermissionResolution, lock};
use crate::EngineError;
use crate::rules::AllowRules;

impl AgentStates {
    /// Gate one permission request. Either a rule answers it now, or it is
    /// parked in the inbox until [`Self::resolve_permission`].
    pub fn open_permission(
        &self,
        chat_id: &str,
        agent_id: &str,
        cwd: &str,
        request: PermissionRequest,
        responder: oneshot::Sender<PermissionDecision>,
    ) -> PermissionOpen {
        lock(&self.inner.counters).permissions_asked += 1;
        if let Some(rule) = self.inner.rules.matching(&request, cwd) {
            lock(&self.inner.counters).permissions_auto_allowed += 1;
            let _ = responder.send(PermissionDecision::Allow);
            return PermissionOpen::AutoAllowed(Box::new(rule));
        }
        // A rule is checked first so its name still reaches the transcript
        // when both would allow: a rule is a thing the user wrote down, and
        // it is the more specific answer to "why did this run".
        if self.inner.yolo.is_on(chat_id) {
            lock(&self.inner.counters).permissions_auto_allowed += 1;
            let _ = responder.send(PermissionDecision::Allow);
            return PermissionOpen::Yolo;
        }
        lock(&self.inner.permissions).insert(
            request.request_id.clone(),
            PendingPermission {
                agent_id: agent_id.to_string(),
                chat_id: chat_id.to_string(),
                cwd: cwd.to_string(),
                request,
                responder,
                created_at: Utc::now(),
            },
        );
        self.touch(agent_id, chat_id);
        self.recompute();
        PermissionOpen::Parked
    }

    /// Answer a parked permission, optionally turning the answer into a rule.
    pub fn resolve_permission(
        &self,
        request_id: &str,
        decision: PermissionDecision,
        remember: Option<&RememberRule>,
    ) -> Result<PermissionResolution, EngineError> {
        // Build the rule BEFORE taking the request out of the inbox. A
        // refused rule must leave the card exactly where it was: the user
        // has not answered yet, and dropping the request here would drop its
        // responder, which the gate reads as Deny — a decision nobody made.
        let created_rule = {
            let peek = lock(&self.inner.permissions);
            let pending = peek
                .get(request_id)
                .ok_or_else(|| EngineError::Other(format!("no pending permission {request_id}")))?;
            // The rule is created only for an Allow: "always allow" is the
            // only shape decision 20 gives the card, and a remembered Deny
            // would be a block-list nobody asked for.
            match (remember, decision) {
                (Some(remember), PermissionDecision::Allow) => {
                    let rule = AllowRules::from_remember(remember, &pending.request, &pending.cwd)
                        .map_err(|err| EngineError::Other(err.to_string()))?;
                    drop(peek);
                    Some(
                        self.inner
                            .rules
                            .add(rule)
                            .map_err(|err| EngineError::Other(err.to_string()))?,
                    )
                }
                _ => None,
            }
        };
        let pending = lock(&self.inner.permissions)
            .remove(request_id)
            .ok_or_else(|| EngineError::Other(format!("no pending permission {request_id}")))?;
        lock(&self.inner.counters).permissions_answered += 1;
        let _ = pending.responder.send(decision);
        self.recompute();
        Ok(PermissionResolution {
            chat_id: pending.chat_id,
            agent_id: pending.agent_id,
            request: pending.request,
            decision,
            created_rule,
        })
    }


    /// Answer every permission this chat still has parked with Allow, and
    /// return them so the caller can write each one into the transcript.
    ///
    /// This is what makes yolo work on a RUNNING session: the tool that is
    /// blocked right now is unblocked by the same flip that covers the ones
    /// after it. Shaped like [`Self::drop_chat`], with the opposite answer —
    /// there the responder is dropped and the gate reads Deny; here it is
    /// sent Allow.
    pub fn allow_pending_for_chat(&self, chat_id: &str) -> Vec<PermissionRequest> {
        let flushed: Vec<PermissionRequest> = {
            let mut permissions = lock(&self.inner.permissions);
            let ids: Vec<String> = permissions
                .iter()
                .filter(|(_, p)| p.chat_id == chat_id)
                .map(|(id, _)| id.clone())
                .collect();
            ids.into_iter()
                .filter_map(|id| permissions.remove(&id))
                .map(|pending| {
                    let _ = pending.responder.send(PermissionDecision::Allow);
                    pending.request
                })
                .collect()
        };
        if !flushed.is_empty() {
            // Counted as auto-allowed, not as answered: nobody answered
            // these, the mode did.
            lock(&self.inner.counters).permissions_auto_allowed += flushed.len() as u64;
            self.recompute();
        }
        flushed
    }
}
