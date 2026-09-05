//! Always-allow rules: the table a permission answer can become.
//!
//! surya decision 20: "A permission answer can become a rule. Every
//! permission card carries a second, quieter action of the shape 'Always
//! allow migrations in project-jag', and Settings gains the approval-policy
//! page those rules live on."
//!
//! Rules are per-device and never synced: they authorize commands on THIS
//! machine's checkouts, so carrying them to another device would widen an
//! approval the user never gave there. Storage is one JSON file beside the
//! journals, written whole and atomically — the table is tens of rows, and a
//! torn write on the permission hot path would be a security bug.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::Utc;
use tokio::sync::watch;
use zeron_proto::{AllowRule, PermissionRequest, RememberRule, RuleScope};

#[cfg(test)]
mod tests;

use crate::new_id;

#[derive(Debug, thiserror::Error)]
pub enum RulesError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("a workspace rule needs a workspace path")]
    WorkspaceRuleNeedsPath,
    #[error("no such rule: {0}")]
    NotFound(String),
    #[error("the rule pattern {pattern:?} is not anchored to the command it was made from ({command:?}): {reason}")]
    PatternMissesRequest {
        pattern: String,
        command: String,
        reason: &'static str,
    },
}

struct Inner {
    path: PathBuf,
    rules: Mutex<Vec<AllowRule>>,
    tx: watch::Sender<Vec<AllowRule>>,
}

/// The device's always-allow table.
#[derive(Clone)]
pub struct AllowRules {
    inner: Arc<Inner>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl AllowRules {
    /// Open (or create) the table under `data_dir`. A corrupt file is logged
    /// and treated as empty: a rules file that will not parse must never
    /// widen permissions, and it must never stop the engine booting either.
    pub fn open(data_dir: impl AsRef<Path>) -> Self {
        let path = data_dir.as_ref().join("allow-rules.json");
        let rules = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str::<Vec<AllowRule>>(&text).unwrap_or_else(|err| {
                tracing::error!(error = %err, path = %path.display(), "allow-rules file unreadable; starting empty");
                Vec::new()
            }),
            Err(_) => Vec::new(),
        };
        let (tx, _) = watch::channel(rules.clone());
        Self {
            inner: Arc::new(Inner {
                path,
                rules: Mutex::new(rules),
                tx,
            }),
        }
    }

    /// In-memory table for tests.
    pub fn ephemeral() -> Self {
        Self::open(std::env::temp_dir().join(format!("zeron-rules-{}", new_id())))
    }

    pub fn watch(&self) -> watch::Receiver<Vec<AllowRule>> {
        self.inner.tx.subscribe()
    }

    pub fn list(&self) -> Vec<AllowRule> {
        lock(&self.inner.rules).clone()
    }

    /// The first rule that answers `allow` for this request, if any.
    pub fn matching(&self, request: &PermissionRequest, cwd: &str) -> Option<AllowRule> {
        lock(&self.inner.rules)
            .iter()
            .find(|rule| rule.matches(request, cwd))
            .cloned()
    }

    /// Add a rule. Returns the stored row, id and all.
    pub fn add(&self, rule: AllowRule) -> Result<AllowRule, RulesError> {
        if rule.scope == RuleScope::Workspace
            && rule
                .workspace_path
                .as_deref()
                .is_none_or(|path| path.trim().is_empty())
        {
            return Err(RulesError::WorkspaceRuleNeedsPath);
        }
        let mut guard = lock(&self.inner.rules);
        // Same scope, workspace, tool and pattern is the same rule. Answering
        // "always allow" twice must not grow the table.
        if let Some(existing) = guard.iter().find(|r| {
            r.scope == rule.scope
                && r.workspace_path == rule.workspace_path
                && r.tool_name == rule.tool_name
                && r.pattern == rule.pattern
        }) {
            return Ok(existing.clone());
        }
        guard.push(rule.clone());
        let snapshot = guard.clone();
        drop(guard);
        self.persist(&snapshot)?;
        let _ = self.inner.tx.send(snapshot);
        Ok(rule)
    }

