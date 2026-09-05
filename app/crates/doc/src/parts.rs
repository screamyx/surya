//! Message parts: the event fold, the render-only privacy policy, and continuation splitting.
//!
//! Ports of `packages/control/src/parts.ts` (fold) and
//! `packages/session-doc/src/{render-parts,messages}.ts`.

use serde::{Deserialize, Serialize};

use zeron_proto::{AgentEvent, SUBAGENT_INPUT_KEEP, ToolCall, ToolDiff, UserInputQuestion};

use crate::constants::MSG_INLINE_MAX;

/// Char cap for the tool-output SUMMARY persisted into the doc: first
/// non-empty line, nothing more. The per-part 4KB cap (c951c3e) bounded each
/// part but not the session — chat 1b65e93d measured 917KB (85%) of a 1MB doc
/// in capped outputs across 426 tool parts (docs/chat2-sync.md). Full outputs
/// live in the R2 sidecar behind `output_ref`; t3code ships an 84-char
/// summary, so 160 is generous.
pub const TOOL_OUTPUT_SUMMARY_MAX: usize = 160;

/// The doc-resident form of a tool output (docs/chat2-sync.md A1; the R2
/// sidecar is PARKED as of 2026-08-10, so this IS the whole record in the
/// doc — the full text survives only in the host's local run journal):
///
/// - Markdown code fences are stripped first — ACP harnesses fence every
///   output, so the fence is transport wrapping, never content (pre-fix,
///   every summary read "```console…").
/// - Outputs that fit [`TOOL_OUTPUT_SUMMARY_MAX`] chars ride whole — a
///   summary of a two-line output is more UI than the output.
/// - Bigger outputs keep the first non-empty line, capped, with a `…`
///   marker meaning "there was more".
///
/// `None` for blank output.
pub fn summarize_tool_output(text: &str) -> Option<String> {
    let kept: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("```"))
        .collect();
    let stripped = kept.join("\n");
    let stripped = stripped.trim();
    if stripped.is_empty() {
        return None;
    }
    if stripped.chars().count() <= TOOL_OUTPUT_SUMMARY_MAX {
        return Some(stripped.to_owned());
    }
    // Too big to inline: first non-empty line, capped. There is always more
    // than the summary here (whole output exceeded the budget), so the
    // marker is unconditional.
    let line = stripped
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(stripped)
        .trim_end();
    let mut chars = 0usize;
    let mut end = line.len();
    for (i, _) in line.char_indices() {
        if chars == TOOL_OUTPUT_SUMMARY_MAX {
            end = i;
            break;
        }
        chars += 1;
    }
    let mut out = line[..end].to_owned();
    out.push('…');
    Some(out)
}

/// Per-file diff stats persisted in place of inline diff text (t3's shape).
/// The inline diff was the bigger bomb than outputs — 32KB/edit, unexercised
/// only because the claude harness emits none. Full diff text lives in the
/// sidecar behind `diff_ref`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDiffStat {
    pub path: String,
    pub additions: u64,
    pub deletions: u64,
}

/// Line-level add/delete counts for one file's diff.
pub fn diff_stat(diff: &ToolDiff) -> ToolDiffStat {
    let (additions, deletions) = match &diff.old_text {
        None => (diff.new_text.lines().count() as u64, 0),
        Some(old) => {
            let text_diff = similar::TextDiff::from_lines(old.as_str(), diff.new_text.as_str());
            let mut additions = 0u64;
            let mut deletions = 0u64;
            for change in text_diff.iter_all_changes() {
                match change.tag() {
                    similar::ChangeTag::Insert => additions += 1,
                    similar::ChangeTag::Delete => deletions += 1,
                    similar::ChangeTag::Equal => {}
                }
            }
            (additions, deletions)
        }
    };
    ToolDiffStat {
        path: diff.path.clone(),
        additions,
        deletions,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageStatus {
    Streaming,
    Complete,
    Aborted,
}

/// Lifecycle of a spawned subagent, carried on its spawn chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubagentStatus {
    Running,
    Done,
    Failed,
}

/// One rendered part of an assistant message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MessagePart {
    Text {
        id: String,
        text: String,
    },
    /// Model thinking. Carries its body in a dedicated doc field (`reasoning`,
    /// never `text`) so pre-reasoning desktop builds — whose unknown-kind
    /// fallback renders `text` as prose — degrade to an invisible empty text
    /// part instead of leaking raw thinking into the transcript. iOS drops
    /// unknown kinds entirely.
    Reasoning {
        id: String,
        text: String,
    },
    #[serde(rename_all = "camelCase")]
    Tool {
        id: String,
        call: ToolCall,
        #[serde(default)]
        is_error: bool,
        /// True once a ToolResult arrived.
        #[serde(default)]
        resolved: bool,
        /// One-line tool output summary ([`summarize_tool_output`]). Old
        /// entries (pre-strip) still carry up to 4KB here; old app versions
        /// render this field either way, so the strip is invisible to them.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        /// Inline file diff — written by pre-strip app versions only; new
        /// folds persist [`Self::Tool::diff_stats`] + `diff_ref` instead.
        /// Kept so old docs render their diffs.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        diff: Option<ToolDiff>,
        /// Sidecar key (`{chatId}/{partId}`) of the full output — additive;
        /// stamped by [`apply_sidecar_refs`] (the fold is chat-agnostic).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_ref: Option<String>,
        /// Full-output byte length, so the UI can say "Show full output (12 KB)".
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_bytes: Option<u64>,
        /// Sidecar key (`{chatId}/{partId}.diff`) of the full diff JSON.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        diff_ref: Option<String>,
        /// Per-file diff stats (additive replacement for inline `diff`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        diff_stats: Option<Vec<ToolDiffStat>>,
        /// The SUBAGENT doc id this spawn chip indexes (additive; stamped by
        /// the engine like the sidecar refs — the fold is chat-agnostic).
        /// The chip IS the index: the client learns the doc/blob id from it,
        /// there is no listing endpoint.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subagent_ref: Option<String>,
        /// Subagent lifecycle, DISTINCT from `resolved`: under the eager-done
        /// policy the spawn tool's own result lands while the subagent still
        /// runs. `running` → the ref is a live doc (watch it); `done`/
        /// `failed` → frozen (blob first, doc as fallback).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subagent_status: Option<SubagentStatus>,
        /// One-line live tail of the subagent's latest output, folded from
        /// its tagged text deltas (capped; display-only).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subagent_tail: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Input {
        id: String,
        request_id: String,
        questions: Vec<UserInputQuestion>,
        #[serde(default)]
        resolved: bool,
    },
    /// A tool blocked on the user's permission.
    ///
    /// The twin of [`MessagePart::Input`], and deliberately so: a permission
    /// is a question the agent is asking, and the user should not have to
    /// learn two shapes for "something is waiting on you". It appears when
    /// the ask arrives and flips to resolved when it is answered, which
    /// leaves the answer in the transcript - a tool that did not run
    /// otherwise leaves no trace at all.
    ///
    /// Comet auto-approved every tool and never emitted the event, so a doc
    /// written by an older build simply has no part of this kind, and an
    /// older reader falls through its `default` arm.
    Permission {
        id: String,
        request_id: String,
        tool_name: String,
        /// The command or path the tool would act on, already rendered.
        #[serde(default)]
        command: String,
        #[serde(default)]
        resolved: bool,
        /// How it was answered, once it was.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        decision: Option<zeron_proto::PermissionDecision>,
        /// Why, for a deny.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Error {
        id: String,
        message: String,
    },
    /// A system line: not the agent speaking, but the app saying what it did.
    /// Today that is "allowed by rule <name>" when an always-allow rule
    /// answered a permission without waking anyone (surya decision 20).
    /// Carries its body in `text` on purpose — a client that does not know
    /// this kind falls back to rendering `text` as prose, which is exactly
    /// the sentence we want it to show.
    Notice {
        id: String,
        text: String,
    },
    /// An agent-drawn A2UI card (surya decision 4). `a2ui` is the envelope
    /// list the UI parses with `surya-a2ui`; `id` is the tool_use id of the
    /// `show_card` call that drew it. Large cards follow the tool-output
    /// pattern: `a2ui` omitted, `a2ui_ref` + `a2ui_bytes` set (sidecar,
    /// not wired yet). Old readers degrade the part to invisible empty
    /// text (unknown doc kind).
    #[serde(rename_all = "camelCase")]
    Card {
        id: String,
        card_id: String,
        surface_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        a2ui: Option<Vec<serde_json::Value>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        a2ui_ref: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        a2ui_bytes: Option<u64>,
    },
}

