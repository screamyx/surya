//! The mail table. One SQLite file, `mail.sqlite3`, a sibling of the docs
//! store inside the same profile root, so mail inherits the profile boundary
//! (local mail never appears in a synced profile) without touching the docs
//! schema.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension, params};

use super::envelope::MailMessage;

#[derive(Debug, thiserror::Error)]
pub enum MailStoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub struct MailStore {
    conn: Mutex<Connection>,
}

impl MailStore {
    /// Open (creating directory, database, and schema as needed).
    pub fn open(store_root: impl AsRef<Path>) -> Result<Self, MailStoreError> {
        let root = store_root.as_ref();
        std::fs::create_dir_all(root)?;
        let conn = Connection::open(root.join("mail.sqlite3"))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// In-memory store for tests.
    #[cfg(test)]
    pub fn open_memory() -> Result<Self, MailStoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn insert(&self, message: &MailMessage) -> Result<(), MailStoreError> {
        self.conn().execute(
            "INSERT OR REPLACE INTO mail
               (id, sender, recipient, to_agent, body, created_at,
                delivered_at, acked_at, from_device, to_device, run_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                message.id,
                message.from,
                message.to,
                message.to_agent,
                message.body,
                message.created_at,
                message.delivered_at,
                message.acked_at,
                message.from_device,
                message.to_device,
                message.run_id,
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<MailMessage>, MailStoreError> {
        Ok(self
            .conn()
            .query_row(&format!("{SELECT} WHERE id = ?1"), params![id], row_to_mail)
            .optional()?)
    }

    /// Everything for one agent, oldest first.
    pub fn for_agent(&self, agent: &str) -> Result<Vec<MailMessage>, MailStoreError> {
        self.query(
            &format!("{SELECT} WHERE to_agent = ?1 ORDER BY created_at, id"),
            params![agent],
        )
    }

    /// Undelivered mail for one agent, oldest first — what the next turn carries.
    pub fn queued_for_agent(&self, agent: &str) -> Result<Vec<MailMessage>, MailStoreError> {
        self.query(
            &format!(
                "{SELECT} WHERE to_agent = ?1 AND delivered_at IS NULL ORDER BY created_at, id"
            ),
            params![agent],
        )
    }

    /// Every agent holding undelivered mail for this device.
    pub fn agents_with_queued(&self, device: &str) -> Result<Vec<String>, MailStoreError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT to_agent FROM mail
             WHERE delivered_at IS NULL AND to_device = ?1",
        )?;
        let rows = stmt.query_map(params![device], |row| row.get::<_, String>(0))?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    /// Delivered-but-unacked mail carried by one run.
    pub fn unacked_for_run(&self, run_id: &str) -> Result<Vec<MailMessage>, MailStoreError> {
        self.query(
            &format!(
                "{SELECT} WHERE run_id = ?1 AND delivered_at IS NOT NULL AND acked_at IS NULL
                 ORDER BY created_at, id"
            ),
            params![run_id],
        )
    }

    /// Delivered-but-unacked mail for one agent, whatever run carries it.
    pub fn unacked_for_agent(&self, agent: &str) -> Result<Vec<MailMessage>, MailStoreError> {
        self.query(
            &format!(
                "{SELECT} WHERE to_agent = ?1 AND delivered_at IS NOT NULL AND acked_at IS NULL
                 ORDER BY created_at, id"
            ),
            params![agent],
        )
    }

    pub fn mark_delivered(
        &self,
        id: &str,
        at: i64,
        run_id: Option<&str>,
    ) -> Result<(), MailStoreError> {
        self.conn().execute(
            "UPDATE mail SET delivered_at = ?2, run_id = ?3
             WHERE id = ?1 AND delivered_at IS NULL",
            params![id, at, run_id],
        )?;
        Ok(())
    }

    /// Ack is idempotent and never un-acks; a manual "seen" and the automatic
    /// turn-completion ack write the same column.
    pub fn mark_acked(&self, id: &str, at: i64) -> Result<bool, MailStoreError> {
        let changed = self.conn().execute(
            "UPDATE mail SET acked_at = ?2 WHERE id = ?1 AND acked_at IS NULL",
            params![id, at],
        )?;
        Ok(changed > 0)
    }

    /// The newest rows, newest first — the `WatchMail` feed's backing read.
    pub fn recent(&self, limit: usize) -> Result<Vec<MailMessage>, MailStoreError> {
        self.query(
            &format!("{SELECT} ORDER BY created_at DESC, id DESC LIMIT ?1"),
            params![limit as i64],
        )
    }

    /// Counters for a report: always a pair, never a bare zero.
    pub fn counts(&self) -> Result<(i64, i64, i64), MailStoreError> {
        let conn = self.conn();
        let sent: i64 = conn.query_row("SELECT COUNT(*) FROM mail", [], |r| r.get(0))?;
        let delivered: i64 = conn.query_row(
            "SELECT COUNT(*) FROM mail WHERE delivered_at IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        let acked: i64 = conn.query_row(
            "SELECT COUNT(*) FROM mail WHERE acked_at IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        Ok((sent, delivered, acked))
    }

    fn query(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<MailMessage>, MailStoreError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params, row_to_mail)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

const SELECT: &str = "SELECT id, sender, recipient, to_agent, body, created_at,
        delivered_at, acked_at, from_device, to_device, run_id FROM mail";

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS mail (
  id           TEXT PRIMARY KEY,
  sender       TEXT NOT NULL,
  recipient    TEXT NOT NULL,
  to_agent     TEXT NOT NULL,
  body         TEXT NOT NULL,
  created_at   INTEGER NOT NULL,
  delivered_at INTEGER,
  acked_at     INTEGER,
  from_device  TEXT NOT NULL,
  to_device    TEXT NOT NULL,
  run_id       TEXT
);
CREATE INDEX IF NOT EXISTS mail_pending ON mail (to_agent, delivered_at);
CREATE INDEX IF NOT EXISTS mail_run ON mail (run_id);
";

fn row_to_mail(row: &rusqlite::Row<'_>) -> rusqlite::Result<MailMessage> {
    Ok(MailMessage {
        id: row.get(0)?,
        from: row.get(1)?,
        to: row.get(2)?,
        to_agent: row.get(3)?,
        body: row.get(4)?,
        created_at: row.get(5)?,
        delivered_at: row.get(6)?,
        acked_at: row.get(7)?,
        from_device: row.get(8)?,
        to_device: row.get(9)?,
        run_id: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(id: &str, agent: &str) -> MailMessage {
        MailMessage {
            id: id.into(),
            from: "sender".into(),
            to: agent.into(),
            to_agent: agent.into(),
            body: "body".into(),
            created_at: 1,
            delivered_at: None,
            acked_at: None,
            from_device: "dev".into(),
            to_device: "dev".into(),
            run_id: None,
        }
    }

    #[test]
    fn queue_deliver_ack_walks_one_row_through() {
        let store = MailStore::open_memory().unwrap();
        store.insert(&msg("m1", "chat-b")).unwrap();
        assert_eq!(store.queued_for_agent("chat-b").unwrap().len(), 1);
        assert_eq!(store.agents_with_queued("dev").unwrap(), vec!["chat-b"]);

        store.mark_delivered("m1", 2, Some("run-1")).unwrap();
        assert!(store.queued_for_agent("chat-b").unwrap().is_empty());
        assert_eq!(store.unacked_for_run("run-1").unwrap().len(), 1);

        assert!(store.mark_acked("m1", 3).unwrap());
        assert!(!store.mark_acked("m1", 4).unwrap(), "ack is idempotent");
        assert_eq!(store.get("m1").unwrap().unwrap().acked_at, Some(3));
        assert_eq!(store.counts().unwrap(), (1, 1, 1));
    }

    #[test]
    fn delivery_is_recorded_once() {
        let store = MailStore::open_memory().unwrap();
        store.insert(&msg("m1", "chat-b")).unwrap();
        store.mark_delivered("m1", 2, Some("run-1")).unwrap();
        store.mark_delivered("m1", 9, Some("run-2")).unwrap();
        let row = store.get("m1").unwrap().unwrap();
        assert_eq!(row.delivered_at, Some(2));
        assert_eq!(row.run_id.as_deref(), Some("run-1"));
    }
}
