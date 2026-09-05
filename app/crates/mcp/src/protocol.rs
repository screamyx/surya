//! The MCP wire: JSON-RPC 2.0, one message per line, over stdin and stdout.
//!
//! Hand-rolled rather than pulled from a crate - the surface is three methods
//! (`initialize`, `tools/list`, `tools/call`) plus notifications we ignore,
//! and the handshake below is what Claude Code 2.1.261 actually sent when it
//! was recorded on 2026-09-05:
//!
//! ```text
//! IN  {"method":"initialize","params":{"protocolVersion":"2025-11-25",
//!      "capabilities":{"roots":{"listChanged":true},"elicitation":{}},
//!      "clientInfo":{"name":"claude-code","version":"2.1.261",…}},"id":0}
//! IN  {"jsonrpc":"2.0","method":"notifications/initialized"}
//! IN  {"method":"tools/list","id":1}
//! IN  {"method":"tools/call","params":{"name":"show_card","arguments":{…},
//!      "_meta":{"claudecode/toolUseId":"toolu_…","progressToken":2}},"id":2}
//! ```
//!
//! The version is echoed back rather than pinned, so a client that moves on
//! keeps working.

use serde_json::{Value, json};

use crate::cards;
use crate::config::Config;
use crate::mail;

pub const SERVER_NAME: &str = "surya";
pub const FALLBACK_PROTOCOL_VERSION: &str = "2025-06-18";

/// The tools this server advertises. Names are deliberate: Claude Code
/// exposes them to the model as `mcp__surya__<name>`, and the app detects
/// `mcp__surya__show_card` in the transcript.
pub fn tool_definitions() -> Value {
    let mut tools = json!([
        {
            "name": "show_card",
            "description":
                "Show the user a card instead of writing the answer as prose. Use it for a \
                 choice, a status, a record, a comparison, or a form. Pass a short-form card \
                 - {\"shape\":\"table\",\"title\":…,\"columns\":[…],\"rows\":[[…]]} - or, for \
                 full control, raw A2UI v0.9.1 messages. Call list_cards to see the shapes. \
                 Keep the reply text to one line when you show a card.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "card": {
                        "description":
                            "The card. Either a short-form object carrying \"shape\", or an \
                             A2UI envelope message, or a list of A2UI envelope messages.",
                        "anyOf": [{ "type": "object" }, { "type": "array" }]
                    },
                    "surface_id": {
                        "type": "string",
                        "description":
                            "Reuse an existing surface id to replace a card you already showed. \
                             Omit it for a new card."
                    }
                },
                "required": ["card"]
            }
        },
        {
            "name": "send_message",
            "description":
                "Send a message to another agent. The address is an agent id, or #workspace \
                 or #server for everyone there. Delivery is the host's job: the message is \
                 injected into the recipient's next turn. Returns a delivery id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to": {
                        "type": "string",
                        "description": "An agent id, or #workspace, or #server."
                    },
                    "text": { "type": "string", "description": "The message body." }
                },
                "required": ["to", "text"]
            }
        },
        {
            "name": "list_cards",
            "description":
                "List the card shapes available here: surya's six built-in shapes plus any \
                 cards this workspace brings in .surya/cards. Call it before showing a card \
                 in a workspace you have not shown one in yet.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ]);
    tools
        .as_array_mut()
        .expect("tool_definitions is an array")
        .extend(crate::tasks::tool_specs());
    tools
        .as_array_mut()
        .expect("tool_definitions is an array")
        .extend(crate::browser::tool_specs());
    tools
}

/// A tool result, either content or an error the model can read and retry.
fn text_result(text: String, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    })
}

fn call_tool(config: &Config, params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    match name {
        "show_card" => match cards::show(config, &arguments, cards::tool_use_id(params)) {
            Ok(card) => text_result(
                json!({
                    "card_id": card.card_id,
                    "surface_id": card.surface_id,
                    "messages": card.messages.len(),
                    "shown": true,
                })
                .to_string(),
                false,
            ),
            Err(error) => text_result(error, true),
        },
        "send_message" => match mail::send(config, &arguments) {
            Ok(delivery) => text_result(
                json!({
                    "delivery_id": delivery.delivery_id,
                    "route": delivery.route.as_str(),
                    "queued": true,
                })
                .to_string(),
                false,
            ),
            Err(error) => text_result(error, true),
        },
        "list_cards" => text_result(cards::list(config).to_string(), false),
        name if crate::browser::handles(name) => {
            match crate::browser::call_blocking(config, name, arguments, cards::tool_use_id(params)) {
                Ok(result) => result,
                Err(error) => text_result(error, true),
            }
        }
        name if crate::tasks::handles(name) => {
            match crate::tasks::call_blocking(Some(&config.workspace), name, arguments) {
                Ok(result) => text_result(result.to_string(), false),
                Err(error) => text_result(error, true),
            }
        }
        other => text_result(
            format!(
                "unknown tool \"{other}\"; this server has show_card, send_message, list_cards, \
                 list_tasks, create_task and update_task"
            ),
            true,
        ),
    }
}

