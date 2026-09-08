//! Derived agent state, the needs-you inbox, and the spawned-agent tree.
//!
//! surya decision 20 makes the needs-you queue the spine of the app; decision
//! 15 nests a spawned agent under its spawner and rolls a child's needs-you
//! up every ancestor; decision 17 adds `Stopped`. This module owns all three,
//! so the UI subscribes and draws rather than deriving anything itself.
//!
//! Nothing here is persisted. Every input is a live signal — the session
//! status, the parked control requests, the subagent frames — so a restarted
//! engine rebuilds the tree from the runs it actually has. The one durable
//! neighbour is the always-allow table in [`crate::rules`].

mod derive;
mod permissions;
mod spawns;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use tokio::sync::{oneshot, watch};

use surya_proto::{
    AgentEvent, AgentStateRow, AllowRule, DoneStatus, NeedsYouItem, PermissionDecision,
    PermissionRequest, SessionStatus, UserInputQuestion, child_agent_id,
};

use crate::rules::AllowRules;
use crate::yolo::Yolo;

/// What a caller must do after opening a permission request.
#[derive(Debug)]
pub enum PermissionOpen {
    /// A rule answered before the user saw it. The caller allows the tool and
    /// writes `rule.transcript_line()` into the transcript.
    AutoAllowed(Box<AllowRule>),
    /// The chat's yolo mode answered it. Same outcome as a rule, different
    /// record: nothing was remembered, and the transcript says yolo.
    Yolo,
    /// Parked in the inbox. The caller does nothing; the resolver fires when
    /// the user answers.
    Parked,
}

/// The result of answering one parked permission.
#[derive(Debug, Clone)]
pub struct PermissionResolution {
    pub chat_id: String,
    pub agent_id: String,
    pub request: PermissionRequest,
    pub decision: PermissionDecision,
    /// The rule this answer created, when the client asked to remember it.
    pub created_rule: Option<AllowRule>,
}

/// Counters for a run of the engine. Always read as a set: a zero here only
/// means something next to what it counted out of.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StateCounters {
    /// Permission requests that reached the gate.
    pub permissions_asked: u64,
    /// Of those, answered by a rule without waking anyone.
    pub permissions_auto_allowed: u64,
    /// Of those, answered by a person.
    pub permissions_answered: u64,
    /// Question sets that reached the inbox.
    pub questions_asked: u64,
    /// Subagent children registered.
    pub children_registered: u64,
}

struct PendingPermission {
    agent_id: String,
    chat_id: String,
    cwd: String,
    request: PermissionRequest,
    responder: oneshot::Sender<PermissionDecision>,
    created_at: DateTime<Utc>,
}

struct PendingQuestions {
    agent_id: String,
    chat_id: String,
    questions: Vec<UserInputQuestion>,
    created_at: DateTime<Utc>,
}

/// Why a run ended badly (decision 17's `failure` record).
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    pub reason: String,
    pub detail: String,
    pub retryable: bool,
    pub at: DateTime<Utc>,
}

#[derive(Default)]
pub(super) struct Node {
    parent_id: Option<String>,
    chat_id: String,
    label: Option<String>,
    session: Option<SessionStatus>,
    /// Ended by the user's interrupt rather than by a crash.
    stopped: bool,
    failure: Option<Failure>,
    /// Finished a run and nobody has opened it since.
    finished_unseen: bool,
    updated_at: Option<DateTime<Utc>>,
}

pub(super) struct Inner {
    rules: AllowRules,
    yolo: Yolo,
    nodes: Mutex<HashMap<String, Node>>,
    permissions: Mutex<HashMap<String, PendingPermission>>,
    questions: Mutex<HashMap<String, PendingQuestions>>,
    counters: Mutex<StateCounters>,
    states_tx: watch::Sender<Vec<AgentStateRow>>,
    needs_you_tx: watch::Sender<Vec<NeedsYouItem>>,
}

