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
pub mod origin;
pub(crate) mod preview;
mod rpc;
#[cfg(unix)]
mod socket;
mod store;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tokio::sync::watch;

pub use envelope::{MailAddress, MailMessage, MailState};
pub use ingress::{MailIngress, MailIngressPaths};
pub use origin::MessageOrigin;
pub use preview::text as preview;
pub(crate) use ingress::surya_home_dir;
pub use rpc::MailRpc;
pub use store::{MAX_DELIVERY_ATTEMPTS, MailStore, MailStoreError};

use crate::doc_host::DocHost;
use crate::sessions::SessionsEngine;

/// How many rows the `WatchMail` feed carries. The pane shows a recent list,
/// not an archive; `Mail.List` reads the table for anything older.
const FEED_LIMIT: usize = 200;

/// Delivery lock stripes. 64 is far more than the agents one engine hosts, so
/// two agents colliding is rare and costs only serialized delivery.
const DELIVERY_STRIPES: usize = 64;

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
    /// Delivery locks, striped by recipient: the pump and a fresh send never
    /// dispatch the same queued rows twice, and a slow agent blocks only the
    /// agents that share its stripe. A fixed array, not a map keyed by agent —
    /// a map grows with every address ever mailed, including the ones that
    /// never existed.
    delivering: [tokio::sync::Mutex<()>; DELIVERY_STRIPES],
    pump_started: AtomicBool,
    /// The delivery pump's task. Held so shutdown can stop it: the pump owns a
    /// `Mail` clone, and that clone reaches the sessions engine and the doc
    /// host — a pump left running keeps the whole engine graph alive after the
    /// runtime is replaced.
    pump: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
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
                delivering: std::array::from_fn(|_| tokio::sync::Mutex::new(())),
                pump_started: AtomicBool::new(false),
                pump: std::sync::Mutex::new(None),
            }),
        };
        Ok(mail)
    }

    /// Stop the delivery pump and release the engine handles it holds.
    /// Idempotent, and safe to call before the pump ever started.
    pub async fn shutdown(&self) {
        let pump = self
            .inner
            .pump
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(pump) = pump {
            pump.abort();
            let _ = pump.await;
        }
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

    /// The delivery lock stripe for one agent.
    pub(crate) fn agent_lock(&self, agent: &str) -> &tokio::sync::Mutex<()> {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        agent.hash(&mut hasher);
        &self.inner.delivering[(hasher.finish() as usize) % DELIVERY_STRIPES]
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
        self.send_with_id(from, to, body, to_device, None).await
    }

    /// A send naming the sending agent's own chat. The chat must exist in the
    /// registry — an unknown id is an error, not a queued message — and the
    /// envelope carries that chat's registry title, never the caller's string.
    ///
    /// Debt: this is as far as attribution goes for the RC. `RpcService::handle`
    /// carries no connection identity, so the engine cannot check that the
    /// caller *is* the chat it names; it can only check that the chat is real
    /// and render a name the registry owns.
    pub async fn send_from_chat(
        &self,
        from_chat: &str,
        to: &str,
        body: &str,
        to_device: Option<&str>,
    ) -> Result<MailReceipt, crate::EngineError> {
        let sender = self.sender_name(from_chat)?;
        self.send_with_id(&sender, to, body, to_device, None).await
    }

    /// The registry's name for a chat: its title, or its id when untitled.
    /// Errors when the chat is unknown.
    fn sender_name(&self, chat_id: &str) -> Result<String, crate::EngineError> {
        let chat = self
            .inner
            .doc_host
            .workspace()
            .ok_or_else(|| crate::EngineError::Other("workspace not open".into()))?
            .chat(chat_id)?
            .ok_or_else(|| {
                crate::EngineError::Other(format!("no such chat to send as: {chat_id}"))
            })?;
        let name = chat
            .title
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .unwrap_or(&chat.id);
        // A title is free text; the header is not. Fall back to the id, which
        // the registry mints and is always addressable.
        Ok(if envelope::is_addressable(name) {
            name.to_string()
        } else {
            chat.id.clone()
        })
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
    ) -> Result<MailReceipt, crate::EngineError> {
        // Checked before anything is stored: an id or a sender carrying a
        // bracket or a line break would write a second header into the
        // envelope, and every reader downstream would believe it.
        envelope::check_addressable("sender", from).map_err(crate::EngineError::Other)?;
        if let Some(id) = delivery_id {
            envelope::check_addressable("delivery id", id).map_err(crate::EngineError::Other)?;
        }
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
                from: from.to_string(),
                to: to.to_string(),
                to_agent: agent.clone(),
                body: body.to_string(),
                created_at: now,
                delivered_at: None,
                acked_at: None,
                from_device: self.inner.device_id.clone(),
                to_device: to_device.clone(),
                run_id: None,
                attempts: 0,
                failed_at: None,
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
        self.publish_feed().await;
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
            self.publish_feed().await;
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

    /// Refresh the `WatchMail` feed. Goes through [`Self::with_store`] like
    /// every other read: this runs from async paths, and SQLite blocks.
    async fn publish_feed(&self) {
        let rows = self.with_store(|s| s.recent(FEED_LIMIT)).await;
        match rows {
            Ok(mut rows) => {
                rows.reverse();
                self.inner.feed_tx.send_replace(rows);
            }
            Err(err) => tracing::warn!(error = %err, "mail feed refresh failed"),
        }
    }
}

fn space_basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn new_mail_id() -> String {
    format!("mail-{}", uuid::Uuid::new_v4())
}
