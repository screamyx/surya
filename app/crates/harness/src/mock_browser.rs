//! `ZERON_MOCK_BROWSER=<url>`: the mock harness drives the app's browser
//! pane through the real path. It starts the real `surya-mcp` binary the
//! way Claude Code would, speaks MCP to it over stdio, and calls
//! `browser_open`, `browser_snapshot`, `browser_click` (the first link the
//! snapshot lists) and `browser_screenshot`. Every call becomes a tool call
//! and a tool result in the transcript; the screenshot's card is read back
//! from the card store the server wrote, exactly as the app would show it
//! for a real agent. Prints `browser-mock: tools_called=N ok=M`.
//!
//! Nothing here reaches the pane directly: mock -> surya-mcp -> engine
//! `Browser.Call` -> the app's `Browser.Watch` -> CEF, so a green run proves
//! the whole route an agent takes.

use std::path::PathBuf;
use std::process::Stdio;

use futures::StreamExt;
use futures::stream::BoxStream;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use zeron_proto::{AgentEvent, DoneStatus, ToolCall};

use crate::HarnessError;

/// The URL to open, when the knob is set.
pub fn wanted() -> Option<String> {
    std::env::var("ZERON_MOCK_BROWSER")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// The run: events arrive as the calls complete.
pub fn run(url: String) -> BoxStream<'static, Result<AgentEvent, HarnessError>> {
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move { session(url, tx).await });
    futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|event| (Ok(event), rx))
    })
    .boxed()
}

/// `surya-mcp`: `SURYA_MCP_EXECUTABLE`, then beside this binary, then PATH.
fn mcp_binary() -> Option<PathBuf> {
    let name = if cfg!(windows) { "surya-mcp.exe" } else { "surya-mcp" };
    if let Some(p) = std::env::var_os("SURYA_MCP_EXECUTABLE").filter(|p| !p.is_empty()) {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        let beside = dir.join(name);
        if beside.exists() {
            return Some(beside);
        }
    }
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|d| d.join(name))
            .find(|p| p.exists())
    })
}

/// One MCP server on stdio.
struct Mcp {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    lines: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    next_id: u64,
    card_store: PathBuf,
}

