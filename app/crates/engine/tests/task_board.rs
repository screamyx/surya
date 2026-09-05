//! Task board proof (brief `surya-tasks` deliverable 4):
//! 1. two RPC clients on one registry doc — one creates, the other sees it;
//! 2. an agent tool call (`surya-mcp` `update_task`, over a real WebSocket)
//!    updates status and the `WatchTasks` stream emits;
//! 3. reorder keeps board order across an engine restart.
//!
//! The MCP module is compiled straight from `crates/mcp/src/tasks.rs` so the
//! tool body under test is the file the MCP seat will wire in.

use std::sync::Arc;
use std::time::Duration;

use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_proto::{HarnessId, Task, TaskStatus};
use zeron_rpc::methods;

#[path = "../../mcp/src/tasks.rs"]
#[allow(dead_code)]
mod mcp_tasks;

fn assemble(dir: &std::path::Path) -> EngineCore {
    std::fs::create_dir_all(dir).expect("data dir");
    EngineCore::assemble(dir, Arc::new(HarnessRegistry::new()), HarnessId::Mock, None)
        .expect("engine assembles")
}

async fn next_board(rx: &mut tokio::sync::mpsc::Receiver<serde_json::Value>) -> Vec<Task> {
    let item = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("WatchTasks emits within 30s")
        .expect("stream alive");
    serde_json::from_value(item).expect("board decodes as Vec<Task>")
}

