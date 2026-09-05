//! Derived agent state, the needs-you inbox, and always-allow rules.
//!
//! surya decision 20 makes the needs-you queue the spine of the app: home is
//! "what needs you" first, then what is running, then what is done. Decision
//! 15 nests a spawned agent under its spawner and rolls a child's needs-you
//! up every ancestor row. Decision 17 adds `Stopped`.
//!
//! Everything here is DERIVED. The engine computes it from the run journal,
//! the live [`crate::Session`] rows and the pending control requests; nothing
//! in this module is a second source of truth the UI could disagree with.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Why an agent is waiting on the user.
///
/// `Failed` is the crash / rate-limit / out-of-context case from decision 17.
/// It is listed here (not as a peer of [`AgentState::Stopped`]) because the
/// inbox groups by "what does this row want from me", and a failed run wants
/// the same shaped answer as a permission does: one tap on a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NeedsYouKind {
    /// A tool wants approval before it runs.
    Permission,
    /// The agent asked a question (`AskUserQuestion`).
    Question,
    /// The run died on its own: crash, rate limit, out of context.
    Failed,
}

/// The state one agent row shows.
///
/// Ordering matches the rail's sort priority (decision 15: "needs you,
/// working, done, idle"), with `Stopped` beside `NeedsYou` because it also
/// wants a decision from the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum AgentState {
    /// Blocked on the user. Sorts first.
    #[serde(rename_all = "camelCase")]
    NeedsYou { kind: NeedsYouKind },
    /// Stopped by the user (an interrupt), so it wants Retry or Give up
    /// rather than an answer. Counts as needs-you for roll-up and for the
    /// inbox — decision 17.
    Stopped,
    /// A run is live.
    Working,
    /// Finished and not seen yet on any device.
    Done,
    /// Nothing in flight, nothing unread.
    Idle,
}

impl AgentState {
    /// True when this row belongs in the needs-you inbox and marks every
    /// ancestor. `Stopped` counts, per decision 17.
    pub fn needs_you(self) -> bool {
        matches!(self, AgentState::NeedsYou { .. } | AgentState::Stopped)
    }

    /// Rail sort key: needs-you, working, done, idle (decision 15).
    pub fn sort_rank(self) -> u8 {
        match self {
            AgentState::NeedsYou { .. } => 0,
            AgentState::Stopped => 1,
            AgentState::Working => 2,
            AgentState::Done => 3,
            AgentState::Idle => 4,
        }
    }

    /// The worse of two states, by sort rank — the roll-up operator a parent
    /// applies over itself and every descendant.
    pub fn worse(self, other: Self) -> Self {
        if other.sort_rank() < self.sort_rank() {
            other
        } else {
            self
        }
    }
}

/// One row of the derived agent tree, published alongside the session rows so
/// `WatchSessions` carries state without a second subscription.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStateRow {
    /// Chat id for a top-level agent; `<parent chat id>:<tool_use_id>` for a
    /// spawned child (see [`child_agent_id`]).
    pub id: String,
    /// The agent that spawned this one. `None` means the user spawned it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// The chat this row lives in — equal to `id` for a top-level agent.
    pub chat_id: String,
    /// This row's own state, before any child rolls into it.
    pub state: AgentState,
    /// This row's state after every descendant rolls up. Equal to `state`
    /// when no child needs anything.
    pub rolled_up: AgentState,
    /// How many descendants need the user (this row not counted).
    #[serde(default)]
    pub needs_you_children: u32,
    /// Label for the row: the spawn's description for a child, else `None`
    /// (the chat title is on the [`crate::Chat`] row already).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// The id a spawned agent gets: parent chat id plus the spawning tool call's
/// id, joined by `:`. Deterministic, so a replayed journal rebuilds the same
/// tree and a re-delivered subagent frame updates rather than duplicates.
pub fn child_agent_id(parent_chat_id: &str, tool_use_id: &str) -> String {
    format!("{parent_chat_id}:{tool_use_id}")
}

