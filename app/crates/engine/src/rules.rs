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
    pub fn from_remember(
        remember: &RememberRule,
        request: &PermissionRequest,
        cwd: &str,
    ) -> AllowRule {
        let pattern = remember.pattern.trim();
        let pattern = if pattern.is_empty() {
            request.command.clone()
        } else {
            pattern.to_string()
        };
        let workspace_path = (remember.scope == RuleScope::Workspace).then(|| cwd.to_string());
        let name = remember
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| default_rule_name(&request.tool_name, &pattern, &workspace_path));
        AllowRule {
            id: new_id(),
            name,
            scope: remember.scope,
            workspace_path,
            tool_name: request.tool_name.clone(),
            pattern,
            created_at: Utc::now(),
        }
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
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn request(tool: &str, command: &str) -> PermissionRequest {
        PermissionRequest {
            request_id: "req-1".into(),
            tool_name: tool.into(),
            command: command.into(),
            input: None,
        }
    }

    #[test]
    fn a_remembered_answer_matches_the_identical_next_request() {
        let dir = tempfile::tempdir().unwrap();
        let rules = AllowRules::open(dir.path());
        let req = request("Bash", "php artisan migrate");
        assert!(rules.matching(&req, "/repos/project-jag").is_none());

        let remember = RememberRule {
            scope: RuleScope::Workspace,
            pattern: "php artisan migrate*".into(),
            name: None,
        };
        let rule = AllowRules::from_remember(&remember, &req, "/repos/project-jag");
        assert_eq!(rule.name, "Bash php artisan migrate* in project-jag");
        rules.add(rule).unwrap();

        assert!(rules.matching(&req, "/repos/project-jag").is_some());
        assert!(
            rules
                .matching(&request("Bash", "php artisan migrate --seed"), "/repos/project-jag")
                .is_some()
        );
        // Another checkout is not covered by a workspace rule.
        assert!(rules.matching(&req, "/repos/other").is_none());
        // Another tool is not covered either.
        assert!(
            rules
                .matching(&request("Edit", "php artisan migrate"), "/repos/project-jag")
                .is_none()
        );
    }

    #[test]
    fn an_empty_remember_pattern_pins_the_exact_command() {
        let req = request("Bash", "git push");
        let rule = AllowRules::from_remember(
            &RememberRule {
                scope: RuleScope::Global,
                pattern: "  ".into(),
                name: None,
            },
            &req,
            "/repos/x",
        );
        assert_eq!(rule.pattern, "git push");
        assert_eq!(rule.workspace_path, None);
        assert_eq!(rule.name, "Bash git push");
    }

    #[test]
    fn rules_survive_a_reopen_and_delete_removes_them() {
        let dir = tempfile::tempdir().unwrap();
        let rules = AllowRules::open(dir.path());
        let rule = AllowRules::from_remember(
            &RememberRule {
                scope: RuleScope::Global,
                pattern: "git status*".into(),
                name: Some("read-only git".into()),
            },
            &request("Bash", "git status"),
            "/repos/x",
        );
        let stored = rules.add(rule).unwrap();

        let reopened = AllowRules::open(dir.path());
        assert_eq!(reopened.list().len(), 1);
        assert_eq!(reopened.list()[0].name, "read-only git");

        reopened.delete(&stored.id).unwrap();
        assert!(reopened.list().is_empty());
        assert!(AllowRules::open(dir.path()).list().is_empty());
        assert!(matches!(
            reopened.delete(&stored.id),
            Err(RulesError::NotFound(_))
        ));
    }

    #[test]
    fn adding_the_same_rule_twice_does_not_grow_the_table() {
        let dir = tempfile::tempdir().unwrap();
        let rules = AllowRules::open(dir.path());
        let make = || AllowRules::from_remember(
            &RememberRule {
                scope: RuleScope::Global,
                pattern: "ls*".into(),
                name: None,
            },
            &request("Bash", "ls"),
            "/repos/x",
        );
        let first = rules.add(make()).unwrap();
        let second = rules.add(make()).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(rules.list().len(), 1);
    }

    #[test]
    fn a_workspace_rule_without_a_workspace_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let rules = AllowRules::open(dir.path());
        let mut rule = AllowRules::from_remember(
            &RememberRule {
                scope: RuleScope::Workspace,
                pattern: "*".into(),
                name: None,
            },
            &request("Bash", "ls"),
            "/repos/x",
        );
        rule.workspace_path = None;
        assert!(matches!(
            rules.add(rule),
            Err(RulesError::WorkspaceRuleNeedsPath)
        ));
        assert!(rules.list().is_empty());
    }

    #[test]
    fn a_corrupt_file_starts_empty_rather_than_allowing_anything() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("allow-rules.json"), "{ not json").unwrap();
        let rules = AllowRules::open(dir.path());
        assert!(rules.list().is_empty());
        assert!(
            rules
                .matching(&request("Bash", "anything"), "/repos/x")
                .is_none()
        );
    }
}
