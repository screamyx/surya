//! What the inbox views draw, worked out without a window.
//!
//! Every decision the needs-you list and the agent rail make — which rows
//! exist, what each one says, what a button sends, how children nest under
//! their parent — lives here as plain data. The views in `needs_you.rs`,
//! `agents.rs` and `rules.rs` only lay it out.
//!
//! Splitting it this way is not ceremony: the roll-up and the "always allow"
//! scope are the two places a mistake would be expensive, and both are
//! testable here without a GPU.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use zeron_doc::SessionCommandPayload;
use zeron_proto::{
    AgentState, AgentStateRow, Chat, NeedsYouItem, NeedsYouKind, PermissionDecision, RememberRule,
    RuleScope, UserInputAnswer,
};

/// One row of the needs-you list.
#[derive(Debug, Clone, PartialEq)]
pub struct InboxRow {
    /// The item id: a permission request id, `<request>:<question>`, or
    /// `<agent>:failure`.
    pub id: String,
    pub chat_id: String,
    pub agent_id: String,
    /// The chat's title, or a readable stand-in when it has none yet.
    pub chat_name: String,
    pub kind: NeedsYouKind,
    /// "Permission", "Question" or "Stopped" — the badge text.
    pub badge: &'static str,
    pub title: String,
    pub prompt: String,
    pub options: Vec<String>,
    pub multi_select: bool,
    /// Set for a permission: the command the tool would run.
    pub tool_command: Option<String>,
    pub tool_name: Option<String>,
    pub retryable: bool,
}

/// The badge text for each kind. `Failed` reads "Stopped" because that is
/// what decision 17 tells the user it is called.
pub fn badge_for(kind: NeedsYouKind) -> &'static str {
    match kind {
        NeedsYouKind::Permission => "Permission",
        NeedsYouKind::Question => "Question",
        NeedsYouKind::Failed => "Stopped",
    }
}

/// A chat with no title yet. Showing the raw id would be worse than saying
/// plainly that it has no name.
pub const UNTITLED_CHAT: &str = "Untitled chat";

/// Is this row's own answer surface already on screen?
///
/// A question for the chat the user is reading can be offered twice: once as
/// a card in this list, and again as the question sheet in that chat's
/// transcript, with the same options on both (round 4, I2). The sheet is the
/// real one - it is where the answer is typed - so the card collapses to one
/// line that only says the queue still holds it.
///
/// `open_sheet` is the request id the open chat's transcript is actually
/// asking about (`composer::pending_input_request`). Passing it makes this a
/// fact rather than a guess, and every trap here is a case where the guess
/// would have been wrong:
///
/// - A SUBAGENT's question is filed under the PARENT's chat id with the
///   child's agent id (`engine/src/agent_states.rs`
///   `note_subagent_event`), and subagent events never fold into the parent
///   transcript (`engine/src/sessions.rs`, "Tagged events NEVER fold into
///   the parent transcript"). Collapsing on the chat id alone would point the
///   user at a sheet that is not there.
/// - A chat the user just opened has no transcript yet for a frame or two.
/// - An answered question keeps its row until the next `WatchNeedsYou`
///   frame, but its sheet is resolved the moment it is answered.
///
/// Questions only. A permission and a stopped run have no second surface, so
/// collapsing them would leave the user with nothing to press.
pub fn answered_in_the_open_chat(
    row: &InboxRow,
    open_chat: Option<&str>,
    open_sheet: Option<&str>,
) -> bool {
    row.kind == NeedsYouKind::Question
        && open_chat == Some(row.chat_id.as_str())
        // A top-level question carries the chat id as its agent id
        // (`open_questions(&chat_id, &chat_id, ..)`); a child carries
        // `child_agent_id`.
        && row.agent_id == row.chat_id
        && split_question_id(&row.id)
            .is_some_and(|(request_id, _)| open_sheet == Some(request_id))
}

/// Does the bold title say anything the badge above it does not?
///
/// A question's title is the model's own header, and a model that writes
/// "Question" leaves the card reading "Question" twice, once in the badge and
/// once in bold under it (round 4, I3).
pub fn title_adds_to_badge(row: &InboxRow) -> bool {
    let title = row.title.trim();
    !title.is_empty() && !title.eq_ignore_ascii_case(row.badge.trim())
}