/// One entry in the needs-you inbox, newest first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeedsYouItem {
    /// Stable id: the permission request id, the question id, or the chat id
    /// for a failure.
    pub id: String,
    /// The chat to open when the card is tapped.
    pub chat_id: String,
    /// The agent row this belongs to — a child id when a subagent asked.
    pub agent_id: String,
    pub kind: NeedsYouKind,
    /// One-line title for the card.
    pub title: String,
    /// The question text, or the permission's reason. Empty for a failure
    /// with no detail.
    #[serde(default)]
    pub prompt: String,
    /// Answer options, in order. Empty for a permission (the card draws
    /// Allow / Always allow / Deny itself) and for a failure.
    #[serde(default)]
    pub options: Vec<String>,
    /// True when more than one option may be picked.
    #[serde(default)]
    pub multi_select: bool,
    /// For a permission: the tool name (`Bash`, `Edit`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// For a permission: the command or path the tool would act on, already
    /// rendered for display and for rule matching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_command: Option<String>,
    /// For a failure: whether Retry is offered (decision 17's `retryable`).
    #[serde(default)]
    pub retryable: bool,
    pub created_at: DateTime<Utc>,
}

/// A permission the harness is blocked on, as the engine parks it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequest {
    pub request_id: String,
    pub tool_name: String,
    /// The command / path / pattern the tool acts on. This is the string a
    /// rule pattern is matched against.
    #[serde(default)]
    pub command: String,
    /// The raw tool input, for the card's detail view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
}

/// The answer to one permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionDecision {
    Allow,
    Deny,
}

/// How far an always-allow rule reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleScope {
    /// Only in the workspace (checkout path) the rule was made in.
    Workspace,
    /// Everywhere on this device.
    Global,
}

/// One always-allow rule: "Always allow migrations in project-jag".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowRule {
    pub id: String,
    /// Human label shown in Settings and quoted in the transcript line.
    pub name: String,
    pub scope: RuleScope,
    /// Absolute checkout path the rule is confined to. Required for
    /// [`RuleScope::Workspace`], ignored for [`RuleScope::Global`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    /// Tool the rule applies to, matched exactly (`Bash`, `Edit`, …).
    pub tool_name: String,
    /// Glob over the tool's command string. `*` matches any run of
    /// characters, `?` one character. An empty pattern matches everything the
    /// tool does — unless [`Self::exact`] is set.
    #[serde(default)]
    pub pattern: String,
    /// Compare the pattern to the command literally, `*` and `?` included.
    ///
    /// "Remember this one command" has to survive a command that CONTAINS a
    /// wildcard: storing `rm -rf build/*` as a glob would silently approve
    /// `rm -rf build/anything`, which is not what the user pinned. A flag
    /// rather than backslash escaping, because escaping would also have to be
    /// understood by the rule-anchoring check and by the settings page, and
    /// three places that must agree about escapes is three places to get it
    /// wrong. Additive + serde-defaulted: an old rules file reads as a glob,
    /// which is what it was.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exact: bool,
    pub created_at: DateTime<Utc>,
}

impl AllowRule {
    /// Does this rule answer `allow` for a request made in `cwd`?
    pub fn matches(&self, request: &PermissionRequest, cwd: &str) -> bool {
        if self.tool_name != request.tool_name {
            return false;
        }
        if self.exact {
            return self.tool_name == request.tool_name
                && self.workspace_ok(cwd)
                && self.pattern == request.command;
        }
        if !self.workspace_ok(cwd) {
            return false;
        }
        glob_match(&self.pattern, &request.command)
    }

    /// Is `cwd` inside this rule's reach?
    fn workspace_ok(&self, cwd: &str) -> bool {
        match self.scope {
            RuleScope::Global => true,
            // A workspace rule with no workspace cannot be confined, so it
            // never fires. Refusing here beats silently going global.
            RuleScope::Workspace => self
                .workspace_path
                .as_deref()
                .is_some_and(|root| path_within(cwd, root)),
        }
    }

    /// The transcript line the engine writes when this rule answers.
    pub fn transcript_line(&self) -> String {
        format!("allowed by rule {}", self.name)
    }
}