impl MessagePart {
    pub fn id(&self) -> &str {
        match self {
            MessagePart::Text { id, .. }
            | MessagePart::Reasoning { id, .. }
            | MessagePart::Tool { id, .. }
            | MessagePart::Input { id, .. }
            | MessagePart::Permission { id, .. }
            | MessagePart::Notice { id, .. }
            | MessagePart::Error { id, .. }
            | MessagePart::Card { id, .. } => id,
        }
    }

    pub fn byte_len(&self) -> usize {
        match self {
            MessagePart::Text { text, .. }
            | MessagePart::Reasoning { text, .. }
            | MessagePart::Notice { text, .. } => text.len(),
            MessagePart::Tool {
                call,
                output,
                diff,
                diff_stats,
                ..
            } => {
                serde_json::to_vec(call).map_or(0, |v| v.len())
                    + output.as_ref().map_or(0, String::len)
                    + diff
                        .as_ref()
                        .map_or(0, |d| serde_json::to_vec(d).map_or(0, |v| v.len()))
                    + diff_stats
                        .as_ref()
                        .map_or(0, |s| serde_json::to_vec(s).map_or(0, |v| v.len()))
            }
            MessagePart::Input { questions, .. } => {
                serde_json::to_vec(questions).map_or(0, |v| v.len())
            }
            MessagePart::Permission {
                tool_name,
                command,
                reason,
                ..
            } => tool_name.len() + command.len() + reason.as_ref().map_or(0, String::len),
            MessagePart::Error { message, .. } => message.len(),
            MessagePart::Card { a2ui, .. } => a2ui
                .as_ref()
                .map_or(0, |v| serde_json::to_vec(v).map_or(0, |b| b.len())),
        }
    }
}

