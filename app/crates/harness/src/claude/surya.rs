//! Wiring surya's own abilities into a Claude Code run (decision 5).
//!
//! Nothing about the CLI is forked or patched. Two of its own flags carry
//! everything:
//!
//! - `--mcp-config <file>` starts the `surya-mcp` sidecar, which gives the
//!   agent `show_card`, `send_message` and `list_cards`. Claude Code exposes
//!   them to the model as `mcp__surya__<tool>`, and the app watches the
//!   transcript for `mcp__surya__show_card`.
//! - `--append-system-prompt-file <file>` tells the agent it is inside a
//!   desktop app and when a card beats prose. The flag is absent from
//!   `claude --help` on 2.1.261 but the CLI accepts it and honors it
//!   (probed on 2026-09-05, canary text came back through it).
//!
//! Both files are generated per agent under the temp dir and rewritten on
//! every run, so a changed option takes effect on the next turn and nothing
//! accumulates.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use zeron_proto::SuryaOptions;

/// The prompt append, compiled in so an installed binary needs no asset path.
const SYSTEM_APPEND: &str = include_str!("../../../../assets/surya-system-append.md");

/// Claude Code's own cross-session messaging, denied while surya is host so
/// `mcp__surya__send_message` is the only way an agent reaches another agent.
pub const DENIED_TOOLS: &str = "SendMessage,ListAgents";

/// The tool name Claude Code exposes for the sidecar's `show_card`. The
/// normalizer watches for it; the app's transcript detection uses the same
/// string.
pub const SHOW_CARD_TOOL: &str = "mcp__surya__show_card";

/// The MCP server name. Tool ids the model sees are `mcp__surya__<tool>`; the
/// app's transcript detection depends on this string.
pub const SERVER_NAME: &str = "surya";

/// The two generated paths, ready to hand to the CLI.
pub struct SuryaFiles {
    pub mcp_config: PathBuf,
    pub system_append: PathBuf,
}

/// Locate the `surya-mcp` binary: the explicit option, then
/// `SURYA_MCP_EXECUTABLE`, then beside the running executable (how a packaged
/// surya ships it), then PATH.
fn resolve_mcp_binary(options: &SuryaOptions) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "surya-mcp.exe"
    } else {
        "surya-mcp"
    };
    if let Some(path) = options.mcp_binary.as_ref().filter(|p| !p.is_empty()) {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    if let Some(path) = std::env::var_os("SURYA_MCP_EXECUTABLE").filter(|p| !p.is_empty()) {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf))
    {
        let beside = dir.join(name);
        if beside.exists() {
            return Some(beside);
        }
    }
    std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .filter(|d| !d.as_os_str().is_empty())
                .map(|d| d.join(name))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
        .find(|p| p.exists())
}

/// One directory per agent, so concurrent agents never share a config file.
fn run_dir(agent_id: &str) -> PathBuf {
    let safe: String = agent_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let safe = if safe.is_empty() {
        "anonymous".to_string()
    } else {
        safe
    };
    std::env::temp_dir().join("surya-mcp").join(safe)
}

/// The `--mcp-config` body. Every surya path the sidecar needs rides the
/// server's `env` block, so the binary itself hardcodes nothing.
fn mcp_config(binary: &Path, options: &SuryaOptions, cwd: &str) -> Value {
    let mut env = serde_json::Map::new();
    env.insert("SURYA_AGENT_ID".into(), json!(options.agent_id));
    env.insert("SURYA_WORKSPACE".into(), json!(options.workspace));
    if !cwd.is_empty() {
        env.insert("SURYA_WORKSPACE_ROOT".into(), json!(cwd));
    }
    for (key, value) in [
        ("SURYA_CARD_STORE", &options.card_store),
        ("SURYA_MAIL_SOCKET", &options.mail_socket),
        ("SURYA_CATALOG_ID", &options.catalog_id),
    ] {
        if let Some(value) = value.as_ref().filter(|v| !v.is_empty()) {
            env.insert(key.into(), json!(value));
        }
    }
    json!({
        "mcpServers": {
            SERVER_NAME: {
                "type": "stdio",
                "command": binary.to_string_lossy(),
                "args": [],
                "env": Value::Object(env),
            }
        }
    })
}

/// Write both files. `None` means the sidecar is not installed here: the run
/// goes ahead without cards and mail rather than failing.
pub fn prepare(options: &SuryaOptions, cwd: &str) -> Option<SuryaFiles> {
    let binary = match resolve_mcp_binary(options) {
        Some(binary) => binary,
        None => {
            tracing::warn!(
                "surya-mcp not found beside the executable, on PATH, or at \
                 SURYA_MCP_EXECUTABLE; this agent runs without show_card and send_message"
            );
            return None;
        }
    };
    let dir = run_dir(&options.agent_id);
    if let Err(error) = std::fs::create_dir_all(&dir) {
        tracing::warn!("could not create {dir:?} for the surya MCP config: {error}");
        return None;
    }
    let mcp_config_path = dir.join("mcp.json");
    let system_append_path = dir.join("system-append.md");
    let config = mcp_config(&binary, options, cwd);
    if let Err(error) = std::fs::write(&mcp_config_path, config.to_string()) {
        tracing::warn!("could not write {mcp_config_path:?}: {error}");
        return None;
    }
    if let Err(error) = std::fs::write(&system_append_path, SYSTEM_APPEND) {
        tracing::warn!("could not write {system_append_path:?}: {error}");
        return None;
    }
    Some(SuryaFiles {
        mcp_config: mcp_config_path,
        system_append: system_append_path,
    })
}

