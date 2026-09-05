//! Agent mail — decision 19: "thin it is, and cross-server mail is day one".
//!
//! surya hosts every agent, so mail needs no broker and no wake service. One
//! table holds the messages. Delivery injects the envelope into the
//! recipient's next turn: a live steerable run folds it in at its step
//! boundary, an idle agent starts a turn carrying it, an agent with no chat
//! row keeps it queued. Ack is automatic when the carrying turn completes;
//! `Mail.Ack` remains for a manual "seen".
//!
//! What is deliberately not here: the Messages pane (a later seat reads
//! `WatchMail`), device-to-device forwarding (`to_device` is honoured only
//! when it equals the local device), and auth.

mod delivery;
mod envelope;
mod ingress;
mod rpc;
mod store;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tokio::sync::watch;

pub use envelope::{MailAddress, MailMessage, MailState};
pub use ingress::{MailIngress, MailIngressPaths};
pub use rpc::MailRpc;
pub use store::{MailStore, MailStoreError};

use crate::doc_host::DocHost;
use crate::sessions::SessionsEngine;

/// How many rows the `WatchMail` feed carries. The pane shows a recent list,
/// not an archive; `Mail.List` reads the table for anything older.
const FEED_LIMIT: usize = 200;

/// One accepted send, and what it fanned out to.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailReceipt {
    /// Delivery ids, one per recipient — the agb-shaped answer to a send.
    pub ids: Vec<String>,
    /// Agents the address resolved to.
    pub recipients: Vec<String>,
}

pub struct Mail {
    inner: Arc<Inner>,
}

pub(crate) struct Inner {
    store: Arc<MailStore>,
    device_id: String,
    sessions: SessionsEngine,
    doc_host: DocHost,
    feed_tx: watch::Sender<Vec<MailMessage>>,
    /// One lock per agent, so the pump and a fresh send never dispatch the
    /// same queued rows twice — and delivering to a slow agent never blocks
    /// delivery to every other one.
    delivering: std::sync::Mutex<std::collections::HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    pump_started: AtomicBool,
}

impl Clone for Mail {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Mail {
    pub fn open(
        store_root: &std::path::Path,
        device_id: &str,
        sessions: SessionsEngine,
        doc_host: DocHost,
    ) -> Result<Self, MailStoreError> {
        let store = Arc::new(MailStore::open(store_root)?);
        let (feed_tx, _) = watch::channel(Vec::new());
        let mail = Self {
            inner: Arc::new(Inner {
                store,
                device_id: device_id.to_string(),
                sessions,
                doc_host,
                feed_tx,
                delivering: std::sync::Mutex::new(std::collections::HashMap::new()),
                pump_started: AtomicBool::new(false),
            }),
        };
        mail.publish_feed();
        Ok(mail)
    }

    pub fn device_id(&self) -> &str {
        &self.inner.device_id
    }

    /// Run one store call off the async runtime. SQLite is a blocking API; a
    /// busy WAL writer must not park a reactor thread.
    pub(crate) async fn with_store<T, F>(&self, work: F) -> Result<T, crate::EngineError>
    where
        F: FnOnce(&MailStore) -> Result<T, MailStoreError> + Send + 'static,
        T: Send + 'static,
    {
        let store = self.inner.store.clone();
        tokio::task::spawn_blocking(move || work(&store))
            .await
            .map_err(|e| crate::EngineError::Other(format!("mail store task: {e}")))?
            .map_err(Into::into)
    }

