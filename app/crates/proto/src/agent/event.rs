//! The event stream a harness emits, and the payloads it carries.
//!
//! Split out of `agent.rs` under decision 13, part of surya#159. Moved
//! verbatim: every line below is unchanged from the range it came from, and
//! the parent re-exports the lot, so no caller changes.

use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub text: String,
    pub done: bool,
}

/// A slash command advertised by the agent (ACP `availableCommands`): typed as
/// `/name` at the start of the composer, sent to the agent as prompt text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommand {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Placeholder hint for the command's argument, when it takes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_hint: Option<String>,
}

/// A file modification carried inline on a tool result (ACP
/// `ToolCallContent::Diff`). `old_text: None` means a new file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDiff {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_text: Option<String>,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInputQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    pub options: Vec<String>,
    #[serde(default)]
    pub multi_select: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInputAnswer {
    pub question_id: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DoneStatus {
    Completed,
    Interrupted,
    Errored,
}

/// The normalized streaming event every harness emits.
///
/// Mirrors surya's `AgentEvent` tagged enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentEvent {
    #[serde(rename_all = "camelCase")]
    SessionStarted {
        harness: HarnessId,
        model: String,
        #[serde(default)]
        tools: Vec<String>,
        cwd: String,
        /// Harness-native session id (used for resume).
        session_id: String,
        assistant_message_id: String,
    },
    TextDelta {
        text: String,
    },
    ReasoningDelta {
        text: String,
    },
    /// Backend-internal steering boundary marker.
    #[serde(rename_all = "camelCase")]
    AssistantMessageCompleted {
        assistant_message_id: String,
    },
    ToolCall {
        id: String,
        call: ToolCall,
    },
    #[serde(rename_all = "camelCase")]
    ToolResult {
        id: String,
        is_error: bool,
        /// Tool output text, capped by the emitting harness (ACP tool-call
        /// content; claude/codex adapters never populate it). The doc-side
        /// fold applies its own byte cap before anything persists.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        /// Inline file diff for edit-shaped tools (ACP `Diff` content).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        diff: Option<ToolDiff>,
    },
    /// Kept as a harness passthrough (rate-limit probes); never persisted to docs.
    #[serde(rename_all = "camelCase")]
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    /// The agent advertised (or changed) its slash-command set - ACP
    /// `available_commands_update`. The engine caches the latest list per
    /// harness for the composer's `/` popup; never persisted to docs.
    #[serde(rename_all = "camelCase")]
    AvailableCommands {
        commands: Vec<SlashCommand>,
    },
    Error {
        message: String,
    },
    #[serde(rename_all = "camelCase")]
    InputRequested {
        request_id: String,
        questions: Vec<UserInputQuestion>,
    },
    #[serde(rename_all = "camelCase")]
    InputResolved {
        request_id: String,
    },
    /// A tool is blocked waiting for the user to allow or deny it. surya
    /// decision 20: the permission card is one of the three things that can
    /// put an agent in the needs-you queue. Comet auto-approved these and
    /// never emitted them, so an old consumer simply never sees the variant.
    #[serde(rename_all = "camelCase")]
    PermissionRequested {
        request_id: String,
        tool_name: String,
        /// The command / path the tool would act on, already rendered.
        #[serde(default)]
        command: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input: Option<serde_json::Value>,
    },
    /// The permission above was answered. `rule` names the always-allow rule
    /// that answered it, when a rule did rather than a person.
    #[serde(rename_all = "camelCase")]
    PermissionResolved {
        request_id: String,
        decision: crate::PermissionDecision,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rule: Option<String>,
        /// Why, for a `Deny`. The transcript says this out loud: a tool that
        /// did not run leaves no other trace, and "nothing happened" is not
        /// something the user should have to infer.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        /// The chat's yolo mode answered this one, not a rule and not a
        /// person. Its own field rather than a `rule` name: yolo is not in
        /// the always-allow table, and naming a rule that does not exist
        /// would both mislead the transcript and light the card's by-rule
        /// slot. Additive + serde-defaulted for wire compat.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        yolo: bool,
    },
    #[serde(rename_all = "camelCase")]
    Steered {
        assistant_message_id: Option<String>,
        next_assistant_message_id: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Done {
        status: DoneStatus,
        result: Option<String>,
        error: Option<String>,
        session_id: Option<String>,
    },
    /// A USER-role message injected into a running session - today only seen
    /// wrapped in [`AgentEvent::Subagent`]: the PARENT agent steering its
    /// subagent mid-run (claude: a tagged user frame's text blocks). The
    /// engine writes it to the subagent doc as its own user entry, closing
    /// the streaming assistant segment above it - the subagent transcript
    /// then reads like any steered chat. Never emitted untagged (the parent
    /// chat's user messages come from doc commands, not the wire).
    #[serde(rename_all = "camelCase")]
    UserMessage {
        text: String,
    },
    /// An agent-drawn A2UI card (surya decision 4): the harness saw a
    /// `show_card` tool call and lifted it out as a card instead of a tool
    /// chip. `a2ui` is the A2UI v0.9.1 envelope list (`createSurface`,
    /// `updateComponents`, optional `updateDataModel`) `surya-a2ui` parses;
    /// `card_id` is the card store key (the tool_use id when there is no
    /// store), `surface_id` the A2UI surface, `tool_use_id` the call that
    /// drew it - the doc part keys on the latter so a retry refreshes in
    /// place.
    ///
    /// Emitted on the call's tool_result, not on the call itself, so the
    /// harness can read the sidecar's own record for it; the card takes the
    /// place of the tool chip that would otherwise sit above it.
    #[serde(rename_all = "camelCase")]
    Card {
        card_id: String,
        surface_id: String,
        tool_use_id: String,
        a2ui: Vec<serde_json::Value>,
    },
    /// An event belonging to a SUBAGENT's nested transcript, attributed to
    /// the spawning tool call (`parent_tool_use_id` = the parent-feed
    /// `ToolCall::id` that launched it). Never folded into the parent chat
    /// doc - the engine routes these to the subagent's own doc; the parent
    /// keeps only the spawn chip. Additive: old consumers that don't match
    /// this variant drop the nested traffic, which is the pre-subagent-viz
    /// behavior.
    #[serde(rename_all = "camelCase")]
    Subagent {
        parent_tool_use_id: String,
        event: Box<AgentEvent>,
    },
}