impl Mcp {
    async fn start() -> Result<Self, String> {
        let bin = mcp_binary().ok_or("surya-mcp not found beside the binary, on PATH, or at SURYA_MCP_EXECUTABLE")?;
        let dir = std::env::temp_dir().join(format!("surya-mock-browser-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let card_store = dir.join("cards.jsonl");
        let mut child = tokio::process::Command::new(&bin)
            .env("SURYA_AGENT_ID", "mock")
            .env("SURYA_WORKSPACE", "mock")
            .env("SURYA_CARD_STORE", &card_store)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("could not start {}: {e}", bin.display()))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let mut mcp = Self {
            child,
            stdin,
            lines: BufReader::new(stdout).lines(),
            next_id: 0,
            card_store,
        };
        mcp.request("initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "zeron-mock", "version": "0" } }))
            .await?;
        mcp.stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .await
            .map_err(|e| e.to_string())?;
        Ok(mcp)
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string();
        self.stdin
            .write_all(format!("{line}\n").as_bytes())
            .await
            .map_err(|e| format!("surya-mcp stdin: {e}"))?;
        loop {
            let Some(line) = self.lines.next_line().await.map_err(|e| e.to_string())? else {
                return Err("surya-mcp closed its stdout".into());
            };
            let Ok(frame) = serde_json::from_str::<Value>(&line) else { continue };
            if frame["id"].as_u64() != Some(id) {
                continue;
            }
            if let Some(err) = frame.get("error") {
                return Err(err["message"].as_str().unwrap_or("rpc error").to_string());
            }
            return Ok(frame["result"].clone());
        }
    }

    /// One tool call: `(is_error, text of the content blocks)`. Image
    /// blocks are named, not copied, so the transcript never carries the
    /// base64.
    async fn call(&mut self, tool: &str, args: Value, tool_use_id: &str) -> (bool, String) {
        let params = json!({
            "name": tool,
            "arguments": args,
            "_meta": { "claudecode/toolUseId": tool_use_id },
        });
        match self.request("tools/call", params).await {
            Ok(result) => {
                let is_error = result["isError"].as_bool().unwrap_or(false);
                let text = result["content"]
                    .as_array()
                    .map(|blocks| {
                        blocks
                            .iter()
                            .map(|b| match b["type"].as_str() {
                                Some("image") => format!(
                                    "(image {} {} bytes base64)",
                                    b["mimeType"].as_str().unwrap_or("?"),
                                    b["data"].as_str().map(str::len).unwrap_or(0)
                                ),
                                _ => b["text"].as_str().unwrap_or("").to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                (is_error, text)
            }
            Err(e) => (true, e),
        }
    }

    /// The last card the server recorded, as the app shows it: its own row
    /// after the chip, under `{tool_use_id}:shot` like the claude driver
    /// (`normalize.rs`), because a part id is one kind only in the doc.
    fn last_card(&self) -> Option<AgentEvent> {
        let text = std::fs::read_to_string(&self.card_store).ok()?;
        let record: Value = serde_json::from_str(text.lines().last()?).ok()?;
        Some(AgentEvent::Card {
            card_id: record["card_id"].as_str()?.to_string(),
            surface_id: record["surface_id"].as_str()?.to_string(),
            tool_use_id: format!("{}:shot", record["tool_use_id"].as_str()?),
            a2ui: record["a2ui"].as_array()?.clone(),
        })
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// The first control the snapshot lists, links first: the `[n]` of its line.
pub fn first_link_id(snapshot: &str) -> Option<u64> {
    let id_of = |line: &str| -> Option<u64> {
        let rest = line.strip_prefix('[')?;
        let (n, _) = rest.split_once(']')?;
        n.parse().ok()
    };
    snapshot
        .lines()
        .find(|l| l.starts_with('[') && l.contains("] link "))
        .or_else(|| snapshot.lines().find(|l| l.starts_with('[')))
        .and_then(id_of)
}

async fn session(url: String, tx: mpsc::UnboundedSender<AgentEvent>) {
    let emit = |e: AgentEvent| {
        let _ = tx.send(e);
    };
    emit(AgentEvent::TextDelta {
        text: format!("Opening {url} in the browser pane and reading it.\n\n"),
    });
    let mut mcp = match Mcp::start().await {
        Ok(m) => m,
        Err(e) => {
            println!("browser-mock: tools_called=0 ok=0 ({e})");
            emit(AgentEvent::Error { message: e });
            emit(AgentEvent::Done { status: DoneStatus::Errored, result: None, error: None, session_id: None });
            return;
        }
    };
    let mut called = 0u32;
    let mut ok = 0u32;
    let mut snapshot = String::new();
    let steps: [(&str, Box<dyn Fn(&str) -> Option<Value> + Send + Sync>); 4] = [
        ("browser_open", Box::new({
            let url = url.clone();
            move |_| Some(json!({ "url": url }))
        })),
        ("browser_snapshot", Box::new(|_| Some(json!({})))),
        ("browser_click", Box::new(|snap| first_link_id(snap).map(|id| json!({ "id": id })))),
        ("browser_screenshot", Box::new(|_| Some(json!({})))),
    ];
    for (n, (tool, args_for)) in steps.iter().enumerate() {
        let Some(args) = args_for(&snapshot) else {
            emit(AgentEvent::TextDelta {
                text: "The page lists no control to click; skipping the click.\n".into(),
            });
            continue;
        };
        let id = format!("toolu_mock_browser_{}", n + 1);
        emit(AgentEvent::ToolCall {
            id: id.clone(),
            call: ToolCall::Mcp { server: "surya".into(), tool: tool.to_string(), input: Some(args.clone()) },
        });
        called += 1;
        let (is_error, output) = mcp.call(tool, args, &id).await;
        if !is_error {
            ok += 1;
        }
        if *tool == "browser_snapshot" && !is_error {
            snapshot = output.clone();
        }
        println!("browser-mock: {tool} ok={} tools_called={called} ok={ok}", u8::from(!is_error));
        if is_error {
            println!("browser-mock: {tool} error: {}", output.lines().next().unwrap_or(""));
        }
        emit(AgentEvent::ToolResult { id: id.clone(), is_error, output: Some(output), diff: None });
        if *tool == "browser_screenshot" && !is_error {
            match mcp.last_card() {
                Some(card) => {
                    if let AgentEvent::Card { card_id, .. } = &card {
                        println!("browser-mock: card emitted=1 card_id={card_id}");
                    }
                    emit(card);
                }
                None => println!("browser-mock: card emitted=0 (no record in {})", mcp.card_store.display()),
            }
        }
    }
    println!("browser-mock: tools_called={called} ok={ok}");
    emit(AgentEvent::TextDelta {
        text: format!("Done: {ok} of {called} browser tools succeeded.\n"),
    });
    emit(AgentEvent::Done {
        status: if ok == called { DoneStatus::Completed } else { DoneStatus::Errored },
        result: None,
        error: None,
        session_id: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_link_wins_over_an_earlier_button() {
        let snap = "url: x\n[1] button \"Menu\"\nheading \"Hi\"\n[2] link \"More\" href=\"/more\"\n[3] link \"Other\"";
        assert_eq!(first_link_id(snap), Some(2));
    }

    #[test]
    fn without_a_link_any_control_will_do_and_none_is_none() {
        assert_eq!(first_link_id("[4] button \"Go\""), Some(4));
        assert_eq!(first_link_id("heading \"Nothing to click\""), None);
    }
}
