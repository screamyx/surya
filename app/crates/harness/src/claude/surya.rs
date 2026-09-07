//! Wiring surya's own abilities into a Claude Code run (decision 5).
//!
//! Nothing about the CLI is forked or patched. Two of its own flags carry
//! everything:
//!
//! - `--mcp-config <file>` starts the `surya-mcp` sidecar, which gives the
//!   agent cards (`show_card`, `list_cards`), agent mail (`send_message`),
//!   the task board (`list_tasks`, `create_task`, `update_task`) and the
//!   browser pane (`browser_open` and its five siblings). Claude Code exposes
//!   them to the model as `mcp__surya__<tool>`, and the app watches the
//!   transcript for `mcp__surya__show_card`. All of them run without asking
//!   the user; see `AUTO_ALLOWED`.
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

use surya_proto::SuryaOptions;

/// The prompt append, compiled in so an installed binary needs no asset path.
const SYSTEM_APPEND: &str = include_str!("../../../../assets/surya-system-append.md");

/// The cards skill, compiled in for the same reason. The append says a card
/// beats prose and names the six shapes; this carries the JSON for each one
/// and the A2UI escape hatch. It is a skill rather than more append because
/// the append rides every turn's system prompt and this is 160 lines the
/// agent needs only when it actually draws a card.
const CARDS_SKILL: &str = include_str!("../../../../assets/skills/surya-cards/SKILL.md");

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

/// Every sidecar tool a run may call without asking the user.
///
/// These are surya's own abilities, not the machine's. The agent draws a
/// card, reads and writes the workspace task board, reaches another agent,
/// and drives the app's browser pane. Each of those is how the agent answers
/// at all, and stopping the run for a prompt every time made them unusable:
/// a gated run parked on its first `list_tasks` and the user saw a permission
/// row where they expected a board.
///
/// The owner widened this to the whole sidecar on 2026-09-08, browser tools
/// included, having been told those drive the owner's own browser profile.
/// Decisions 8, 11 and 19 carry the note.
///
/// It stays an explicit list of exact names rather than an
/// `mcp__surya__` prefix, and that is the point of the list: a prefix would
/// auto-allow whatever a future sidecar grows, sight unseen, and would also
/// pass a lookalike. A new tool has to be added here on purpose.
const AUTO_ALLOWED: [&str; 12] = [
    // cards
    "mcp__surya__show_card",
    "mcp__surya__list_cards",
    // agent mail
    "mcp__surya__send_message",
    // task board
    "mcp__surya__list_tasks",
    "mcp__surya__create_task",
    "mcp__surya__update_task",
    // browser pane
    "mcp__surya__browser_open",
    "mcp__surya__browser_snapshot",
    "mcp__surya__browser_click",
    "mcp__surya__browser_type",
    "mcp__surya__browser_screenshot",
    "mcp__surya__browser_eval",
];

/// The prefix Claude Code puts on the sidecar's tools. Named because the
/// test builds the same ids the CLI does.
pub const TOOL_PREFIX: &str = "mcp__surya__";

/// Exact match only. A lookalike like `mcp__surya__show_card_evil` is a
/// different tool and stays gated.
pub fn is_auto_allowed(tool: &str) -> bool {
    AUTO_ALLOWED.contains(&tool)
}

/// The generated paths, ready to hand to the CLI.
pub struct SuryaFiles {
    /// `None` when the sidecar is not installed here. The run still gets the
    /// prompt append and the denied built-ins: neither needs the binary, and
    /// dropping them would hand the agent back Claude Code's own SendMessage
    /// (decision 19) over a missing file.
    pub mcp_config: Option<PathBuf>,
    pub system_append: PathBuf,
    /// A one-skill plugin directory. Claude Code loads it with `--plugin-dir`
    /// and the agent sees `surya:surya-cards` - measured against 2.1.261, the
    /// skill lands in both the init frame's `skills` and its
    /// `slash_commands`.
    ///
    /// `None` when it could not be written. It is a reference the agent opens
    /// on demand, so losing it costs the card examples and nothing else -
    /// the tools, the prompt append and the denied built-ins all still apply.
    pub plugin_dir: Option<PathBuf>,
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

/// The root the generated per-agent directories live under.
///
/// `$XDG_RUNTIME_DIR` first: it is already per-user and 0700. Without it, a
/// temp dir suffixed with our uid, because a fixed `/tmp/surya-mcp` is shared
/// ground - the first user to create it owns it, the second cannot write
/// there, and either could plant an `mcp.json` the other's agent then loads
/// (surya is a public project and a box can have several users on it,
/// decision 18).
pub fn default_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir).join("surya-mcp");
    }
    std::env::temp_dir().join(format!("surya-mcp-{}", current_uid()))
}