/// Handle one decoded request. Returns the response to write, or `None` for a
/// notification (no `id`), which JSON-RPC forbids answering.
pub fn handle(config: &Config, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    let result = match method {
        "initialize" => {
            let version = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(FALLBACK_PROTOCOL_VERSION);
            json!({
                "protocolVersion": version,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "title": "surya",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            })
        }
        "tools/list" => json!({ "tools": tool_definitions() }),
        "tools/call" => call_tool(config, &params),
        "ping" => json!({}),
        _ => {
            let id = id?;
            return Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("method not found: {method}") },
            }));
        }
    };

    let id = id?;
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn config(dir: &std::path::Path) -> Config {
        Config {
            agent_id: "seat-1".into(),
            workspace: "demo".into(),
            workspace_root: dir.to_path_buf(),
            card_store: dir.join("cards.jsonl"),
            mail_socket: PathBuf::from("/nonexistent/mail.sock"),
            mail_log: dir.join("mail.jsonl"),
        }
    }

    #[test]
    fn initialize_echoes_the_version_the_client_asked_for() {
        let dir = tempfile::tempdir().unwrap();
        let response = handle(
            &config(dir.path()),
            &json!({"jsonrpc":"2.0","id":0,"method":"initialize",
                    "params":{"protocolVersion":"2025-11-25"}}),
        )
        .unwrap();
        assert_eq!(response["result"]["protocolVersion"], "2025-11-25");
        assert_eq!(response["result"]["serverInfo"]["name"], "surya");
        assert_eq!(response["id"], 0);
    }

    #[test]
    fn tools_list_advertises_the_v1_tools() {
        let dir = tempfile::tempdir().unwrap();
        let response = handle(
            &config(dir.path()),
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        )
        .unwrap();
        let names: Vec<&str> = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![
                "show_card",
                "send_message",
                "list_cards",
                "list_tasks",
                "create_task",
                "update_task",
                "browser_open",
                "browser_snapshot",
                "browser_click",
                "browser_type",
                "browser_screenshot",
                "browser_eval"
            ]
        );
    }

    #[test]
    fn a_notification_gets_no_response() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            handle(
                &config(dir.path()),
                &json!({"jsonrpc":"2.0","method":"notifications/initialized"})
            )
            .is_none()
        );
    }

    #[test]
    fn show_card_returns_a_card_id_and_records_it() {
        let dir = tempfile::tempdir().unwrap();
        let config = config(dir.path());
        let response = handle(
            &config,
            &json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
                "name":"show_card",
                "arguments":{"card":{"shape":"approval","title":"Run it?","summary":"one table"}}
            }}),
        )
        .unwrap();
        assert_eq!(response["result"]["isError"], false);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let body: Value = serde_json::from_str(text).unwrap();
        assert!(
            body["card_id"].as_str().unwrap().starts_with("card_"),
            "{body}"
        );
        assert_eq!(std::fs::read_to_string(&config.card_store).unwrap().lines().count(), 1);
    }

    #[test]
    fn a_bad_card_comes_back_as_a_readable_tool_error() {
        let dir = tempfile::tempdir().unwrap();
        let response = handle(
            &config(dir.path()),
            &json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
                "name":"show_card","arguments":{"card":{"shape":"chart"}}}}),
        )
        .unwrap();
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("unknown shape"), "{text}");
    }

    #[test]
    fn an_unknown_method_is_a_json_rpc_error() {
        let dir = tempfile::tempdir().unwrap();
        let response = handle(
            &config(dir.path()),
            &json!({"jsonrpc":"2.0","id":9,"method":"resources/list"}),
        )
        .unwrap();
        assert_eq!(response["error"]["code"], -32601);
    }
}
