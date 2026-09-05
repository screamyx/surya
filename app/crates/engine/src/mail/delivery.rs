//! Delivery and ack.
//!
//! The rule: pending mail for an agent becomes a turn carrying one envelope
//! block per message, in send order. `dispatch` is the single call for both
//! shapes of recipient — it folds the prompt into a live steerable run's
//! mailbox at its next step boundary, and starts a fresh run otherwise — and
//! it hands back the run id.
//!
//! The ack is keyed on that run, not on the agent's status alone:
//!
//! ```text
//! queued ──claim──▶ delivering ──run id──▶ delivered ──run ends Idle──▶ acked
//!    ▲                   │                     │
//!    └───dispatch failed─┘                     └──run ends Errored / vanishes──┐
//!    ▲                                                                         │
//!    └─────────────────────────── requeued ◀───────────────────────────────────┘
//! ```
//!
//! A row is claimed before its turn is dispatched, so no second pass picks it
//! up, and its run id lands only after `dispatch` returns — until then it
//! cannot be acked, which is what stops a status tick during dispatch from
//! acking a turn the agent never saw.
//!
//! Requeueing is capped. After [`super::MAX_DELIVERY_ATTEMPTS`] failed turns
//! the row parks as failed and is never dispatched again: a harness that dies
//! on every start would otherwise redeliver the same message forever.

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
        let pump = tokio::spawn(async move {
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
                    if let Err(err) = mail.settle_agent(agent).await {
                        tracing::warn!(agent = %agent, error = %err, "mail settle failed");
                    }
                }
                if let Err(err) = mail.deliver_pending().await {
                    tracing::warn!(error = %err, "mail delivery pass failed");
                }
            }
        });
        *self
            .inner
            .pump
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(pump);
    }

    /// One delivery pass over every agent holding queued mail for this device.
    pub async fn deliver_pending(&self) -> Result<usize, EngineError> {
        let device = self.device_id().to_string();
        let agents = self
            .with_store(move |s| s.agents_with_queued(&device))
            .await?;
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
        // Per-agent: two passes must not dispatch the same rows, and a slow
        // agent must not hold up delivery to any other.
        let _guard = self.agent_lock(agent).lock().await;

        let device = self.device_id().to_string();
        let key = agent.to_string();
        let queued: Vec<_> = self
            .with_store(move |s| s.queued_for_agent(&key))
            .await?
            .into_iter()
            .filter(|m| m.to_device == device)
            .collect();
        if queued.is_empty() {
            return Ok(0);
        }
        let prompt = queued
            .iter()
            .map(|m| m.envelope_block())
            .collect::<Vec<_>>()
            .join("\n");
        let Some(mut request) = self.run_request_for(agent, &prompt) else {
            // No chat row and no prior run: the agent is not on this engine
            // yet. Mail waits — that is what reserve-on-spawn means.
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

        // Claim first: from here no other pass sees these rows, and no ack
        // pass can touch them either — their run id is still unset.
        let ids: Vec<String> = queued.iter().map(|m| m.id.clone()).collect();
        let now = chrono::Utc::now().timestamp_millis();
        let claim = ids.clone();
        self.with_store(move |s| {
            for id in &claim {
                s.mark_delivering(id, now)?;
            }
            Ok(())
        })
        .await?;

        let dispatched = self
            .inner
            .doc_host
            .dispatch_with_source_context(
                &self.inner.sessions,
                agent,
                harness,
                request,
                Some(message_id),
            )
            .await;
        let run_id = match dispatched {
            Ok(run_id) => run_id,
            Err(err) => {
                // Nobody read it. Back in the queue.
                let undo = ids.clone();
                let at = chrono::Utc::now().timestamp_millis();
                self.with_store(move |s| {
                    for id in &undo {
                        s.requeue(id, at)?;
                    }
                    Ok(())
                })
                .await?;
                self.publish_feed().await;
                return Err(err);
            }
        };
        let named = ids.clone();
        let run = run_id.clone();
        self.with_store(move |s| {
            for id in &named {
                s.set_run(id, &run)?;
            }
            Ok(())
        })
        .await?;
        self.publish_feed().await;
        tracing::info!(
            agent = %agent,
            run = %run_id,
            delivered = ids.len(),
            "mail delivered into a turn"
        );
        Ok(ids.len())
    }

    /// The agent's turn ended. Ack what that run carried, or put it back.
    ///
    /// Ack only rows whose carrying run is no longer live: a chat can settle
    /// between two of its own turns, and a row belonging to the turn still to
    /// come is not read yet. An errored run, or one that vanished without
    /// reaching Idle, requeues its rows — nobody can say the agent read them.
    pub(crate) async fn settle_agent(&self, agent: &str) -> Result<(usize, usize), EngineError> {
        let Some(session) = self.inner.sessions.session_status(agent) else {
            return Ok((0, 0));
        };
        if is_active(session.status) {
            return Ok((0, 0));
        }
        let key = agent.to_string();
        let pending = self.with_store(move |s| s.unacked_for_agent(&key)).await?;
        if pending.is_empty() {
            return Ok((0, 0));
        }
        let mut to_ack = Vec::new();
        let mut to_requeue = Vec::new();
        for message in pending {
            let Some(run_id) = message.run_id.clone() else {
                continue; // claimed, dispatch still in flight
            };
            if self.inner.sessions.run_is_live(agent, &run_id) {
                continue; // its turn has not ended
            }
            match session.status {
                SessionStatus::Idle => to_ack.push(message.id),
                _ => to_requeue.push(message.id),
            }
        }
        if to_ack.is_empty() && to_requeue.is_empty() {
            return Ok((0, 0));
        }
        let now = chrono::Utc::now().timestamp_millis();
        let acking = to_ack.clone();
        let requeuing = to_requeue.clone();
        let (acked, parked) = self
            .with_store(move |s| {
                let mut acked = 0;
                for id in &acking {
                    if s.mark_acked(id, now)? {
                        acked += 1;
                    }
                }
                let mut parked = 0;
                for id in &requeuing {
                    if s.requeue(id, now)? {
                        parked += 1;
                    }
                }
                Ok((acked, parked))
            })
            .await?;
        self.publish_feed().await;
        if acked > 0 || !to_requeue.is_empty() {
            tracing::info!(
                agent = %agent,
                acked,
                requeued = to_requeue.len() - parked,
                parked,
                status = ?session.status,
                "mail settled on turn end"
            );
        }
        Ok((acked, to_requeue.len()))
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
