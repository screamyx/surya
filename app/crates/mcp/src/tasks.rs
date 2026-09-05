//! Task board tools for the `surya-mcp` server: `list_tasks`, `create_task`,
//! `update_task`. Each tool is one engine RPC over the localhost IPC socket
//! (`ws://127.0.0.1:$ZERON_IPC_PORT`, default 27654; `SURYA_ENGINE_URL`
//! overrides). No MCP-framework types leak in here: the server's dispatch
//! calls [`TaskTools::call`] with the tool name and the JSON arguments and
//! forwards the JSON result. [`tool_specs`] is the `tools/list` entry set.
//!
//! The server is synchronous stdio; [`call_blocking`] runs one tool call on a
//! throwaway current-thread runtime. `workspace` defaults to the
//! `SURYA_WORKSPACE` the harness put in the server's env, so an agent can
//! omit it.
//!
//! Semantics live in the engine (`zeron_engine::tasks`); this file only shapes
//! arguments and answers for an agent.

use serde_json::{Value, json};
use zeron_rpc::{RpcClient, RpcError, methods};

pub const LIST_TASKS: &str = "list_tasks";
pub const CREATE_TASK: &str = "create_task";
pub const UPDATE_TASK: &str = "update_task";

/// Default engine IPC port (`apps/zeron/src/main.rs`).
const DEFAULT_IPC_PORT: u16 = 27654;

/// Where the engine listens and what it wants to hear first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineTarget {
    pub url: String,
    /// The IPC token when the engine enforces one (`ZERON_IPC_TOKEN`, or a
    /// non-loopback bind whose token lives in `{ZERON_DATA_DIR}/ipc-token`).
    pub token: Option<String>,
}

/// The engine target from the environment the harness passed down. Mirrors
/// `zeron_engine::ipc::IpcConfig::{dial_url, resolve_token}` without pulling
/// the engine crate into this binary:
/// - `SURYA_ENGINE_URL` overrides the address;
/// - `ZERON_BIND` (default loopback) and `ZERON_IPC_PORT` (default 27654)
///   build it otherwise; a wildcard bind dials loopback;
/// - `ZERON_IPC_TOKEN` is the token; without it a non-loopback bind reads
///   `{ZERON_DATA_DIR}/ipc-token`; an open loopback socket has none.
pub fn engine_target() -> EngineTarget {
    resolve_target(
        |key| std::env::var(key).ok(),
        |path| std::fs::read_to_string(path).ok(),
    )
}

