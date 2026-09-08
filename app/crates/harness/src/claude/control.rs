//! Serving the CLI's `can_use_tool` control requests: the permission gate
//! for ordinary tools, and the `AskUserQuestion` round trip. Moved out of
//! `mod.rs` under the 500-line rule (issue #159); the code itself is
//! unchanged.

use std::sync::Arc;

use serde_json::Value;
use tokio::sync::mpsc;

use surya_proto::{UserInputAnswer, UserInputQuestion};

use super::session::StdinMsg;
use super::surya;
use super::wire::{ControlRequestFrame, allow_response, control_response_line, deny_response};
use crate::permission::PermissionGate;

pub(super) type RequestInputFn = Box<
    dyn Fn(Vec<UserInputQuestion>) -> tokio::sync::oneshot::Receiver<Vec<UserInputAnswer>>
        + Send
        + Sync,
>;

/// Serve one `can_use_tool` control request. Ordinary tools go to the host's
/// [`PermissionGate`] - an always-allow rule answers instantly, otherwise the
/// request parks in the needs-you inbox (surya decision 20). The CLI blocks
/// until SOME response arrives, so every request must be answered: an
/// ungated gate allows, and a gate whose host went away DENIES. Neither
/// hangs, and the one that cannot ask fails closed.
/// `AskUserQuestion` is intercepted instead - surface the questions through
/// the engine's input bridge (which owns the `InputRequested`/`InputResolved`
/// lifecycle), wait for the user's answers (in a subtask so the frame loop
/// keeps flowing), and hand them back keyed by question text.
pub(super) fn handle_control_request(
    req: ControlRequestFrame,
    request_input: &Arc<RequestInputFn>,
    permission: &PermissionGate,
    stdin_tx: &mpsc::UnboundedSender<StdinMsg>,
) {
    if req.request.subtype != "can_use_tool" {
        tracing::debug!(
            target: "surya_harness::claude",
            "unhandled control_request subtype: {}", req.request.subtype
        );
        return;
    }
    if req.request.tool_name != "AskUserQuestion" {
        // Ungated (headless parity), or one of the sidecar's own drawing
        // tools: answer on the spot, no round trip.
        if !permission.is_gated() || surya::is_auto_allowed(&req.request.tool_name) {
            let line = control_response_line(&req.request_id, allow_response(req.request.input));
            let _ = stdin_tx.send(StdinMsg::Line(line));
            return;
        }
        let permission = permission.clone();
        let stdin_tx = stdin_tx.clone();
        tokio::spawn(async move {
            let request = surya_proto::PermissionRequest {
                request_id: req.request_id.clone(),
                tool_name: req.request.tool_name.clone(),
                command: crate::permission::describe_tool_command(Some(&req.request.input)),
                input: Some(req.request.input.clone()),
            };
            let response = match permission.ask(request).await {
                surya_proto::PermissionDecision::Allow => allow_response(req.request.input),
                surya_proto::PermissionDecision::Deny => {
                    deny_response("The user did not allow this tool.")
                }
            };
            let _ = stdin_tx.send(StdinMsg::Line(control_response_line(
                &req.request_id,
                response,
            )));
        });
        return;
    }
    let request_input = Arc::clone(request_input);
    let stdin_tx = stdin_tx.clone();
    tokio::spawn(async move {
        let request_id = req.request_id;
        let input = req.request.input;
        let questions = parse_questions(&input);
        // The engine's input bridge is the SOLE emitter of
        // `InputRequested`/`InputResolved`: it mints the request id, parks the
        // resolver for `respond_input`, and surfaces both events. Emitting our
        // own copy here (keyed by Claude's control-request id) folded a SECOND
        // input part into the doc whose id no resolver knew - the QuestionPanel
        // answered that unanswerable twin and the run never resumed.
        //
        // A dropped sender (caller went away) degrades to empty answers so the
        // agent is unblocked rather than wedged.
        let answers = (request_input)(questions.clone()).await.unwrap_or_default();
        let updated = updated_input_with_answers(&input, &questions, &answers);
        let line = control_response_line(&request_id, allow_response(updated));
        let _ = stdin_tx.send(StdinMsg::Line(line));
    });
}

/// Parse Claude's `AskUserQuestion` tool input into [`UserInputQuestion`]s
/// (tolerant of `header`/`title`, `question`/`prompt`, string or object
/// options - option descriptions are dropped, the wire type carries labels).
pub(super) fn parse_questions(input: &Value) -> Vec<UserInputQuestion> {
    let raw = input.get("questions").and_then(Value::as_array);
    raw.map(|a| a.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|q| {
            let field =
                |keys: [&str; 2]| keys.iter().find_map(|k| q.get(*k).and_then(Value::as_str));
            UserInputQuestion {
                id: uuid::Uuid::new_v4().to_string(),
                header: field(["header", "title"]).unwrap_or("Question").into(),
                question: field(["question", "prompt"]).unwrap_or("").into(),
                multi_select: ["multiSelect", "multi_select"]
                    .iter()
                    .find_map(|k| q.get(*k).and_then(Value::as_bool))
                    .unwrap_or(false),
                options: q
                    .get("options")
                    .and_then(Value::as_array)
                    .map(|a| a.as_slice())
                    .unwrap_or_default()
                    .iter()
                    .map(|op| match op {
                        Value::String(s) => s.clone(),
                        other => other
                            .get("label")
                            .or_else(|| other.get("value"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// Merge the user's answers back into the tool input, keyed by question text
/// (single-select ⇒ a string, multi-select ⇒ an array), as the tool expects.
pub(super) fn updated_input_with_answers(
    input: &Value,
    questions: &[UserInputQuestion],
    answers: &[UserInputAnswer],
) -> Value {
    let mut updated = match input {
        Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    let mut by_question = serde_json::Map::new();
    for q in questions {
        let labels: Vec<String> = answers
            .iter()
            .find(|a| a.question_id == q.id)
            .map(|a| a.labels.clone())
            .unwrap_or_default();
        let value = if q.multi_select {
            Value::Array(labels.into_iter().map(Value::String).collect())
        } else {
            Value::String(labels.into_iter().next().unwrap_or_default())
        };
        by_question.insert(q.question.clone(), value);
    }
    updated.insert("answers".into(), Value::Object(by_question));
    Value::Object(updated)
}