/// The derived-state service. Cloneable handle over one shared store.
#[derive(Clone)]
pub struct AgentStates {
    pub(super) inner: Arc<Inner>,
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl AgentStates {
    pub fn new(rules: AllowRules) -> Self {
        let (states_tx, _) = watch::channel(Vec::new());
        let (needs_you_tx, _) = watch::channel(Vec::new());
        Self {
            inner: Arc::new(Inner {
                rules,
                yolo: Yolo::new(),
                nodes: Mutex::new(HashMap::new()),
                permissions: Mutex::new(HashMap::new()),
                questions: Mutex::new(HashMap::new()),
                counters: Mutex::new(StateCounters::default()),
                states_tx,
                needs_you_tx,
            }),
        }
    }

    pub fn rules(&self) -> &AllowRules {
        &self.inner.rules
    }

    /// The live yolo set (see [`crate::yolo`]). Flipping it is
    /// [`crate::sessions::SessionsEngine::set_auto_approve`]'s job: a flip
    /// has to answer what is already parked, and only the sessions engine can
    /// reach the chat's event stream to say so.
    pub fn yolo(&self) -> &Yolo {
        &self.inner.yolo
    }

    /// Every agent row, worst state first. Re-sent on every change.
    pub fn watch_states(&self) -> watch::Receiver<Vec<AgentStateRow>> {
        self.inner.states_tx.subscribe()
    }

    /// Every pending permission, question and failure across every chat,
    /// newest first. Re-sent on every change.
    pub fn watch_needs_you(&self) -> watch::Receiver<Vec<NeedsYouItem>> {
        self.inner.needs_you_tx.subscribe()
    }

    pub fn states(&self) -> Vec<AgentStateRow> {
        self.inner.states_tx.borrow().clone()
    }

    pub fn needs_you(&self) -> Vec<NeedsYouItem> {
        self.inner.needs_you_tx.borrow().clone()
    }

    pub fn counters(&self) -> StateCounters {
        *lock(&self.inner.counters)
    }

    // ── questions ──────────────────────────────────────────────────────────

    /// An `AskUserQuestion` set reached the user.
    pub fn open_questions(
        &self,
        chat_id: &str,
        agent_id: &str,
        request_id: &str,
        questions: Vec<UserInputQuestion>,
    ) {
        lock(&self.inner.counters).questions_asked += 1;
        lock(&self.inner.questions).insert(
            request_id.to_string(),
            PendingQuestions {
                agent_id: agent_id.to_string(),
                chat_id: chat_id.to_string(),
                questions,
                created_at: Utc::now(),
            },
        );
        self.touch(agent_id, chat_id);
        self.recompute();
    }

    pub fn close_questions(&self, request_id: &str) {
        if lock(&self.inner.questions).remove(request_id).is_some() {
            self.recompute();
        }
    }

    // ── sessions, failures, seen ───────────────────────────────────────────

    /// The chat's live run status changed.
    pub fn note_session(&self, chat_id: &str, status: SessionStatus) {
        {
            let mut nodes = lock(&self.inner.nodes);
            let node = Self::entry(&mut nodes, chat_id, chat_id);
            node.session = Some(status);
            node.updated_at = Some(Utc::now());
            if status == SessionStatus::Working {
                // A fresh turn clears whatever the last one ended as.
                node.failure = None;
                node.stopped = false;
                node.finished_unseen = false;
            }
        }
        self.recompute();
    }

    /// A run ended. `status` decides between Done, Stopped and Failed.
    pub fn note_run_end(&self, chat_id: &str, status: DoneStatus, detail: &str) {
        {
            let mut nodes = lock(&self.inner.nodes);
            let node = Self::entry(&mut nodes, chat_id, chat_id);
            node.updated_at = Some(Utc::now());
            // The run is over, so the row must stop reading Working even if
            // the session-status transition has not landed yet. Without this
            // a finished chat shows a spinner for as long as that lag lasts.
            node.session = Some(SessionStatus::Idle);
            match status {
                DoneStatus::Completed => {
                    node.finished_unseen = true;
                    node.stopped = false;
                    node.failure = None;
                }
                DoneStatus::Interrupted => {
                    node.stopped = true;
                    node.failure = None;
                }
                DoneStatus::Errored => {
                    node.stopped = false;
                    node.failure = Some(Failure {
                        reason: "The run failed".into(),
                        detail: detail.to_string(),
                        retryable: true,
                        at: Utc::now(),
                    });
                }
            }
        }
        self.recompute();
    }

    /// The user opened this agent, so Done stops being unread.
    pub fn mark_seen(&self, agent_id: &str) {
        let changed = {
            let mut nodes = lock(&self.inner.nodes);
            match nodes.get_mut(agent_id) {
                Some(node) if node.finished_unseen => {
                    node.finished_unseen = false;
                    true
                }
                _ => false,
            }
        };
        if changed {
            self.recompute();
        }
    }

    /// Forget a chat and everything spawned under it.
    /// Returns the permission request ids that were still parked, so the
    /// caller can say out loud that they were refused. Dropping a responder
    /// silently is what the gate's fail-closed path turns into a Deny — the
    /// tool does not run, and the transcript should not pretend otherwise.
    pub fn drop_chat(&self, chat_id: &str) -> Vec<String> {
        {
            let mut nodes = lock(&self.inner.nodes);
            nodes.retain(|id, node| id != chat_id && node.chat_id != chat_id);
        }
        let dropped: Vec<String> = {
            let mut permissions = lock(&self.inner.permissions);
            let dropped = permissions
                .iter()
                .filter(|(_, p)| p.chat_id == chat_id)
                .map(|(id, _)| id.clone())
                .collect();
            // Dropping the entry drops its responder; the gate reads the
            // closed channel as Deny.
            permissions.retain(|_, p| p.chat_id != chat_id);
            dropped
        };
        lock(&self.inner.questions).retain(|_, q| q.chat_id != chat_id);
        self.recompute();
        dropped
    }

    // ── the spawned-agent tree ─────────────────────────────────────────────

    /// Register (or refresh) a child under its spawner. Returns the child's
    /// agent id. Idempotent: the id is derived, so a re-delivered frame
    /// updates the same row (decision 15).
    pub fn note_subagent(
        &self,
        parent_chat_id: &str,
        tool_use_id: &str,
        label: Option<&str>,
    ) -> String {
        let id = child_agent_id(parent_chat_id, tool_use_id);
        let fresh = {
            let mut nodes = lock(&self.inner.nodes);
            // Make sure the parent row exists, so a child can never dangle.
            Self::entry(&mut nodes, parent_chat_id, parent_chat_id);
            let fresh = !nodes.contains_key(&id);
            let node = Self::entry(&mut nodes, &id, parent_chat_id);
            node.parent_id = Some(parent_chat_id.to_string());
            if let Some(label) = label.filter(|l| !l.trim().is_empty()) {
                node.label = Some(label.trim().to_string());
            }
            node.updated_at = Some(Utc::now());
            if fresh {
                node.session = Some(SessionStatus::Working);
            }
            fresh
        };
        if fresh {
            lock(&self.inner.counters).children_registered += 1;
        }
        self.recompute();
        id
    }

    /// Fold one subagent event into the child's own state. The child's
    /// question and permission asks are opened against the CHILD's agent id
    /// but the PARENT's chat id, so answering still reaches the right run.
    pub fn note_subagent_event(&self, parent_chat_id: &str, tool_use_id: &str, event: &AgentEvent) {
        let id = self.note_subagent(parent_chat_id, tool_use_id, None);
        match event {
            AgentEvent::InputRequested {
                request_id,
                questions,
            } => self.open_questions(parent_chat_id, &id, request_id, questions.clone()),
            AgentEvent::InputResolved { request_id } => self.close_questions(request_id),
            AgentEvent::Done { status, error, .. } => {
                let mut nodes = lock(&self.inner.nodes);
                let node = Self::entry(&mut nodes, &id, parent_chat_id);
                node.updated_at = Some(Utc::now());
                node.session = Some(SessionStatus::Idle);
                match status {
                    DoneStatus::Completed => node.finished_unseen = true,
                    DoneStatus::Interrupted => node.stopped = true,
                    DoneStatus::Errored => {
                        node.failure = Some(Failure {
                            reason: "The subagent failed".into(),
                            detail: error.clone().unwrap_or_default(),
                            retryable: true,
                            at: Utc::now(),
                        })
                    }
                }
                drop(nodes);
                self.recompute();
            }
            AgentEvent::Error { message } => {
                let mut nodes = lock(&self.inner.nodes);
                let node = Self::entry(&mut nodes, &id, parent_chat_id);
                node.failure = Some(Failure {
                    reason: "The subagent failed".into(),
                    detail: message.clone(),
                    retryable: true,
                    at: Utc::now(),
                });
                node.updated_at = Some(Utc::now());
                drop(nodes);
                self.recompute();
            }
            _ => {}
        }
    }

    // ── derivation ─────────────────────────────────────────────────────────

    fn entry<'a>(
        nodes: &'a mut HashMap<String, Node>,
        id: &str,
        chat_id: &str,
    ) -> &'a mut Node {
        nodes.entry(id.to_string()).or_insert_with(|| Node {
            chat_id: chat_id.to_string(),
            ..Node::default()
        })
    }

    fn touch(&self, agent_id: &str, chat_id: &str) {
        let mut nodes = lock(&self.inner.nodes);
        Self::entry(&mut nodes, agent_id, chat_id).updated_at = Some(Utc::now());
    }

    /// Rebuild both watch snapshots from the current signals.
    ///
    /// `send_replace`, never `send`: a `watch::Sender::send` with no live
    /// receiver returns an error AND leaves the old value in place, so the
    /// snapshot the next subscriber reads would be stale. The engine's own
    /// session watch takes the same care.
    fn recompute(&self) {
        let needs_you = self.build_needs_you();
        let states = self.build_states(&needs_you);
        self.inner.states_tx.send_replace(states);
        self.inner.needs_you_tx.send_replace(needs_you);
    }
}