/// What a client sends to turn one answer into a rule
/// (`RespondPermission { remember }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RememberRule {
    pub scope: RuleScope,
    /// Glob over the tool command. Empty means "anything this tool does".
    #[serde(default)]
    pub pattern: String,
    /// Optional label; the engine composes one from tool and pattern when
    /// this is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// True when `path` is `root` or sits under it. Compares whole path
/// components, so `/repo-two` never matches root `/repo`.
pub fn path_within(path: &str, root: &str) -> bool {
    let path = path.trim_end_matches('/');
    let root = root.trim_end_matches('/');
    if root.is_empty() {
        return true;
    }
    if path == root {
        return true;
    }
    path.strip_prefix(root)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// Glob match over the whole string: `*` any run, `?` one char, everything
/// else literal. An empty pattern matches everything.
///
/// Written out rather than pulled in as a dependency: the rule alphabet is
/// two metacharacters, and a regex crate here would let a pattern typed in
/// Settings become a backtracking bomb on the permission hot path.
pub fn glob_match(pattern: &str, value: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    let pattern: Vec<char> = pattern.chars().collect();
    let value: Vec<char> = value.chars().collect();
    // Iterative two-pointer walk with backtracking to the last `*`; linear in
    // the common case and never worse than O(n·m).
    let (mut p, mut v) = (0usize, 0usize);
    let (mut star, mut resume) = (None, 0usize);
    while v < value.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == value[v]) {
            p += 1;
            v += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            resume = v;
            p += 1;
        } else if let Some(s) = star {
            p = s + 1;
            resume += 1;
            v = resume;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(tool: &str, command: &str) -> PermissionRequest {
        PermissionRequest {
            request_id: "r1".into(),
            tool_name: tool.into(),
            command: command.into(),
            input: None,
        }
    }

    fn rule(scope: RuleScope, workspace: Option<&str>, tool: &str, pattern: &str) -> AllowRule {
        AllowRule {
            id: "rule-1".into(),
            name: "test rule".into(),
            scope,
            workspace_path: workspace.map(str::to_string),
            tool_name: tool.into(),
            pattern: pattern.into(),
            exact: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn glob_matches_the_two_metacharacters_and_nothing_else() {
        assert!(glob_match("php artisan migrate*", "php artisan migrate --seed"));
        assert!(glob_match("php artisan migrate*", "php artisan migrate"));
        assert!(!glob_match("php artisan migrate*", "php artisan db:wipe"));
        assert!(glob_match("git ?ush", "git push"));
        assert!(!glob_match("git ?ush", "git rebase"));
        // An empty pattern is "anything this tool does".
        assert!(glob_match("", "rm -rf /"));
        // `.` and `[` are literal, not regex.
        assert!(glob_match("a.b", "a.b"));
        assert!(!glob_match("a.b", "axb"));
        // Leading and repeated stars.
        assert!(glob_match("*migrate*", "php artisan migrate --seed"));
        assert!(glob_match("**", "anything"));
        assert!(!glob_match("a*b", "ab_"));
    }

    #[test]
    fn workspace_rule_is_confined_to_its_checkout() {
        let r = rule(
            RuleScope::Workspace,
            Some("/repos/project-jag"),
            "Bash",
            "php artisan migrate*",
        );
        let req = request("Bash", "php artisan migrate --seed");
        assert!(r.matches(&req, "/repos/project-jag"));
        assert!(r.matches(&req, "/repos/project-jag/backend"));
        assert!(!r.matches(&req, "/repos/project-jag-two"));
        assert!(!r.matches(&req, "/repos/other"));
        // A workspace rule that names no workspace never fires.
        let orphan = rule(RuleScope::Workspace, None, "Bash", "*");
        assert!(!orphan.matches(&req, "/repos/project-jag"));
    }

    #[test]
    fn global_rule_ignores_cwd_but_not_tool_name() {
        let r = rule(RuleScope::Global, None, "Bash", "git status*");
        assert!(r.matches(&request("Bash", "git status"), "/anywhere"));
        assert!(!r.matches(&request("Edit", "git status"), "/anywhere"));
    }

    #[test]
    fn stopped_and_needs_you_both_roll_up_and_sort_first() {
        let needs = AgentState::NeedsYou {
            kind: NeedsYouKind::Permission,
        };
        assert!(needs.needs_you());
        assert!(AgentState::Stopped.needs_you());
        assert!(!AgentState::Working.needs_you());
        assert!(!AgentState::Done.needs_you());
        assert_eq!(AgentState::Working.worse(needs), needs);
        assert_eq!(AgentState::Idle.worse(AgentState::Done), AgentState::Done);
        assert_eq!(
            AgentState::Working.worse(AgentState::Idle),
            AgentState::Working
        );
    }

    #[test]
    fn agent_state_is_externally_tagged_for_the_ui() {
        let json = serde_json::to_value(AgentState::NeedsYou {
            kind: NeedsYouKind::Question,
        })
        .unwrap();
        assert_eq!(json["state"], "needsYou");
        assert_eq!(json["kind"], "question");
        assert_eq!(
            serde_json::to_value(AgentState::Stopped).unwrap()["state"],
            "stopped"
        );
    }

    /// "Remember this exact command" has to survive a command that itself
    /// contains a wildcard, or pinning `rm -rf build/*` would quietly approve
    /// `rm -rf build/anything`.
    #[test]
    fn an_exact_rule_compares_literally_wildcards_and_all() {
        let mut r = rule(RuleScope::Global, None, "Bash", "ls *.txt");
        r.exact = true;
        assert!(r.matches(&request("Bash", "ls *.txt"), "/repos/x"));
        assert!(!r.matches(&request("Bash", "ls notes.txt"), "/repos/x"));
        assert!(!r.matches(&request("Bash", "ls "), "/repos/x"));

        let mut danger = rule(RuleScope::Global, None, "Bash", "rm -rf build/*");
        danger.exact = true;
        assert!(danger.matches(&request("Bash", "rm -rf build/*"), "/repos/x"));
        assert!(
            !danger.matches(&request("Bash", "rm -rf build/src"), "/repos/x"),
            "an exact pin is not a glob"
        );
        // The same pattern WITHOUT the flag is a glob, which is why the flag
        // exists.
        let as_glob = rule(RuleScope::Global, None, "Bash", "rm -rf build/*");
        assert!(as_glob.matches(&request("Bash", "rm -rf build/src"), "/repos/x"));
    }

    #[test]
    fn an_exact_workspace_rule_is_still_confined() {
        let mut r = rule(
            RuleScope::Workspace,
            Some("/repos/project-jag"),
            "Bash",
            "ls *.txt",
        );
        r.exact = true;
        let req = request("Bash", "ls *.txt");
        assert!(r.matches(&req, "/repos/project-jag/backend"));
        assert!(!r.matches(&req, "/repos/other"));
        r.workspace_path = None;
        assert!(!r.matches(&req, "/repos/project-jag"));
    }

    #[test]
    fn the_exact_flag_is_additive_on_the_wire() {
        // An old rules file has no `exact` key and reads as a glob.
        let old = serde_json::json!({
            "id": "r", "name": "n", "scope": "global", "toolName": "Bash",
            "pattern": "ls*", "createdAt": "2026-09-05T00:00:00Z"
        });
        let parsed: AllowRule = serde_json::from_value(old).unwrap();
        assert!(!parsed.exact);
        // …and a glob rule never writes the key, so an old reader is unchanged.
        assert!(serde_json::to_value(&parsed).unwrap().get("exact").is_none());
    }

    #[test]
    fn child_id_is_deterministic() {
        assert_eq!(child_agent_id("chat-1", "tool-9"), "chat-1:tool-9");
    }

    #[test]
    fn path_within_compares_components() {
        assert!(path_within("/a/b", "/a"));
        assert!(path_within("/a", "/a"));
        assert!(path_within("/a/", "/a"));
        assert!(!path_within("/ab", "/a"));
        assert!(path_within("/anything", ""));
    }
}