/// Fold one agent event into a parts accumulator, in place.
///
/// In place because the fold runs once per streamed event: rebuilding the
/// accumulator each time made long turns O(n²) in allocations.
///
/// Semantics from zeron `foldEventIntoParts`:
/// - `SessionStarted` / `Steered` reset the accumulator (turn boundary — makes replay safe).
/// - `TextDelta` appends to the trailing text part, or starts a new one if the trail is not text
///   (a tool call in between breaks the text block).
/// - `ToolCall` appends, or refreshes in place when the id already exists (SDK retry idempotence).
/// - `ToolResult` marks the matching tool part resolved / errored in place.
/// - `InputRequested` appends an input part; `InputResolved` marks it resolved.
/// - `Error` and `Done{error}` become visible error parts.
pub fn fold_event_into_parts(out: &mut Vec<MessagePart>, event: &AgentEvent) {
    match event {
        AgentEvent::SessionStarted { .. } | AgentEvent::Steered { .. } => {
            out.clear();
        }
        AgentEvent::TextDelta { text } => {
            if let Some(MessagePart::Text { text: tail, .. }) = out.last_mut() {
                tail.push_str(text);
            } else {
                let id = format!("t{}", out.len());
                out.push(MessagePart::Text {
                    id,
                    text: text.clone(),
                });
            }
        }
        AgentEvent::ReasoningDelta { text } => {
            // Empty deltas are liveness heartbeats (redacted thinking) — the
            // engine drops them before the fold, but stay tolerant here too.
            if text.is_empty() {
                return;
            }
            if let Some(MessagePart::Reasoning { text: tail, .. }) = out.last_mut() {
                tail.push_str(text);
            } else {
                let id = format!("r{}", out.len());
                out.push(MessagePart::Reasoning {
                    id,
                    text: text.clone(),
                });
            }
        }
        AgentEvent::ToolCall { id, call } => {
            if let Some(existing) = out.iter_mut().find_map(|p| match p {
                MessagePart::Tool {
                    id: pid, call: c, ..
                } if pid == id => Some(c),
                _ => None,
            }) {
                *existing = call.clone();
            } else {
                out.push(MessagePart::Tool {
                    id: id.clone(),
                    call: call.clone(),
                    is_error: false,
                    resolved: false,
                    output: None,
                    diff: None,
                    output_ref: None,
                    output_bytes: None,
                    diff_ref: None,
                    diff_stats: None,
                    subagent_ref: None,
                    subagent_status: None,
                    subagent_tail: None,
                });
            }
        }
        AgentEvent::ToolResult {
            id,
            is_error,
            output,
            diff,
        } => {
            for p in out.iter_mut() {
                if let MessagePart::Tool {
                    id: pid,
                    is_error: e,
                    resolved,
                    output: out_slot,
                    diff: diff_slot,
                    output_bytes,
                    diff_stats,
                    ..
                } = p
                    && pid == id
                {
                    *e = *is_error;
                    *resolved = true;
                    // Tool OUTPUTS never enter the doc (2026-08-10 product
                    // call: chips are one-liners — name + call info — like
                    // pre-output builds; the R2 sidecar is parked with them,
                    // docs/chat2-sync.md A2). Full text lives only in the
                    // host's run journal. Inline diffs die the same way:
                    // stats only, never text. `is_error` still folds so
                    // failed chips read as failed.
                    let _ = output; // journal-only
                    *out_slot = None;
                    *output_bytes = None;
                    *diff_slot = None;
                    *diff_stats = diff.as_ref().map(|d| vec![diff_stat(d)]);
                }
            }
        }
        AgentEvent::InputRequested {
            request_id,
            questions,
        } => {
            let id = format!("in-{request_id}");
            if !out.iter().any(|p| p.id() == id) {
                out.push(MessagePart::Input {
                    id,
                    request_id: request_id.clone(),
                    questions: questions.clone(),
                    resolved: false,
                });
            }
        }
        AgentEvent::InputResolved { request_id } => {
            for p in out.iter_mut() {
                if let MessagePart::Input {
                    request_id: rid,
                    resolved,
                    ..
                } = p
                    && rid == request_id
                {
                    *resolved = true;
                }
            }
        }
        AgentEvent::PermissionRequested {
            request_id,
            tool_name,
            command,
            ..
        } => {
            let id = format!("perm-{request_id}");
            if !out.iter().any(|p| p.id() == id) {
                out.push(MessagePart::Permission {
                    id,
                    request_id: request_id.clone(),
                    tool_name: tool_name.clone(),
                    command: command.clone(),
                    resolved: false,
                    decision: None,
                    reason: None,
                });
            }
        }
        AgentEvent::Error { message } => {
            let id = format!("e{}", out.len());
            out.push(MessagePart::Error {
                id,
                message: message.clone(),
            });
        }
        AgentEvent::Done { error, .. } => {
            if let Some(message) = error {
                let id = format!("e{}", out.len());
                out.push(MessagePart::Error {
                    id,
                    message: message.clone(),
                });
            }
        }
        // A card refreshes in place when its id already exists (the harness
        // re-emits on retry, like ToolCall), otherwise appends. It breaks the
        // trailing text block the way a tool call does.
        AgentEvent::Card {
            card_id,
            surface_id,
            tool_use_id,
            a2ui,
        } => {
            if let Some(existing) = out.iter_mut().find(|p| p.id() == tool_use_id) {
                *existing = MessagePart::Card {
                    id: tool_use_id.clone(),
                    card_id: card_id.clone(),
                    surface_id: surface_id.clone(),
                    a2ui: Some(a2ui.clone()),
                    a2ui_ref: None,
                    a2ui_bytes: None,
                };
            } else {
                out.push(MessagePart::Card {
                    id: tool_use_id.clone(),
                    card_id: card_id.clone(),
                    surface_id: surface_id.clone(),
                    a2ui: Some(a2ui.clone()),
                    a2ui_ref: None,
                    a2ui_bytes: None,
                });
            }
        }
        // Subagent-attributed CONTENT belongs to the subagent's own doc (the
        // engine routes it there); the parent doc keeps only the spawn chip —
        // which these events refresh in place: LIFECYCLE ONLY. A live tail
        // was tried and rejected: rewriting the chip per delta batch grew
        // the parent doc's oplog for the whole subagent run and rendered as
        // distracting mid-stream fragments (user call, 2026-08-18). The
        // `subagent_tail` field stays in the schema for docs that carry it.
        AgentEvent::Subagent {
            parent_tool_use_id,
            event,
        } => {
            let status = match event.as_ref() {
                AgentEvent::Done { status, .. } => Some(match status {
                    zeron_proto::DoneStatus::Errored => SubagentStatus::Failed,
                    _ => SubagentStatus::Done,
                }),
                // A steer RESURRECTS a settled chip — it announces more work
                // (claude: a queued SendMessage relaunches the agent), so
                // this is the one event allowed past the no-regress guard.
                AgentEvent::UserMessage { .. } => Some(SubagentStatus::Running),
                _ => None,
            };
            for p in out.iter_mut() {
                if let MessagePart::Tool {
                    id,
                    call,
                    subagent_status,
                    ..
                } = p
                    && id == parent_tool_use_id
                    // Genus gate: only a SPAWN call ever carries subagent
                    // lifecycle. A driver keying bug (claude's background
                    // shells settled through the subagent subtype,
                    // 2026-08-20) must not decorate an ordinary chip.
                    && call.is_subagent_spawn()
                {
                    match status {
                        Some(s) => *subagent_status = Some(s),
                        // Any tagged traffic proves the subagent is live;
                        // never regress a terminal state.
                        None if !matches!(
                            subagent_status,
                            Some(SubagentStatus::Done) | Some(SubagentStatus::Failed)
                        ) =>
                        {
                            *subagent_status = Some(SubagentStatus::Running);
                        }
                        None => {}
                    }
                }
            }
        }
        // AvailableCommands feeds the engine's per-harness command cache, not
        // the transcript. UserMessage becomes its own doc ENTRY (the engine's
        // subagent sink writes it), never a part of the assistant message.
        // A permission the user still has to answer is NOT a transcript
        // part: it lives in the needs-you inbox until it is answered
        // (surya decision 20). What belongs here is the record that a rule
        // answered one on the user's behalf, so the transcript never has a
        // silent gap where a tool was approved.
        AgentEvent::PermissionResolved {
            request_id,
            decision,
            rule,
            reason,
        } => {
            // Mark the ask, if the user was asked. An auto-allowed tool
            // resolves without ever having been asked, so there is nothing
            // to mark and nothing should have appeared.
            let mut asked = false;
            for p in out.iter_mut() {
                if let MessagePart::Permission {
                    request_id: rid,
                    resolved,
                    decision: answered,
                    reason: why,
                    ..
                } = p
                    && rid == request_id
                {
                    asked = true;
                    *resolved = true;
                    *answered = Some(*decision);
                    *why = reason.clone();
                }
            }
            // The chip above states the outcome, so a second line about the
            // same answer would say it twice. What is left for a notice is
            // the case with no chip at all: a rule that allowed the tool
            // without asking, which the user should still see happen.
            let text = if asked {
                None
            } else {
                match (decision, rule, reason) {
                (zeron_proto::PermissionDecision::Allow, Some(rule), _) => {
                    Some(format!("allowed by rule {rule}"))
                }
                (zeron_proto::PermissionDecision::Deny, _, Some(reason)) => {
                    Some(format!("not allowed: {reason}"))
                }
                (zeron_proto::PermissionDecision::Deny, _, None) => {
                    Some("not allowed".to_string())
                }
                (zeron_proto::PermissionDecision::Allow, None, _) => None,
                }
            };
            if let Some(text) = text {
                let id = format!("{request_id}-permission");
                if !out.iter().any(
                    |p| matches!(p, MessagePart::Notice { id: existing, .. } if existing == &id),
                ) {
                    out.push(MessagePart::Notice { id, text });
                }
            }
        }
        AgentEvent::AssistantMessageCompleted { .. }
        | AgentEvent::Usage { .. }
        | AgentEvent::AvailableCommands { .. }
        | AgentEvent::PermissionRequested { .. }
        | AgentEvent::UserMessage { .. } => {}
    }
}

