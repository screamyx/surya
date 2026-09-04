//! Drives the built `surya-mcp` binary over its real stdio pipe, with the
//! same three-message handshake Claude Code sends. The unit tests cover the
//! handlers; this covers the process: line framing, flushing, and the env
//! block the harness writes into the MCP config.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// The binary under test, built beside this integration test by cargo.
fn binary() -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("the test binary has a path");
    path.pop(); // deps/
    path.pop(); // debug/
    path.join(env!("CARGO_PKG_NAME"))
}

/// Send every request in order, return one response per request that has an id.
fn converse(requests: &[Value], env: &[(&str, &str)]) -> Vec<Value> {
    let mut child = Command::new(binary())
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("surya-mcp starts");
    {
        let stdin = child.stdin.as_mut().expect("stdin is piped");
        for request in requests {
            writeln!(stdin, "{request}").expect("the server accepts a line");
        }
    }
    // Dropping stdin ends the loop, so the server exits on its own.
    drop(child.stdin.take());
    let stdout = BufReader::new(child.stdout.take().expect("stdout is piped"));
    let responses: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(&line.expect("a whole line")).expect("valid JSON"))
        .collect();
    child.wait().expect("the server exits");
    responses
}

#[test]
fn the_recorded_claude_code_handshake_works_against_the_real_binary() {
    let dir = tempfile::tempdir().unwrap();
    let cards = dir.path().join("cards.jsonl");
    let mail = dir.path().join("mail.jsonl");
    let env = [
        ("SURYA_AGENT_ID", "seat-1"),
        ("SURYA_WORKSPACE", "demo"),
        ("SURYA_WORKSPACE_ROOT", dir.path().to_str().unwrap()),
        ("SURYA_CARD_STORE", cards.to_str().unwrap()),
        ("SURYA_MAIL_SOCKET", "/nonexistent/surya-test.sock"),
        ("SURYA_MAIL_LOG", mail.to_str().unwrap()),
    ];

    let responses = converse(
        &[
            json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{
                "protocolVersion":"2025-11-25",
                "clientInfo":{"name":"claude-code","version":"2.1.261"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
                "name":"show_card",
                "arguments":{"card":{"shape":"approval","title":"Run it?","summary":"one table"}},
                "_meta":{"claudecode/toolUseId":"toolu_test","progressToken":2}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
                "name":"send_message",
                "arguments":{"to":"#demo","text":"the migration is ready"}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
                "name":"list_cards","arguments":{}}}),
        ],
        &env,
    );

    // The notification is the one request with no id, so five come back.
    assert_eq!(responses.len(), 5, "asked=6 answered={}", responses.len());
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "surya");
    assert_eq!(responses[1]["result"]["tools"].as_array().unwrap().len(), 3);

    let card: Value =
        serde_json::from_str(responses[2]["result"]["content"][0]["text"].as_str().unwrap())
            .unwrap();
    assert!(card["card_id"].as_str().unwrap().starts_with("card_"));

    let delivery: Value =
        serde_json::from_str(responses[3]["result"]["content"][0]["text"].as_str().unwrap())
            .unwrap();
    assert_eq!(delivery["route"], "log", "no daemon here, so the log takes it");

    let listed: Value =
        serde_json::from_str(responses[4]["result"]["content"][0]["text"].as_str().unwrap())
            .unwrap();
    assert_eq!(listed["built_in"].as_array().unwrap().len(), 6);

    // Both stores landed exactly one record each.
    let cards_written = std::fs::read_to_string(&cards).unwrap().lines().count();
    let mail_written = std::fs::read_to_string(&mail).unwrap().lines().count();
    assert_eq!(
        (cards_written, mail_written),
        (1, 1),
        "cards asked=1 written={cards_written}, mail asked=1 written={mail_written}"
    );

    // The recorded card is a whole A2UI surface, not a fragment.
    let record: Value =
        serde_json::from_str(std::fs::read_to_string(&cards).unwrap().trim()).unwrap();
    assert_eq!(record["agent_id"], "seat-1");
    let components = record["a2ui"][1]["updateComponents"]["components"]
        .as_array()
        .unwrap();
    assert!(components.iter().any(|c| c["id"] == "root"));
}

#[test]
fn a_malformed_line_is_a_parse_error_and_the_server_keeps_going() {
    let dir = tempfile::tempdir().unwrap();
    let env = [("SURYA_WORKSPACE_ROOT", dir.path().to_str().unwrap())];
    let mut child = Command::new(binary())
        .envs(env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(stdin, "not json at all").unwrap();
        writeln!(stdin, "{}", json!({"jsonrpc":"2.0","id":7,"method":"tools/list"})).unwrap();
    }
    drop(child.stdin.take());
    let responses: Vec<Value> = BufReader::new(child.stdout.take().unwrap())
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
        .collect();
    child.wait().unwrap();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["error"]["code"], -32700);
    assert_eq!(responses[1]["id"], 7, "the next request is still served");
}
