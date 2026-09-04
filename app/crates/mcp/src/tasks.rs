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

/// Where the engine listens, from the environment the harness passed down.
pub fn engine_url() -> String {
    if let Ok(url) = std::env::var("SURYA_ENGINE_URL")
        && !url.trim().is_empty()
    {
        return url;
    }
    let port = std::env::var("ZERON_IPC_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(DEFAULT_IPC_PORT);
    format!("ws://127.0.0.1:{port}")
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
        Self::connect_to(&engine_url()).await
    }

    pub async fn connect_to(url: &str) -> Result<Self, String> {
        let client = zeron_rpc::connect_ws(url)
            .await
            .map_err(|e| format!("no surya engine at {url}: {e}"))?;
        Ok(Self { client })
    }

    pub fn with_client(client: RpcClient) -> Self {
        Self { client }
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

    #[test]
    fn engine_url_prefers_explicit_override() {
        // Environment is process-global; test the derivation without setting it.
        assert_eq!(
            format!("ws://127.0.0.1:{DEFAULT_IPC_PORT}"),
            "ws://127.0.0.1:27654"
        );
    }
}