/// Build the list the needs-you view draws. `items` arrives newest-first from
/// `WatchNeedsYou` and that order is kept: the engine already sorted it, and
/// re-sorting here would be a second opinion that can disagree.
pub fn inbox_rows(items: &[NeedsYouItem], chats: &[Chat]) -> Vec<InboxRow> {
    let names: HashMap<&str, &str> = chats
        .iter()
        .filter_map(|chat| {
            let title = chat.title.as_deref()?.trim();
            (!title.is_empty()).then_some((chat.id.as_str(), title))
        })
        .collect();
    items
        .iter()
        .map(|item| InboxRow {
            id: item.id.clone(),
            chat_id: item.chat_id.clone(),
            agent_id: item.agent_id.clone(),
            chat_name: names
                .get(item.chat_id.as_str())
                .map(|name| (*name).to_string())
                .unwrap_or_else(|| UNTITLED_CHAT.to_string()),
            kind: item.kind,
            badge: badge_for(item.kind),
            title: item.title.clone(),
            prompt: item.prompt.clone(),
            options: item.options.clone(),
            multi_select: item.multi_select,
            tool_command: item.tool_command.clone(),
            tool_name: item.tool_name.clone(),
            retryable: item.retryable,
        })
        .collect()
}

/// How far an "Always allow" click reaches. The toggle has exactly these two
/// positions because decision 20's example — "Always allow migrations in
/// project-jag" — is the workspace one, and anything wider has to be said out
/// loud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlwaysAllowScope {
    #[default]
    ThisWorkspace,
    Everywhere,
}

impl AlwaysAllowScope {
    /// Lower case: it is rendered after "scope: ", not as a title.
    pub fn label(self) -> &'static str {
        match self {
            Self::ThisWorkspace => "this workspace",
            Self::Everywhere => "everywhere",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::ThisWorkspace => Self::Everywhere,
            Self::Everywhere => Self::ThisWorkspace,
        }
    }

    fn rule_scope(self) -> RuleScope {
        match self {
            Self::ThisWorkspace => RuleScope::Workspace,
            Self::Everywhere => RuleScope::Global,
        }
    }
}

/// The `remember` block an "Always allow" click sends.
///
/// The pattern is EMPTY on purpose. An empty pattern tells the engine "pin
/// this exact command", which it stores literally (`exact`). Sending the
/// command as a pattern instead would store it as a GLOB: a card showing
/// `rm -rf build/*` would become a standing wildcard rule, and `git *` would
/// be refused by the engine's anchoring check so the click would just error.
/// Broadening a rule happens on the approval-policy page, deliberately, not
/// by this view guessing what the user meant.
pub fn remember_for(_row: &InboxRow, scope: AlwaysAllowScope) -> RememberRule {
    RememberRule {
        scope: scope.rule_scope(),
        pattern: String::new(),
        name: None,
    }
}

/// `RespondPermission` params. Allow, deny, or allow-and-remember.
pub fn respond_permission_params(
    request_id: &str,
    decision: PermissionDecision,
    remember: Option<&RememberRule>,
) -> serde_json::Value {
    let mut params = serde_json::json!({
        "requestId": request_id,
        "decision": decision,
    });
    if let Some(remember) = remember
        && let (Some(object), Ok(value)) = (params.as_object_mut(), serde_json::to_value(remember))
    {
        object.insert("remember".into(), value);
    }
    params
}

/// `QueueCommand` params for answering a question — the same `RespondInput`
/// doc command the composer's question panel sends, so one answer path serves
/// both surfaces.
pub fn respond_input_params(
    chat_id: &str,
    request_id: &str,
    answers: Vec<UserInputAnswer>,
) -> Option<serde_json::Value> {
    let command = SessionCommandPayload::RespondInput {
        request_id: request_id.to_string(),
        answers,
    };
    let command = serde_json::to_value(&command).ok()?;
    Some(serde_json::json!({ "chatId": chat_id, "command": command }))
}

/// A question row's id is `<request id>:<question id>`; both halves are
/// needed to answer — the request id keys the parked resolver, the question
/// id keys the answer inside it.
pub fn split_question_id(row_id: &str) -> Option<(&str, &str)> {
    row_id.split_once(':')
}

/// The four groups the agent list shows, in the order decision 15 gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentGroup {
    WaitingForYou,
    Running,
    Done,
    Idle,
}