#[cfg(unix)]
fn current_uid() -> String {
    // SAFETY: getuid(2) reads a process attribute and cannot fail.
    unsafe { libc::getuid() }.to_string()
}

#[cfg(not(unix))]
fn current_uid() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| "user".into())
}

/// One directory per agent under `root`, so concurrent agents never share a
/// config file.
fn run_dir(root: &Path, agent_id: &str) -> PathBuf {
    let safe: String = agent_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let safe = if safe.is_empty() {
        "anonymous".to_string()
    } else {
        safe
    };
    root.join(safe)
}

/// Create `dir` and every parent, owner-only on unix. The config carries the
/// agent's own paths and is read by a process we launch, so no other user has
/// business reading or writing it.
///
/// The mode is set as the directory is created, not chmod-ed after: a
/// create-then-tighten leaves a window where the directory is world-writable,
/// which is the whole hazard this function exists to close.
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
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

/// Write both files under [`default_root`]. `None` means the sidecar is not
/// installed here: the run goes ahead without cards and mail rather than
/// failing.
pub fn prepare(options: &SuryaOptions, cwd: &str) -> Option<SuryaFiles> {
    prepare_in(&default_root(), options, cwd)
}

/// [`prepare`], with the root named. Tests point it at a temp dir so they
/// never touch a path another user on the box may own.
pub fn prepare_in(root: &Path, options: &SuryaOptions, cwd: &str) -> Option<SuryaFiles> {
    prepare_with(root, options, cwd, resolve_mcp_binary(options))
}

/// [`prepare_in`], with the binary already resolved.
///
/// Resolution reads `SURYA_MCP_EXECUTABLE`, PATH and the directory beside the
/// running executable - none of which a test can clear without racing every
/// other test in the process. Splitting it out lets the missing-sidecar tests
/// say "no binary" and mean it, on a box where one happens to be installed.
pub fn prepare_with(
    root: &Path,
    options: &SuryaOptions,
    cwd: &str,
    binary: Option<PathBuf>,
) -> Option<SuryaFiles> {
    // A missing sidecar costs the card tools. It must not cost the prompt
    // append or the denial - measured 2026-09-06, a real run with the binary
    // absent came back with the ambient user config and SendMessage live.
    if binary.is_none() {
        tracing::warn!(
            searched_option = ?options.mcp_binary,
            searched_env = ?std::env::var_os("SURYA_MCP_EXECUTABLE"),
            searched_beside = ?std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)),
            "surya-mcp not found; this run gets the prompt append and the \
             denied built-ins but no show_card"
        );
    }
    let dir = run_dir(root, &options.agent_id);
    if let Err(error) = create_private_dir(&dir) {
        tracing::warn!("could not create {dir:?} for the surya MCP config: {error}");
        return None;
    }
    let system_append_path = dir.join("system-append.md");
    let mcp_config_path = match &binary {
        Some(binary) => {
            let path = dir.join("mcp.json");
            let config = mcp_config(binary, options, cwd);
            match std::fs::write(&path, config.to_string()) {
                Ok(()) => Some(path),
                Err(error) => {
                    tracing::warn!("could not write {path:?}: {error}");
                    None
                }
            }
        }
        None => None,
    };
    if let Err(error) = std::fs::write(&system_append_path, SYSTEM_APPEND) {
        tracing::warn!("could not write {system_append_path:?}: {error}");
        return None;
    }
    // The cards skill is a reference the agent opens on demand, not a
    // precondition. Returning None here would drop the WHOLE surya block at
    // the call site - the MCP config, the prompt append, and the denied
    // built-in messaging tools with it - so a missing skill would silently
    // hand the agent back Claude Code's own SendMessage, which decision 19
    // exists to deny.
    // The skill documents show_card, so it is pointless without it.
    let plugin_dir = mcp_config_path.as_ref().and_then(|_| {
        write_cards_plugin(&dir)
            .inspect_err(|error| {
                tracing::warn!("could not write the surya cards plugin: {error}");
            })
            .ok()
    });
    Some(SuryaFiles {
        mcp_config: mcp_config_path,
        system_append: system_append_path,
        plugin_dir,
    })
}