/// [`engine_target`] over injectable env and file readers (unit-tested).
pub fn resolve_target(
    env: impl Fn(&str) -> Option<String>,
    read_file: impl Fn(&std::path::Path) -> Option<String>,
) -> EngineTarget {
    let non_empty = |key: &str| {
        env(key)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let bind: std::net::IpAddr = non_empty("ZERON_BIND")
        .and_then(|b| b.parse().ok())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
    let loopback = bind.is_loopback();
    let mut token = non_empty("ZERON_IPC_TOKEN");
    if token.is_none()
        && !loopback
        && let Some(dir) = non_empty("ZERON_DATA_DIR")
    {
        token = read_file(&std::path::Path::new(&dir).join("ipc-token"))
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
    }
    let url = match non_empty("SURYA_ENGINE_URL") {
        Some(url) => url,
        None => {
            let port = non_empty("ZERON_IPC_PORT")
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(DEFAULT_IPC_PORT);
            let host = if bind.is_unspecified() {
                match bind {
                    std::net::IpAddr::V4(_) => std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                    std::net::IpAddr::V6(_) => std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
                }
            } else {
                bind
            };
            format!("ws://{}", std::net::SocketAddr::new(host, port))
        }
    };
    EngineTarget { url, token }
}

pub fn handles(name: &str) -> bool {
    matches!(name, LIST_TASKS | CREATE_TASK | UPDATE_TASK)
}

/// One tool call from the synchronous server loop: dial the engine, run the
/// call, tear the runtime down. Cheap enough per call (one loopback socket).
pub fn call_blocking(
    default_workspace: Option<&str>,
    name: &str,
    mut args: Value,
) -> Result<Value, String> {
    if let Some(obj) = args.as_object_mut()
        && obj
            .get("workspace")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        && let Some(workspace) = default_workspace.filter(|w| !w.is_empty())
    {
        obj.insert("workspace".into(), json!(workspace));
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;
    runtime.block_on(async { TaskTools::connect().await?.call(name, args).await })
}

/// MCP `tools/list` entries: `{name, description, inputSchema}`.
pub fn tool_specs() -> Vec<Value> {
    let status = json!({
        "type": "string",
        "enum": ["queued", "running", "done", "blocked"],
    });
    vec![
        json!({
            "name": LIST_TASKS,
            "description": "Read the workspace task board (board order). Read it before starting work; pick up the task you are on.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workspace": {"type": "string", "description": "Space id of the board. Default: this session's workspace."},
                },
                "required": [],
            },
        }),
        json!({
            "name": CREATE_TASK,
            "description": "Add a task to the bottom of the workspace task board.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workspace": {"type": "string", "description": "Space id of the board. Default: this session's workspace."},
                    "title": {"type": "string"},
                    "notes": {"type": "string"},
                    "owner": {"type": "string", "description": "Agent id, or 'user'. Default: unowned."},
                    "status": status,
                },
                "required": ["title"],
            },
        }),
        json!({
            "name": UPDATE_TASK,
            "description": "Update the task you are on: status, notes, owner, links. Omitted fields keep their value.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "status": status,
                    "notes": {"type": "string"},
                    "owner": {"type": "string"},
                    "title": {"type": "string"},
                    "links": {"type": "array", "items": {"type": "string"}, "description": "PR / issue URLs; replaces the list."},
                },
                "required": ["id"],
            },
        }),
    ]
}

pub struct TaskTools {
    client: RpcClient,
}

impl TaskTools {
    /// Dial the engine named by the environment.
    pub async fn connect() -> Result<Self, String> {
        let target = engine_target();
        Self::connect_to_with_token(&target.url, target.token.as_deref()).await
    }

    /// Dial presenting the IPC token when the engine enforces one; a missing
    /// or wrong token is refused at the handshake and reported as such.
    pub async fn connect_to_with_token(url: &str, token: Option<&str>) -> Result<Self, String> {
        let client =
            zeron_rpc::connect_ws_with_token(url, token)
                .await
                .map_err(|e| match token {
                    Some(_) => {
                        format!("surya engine at {url} refused the connection (token?): {e}")
                    }
                    None => format!("no surya engine at {url}, or it needs an IPC token: {e}"),
                })?;
        Ok(Self { client })
    }

    /// Dispatch one tool call. Errors are agent-readable strings (the MCP
    /// server puts them in `isError: true` results).
    pub async fn call(&self, name: &str, args: Value) -> Result<Value, String> {
        match name {
            LIST_TASKS => self.list_tasks(args).await,
            CREATE_TASK => self.create_task(args).await,
            UPDATE_TASK => self.update_task(args).await,
            other => Err(format!("unknown task tool {other}")),
        }
    }

    async fn list_tasks(&self, args: Value) -> Result<Value, String> {
        let workspace = required_str(&args, "workspace")?;
        // WatchTasks is a stream; the first item is the current board.
        let mut rx = self
            .client
            .subscribe(methods::WATCH_TASKS, json!({ "spaceId": workspace }))
            .await
            .map_err(rpc_err)?;
        let tasks = rx
            .recv()
            .await
            .ok_or_else(|| "engine closed the task stream before the first snapshot".to_string())?;
        Ok(json!({ "workspace": workspace, "tasks": tasks }))
    }

    async fn create_task(&self, args: Value) -> Result<Value, String> {
        let workspace = required_str(&args, "workspace")?;
        let title = required_str(&args, "title")?;
        let task_id = uuid::Uuid::new_v4().to_string();
        let mut params = json!({
            "op": "createTask",
            "taskId": task_id,
            "spaceId": workspace,
            "title": title,
        });
        copy_optional(&args, &mut params, &["notes", "owner", "status"]);
        self.client
            .call(methods::MUTATE, params)
            .await
            .map_err(rpc_err)?;
        Ok(json!({ "id": task_id, "workspace": workspace, "title": title }))
    }