impl AgentGroup {
    pub fn heading(self) -> &'static str {
        match self {
            Self::WaitingForYou => "Waiting for you",
            Self::Running => "Running",
            Self::Done => "Done",
            Self::Idle => "Idle",
        }
    }

    /// Which group a row belongs to, by its ROLLED-UP state: a parent whose
    /// child is blocked belongs with the blocked ones, or the user would have
    /// to open a folded branch to find out anything is wrong.
    pub fn of(state: AgentState) -> Self {
        match state {
            AgentState::NeedsYou { .. } | AgentState::Stopped => Self::WaitingForYou,
            AgentState::Working => Self::Running,
            AgentState::Done => Self::Done,
            AgentState::Idle => Self::Idle,
        }
    }

    pub const ORDER: [AgentGroup; 4] = [
        Self::WaitingForYou,
        Self::Running,
        Self::Done,
        Self::Idle,
    ];
}

/// One drawn row of the agent tree: the state row plus how deep it sits.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentRow {
    pub id: String,
    pub chat_id: String,
    /// Chat title for a top-level agent, the spawn's description for a child.
    pub name: String,
    /// Nesting depth. 0 is a top-level agent.
    pub depth: usize,
    pub state: AgentState,
    pub rolled_up: AgentState,
    pub needs_you_children: u32,
    /// True when the row's own state is calm but a descendant is not — the
    /// case the roll-up badge exists for.
    pub rolled_up_only: bool,
}

/// One group heading plus its rows, children already nested under parents.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentSection {
    pub group: AgentGroup,
    pub rows: Vec<AgentRow>,
}

/// Group the agent rows and flatten each group's trees depth-first, so a
/// child always renders directly under its parent.
///
/// Grouping is by the TOP-LEVEL row's rolled-up state; a child rides in its
/// parent's group whatever its own state, because a tree split across two
/// headings is not a tree.
pub fn agent_sections(rows: &[AgentStateRow], chats: &[Chat]) -> Vec<AgentSection> {
    let names: HashMap<&str, &str> = chats
        .iter()
        .filter_map(|chat| {
            let title = chat.title.as_deref()?.trim();
            (!title.is_empty()).then_some((chat.id.as_str(), title))
        })
        .collect();
    let mut children: HashMap<&str, Vec<&AgentStateRow>> = HashMap::new();
    for row in rows {
        if let Some(parent) = row.parent_id.as_deref() {
            children.entry(parent).or_default().push(row);
        }
    }
    // Keep the engine's order inside each parent: it already sorted worst
    // first, and the tree walk must not undo that.
    let mut sections: Vec<AgentSection> = AgentGroup::ORDER
        .iter()
        .map(|group| AgentSection {
            group: *group,
            rows: Vec::new(),
        })
        .collect();
    for row in rows.iter().filter(|r| r.parent_id.is_none()) {
        let group = AgentGroup::of(row.rolled_up);
        let Some(section) = sections.iter_mut().find(|s| s.group == group) else {
            continue;
        };
        push_subtree(&mut section.rows, row, 0, &children, &names);
    }
    sections.retain(|section| !section.rows.is_empty());
    sections
}

fn push_subtree(
    out: &mut Vec<AgentRow>,
    row: &AgentStateRow,
    depth: usize,
    children: &HashMap<&str, Vec<&AgentStateRow>>,
    names: &HashMap<&str, &str>,
) {
    // A malformed frame could in principle cycle; the engine builds these ids
    // deterministically, but a depth cap costs nothing and cannot hang a paint.
    const MAX_DEPTH: usize = 16;
    out.push(AgentRow {
        id: row.id.clone(),
        chat_id: row.chat_id.clone(),
        name: row
            .label
            .clone()
            .or_else(|| names.get(row.id.as_str()).map(|n| (*n).to_string()))
            .unwrap_or_else(|| UNTITLED_CHAT.to_string()),
        depth,
        state: row.state,
        rolled_up: row.rolled_up,
        needs_you_children: row.needs_you_children,
        rolled_up_only: !row.state.needs_you() && row.rolled_up.needs_you(),
    });
    if depth >= MAX_DEPTH {
        return;
    }
    for child in children.get(row.id.as_str()).into_iter().flatten() {
        push_subtree(out, child, depth + 1, children, names);
    }
}