/// How much of the tail of the card store one lookup reads.
///
/// The card being looked up was written moments ago, so it is at the very
/// end. Reading the whole file per card event made every lookup cost the
/// length of the run; this bounds it. A card pushed out of this window by a
/// burst is simply not found, and the caller falls back to showing the tool
/// chip - a missing card, never a wrong one.
const CARD_TAIL_BYTES: u64 = 1 << 20;

/// Read back the card the sidecar recorded for one `show_card` tool call.
///
/// The store is append-only JSON lines and a card is looked up by the
/// `tool_use_id` the sidecar stamped on it, so the match is exact: no
/// ordering guess, no parsing of the tool result text. The scan runs backwards
/// over the tail because the card just written is the last line.
pub fn read_card(store: &Path, tool_use_id: &str) -> Option<surya_proto::AgentEvent> {
    if tool_use_id.is_empty() {
        return None;
    }
    let body = read_tail(store, CARD_TAIL_BYTES).ok()?;
    let record: Value = body
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|record| record["tool_use_id"] == tool_use_id)?;
    Some(surya_proto::AgentEvent::Card {
        card_id: record["card_id"].as_str()?.to_string(),
        surface_id: record["surface_id"].as_str()?.to_string(),
        tool_use_id: tool_use_id.to_string(),
        a2ui: record["a2ui"].as_array()?.clone(),
    })
}

/// The last `bytes` of `path` as text, whole lines only.
///
/// A byte window can land mid-character and mid-line, so the first partial
/// line is dropped: the caller parses lines, and half a line is not a record.
fn read_tail(path: &Path, bytes: u64) -> std::io::Result<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    if len <= bytes {
        let mut body = String::new();
        file.read_to_string(&mut body)?;
        return Ok(body);
    }
    file.seek(SeekFrom::Start(len - bytes))?;
    let mut raw = Vec::with_capacity(bytes as usize);
    file.read_to_end(&mut raw)?;
    let body = String::from_utf8_lossy(&raw).into_owned();
    Ok(match body.find('\n') {
        Some(at) => body[at + 1..].to_owned(),
        None => String::new(),
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

/// Lay the cards skill out as a one-skill plugin, the shape Claude Code's
/// `--plugin-dir` reads: a `.claude-plugin/plugin.json` naming the skills
/// folder, and `skills/<name>/SKILL.md`.
fn write_cards_plugin(dir: &Path) -> std::io::Result<PathBuf> {
    let plugin = dir.join("plugin");
    let skill = plugin.join("skills").join("surya-cards");
    create_private_dir(&skill)?;
    create_private_dir(&plugin.join(".claude-plugin"))?;
    std::fs::write(
        plugin.join(".claude-plugin").join("plugin.json"),
        json!({
            "name": SERVER_NAME,
            "description": "surya's own abilities: the card shapes show_card draws.",
            "version": env!("CARGO_PKG_VERSION"),
            "skills": "./skills/",
        })
        .to_string(),
    )?;
    std::fs::write(skill.join("SKILL.md"), CARDS_SKILL)?;
    Ok(plugin)
}

#[cfg(test)]
mod tests;
