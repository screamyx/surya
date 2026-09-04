//! Delivery and ack.
//!
//! One rule, two hooks. The rule: pending mail for an agent becomes a turn
//! carrying `[MAIL <id> from <sender>] <body>`, one line per message, in send
//! order. The hooks: a send tries immediately, and the session-status watch
//! retries whenever an agent settles. `dispatch` is the single call for both
//! shapes of recipient — it folds the prompt into a live steerable run's
//! mailbox at its next step boundary, and starts a fresh run otherwise — and
//! it hands back the run id the ack is keyed on.
//!
//! Ack: an agent whose status is settled has no turn in flight, so every
//! delivered-but-unacked row it holds was carried by a turn that has ended.

use std::sync::atomic::Ordering;

use zeron_proto::SessionStatus;

use super::Mail;
use crate::EngineError;

impl Mail {
    /// Start the status-driven pump. Idempotent: only the first call spawns.
    pub fn start_pump(&self) {
        if self.inner.pump_started.swap(true, Ordering::SeqCst) {
            return;
        }
        let mail = self.clone();
        let mut statuses = self.inner.sessions.watch_sessions();
        tokio::spawn(async move {
            loop {
                if statuses.changed().await.is_err() {
                    break;
                }
                let settled: Vec<String> = statuses
                    .borrow()
                    .iter()
                    .filter(|s| !is_active(s.status))
                    .map(|s| s.chat_id.clone())
                    .collect();
                for agent in &settled {
                    if let Err(err) = mail.ack_settled(agent) {
                        tracing::warn!(agent = %agent, error = %err, "mail auto-ack failed");
                    }
                }
                if let Err(err) = mail.deliver_pending().await {
                    tracing::warn!(error = %err, "mail delivery pass failed");
                }
            }
        });
    }

    /// One delivery pass over every agent holding queued mail for this device.
    pub async fn deliver_pending(&self) -> Result<usize, EngineError> {
        let agents = self.store().agents_with_queued(self.device_id())?;
        let mut delivered = 0;
        for agent in agents {
            delivered += self.deliver_agent(&agent).await?;
        }
        Ok(delivered)
    }

    /// Deliver everything queued for one agent as a single turn. Returns the
    /// number of messages that went out; 0 means they stay queued (no chat row
    /// yet, or the row names another device).
    pub(crate) async fn deliver_agent(&self, agent: &str) -> Result<usize, EngineError> {
        // One delivery at a time: a send and the pump must not dispatch the
        // same queued rows as two turns.
        let _guard = self.inner.delivering.lock().await;
        let queued: Vec<_> = self
            .store()
            .queued_for_agent(agent)?
            .into_iter()
            .filter(|m| m.to_device == self.device_id())
            .collect();
        if queued.is_empty() {
            return Ok(0);
        }
        let prompt = queued
            .iter()
            .map(|m| m.envelope_line())
            .collect::<Vec<_>>()
            .join("\n");
        let Some(mut request) = self.run_request_for(agent, &prompt) else {
            // No chat row and no prior run: the agent is not on this engine
            // yet. Mail waits — that is the whole point of reserve-on-spawn.
            tracing::debug!(agent = %agent, queued = queued.len(), "mail held: agent not runnable here");
            return Ok(0);
        };
        request.prompt = prompt;
        // A mail turn never resumes on the caller's behalf; the engine
        // re-derives the harness session inside dispatch.
        request.resume = None;
        request.attachments = Vec::new();
        let harness = self.inner.doc_host.harness_for_request(agent, &request);
        let message_id = format!("mailmsg-{}", queued[0].id);
        let run_id = self
            .inner
            .doc_host
            .dispatch_with_source_context(
                &self.inner.sessions,
                agent,
                harness,
                request,
                Some(message_id),
            )
            .await?;
        let now = chrono::Utc::now().timestamp_millis();
        for message in &queued {
            self.store().mark_delivered(&message.id, now, Some(&run_id))?;
        }
        self.publish_feed();
        tracing::info!(
            agent = %agent,
            run = %run_id,
            delivered = queued.len(),
            "mail delivered into a turn"
        );
        Ok(queued.len())
    }

    /// Ack every delivered-but-unacked row for an agent that is not running.
    pub(crate) fn ack_settled(&self, agent: &str) -> Result<usize, EngineError> {
        if self
            .inner
            .sessions
            .session_status(agent)
            .is_some_and(|s| is_active(s.status))
        {
            return Ok(0);
        }
        let pending = self.store().unacked_for_agent(agent)?;
        if pending.is_empty() {
            return Ok(0);
        }
        let now = chrono::Utc::now().timestamp_millis();
        let mut acked = 0;
        for message in &pending {
            if self.store().mark_acked(&message.id, now)? {
                acked += 1;
            }
        }
        if acked > 0 {
            self.publish_feed();
            tracing::info!(agent = %agent, acked, "mail acked on turn completion");
        }
        Ok(acked)
    }

    /// The run configuration a mail turn borrows: the agent's last run if it
    /// has one, else its chat row's stored config.
    fn run_request_for(&self, agent: &str, prompt: &str) -> Option<zeron_proto::RunRequest> {
        self.inner
            .sessions
            .last_request(agent)
            .or_else(|| self.inner.doc_host.request_from_chat_row(agent, prompt))
    }
}

fn is_active(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Working | SessionStatus::AwaitingInput
    )
}
