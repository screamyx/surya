//! The permission gate a harness asks before it lets a tool run.
//!
//! Comet auto-approved every `can_use_tool` control request (claude/mod.rs:
//! "Every tool is auto-approved (unattended parity …)"). surya's spine is the
//! needs-you queue, so a permission has to be able to reach the user — and,
//! once an always-allow rule exists, to be answered without waking anyone.
//!
//! The gate is a host-supplied callback so the harness stays free of the
//! engine's rules table and its inbox. [`PermissionGate::auto_allow`] keeps
//! the old behavior for tests and for harnesses run headless.

use std::sync::Arc;

use tokio::sync::oneshot;
use surya_proto::{PermissionDecision, PermissionRequest};

type PermissionFn =
    dyn Fn(PermissionRequest) -> oneshot::Receiver<PermissionDecision> + Send + Sync;

/// A host callback that answers permission requests, or auto-allow when the
/// host supplied none.
#[derive(Clone)]
pub struct PermissionGate(Option<Arc<PermissionFn>>);

impl PermissionGate {
    /// Allow everything without asking — comet's original behavior, and what
    /// every harness test uses.
    pub fn auto_allow() -> Self {
        Self(None)
    }

    /// Route requests to the host.
    pub fn new(
        ask: impl Fn(PermissionRequest) -> oneshot::Receiver<PermissionDecision>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self(Some(Arc::new(ask)))
    }

    /// True when a host is actually deciding.
    pub fn is_gated(&self) -> bool {
        self.0.is_some()
    }

    /// Ask, and wait.
    ///
    /// A dropped host resolver is a `Deny`, never an `Allow`. The CLI blocks
    /// until SOME answer arrives, so the gate must always answer — but the
    /// answer to "nobody is there to approve this" is no. Deny unblocks the
    /// agent exactly as well as Allow does, and it is the only reading that
    /// is safe: deleting a chat with a permission still parked drops its
    /// responder, and that must not run the tool.
    pub async fn ask(&self, request: PermissionRequest) -> PermissionDecision {
        let Some(ask) = self.0.as_ref() else {
            return PermissionDecision::Allow;
        };
        ask(request).await.unwrap_or(PermissionDecision::Deny)
    }
}

impl Default for PermissionGate {
    fn default() -> Self {
        Self::auto_allow()
    }
}

impl std::fmt::Debug for PermissionGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PermissionGate")
            .field(&if self.is_gated() { "host" } else { "auto-allow" })
            .finish()
    }
}

/// Keys that carry the thing a tool acts on, in the order a card should
/// prefer them. Drivers spell the same idea several ways, so the lookup is by
/// key set rather than by a table of tool names — a new tool needs no change
/// here as long as it names its argument one of these.
const COMMAND_KEYS: [&str; 8] = [
    "command",
    "file_path",
    "filePath",
    "path",
    "pattern",
    "url",
    "query",
    "notebook_path",
];

/// The one-line command string a permission card shows and a rule pattern is
/// matched against. Empty when the input names nothing recognizable — a rule
/// with an empty pattern still matches it, which is the right reading of
/// "always allow this tool".
pub fn describe_tool_command(input: Option<&serde_json::Value>) -> String {
    let Some(input) = input else {
        return String::new();
    };
    COMMAND_KEYS
        .iter()
        .find_map(|key| input.get(*key).and_then(serde_json::Value::as_str))
        .unwrap_or_default()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn command_is_read_off_whichever_key_the_tool_used() {
        assert_eq!(
            describe_tool_command(Some(&json!({"command": "php artisan migrate"}))),
            "php artisan migrate"
        );
        assert_eq!(
            describe_tool_command(Some(&json!({"file_path": "/repo/src/main.rs"}))),
            "/repo/src/main.rs"
        );
        assert_eq!(
            describe_tool_command(Some(&json!({"url": "https://example.test"}))),
            "https://example.test"
        );
        // Nothing recognizable, and no input at all, both read as empty.
        assert_eq!(describe_tool_command(Some(&json!({"foo": 1}))), "");
        assert_eq!(describe_tool_command(None), "");
        // `command` wins over a path when a tool carries both.
        assert_eq!(
            describe_tool_command(Some(&json!({"path": "/x", "command": "ls"}))),
            "ls"
        );
    }

    #[tokio::test]
    async fn auto_allow_answers_without_a_host() {
        let gate = PermissionGate::auto_allow();
        assert!(!gate.is_gated());
        let decision = gate
            .ask(PermissionRequest {
                request_id: "r".into(),
                tool_name: "Bash".into(),
                command: "ls".into(),
                input: None,
            })
            .await;
        assert_eq!(decision, PermissionDecision::Allow);
    }

    /// The gate must fail CLOSED. A host that goes away mid-request — the
    /// chat was deleted, the engine dropped the responder — must not have
    /// the tool run anyway. It still answers, so the agent is never wedged.
    #[tokio::test]
    async fn a_dropped_host_resolver_denies_rather_than_allowing() {
        let gate = PermissionGate::new(|_| {
            let (tx, rx) = oneshot::channel();
            drop(tx);
            rx
        });
        assert!(gate.is_gated());
        let decision = gate
            .ask(PermissionRequest {
                request_id: "r".into(),
                tool_name: "Bash".into(),
                command: "rm -rf /".into(),
                input: None,
            })
            .await;
        assert_eq!(decision, PermissionDecision::Deny);
    }

    #[tokio::test]
    async fn a_host_deny_reaches_the_harness() {
        let gate = PermissionGate::new(|req| {
            let (tx, rx) = oneshot::channel();
            let decision = if req.command.starts_with("rm ") {
                PermissionDecision::Deny
            } else {
                PermissionDecision::Allow
            };
            let _ = tx.send(decision);
            rx
        });
        let ask = |command: &str| {
            let gate = gate.clone();
            let command = command.to_string();
            async move {
                gate.ask(PermissionRequest {
                    request_id: "r".into(),
                    tool_name: "Bash".into(),
                    command,
                    input: None,
                })
                .await
            }
        };
        assert_eq!(ask("rm -rf /").await, PermissionDecision::Deny);
        assert_eq!(ask("ls").await, PermissionDecision::Allow);
    }
}