    async fn update_task(&self, args: Value) -> Result<Value, String> {
        let id = required_str(&args, "id")?;
        let mut params = json!({ "op": "updateTask", "taskId": id });
        copy_optional(
            &args,
            &mut params,
            &["status", "notes", "owner", "title", "links"],
        );
        if params.as_object().map_or(0, |o| o.len()) == 2 {
            return Err(
                "update_task needs at least one of status, notes, owner, title, links".into(),
            );
        }
        self.client
            .call(methods::MUTATE, params)
            .await
            .map_err(rpc_err)?;
        Ok(json!({ "id": id, "updated": true }))
    }
}

fn required_str(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing required argument `{key}`"))
}

/// Copy present, non-null keys from the tool arguments into the RPC params.
fn copy_optional(args: &Value, params: &mut Value, keys: &[&str]) {
    let Some(out) = params.as_object_mut() else {
        return;
    };
    for key in keys {
        if let Some(value) = args.get(*key)
            && !value.is_null()
        {
            out.insert((*key).to_string(), value.clone());
        }
    }
}

fn rpc_err(err: RpcError) -> String {
    match err {
        RpcError::BadParams(msg) => format!("bad arguments: {msg}"),
        RpcError::Failed(msg) => msg,
        other => format!("engine call failed: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs_name_the_three_tools_with_required_fields() {
        let specs = tool_specs();
        let names: Vec<&str> = specs.iter().map(|s| s["name"].as_str().unwrap()).collect();
        assert_eq!(names, [LIST_TASKS, CREATE_TASK, UPDATE_TASK]);
        assert_eq!(specs[1]["inputSchema"]["required"], json!(["title"]));
        assert!(names.iter().all(|n| handles(n)));
        assert!(!handles("show_card"));
    }

    fn target(vars: &[(&str, &str)], file: Option<&str>) -> EngineTarget {
        resolve_target(
            |key| {
                vars.iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.to_string())
            },
            |_path| file.map(str::to_string),
        )
    }

    #[test]
    fn target_defaults_to_open_loopback() {
        let t = target(&[], None);
        assert_eq!(t.url, format!("ws://127.0.0.1:{DEFAULT_IPC_PORT}"));
        assert_eq!(t.token, None, "loopback without an env token stays open");
    }

    #[test]
    fn target_takes_env_token_and_port() {
        let t = target(
            &[("ZERON_IPC_PORT", "4000"), ("ZERON_IPC_TOKEN", " tok ")],
            None,
        );
        assert_eq!(t.url, "ws://127.0.0.1:4000");
        assert_eq!(t.token.as_deref(), Some("tok"));
    }

    #[test]
    fn target_reads_the_data_dir_token_for_a_network_bind() {
        let vars = [("ZERON_BIND", "0.0.0.0"), ("ZERON_DATA_DIR", "/data")];
        let t = target(&vars, Some("file-token\n"));
        assert_eq!(
            t.url,
            format!("ws://127.0.0.1:{DEFAULT_IPC_PORT}"),
            "wildcard dials loopback"
        );
        assert_eq!(t.token.as_deref(), Some("file-token"));
        let t = target(
            &[("ZERON_BIND", "192.168.1.9"), ("ZERON_DATA_DIR", "/data")],
            None,
        );
        assert_eq!(t.url, format!("ws://192.168.1.9:{DEFAULT_IPC_PORT}"));
        assert_eq!(t.token, None, "no file yet: dial and let the engine refuse");
        // A loopback bind never reads the file: the engine does not enforce.
        let t = target(&[("ZERON_DATA_DIR", "/data")], Some("ignored"));
        assert_eq!(t.token, None);
    }

    #[test]
    fn explicit_url_wins_but_keeps_the_token() {
        let t = target(
            &[
                ("SURYA_ENGINE_URL", "ws://host:1"),
                ("ZERON_IPC_TOKEN", "t"),
            ],
            None,
        );
        assert_eq!(t.url, "ws://host:1");
        assert_eq!(t.token.as_deref(), Some("t"));
    }
}