async fn create_space(client: &zeron_rpc::RpcClient, device_id: &str) {
    client
        .call(
            methods::MUTATE,
            serde_json::json!({
                "op": "createSpace", "spaceId": "space-1", "deviceId": device_id, "path": "/tmp"
            }),
        )
        .await
        .expect("create space");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_clients_one_creates_the_other_sees_it_and_the_tool_updates_it() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let core = assemble(&tmp.path().join("data"));
    let client_a = zeron_rpc::memory_client(core.rpc_service());
    let client_b = zeron_rpc::memory_client(core.rpc_service());
    create_space(&client_a, &core.device_id).await;

    // B watches the board first: the initial snapshot is empty.
    let mut board_b = client_b
        .subscribe(
            methods::WATCH_TASKS,
            serde_json::json!({ "spaceId": "space-1" }),
        )
        .await
        .expect("subscribe WatchTasks");
    let initial = next_board(&mut board_b).await;
    assert!(initial.is_empty(), "fresh board is empty, got {initial:?}");

    // A creates one task.
    client_a
        .call(
            methods::MUTATE,
            serde_json::json!({
                "op": "createTask", "taskId": "t-1", "spaceId": "space-1",
                "title": "Wire the task board", "notes": "engine side",
            }),
        )
        .await
        .expect("createTask");
    let seen = next_board(&mut board_b).await;
    let created = 1;
    let seen_count = seen.iter().filter(|t| t.id == "t-1").count();
    println!("created={created} seen={seen_count}");
    assert_eq!(seen_count, 1, "B sees A's task: {seen:?}");
    assert_eq!(seen[0].title, "Wire the task board");
    assert_eq!(seen[0].status, TaskStatus::Queued);
    assert_eq!(seen[0].rank, 1.0, "first task lands at rank 1");

    // An agent tool call over a REAL WebSocket updates the status.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral port");
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(zeron_rpc::serve_ws_listener(listener, core.rpc_service()));
    let tools =
        mcp_tasks::TaskTools::connect_to_with_token(&format!("ws://127.0.0.1:{port}"), None)
            .await
            .expect("tool dials the engine");

    let listed = tools
        .call(
            mcp_tasks::LIST_TASKS,
            serde_json::json!({ "workspace": "space-1" }),
        )
        .await
        .expect("list_tasks");
    assert_eq!(listed["tasks"].as_array().map(Vec::len), Some(1));

    let updated = tools
        .call(
            mcp_tasks::UPDATE_TASK,
            serde_json::json!({ "id": "t-1", "status": "running", "owner": "agent-7" }),
        )
        .await
        .expect("update_task");
    assert_eq!(updated["updated"], serde_json::json!(true));
    let emitted = next_board(&mut board_b).await;
    let emitted_count = emitted
        .iter()
        .filter(|t| t.id == "t-1" && t.status == TaskStatus::Running)
        .count();
    println!("updates=1 emitted={emitted_count}");
    assert_eq!(
        emitted_count, 1,
        "watch emits the status change: {emitted:?}"
    );
    assert_eq!(emitted[0].owner.as_deref(), Some("agent-7"));
    assert!(emitted[0].updated_at >= emitted[0].created_at);

    // The tool creates too, and the board keeps rank order (append = bottom).
    let made = tools
        .call(
            mcp_tasks::CREATE_TASK,
            serde_json::json!({ "workspace": "space-1", "title": "Second" }),
        )
        .await
        .expect("create_task");
    let new_id = made["id"]
        .as_str()
        .expect("tool returns the id")
        .to_string();
    let board = next_board(&mut board_b).await;
    let ids: Vec<&str> = board.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["t-1", new_id.as_str()]);

    // Bad input is an agent-readable error, not a panic.
    let err = tools
        .call(mcp_tasks::UPDATE_TASK, serde_json::json!({ "id": "t-1" }))
        .await
        .expect_err("empty patch rejected");
    assert!(err.contains("at least one of"), "{err}");
    let err = tools
        .call(
            mcp_tasks::CREATE_TASK,
            serde_json::json!({ "workspace": "no-such-space", "title": "x" }),
        )
        .await
        .expect_err("unknown space rejected");
    assert!(err.contains("unknown space"), "{err}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reorder_keeps_board_order_across_a_restart() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    {
        let core = assemble(&data);
        let client = zeron_rpc::memory_client(core.rpc_service());
        create_space(&client, &core.device_id).await;
        for (id, title) in [
            ("t-1", "one"),
            ("t-2", "two"),
            ("t-3", "three"),
            ("t-4", "four"),
        ] {
            client
                .call(
                    methods::MUTATE,
                    serde_json::json!({
                        "op": "createTask", "taskId": id, "spaceId": "space-1", "title": title
                    }),
                )
                .await
                .expect("createTask");
        }
        // Move t-3 to the top: rank below t-1's 1.0.
        client
            .call(
                methods::MUTATE,
                serde_json::json!({ "op": "reorderTask", "taskId": "t-3", "rank": 0.5 }),
            )
            .await
            .expect("reorderTask");
        let ids: Vec<String> = core
            .workspace
            .read_tasks(Some("space-1"))
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, ["t-3", "t-1", "t-2", "t-4"]);
        // Delete is a tombstone that survives too.
        client
            .call(
                methods::MUTATE,
                serde_json::json!({ "op": "deleteTask", "taskId": "t-4" }),
            )
            .await
            .expect("deleteTask");
        core.workspace.flush();
        core.shutdown().await;
    }

    let core = assemble(&data);
    let tasks = core.workspace.read_tasks(Some("space-1")).unwrap();
    let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    let expected = ["t-3", "t-1", "t-2"];
    let ordered = ids
        .iter()
        .zip(expected.iter())
        .filter(|(a, b)| a == b)
        .count();
    println!("asked={} ordered={ordered}", expected.len());
    assert_eq!(ids, expected, "order after restart");
    assert_eq!(tasks[0].rank, 0.5);
    let client = zeron_rpc::memory_client(core.rpc_service());
    let mut all = client
        .subscribe(methods::WATCH_TASKS, serde_json::json!({}))
        .await
        .expect("subscribe all boards");
    let snapshot = next_board(&mut all).await;
    assert_eq!(snapshot.len(), 3, "unfiltered watch carries every board");
    // No params at all (`null` on the wire) means the same thing.
    let mut null_params = client
        .subscribe(methods::WATCH_TASKS, serde_json::Value::Null)
        .await
        .expect("subscribe with null params");
    let snapshot = next_board(&mut null_params).await;
    println!("null_params_asked=1 boards_seen={}", snapshot.len());
    assert_eq!(snapshot.len(), 3, "null params = every board");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_tools_present_the_ipc_token_to_a_protected_engine() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let core = assemble(&tmp.path().join("data"));
    let client = zeron_rpc::memory_client(core.rpc_service());
    create_space(&client, &core.device_id).await;

    // A token-protected loopback engine (`ZERON_IPC_TOKEN` set, PR #3).
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral port");
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(zeron_rpc::serve_ws_listener_with_auth(
        listener,
        core.rpc_service(),
        Some(Arc::from("secret-token")),
    ));
    let url = format!("ws://127.0.0.1:{port}");

    let rejected = mcp_tasks::TaskTools::connect_to_with_token(&url, None)
        .await
        .is_err() as usize;
    let accepted =
        match mcp_tasks::TaskTools::connect_to_with_token(&url, Some("secret-token")).await {
            Ok(tools) => {
                let listed = tools
                    .call(
                        mcp_tasks::LIST_TASKS,
                        serde_json::json!({ "workspace": "space-1" }),
                    )
                    .await
                    .expect("list_tasks through the token gate");
                usize::from(listed["tasks"].is_array())
            }
            Err(err) => panic!("right token refused: {err}"),
        };
    println!("asked=2 rejected={rejected} accepted={accepted}");
    assert_eq!((rejected, accepted), (1, 1));

    // The env-derived target carries the token the same way.
    let target = mcp_tasks::resolve_target(
        |key| match key {
            "ZERON_IPC_TOKEN" => Some("secret-token".to_string()),
            "ZERON_IPC_PORT" => Some(port.to_string()),
            _ => None,
        },
        |_| None,
    );
    assert_eq!(target.url, url);
    mcp_tasks::TaskTools::connect_to_with_token(&target.url, target.token.as_deref())
        .await
        .expect("target from env connects");
}