    /// Build a rule from a permission answer's `remember` block.
    ///
    /// The pattern must match the command the user was actually looking at.
    /// Without that check a client answering "Bash: ls" could send pattern
    /// `*` with global scope and silently auto-allow every Bash command
    /// forever — the card said one thing and the rule meant another. An
    /// empty pattern is not a wildcard here: it pins the exact command.
    pub fn from_remember(
        remember: &RememberRule,
        request: &PermissionRequest,
        cwd: &str,
    ) -> Result<AllowRule, RulesError> {
        let pattern = remember.pattern.trim();
        let pattern = if pattern.is_empty() {
            request.command.clone()
        } else {
            pattern.to_string()
        };
        if let Err(reason) = literal_prefix_covers(&pattern, &request.command) {
            return Err(RulesError::PatternMissesRequest {
                pattern,
                command: request.command.clone(),
                reason,
            });
        }
        let workspace_path = (remember.scope == RuleScope::Workspace).then(|| cwd.to_string());
        let name = remember
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| default_rule_name(&request.tool_name, &pattern, &workspace_path));
        Ok(AllowRule {
            id: new_id(),
            name,
            scope: remember.scope,
            workspace_path,
            tool_name: request.tool_name.clone(),
            pattern,
            created_at: Utc::now(),
        })
    }

    pub fn delete(&self, rule_id: &str) -> Result<(), RulesError> {
        let mut guard = lock(&self.inner.rules);
        let before = guard.len();
        guard.retain(|r| r.id != rule_id);
        if guard.len() == before {
            return Err(RulesError::NotFound(rule_id.to_string()));
        }
        let snapshot = guard.clone();
        drop(guard);
        self.persist(&snapshot)?;
        let _ = self.inner.tx.send(snapshot);
        Ok(())
    }

    /// Write the whole table through a temp file and a rename, so a crash
    /// mid-write leaves the previous table rather than half of the new one.
    fn persist(&self, rules: &[AllowRule]) -> Result<(), RulesError> {
        let path = &self.inner.path;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(rules)?)?;
        // 0600 before the rename, never after: the table says which commands
        // run without asking, so another local account must not be able to
        // read it — and must certainly never win a race to write it.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

/// Is this pattern anchored to the command the card actually showed?
///
/// `glob_match` alone is not the check: `*` matches every command, so a
/// client that lies in its JSON could answer a card reading "Bash: ls" with
/// pattern `*` and walk away with blanket Bash approval. The card is the
/// user's whole view of what they agreed to, so the rule has to stay tied to
/// it.
///
/// The literal prefix — everything before the first `*` or `?` — must:
/// 1. be non-empty, which rejects `*`, `**` and `?*`;
/// 2. be a prefix of the command, which rejects `rm*` for `ls`;
/// 3. reach past the command's first whitespace token, which rejects
///    `php *` for `php artisan migrate` — approving one `php` subcommand is
///    not approving every one. A pattern that spells the whole command out
///    passes this by equality, so a single-word command like `ls*` is fine.
///
/// This is the answer-time check only. `AddAllowRule` from the settings page
/// is deliberately NOT filtered: there the user is writing the rule with
/// their eyes open, and a broad pattern is the point.
fn literal_prefix_covers(pattern: &str, command: &str) -> Result<(), &'static str> {
    let literal = pattern
        .split(['*', '?'])
        .next()
        .unwrap_or_default()
        .trim_end();
    if literal.is_empty() {
        return Err("it starts with a wildcard, so it would match every command this tool runs");
    }
    if !command.starts_with(literal) {
        return Err("its literal part is not how the command starts");
    }
    if literal == command.trim_end() {
        return Ok(());
    }
    let first_token = command.split_whitespace().next().unwrap_or_default();
    if literal.len() <= first_token.len() {
        return Err("it stops at the first word, so it would match every command that starts that way");
    }
    Ok(())
}

/// "Always allow `php artisan migrate*` (Bash) in project-jag" — the label
/// the card offers and the transcript quotes.
fn default_rule_name(tool: &str, pattern: &str, workspace_path: &Option<String>) -> String {
    let what = if pattern.is_empty() {
        format!("any {tool}")
    } else {
        format!("{tool} {pattern}")
    };
    match workspace_path.as_deref().and_then(folder_name) {
        Some(folder) => format!("{what} in {folder}"),
        None => what,
    }
}

fn folder_name(path: &str) -> Option<&str> {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
}