/// Read back the card the sidecar recorded for one `show_card` tool call.
///
/// The store is append-only JSON lines and a card is looked up by the
/// `tool_use_id` the sidecar stamped on it, so the match is exact: no
/// ordering guess, no parsing of the tool result text. The scan runs backwards
/// because the card just written is the last line.
pub fn read_card(store: &Path, tool_use_id: &str) -> Option<zeron_proto::AgentEvent> {
    if tool_use_id.is_empty() {
        return None;
    }
    let body = std::fs::read_to_string(store).ok()?;
    let record: Value = body
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|record| record["tool_use_id"] == tool_use_id)?;
    Some(zeron_proto::AgentEvent::Card {
        card_id: record["card_id"].as_str()?.to_string(),
        surface_id: record["surface_id"].as_str()?.to_string(),
        tool_use_id: tool_use_id.to_string(),
        a2ui: record["a2ui"].as_array()?.clone(),
    })
}

/// Where the sidecar was told to write cards for this run. Mirrors the
/// sidecar's own defaulting so the harness reads the file the sidecar wrote.
pub fn card_store(options: &SuryaOptions) -> PathBuf {
    if let Some(path) = options.card_store.as_ref().filter(|p| !p.is_empty()) {
        return PathBuf::from(path);
    }
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join(".surya").join("cards.jsonl"),
        None => std::env::temp_dir().join("surya").join("cards.jsonl"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(binary: &Path) -> SuryaOptions {
        SuryaOptions {
            agent_id: "seat/one".into(),
            workspace: "demo".into(),
            mcp_binary: Some(binary.to_string_lossy().into()),
            card_store: Some("/var/surya/cards.jsonl".into()),
            mail_socket: None,
            catalog_id: None,
        }
    }

    #[test]
    fn the_config_names_the_server_surya_and_carries_the_paths() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("surya-mcp");
        std::fs::write(&binary, "").unwrap();
        let config = mcp_config(&binary, &options(&binary), "/repo");
        let server = &config["mcpServers"]["surya"];
        assert_eq!(server["type"], "stdio");
        assert_eq!(server["command"], binary.to_string_lossy().to_string());
        assert_eq!(server["env"]["SURYA_AGENT_ID"], "seat/one");
        assert_eq!(server["env"]["SURYA_WORKSPACE_ROOT"], "/repo");
        assert_eq!(server["env"]["SURYA_CARD_STORE"], "/var/surya/cards.jsonl");
        assert!(
            server["env"].get("SURYA_MAIL_SOCKET").is_none(),
            "an unset option leaves the sidecar on its own default"
        );
    }

    #[test]
    fn an_agent_id_with_a_slash_still_makes_one_directory() {
        let dir = run_dir("seat/one");
        assert_eq!(dir.file_name().unwrap(), "seat-one");
    }

    #[test]
    fn prepare_writes_both_files() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("surya-mcp");
        std::fs::write(&binary, "").unwrap();
        let files = prepare(&options(&binary), dir.path().to_str().unwrap())
            .expect("the binary exists, so both files are written");
        let config: Value =
            serde_json::from_str(&std::fs::read_to_string(&files.mcp_config).unwrap()).unwrap();
        assert!(config["mcpServers"]["surya"].is_object());
        let append = std::fs::read_to_string(&files.system_append).unwrap();
        assert!(append.contains("show_card"), "the append teaches show_card");
    }

    #[test]
    fn a_missing_binary_skips_the_wiring_instead_of_failing() {
        let mut options = options(Path::new("/nonexistent/surya-mcp"));
        options.mcp_binary = Some("/nonexistent/surya-mcp".into());
        // SURYA_MCP_EXECUTABLE and PATH are not ours to clear inside a test
        // process, so this only asserts the explicit option does not panic.
        let _ = prepare(&options, "");
    }

    #[test]
    fn a_card_is_found_by_its_tool_use_id_not_by_position() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("cards.jsonl");
        let lines = [
            json!({"card_id":"card_a","surface_id":"s_a","tool_use_id":"toolu_1","a2ui":[{"x":1}]}),
            json!({"card_id":"card_b","surface_id":"s_b","tool_use_id":"toolu_2","a2ui":[{"x":2}]}),
        ];
        std::fs::write(
            &store,
            lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n"),
        )
        .unwrap();

        let event = read_card(&store, "toolu_1").expect("the first card is still reachable");
        let zeron_proto::AgentEvent::Card { card_id, a2ui, .. } = event else {
            panic!("read_card returns a Card");
        };
        assert_eq!(card_id, "card_a");
        assert_eq!(a2ui, vec![json!({"x": 1})]);

        assert!(read_card(&store, "toolu_missing").is_none());
        assert!(read_card(&store, "").is_none(), "no id, no lookup");
        assert!(read_card(&dir.path().join("absent.jsonl"), "toolu_1").is_none());
    }

    #[test]
    fn a_malformed_line_does_not_hide_the_cards_around_it() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("cards.jsonl");
        std::fs::write(
            &store,
            "{ truncated\n{\"card_id\":\"card_a\",\"surface_id\":\"s\",\"tool_use_id\":\"t1\",\"a2ui\":[]}\n",
        )
        .unwrap();
        assert!(read_card(&store, "t1").is_some());
    }
}