/// Stamp sidecar keys onto resolved tool parts that have sidecar content.
///
/// Separate from the fold because the fold is chat-agnostic and pure; the
/// caller (who knows the chat id) runs this right after each fold step, before
/// the parts hit the doc. Idempotent. Key shape `{chatId}/{partId}` (+
/// `.diff`) matches the edge's `/blob/{chatId}/{partId}` route.
pub fn apply_sidecar_refs(chat_id: &str, parts: &mut [MessagePart]) {
    for part in parts.iter_mut() {
        if let MessagePart::Tool {
            id,
            resolved: true,
            output_ref,
            output_bytes,
            diff_ref,
            diff_stats,
            ..
        } = part
        {
            if output_ref.is_none() && output_bytes.is_some() {
                *output_ref = Some(format!("{chat_id}/{id}"));
            }
            if diff_ref.is_none() && diff_stats.is_some() {
                *diff_ref = Some(format!("{chat_id}/{id}.diff"));
            }
        }
    }
}

/// What a [`AgentEvent::ToolResult`] owes the sidecar: the full output text
/// and/or the full diff (as JSON), keyed by part id. `None` when the event
/// carries nothing worth uploading.
#[derive(Debug, Clone, PartialEq)]
pub struct SidecarPayload {
    pub part_id: String,
    pub output: Option<String>,
    pub diff: Option<ToolDiff>,
}

pub fn sidecar_payload(event: &AgentEvent) -> Option<SidecarPayload> {
    let AgentEvent::ToolResult {
        id, output, diff, ..
    } = event
    else {
        return None;
    };
    let output = output.clone().filter(|o| !o.trim().is_empty());
    if output.is_none() && diff.is_none() {
        return None;
    }
    Some(SidecarPayload {
        part_id: id.clone(),
        output,
        diff: diff.clone(),
    })
}

/// Render-only privacy policy — strip heavy/sensitive tool inputs before a call enters the doc.
///
/// Keeps: command / path / pattern / url / query / todo items / server+tool names,
/// and a subagent spawn's model/type (see [`spawn_badge`] — a couple of short
/// identifiers the chip names the child by).
/// Drops: WriteFile content, EditFile old/new strings, WebFetch prompt, Mcp/Unknown input.
/// Full inputs remain only in the host's local run journal. Idempotent.
pub fn sanitize_tool_call(call: &ToolCall) -> ToolCall {
    match call {
        ToolCall::WriteFile { path, .. } => ToolCall::WriteFile {
            path: path.clone(),
            content: None,
        },
        ToolCall::EditFile { path, .. } => ToolCall::EditFile {
            path: path.clone(),
            old_string: None,
            new_string: None,
        },
        ToolCall::WebFetch { url, .. } => ToolCall::WebFetch {
            url: url.clone(),
            prompt: None,
        },
        ToolCall::Mcp { server, tool, .. } => ToolCall::Mcp {
            server: server.clone(),
            tool: tool.clone(),
            input: spawn_badge(call),
        },
        ToolCall::Unknown { name, .. } => ToolCall::Unknown {
            name: name.clone(),
            input: spawn_badge(call),
        },
        other => other.clone(),
    }
}

/// The only slice of a tool input allowed into the doc: a subagent spawn's
/// [`SUBAGENT_INPUT_KEEP`] keys, so the chip can say WHICH model the child
/// runs on (`Agent · haiku`) without the reader opening the subagent tab.
///
/// Everything else — the prompt above all — stays in the host's run journal,
/// so this stays a whitelist of short identifiers rather than a size cap.
/// `None` for anything that is not a spawn, and for a spawn that named
/// neither, which keeps it idempotent: re-sanitizing a sanitized call is a
/// fixpoint (the kept keys are themselves kept).
fn spawn_badge(call: &ToolCall) -> Option<serde_json::Value> {
    if !call.is_subagent_spawn() {
        return None;
    }
    let input = match call {
        ToolCall::Unknown { input, .. } | ToolCall::Mcp { input, .. } => input.as_ref()?,
        _ => return None,
    };
    let kept: serde_json::Map<String, serde_json::Value> = SUBAGENT_INPUT_KEEP
        .iter()
        .filter_map(|key| {
            let value = input.get(key)?.as_str()?.trim();
            (!value.is_empty()).then(|| ((*key).to_owned(), serde_json::Value::from(value)))
        })
        .collect();
    (!kept.is_empty()).then(|| serde_json::Value::Object(kept))
}

