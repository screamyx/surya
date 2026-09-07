//! surya-mcp - the stdio MCP server surya hands to every agent it starts.
//!
//! Three tools (decision 5, "surya abilities reach the agent as MCP servers"):
//! `show_card` draws an A2UI card in the app, `send_message` posts agent mail,
//! `list_cards` says which card shapes exist here. `list_tasks`, `create_task`
//! and `update_task` (`tasks.rs`) read and write the workspace task board.
//! `browser_open`, `browser_snapshot`, `browser_click`, `browser_type`,
//! `browser_screenshot` and `browser_eval` (`browser.rs`) drive the app's
//! browser pane. The harness launches this
//! binary through Claude Code's own `--mcp-config`, so nothing about Claude
//! Code is forked or patched.
//!
//! It knows no business (decision 14): every path, address and catalog is an
//! environment variable the host sets.
//!
//! Run it by hand to check it:
//! ```text
//! printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | surya-mcp
//! ```

use std::io::{BufRead, Write};

use surya_mcp::{config, protocol};

fn main() {
    let config = config::Config::from_env();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<serde_json::Value>(line) {
            Ok(request) => protocol::handle(&config, &request),
            Err(error) => Some(serde_json::json!({
                "jsonrpc": "2.0",
                "id": serde_json::Value::Null,
                "error": { "code": -32700, "message": format!("parse error: {error}") },
            })),
        };
        let Some(response) = response else { continue };
        if writeln!(stdout, "{response}").is_err() || stdout.flush().is_err() {
            // The client closed the pipe; nothing left to answer to.
            break;
        }
    }
}