    /// The delivery lock for one agent.
    pub(crate) fn agent_lock(&self, agent: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self
            .inner
            .delivering
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        locks
            .entry(agent.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    /// Accept one send. The address resolves now; every resolved recipient gets
    /// its own row and its own delivery id. An address that names nothing known
    /// is still accepted and queued — reserve-on-spawn, so a brief that mails an
    /// agent a moment before it exists does not fail.
    pub async fn send(
        &self,
        from: &str,
        to: &str,
        body: &str,
        to_device: Option<&str>,
    ) -> Result<MailReceipt, crate::EngineError> {
        self.send_with_id(from, to, body, to_device, None, false)
            .await
    }

    /// A send the engine can attribute: `from` is the sending session's own
    /// chat id, so it rides the envelope as written.
    pub async fn send_verified(
        &self,
        from_chat: &str,
        to: &str,
        body: &str,
        to_device: Option<&str>,
    ) -> Result<MailReceipt, crate::EngineError> {
        self.send_with_id(from_chat, to, body, to_device, None, true)
            .await
    }

    /// [`Self::send`] carrying a delivery id the caller already minted. The
    /// surya-mcp seat's `send_message` hands its agent an id before the engine
    /// ever sees the record, so that id has to be the one `Mail.Ack` takes. A
    /// fan-out needs one id per row, so extra recipients get `<id>-2`, `<id>-3`
    /// and so on; the first row keeps the id as written.
    pub async fn send_with_id(
        &self,
        from: &str,
        to: &str,
        body: &str,
        to_device: Option<&str>,
        delivery_id: Option<&str>,
        verified: bool,
    ) -> Result<MailReceipt, crate::EngineError> {
        let address = MailAddress::parse(to).map_err(crate::EngineError::Other)?;
        let recipients = self.resolve(&address);
        if recipients.is_empty() {
            // Only a `#workspace` address can resolve to nothing (an unknown
            // agent id resolves to itself). Saying so beats an empty receipt
            // the sender reads as success.
            return Err(crate::EngineError::Other(format!(
                "no agents in {to}: the workspace is unknown or has no chats"
            )));
        }
        let from = envelope::attribute(from, verified);
        let to_device = to_device.unwrap_or(&self.inner.device_id).to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let mut ids = Vec::with_capacity(recipients.len());
        for (index, agent) in recipients.iter().enumerate() {
            let message = MailMessage {
                id: match (delivery_id, index) {
                    (Some(id), 0) => id.to_string(),
                    (Some(id), n) => format!("{id}-{}", n + 1),
                    (None, _) => new_mail_id(),
                },
                from: from.clone(),
                to: to.to_string(),
                to_agent: agent.clone(),
                body: body.to_string(),
                created_at: now,
                delivered_at: None,
                acked_at: None,
                from_device: self.inner.device_id.clone(),
                to_device: to_device.clone(),
                run_id: None,
            };
            let id = message.id.clone();
            let fresh = self.with_store(move |s| s.insert(&message)).await?;
            if !fresh {
                // A replayed delivery id. The row already in the table owns
                // its state; re-inserting would reset it and deliver twice.
                tracing::debug!(id = %id, "mail id already known, send is a no-op");
            }
            ids.push(id);
        }
        self.publish_feed();
        // Try immediately: a live recipient reads the mail inside its running
        // turn instead of waiting for the next status change.
        for agent in &recipients {
            if let Err(err) = self.deliver_agent(agent).await {
                tracing::warn!(agent = %agent, error = %err, "mail delivery attempt failed");
            }
        }
        Ok(MailReceipt {
            ids,
            recipients: recipients.clone(),
        })
    }

    /// Mail for one agent, oldest first. With no agent, the recent feed.
    pub async fn list(&self, agent: Option<&str>) -> Result<Vec<MailMessage>, crate::EngineError> {
        match agent {
            Some(agent) => {
                let agent = agent.to_string();
                self.with_store(move |s| s.for_agent(&agent)).await
            }
            None => Ok(self.inner.feed_tx.borrow().clone()),
        }
    }

    /// Manual "seen". Returns false when the row is unknown or already acked.
    pub async fn ack(&self, id: &str) -> Result<bool, crate::EngineError> {
        let id = id.to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let acked = self.with_store(move |s| s.mark_acked(&id, now)).await?;
        if acked {
            self.publish_feed();
        }
        Ok(acked)
    }

    pub fn watch(&self) -> watch::Receiver<Vec<MailMessage>> {
        self.inner.feed_tx.subscribe()
    }

    /// `sent=N delivered=N acked=N` — the counter triple, always as a set.
    pub async fn counts(&self) -> Result<(i64, i64, i64), crate::EngineError> {
        self.with_store(|s| s.counts()).await
    }

    /// Resolve an address to the agents that receive it.
    ///
    /// An agent address is a chat id, or a chat title used as a human alias.
    /// A `#workspace` address fans out to every non-archived chat in that
    /// space — a chat row IS an agent here, and one that has not run yet holds
    /// its mail until it does, which is what reserve-on-spawn means.
    fn resolve(&self, address: &MailAddress) -> Vec<String> {
        let chats = self
            .inner
            .doc_host
            .workspace()
            .and_then(|ws| ws.read_chats().ok())
            .unwrap_or_default();
        match address {
            MailAddress::Agent(id) => {
                if chats.iter().any(|c| &c.id == id) {
                    return vec![id.clone()];
                }
                if let Some(chat) = chats
                    .iter()
                    .find(|c| c.title.as_deref() == Some(id.as_str()))
                {
                    return vec![chat.id.clone()];
                }
                // Unknown: queue under the literal address so a later
                // reserve/create still finds it.
                vec![id.clone()]
            }
            MailAddress::Workspace(name) => {
                let spaces = self
                    .inner
                    .doc_host
                    .workspace()
                    .map(|ws| ws.watch_spaces().borrow().clone())
                    .unwrap_or_default();
                let space_ids: Vec<String> = spaces
                    .iter()
                    .filter(|s| {
                        &s.id == name
                            || s.name.as_deref() == Some(name.as_str())
                            || space_basename(&s.path) == name.as_str()
                    })
                    .map(|s| s.id.clone())
                    .collect();
                chats
                    .iter()
                    .filter(|c| !c.archived)
                    .filter(|c| {
                        c.space_id
                            .as_ref()
                            .is_some_and(|id| space_ids.iter().any(|s| s == id))
                    })
                    .map(|c| c.id.clone())
                    .collect()
            }
        }
    }

    fn publish_feed(&self) {
        let mut rows = self.inner.store.recent(FEED_LIMIT).unwrap_or_default();
        rows.reverse();
        self.inner.feed_tx.send_replace(rows);
    }
}

fn space_basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn new_mail_id() -> String {
    format!("mail-{}", uuid::Uuid::new_v4())
}