/// Deterministic continuation id: `"{root}#c{n}"`.
pub fn continuation_id(root: &str, index: usize) -> String {
    format!("{root}#c{index}")
}

/// Split an oversized parts list into chunks each under `MSG_INLINE_MAX` bytes.
///
/// Splitting happens at part boundaries; an oversized text part is itself chunked at char
/// boundaries. Returns one Vec per resulting entry — the first keeps the root id, the rest are
/// continuations (`continuation_id(root, i)`), matching `splitMessageEntry` in zeron.
pub fn split_parts(parts: &[MessagePart]) -> Vec<Vec<MessagePart>> {
    let mut chunks: Vec<Vec<MessagePart>> = vec![Vec::new()];
    let mut current_bytes = 0usize;

    let push_part = |chunks: &mut Vec<Vec<MessagePart>>, current: &mut usize, part: MessagePart| {
        let len = part.byte_len();
        if *current > 0 && *current + len > MSG_INLINE_MAX {
            chunks.push(Vec::new());
            *current = 0;
        }
        *current += len;
        chunks.last_mut().unwrap().push(part);
    };

    for part in parts {
        // Both text-bodied kinds chunk the same way; extended thinking can
        // exceed the cap just as easily as a long reply.
        let (id, text, reasoning) = match part {
            MessagePart::Text { id, text } if text.len() > MSG_INLINE_MAX => (id, text, false),
            MessagePart::Reasoning { id, text } if text.len() > MSG_INLINE_MAX => (id, text, true),
            other => {
                push_part(&mut chunks, &mut current_bytes, other.clone());
                continue;
            }
        };
        // Chunk oversized text at char boundaries.
        let mut start = 0usize;
        let mut piece = 0usize;
        while start < text.len() {
            let mut end = (start + MSG_INLINE_MAX).min(text.len());
            while end < text.len() && !text.is_char_boundary(end) {
                end -= 1;
            }
            // Guard: ensure forward progress on pathological boundaries.
            if end <= start {
                end = text.len();
            }
            let sub_id = if piece == 0 {
                id.clone()
            } else {
                format!("{id}~{piece}")
            };
            let body = text[start..end].to_string();
            let sub = if reasoning {
                MessagePart::Reasoning {
                    id: sub_id,
                    text: body,
                }
            } else {
                MessagePart::Text {
                    id: sub_id,
                    text: body,
                }
            };
            push_part(&mut chunks, &mut current_bytes, sub);
            start = end;
            piece += 1;
        }
    }
    chunks
}

/// Render-time inverse of splitting: concatenate continuation entries' parts in list order.
pub fn join_continuations(entries: Vec<Vec<MessagePart>>) -> Vec<MessagePart> {
    entries.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {

    /// A tool that did NOT run leaves no other trace in the transcript, so
    /// the refusal has to say so itself — otherwise the turn just has a
    /// silent gap where the user said no.
    #[test]
    fn the_ask_is_a_part_and_the_answer_lands_on_it() {
        use zeron_proto::PermissionDecision;
        let mut parts = Vec::new();
        // The ask used to live only in the needs-you inbox. The owner
        // reversed that on 2026-09-05: a tool waiting on you belongs in the
        // conversation, where the user is already looking.
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::PermissionRequested {
                request_id: "perm-1".into(),
                tool_name: "Bash".into(),
                command: "rm -rf /".into(),
                input: None,
            },
        );
        assert!(matches!(
            &parts[0],
            MessagePart::Permission { tool_name, command, resolved: false, .. }
                if tool_name == "Bash" && command == "rm -rf /"
        ));

        // Re-delivery of the ask does not double it.
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::PermissionRequested {
                request_id: "perm-1".into(),
                tool_name: "Bash".into(),
                command: "rm -rf /".into(),
                input: None,
            },
        );
        assert_eq!(parts.len(), 1);

        fold_event_into_parts(
            &mut parts,
            &AgentEvent::PermissionResolved {
                request_id: "perm-1".into(),
                decision: PermissionDecision::Deny,
                rule: None,
                reason: Some("you denied it".into()),
            },
        );
        // One part still, now answered: the chip states the outcome, so the
        // notice that used to carry it would say the same thing twice.
        assert_eq!(parts.len(), 1);
        let MessagePart::Permission {
            resolved,
            decision,
            reason,
            ..
        } = &parts[0]
        else {
            panic!("expected the Permission part, got {:?}", parts[0]);
        };
        assert!(resolved);
        assert_eq!(*decision, Some(PermissionDecision::Deny));
        assert_eq!(reason.as_deref(), Some("you denied it"));
    }

    #[test]
    fn a_deny_nobody_was_asked_for_still_says_something() {
        use zeron_proto::PermissionDecision;
        let mut parts = Vec::new();
        // No ask was folded, so there is no chip to carry the outcome and
        // the notice is the only trace the turn would otherwise have.
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::PermissionResolved {
                request_id: "perm-2".into(),
                decision: PermissionDecision::Deny,
                rule: None,
                reason: None,
            },
        );
        assert!(matches!(
            &parts[0],
            MessagePart::Notice { text, .. } if text == "not allowed"
        ));
    }

    #[test]
    fn a_rule_allow_says_which_rule_and_a_plain_allow_says_nothing() {
        use zeron_proto::PermissionDecision;
        let mut parts = Vec::new();
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::PermissionResolved {
                request_id: "perm-3".into(),
                decision: PermissionDecision::Allow,
                rule: Some("Bash php artisan migrate* in project-jag".into()),
                reason: None,
            },
        );
        assert!(matches!(
            &parts[0],
            MessagePart::Notice { text, .. }
                if text == "allowed by rule Bash php artisan migrate* in project-jag"
        ));

        // A plain Allow the user clicked needs no line: the tool call that
        // follows is the record.
        let mut plain = Vec::new();
        fold_event_into_parts(
            &mut plain,
            &AgentEvent::PermissionResolved {
                request_id: "perm-4".into(),
                decision: PermissionDecision::Allow,
                rule: None,
                reason: None,
            },
        );
        assert!(plain.is_empty());
    }
    use super::*;

    /// A Card event appends a card part, breaks the text block like a tool
    /// call, and refreshes in place when its id repeats (retry idempotence).
    #[test]
    fn card_event_folds_to_a_card_part_and_refreshes_in_place() {
        let mut parts = Vec::new();
        fold_event_into_parts(&mut parts, &AgentEvent::TextDelta { text: "Here:".into() });
        let card = vec![serde_json::json!({"updateComponents": {"surfaceId": "s", "components": [{"id": "root", "component": "Text", "text": "hi"}]}})];
        let event = |a2ui: Vec<serde_json::Value>| AgentEvent::Card {
            card_id: "card-1".into(),
            surface_id: "s".into(),
            tool_use_id: "toolu_1".into(),
            a2ui,
        };
        fold_event_into_parts(&mut parts, &event(card.clone()));
        fold_event_into_parts(&mut parts, &AgentEvent::TextDelta { text: "after".into() });
        assert_eq!(parts.len(), 3, "{parts:?}");
        assert!(matches!(&parts[1], MessagePart::Card { id, card_id, a2ui, .. } if id == "toolu_1" && card_id == "card-1" && a2ui.as_ref() == Some(&card)));
        assert!(matches!(&parts[2], MessagePart::Text { text, .. } if text == "after"));
        let card2 = vec![serde_json::json!({"updateComponents": {"surfaceId": "s", "components": []}})];
        fold_event_into_parts(&mut parts, &event(card2.clone()));
        assert_eq!(parts.len(), 3);
        assert!(matches!(&parts[1], MessagePart::Card { a2ui, .. } if a2ui.as_ref() == Some(&card2)));
        assert_eq!(parts[1].id(), "toolu_1");
        assert_eq!(parts[1].byte_len(), serde_json::to_vec(&card2).unwrap().len());
    }

    fn text_delta(s: &str) -> AgentEvent {
        AgentEvent::TextDelta { text: s.into() }
    }

    #[test]
    fn reasoning_deltas_fold_into_their_own_part() {
        let mut parts = Vec::new();
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ReasoningDelta {
                text: "let me ".into(),
            },
        );
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ReasoningDelta {
                text: "think".into(),
            },
        );
        // Empty deltas (redacted-thinking heartbeats) never mint a part.
        fold_event_into_parts(&mut parts, &AgentEvent::ReasoningDelta { text: "".into() });
        assert_eq!(parts.len(), 1);
        assert_eq!(
            parts[0],
            MessagePart::Reasoning {
                id: "r0".into(),
                text: "let me think".into()
            }
        );
        // Text breaks the reasoning block; a later thought starts a new part.
        fold_event_into_parts(&mut parts, &text_delta("Answer"));
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ReasoningDelta {
                text: "more".into(),
            },
        );
        assert_eq!(parts.len(), 3);
        assert!(matches!(
            &parts[2],
            MessagePart::Reasoning { id, text } if id == "r2" && text == "more"
        ));
    }

    #[test]
    fn oversized_reasoning_splits_like_text() {
        let big = "x".repeat(MSG_INLINE_MAX + 10);
        let parts = vec![MessagePart::Reasoning {
            id: "r0".into(),
            text: big,
        }];
        let chunks = split_parts(&parts);
        let flat = join_continuations(chunks);
        assert!(flat.len() >= 2, "{}", flat.len());
        assert!(
            flat.iter()
                .all(|p| matches!(p, MessagePart::Reasoning { .. }))
        );
        assert!(flat.iter().all(|p| p.byte_len() <= MSG_INLINE_MAX));
        assert_eq!(flat[1].id(), "r0~1");
    }

    #[test]
    fn text_deltas_merge_until_broken_by_tool() {
        let mut parts = Vec::new();
        fold_event_into_parts(&mut parts, &text_delta("Hello "));
        fold_event_into_parts(&mut parts, &text_delta("world"));
        assert_eq!(parts.len(), 1);
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ToolCall {
                id: "tool-1".into(),
                call: ToolCall::Exec {
                    command: "ls".into(),
                },
            },
        );
        fold_event_into_parts(&mut parts, &text_delta("after"));
        assert_eq!(parts.len(), 3);
        match &parts[2] {
            MessagePart::Text { text, .. } => assert_eq!(text, "after"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn session_started_resets_accumulator() {
        let mut parts = Vec::new();
        fold_event_into_parts(&mut parts, &text_delta("junk"));
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::SessionStarted {
                harness: zeron_proto::HarnessId::Mock,
                model: "m".into(),
                tools: vec![],
                cwd: "/".into(),
                session_id: "s".into(),
                assistant_message_id: "a".into(),
            },
        );
        assert!(parts.is_empty());
    }

    #[test]
    fn tool_call_refresh_is_idempotent() {
        let call = AgentEvent::ToolCall {
            id: "t".into(),
            call: ToolCall::Exec {
                command: "ls".into(),
            },
        };
        let mut once = Vec::new();
        fold_event_into_parts(&mut once, &call);
        let mut twice = once.clone();
        fold_event_into_parts(&mut twice, &call);
        assert_eq!(once, twice);
    }

    #[test]
    fn tool_result_marks_resolution() {
        let mut parts = Vec::new();
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ToolCall {
                id: "t".into(),
                call: ToolCall::Exec {
                    command: "ls".into(),
                },
            },
        );
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ToolResult {
                id: "t".into(),
                is_error: true,
                output: None,
                diff: None,
            },
        );
        match &parts[0] {
            MessagePart::Tool {
                is_error, resolved, ..
            } => {
                assert!(*is_error);
                assert!(*resolved);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn sanitize_strips_heavy_inputs_and_is_idempotent() {
        let call = ToolCall::WriteFile {
            path: "/x".into(),
            content: Some("secret".into()),
        };
        let clean = sanitize_tool_call(&call);
        assert_eq!(
            clean,
            ToolCall::WriteFile {
                path: "/x".into(),
                content: None
            }
        );
        assert_eq!(sanitize_tool_call(&clean), clean);
    }

    /// A spawn keeps the two short identifiers its chip names the child by and
    /// drops the prompt — the whole point of the whitelist. Still a fixpoint.
    #[test]
    fn sanitize_keeps_a_spawns_model_and_drops_its_prompt() {
        let call = ToolCall::Unknown {
            name: "Agent: Explore theme system".into(),
            input: Some(serde_json::json!({
                "description": "Explore theme system",
                "subagent_type": "Explore",
                "model": "haiku",
                "prompt": "a very long private prompt",
            })),
        };
        let clean = sanitize_tool_call(&call);
        assert_eq!(
            clean,
            ToolCall::Unknown {
                name: "Agent: Explore theme system".into(),
                input: Some(serde_json::json!({
                    "model": "haiku",
                    "subagent_type": "Explore",
                })),
            }
        );
        assert_eq!(clean.subagent_model(), Some("haiku"));
        assert_eq!(sanitize_tool_call(&clean), clean);
    }

    /// An ordinary tool's input still goes, even when it happens to carry a
    /// `model` argument — the badge is gated on the spawn genus, not the key.
    #[test]
    fn sanitize_still_strips_a_non_spawn_carrying_a_model_argument() {
        let call = ToolCall::Unknown {
            name: "SomeTool".into(),
            input: Some(serde_json::json!({ "model": "haiku", "prompt": "secret" })),
        };
        assert_eq!(
            sanitize_tool_call(&call),
            ToolCall::Unknown {
                name: "SomeTool".into(),
                input: None,
            }
        );
    }

    /// A spawn that named no model keeps no input at all — `None`, not an
    /// empty object, so the doc gains nothing and the fixpoint is exact.
    #[test]
    fn sanitize_drops_a_spawn_input_that_names_nothing_worth_keeping() {
        let call = ToolCall::Unknown {
            name: "Agent".into(),
            input: Some(serde_json::json!({ "prompt": "secret", "model": "  " })),
        };
        let clean = sanitize_tool_call(&call);
        assert_eq!(
            clean,
            ToolCall::Unknown {
                name: "Agent".into(),
                input: None,
            }
        );
        assert_eq!(clean.subagent_model(), None);
    }

    #[test]
    fn split_and_join_round_trip() {
        let big = "x".repeat(MSG_INLINE_MAX * 2 + 100);
        let parts = vec![
            MessagePart::Text {
                id: "t0".into(),
                text: big.clone(),
            },
            MessagePart::Tool {
                id: "tool-1".into(),
                call: ToolCall::Exec {
                    command: "ls".into(),
                },
                is_error: false,
                resolved: true,
                output: None,
                diff: None,
                output_ref: None,
                output_bytes: None,
                diff_ref: None,
                diff_stats: None,
                subagent_ref: None,
                subagent_status: None,
                subagent_tail: None,
            },
        ];
        let chunks = split_parts(&parts);
        assert!(
            chunks.len() >= 3,
            "expected >=3 chunks, got {}",
            chunks.len()
        );
        for chunk in &chunks {
            let bytes: usize = chunk.iter().map(|p| p.byte_len()).sum();
            assert!(bytes <= MSG_INLINE_MAX, "chunk over cap: {bytes}");
        }
        let joined = join_continuations(chunks);
        let text: String = joined
            .iter()
            .filter_map(|p| match p {
                MessagePart::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, big);
        assert!(matches!(joined.last().unwrap(), MessagePart::Tool { .. }));
    }

    #[test]
    fn continuation_ids_are_deterministic() {
        assert_eq!(continuation_id("m1", 1), "m1#c1");
    }

    // ── A1 strip (docs/chat2-sync.md) ───────────────────────────────────────

    #[test]
    fn summarize_inlines_small_outputs_and_marks_big_cuts() {
        assert_eq!(summarize_tool_output(""), None);
        assert_eq!(summarize_tool_output("  \n\t\n"), None);
        assert_eq!(summarize_tool_output("one line"), Some("one line".into()));
        // Small multi-line outputs ride whole — no summary, no "…".
        assert_eq!(
            summarize_tool_output("\n\nfirst real\nsecond"),
            Some("first real\nsecond".into())
        );
        assert_eq!(
            summarize_tool_output("only line\n\n  \n"),
            Some("only line".into())
        );
        // Markdown fences are transport wrapping, never content: stripped
        // even when they'd otherwise be the first line, and a fence-only
        // output is blank.
        assert_eq!(
            summarize_tool_output("```console\nreal content\n```"),
            Some("real content".into())
        );
        assert_eq!(summarize_tool_output("```\n```"), None);
        // Big outputs: first non-empty (post-fence) line + unconditional "…".
        let big = format!("```console\nhead line\n{}\n```", "x".repeat(300));
        assert_eq!(summarize_tool_output(&big), Some("head line…".into()));
        let long = "x".repeat(TOOL_OUTPUT_SUMMARY_MAX + 40);
        let summary = summarize_tool_output(&long).unwrap();
        assert_eq!(summary.chars().count(), TOOL_OUTPUT_SUMMARY_MAX + 1);
        assert!(summary.ends_with('…'));
        // Char-boundary safety on multibyte input.
        let wide = "é".repeat(TOOL_OUTPUT_SUMMARY_MAX + 5);
        let summary = summarize_tool_output(&wide).unwrap();
        assert_eq!(summary.chars().count(), TOOL_OUTPUT_SUMMARY_MAX + 1);
    }

    #[test]
    fn diff_stat_counts_line_changes() {
        let stat = diff_stat(&ToolDiff {
            path: "/w/a.rs".into(),
            old_text: Some("a\nb\nc\n".into()),
            new_text: "a\nB\nc\nd\n".into(),
        });
        assert_eq!(stat.path, "/w/a.rs");
        assert_eq!(stat.additions, 2); // B + d
        assert_eq!(stat.deletions, 1); // b
        // New file: every line is an addition.
        let stat = diff_stat(&ToolDiff {
            path: "/w/new.rs".into(),
            old_text: None,
            new_text: "one\ntwo\n".into(),
        });
        assert_eq!((stat.additions, stat.deletions), (2, 0));
    }

    #[test]
    fn fold_strips_output_to_summary_and_diff_to_stats() {
        let mut parts = Vec::new();
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ToolCall {
                id: "t".into(),
                call: ToolCall::Exec {
                    command: "cargo test".into(),
                },
            },
        );
        let full = "running 42 tests\n".repeat(300); // ~5KB, was 4KB inline pre-strip
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ToolResult {
                id: "t".into(),
                is_error: false,
                output: Some(full.clone()),
                diff: Some(ToolDiff {
                    path: "/w/a.rs".into(),
                    old_text: Some("a\n".into()),
                    new_text: "b\n".into(),
                }),
            },
        );
        match &parts[0] {
            MessagePart::Tool {
                output,
                output_bytes,
                diff,
                diff_stats,
                ..
            } => {
                // One-liner chips: outputs never enter the doc at all
                // (journal-only); diff text neither — stats survive.
                assert_eq!(output.as_deref(), None);
                assert_eq!(*output_bytes, None);
                assert!(diff.is_none(), "inline diff text must not enter the doc");
                let stats = diff_stats.as_ref().unwrap();
                assert_eq!(stats.len(), 1);
                assert_eq!((stats[0].additions, stats[0].deletions), (1, 1));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn sidecar_refs_stamp_once_and_only_where_content_exists() {
        let mut parts = Vec::new();
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ToolCall {
                id: "t1".into(),
                call: ToolCall::Exec {
                    command: "ls".into(),
                },
            },
        );
        // Unresolved: no refs yet.
        apply_sidecar_refs("chat-9", &mut parts);
        assert!(matches!(
            &parts[0],
            MessagePart::Tool {
                output_ref: None,
                diff_ref: None,
                ..
            }
        ));
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ToolResult {
                id: "t1".into(),
                is_error: false,
                output: Some("hello".into()),
                diff: Some(ToolDiff {
                    path: "/w/a".into(),
                    old_text: None,
                    new_text: "x\n".into(),
                }),
            },
        );
        apply_sidecar_refs("chat-9", &mut parts);
        apply_sidecar_refs("chat-9", &mut parts); // idempotent
        match &parts[0] {
            MessagePart::Tool {
                output_ref,
                diff_ref,
                ..
            } => {
                // One-liner fold: outputs never reach the doc, so there is
                // no output content to key even after resolution; diff
                // STATS exist, so the diff ref still stamps.
                assert_eq!(output_ref.as_deref(), None);
                assert_eq!(diff_ref.as_deref(), Some("chat-9/t1.diff"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn subagent_events_refresh_the_spawn_chip_in_place() {
        use zeron_proto::DoneStatus;
        let mut parts = Vec::new();
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ToolCall {
                id: "toolu_sub".into(),
                call: ToolCall::Unknown {
                    name: "Agent".into(),
                    input: None,
                },
            },
        );
        // Tagged traffic marks the chip running — and ONLY that: the tail
        // stays untouched (per-delta chip rewrites polluted the parent doc).
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::Subagent {
                parent_tool_use_id: "toolu_sub".into(),
                event: Box::new(AgentEvent::TextDelta {
                    text: "scanning\nfound 3 issues".into(),
                }),
            },
        );
        match &parts[0] {
            MessagePart::Tool {
                subagent_status,
                subagent_tail,
                ..
            } => {
                assert_eq!(*subagent_status, Some(SubagentStatus::Running));
                assert_eq!(*subagent_tail, None);
            }
            other => panic!("{other:?}"),
        }
        // A tagged Done is terminal; later traffic must not regress it.
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::Subagent {
                parent_tool_use_id: "toolu_sub".into(),
                event: Box::new(AgentEvent::Done {
                    status: DoneStatus::Completed,
                    result: None,
                    error: None,
                    session_id: None,
                }),
            },
        );
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::Subagent {
                parent_tool_use_id: "toolu_sub".into(),
                event: Box::new(AgentEvent::TextDelta {
                    text: "late flush".into(),
                }),
            },
        );
        match &parts[0] {
            MessagePart::Tool {
                subagent_status, ..
            } => assert_eq!(*subagent_status, Some(SubagentStatus::Done)),
            other => panic!("{other:?}"),
        }
        // Content never leaked into the parent parts.
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn subagent_events_never_decorate_a_non_spawn_chip() {
        // Mis-keyed tagged traffic (claude's background shells settled
        // through the subagent subtype, 2026-08-20) must not stamp lifecycle
        // onto an ordinary tool chip — the genus gate is the CALL.
        use zeron_proto::DoneStatus;
        let mut parts = Vec::new();
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::ToolCall {
                id: "toolu_bash".into(),
                call: ToolCall::Exec {
                    command: "git clone …".into(),
                },
            },
        );
        fold_event_into_parts(
            &mut parts,
            &AgentEvent::Subagent {
                parent_tool_use_id: "toolu_bash".into(),
                event: Box::new(AgentEvent::Done {
                    status: DoneStatus::Completed,
                    result: None,
                    error: None,
                    session_id: None,
                }),
            },
        );
        match &parts[0] {
            MessagePart::Tool {
                subagent_status, ..
            } => assert_eq!(*subagent_status, None),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn sidecar_payload_carries_full_texts() {
        assert_eq!(
            sidecar_payload(&AgentEvent::TextDelta { text: "x".into() }),
            None
        );
        assert_eq!(
            sidecar_payload(&AgentEvent::ToolResult {
                id: "t".into(),
                is_error: false,
                output: Some("   \n".into()),
                diff: None,
            }),
            None,
            "blank output uploads nothing"
        );
        let payload = sidecar_payload(&AgentEvent::ToolResult {
            id: "t".into(),
            is_error: true,
            output: Some("full output".into()),
            diff: None,
        })
        .unwrap();
        assert_eq!(payload.part_id, "t");
        assert_eq!(payload.output.as_deref(), Some("full output"));
    }
}
