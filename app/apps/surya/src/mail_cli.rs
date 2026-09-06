//! `surya mail send|drain|ack` — the thin CLI shim over the engine's `Mail.*`
//! RPCs (decision 19: "a thin `agb` command that calls the daemon over its
//! local socket keeps `agb send`, `agb drain`, `agb ack` working in every
//! existing skill"). Output is deliberately agb-shaped so a skill can alias
//! one command to the other.

use surya_engine::ipc::IpcConfig;
use surya_rpc::methods;

/// Same dial as `surya sync`: the engine's IPC config carries the bind and the
/// token, so an off-loopback engine authenticates instead of refusing.
async fn client(ipc: &IpcConfig) -> anyhow::Result<surya_rpc::RpcClient> {
    ipc.connect().await.map_err(|e| {
        anyhow::anyhow!(
            "no engine listening on {} ({e}) — is surya running?",
            ipc.dial_addr()
        )
    })
}

/// `surya mail send <to> <body>` → one line per delivery id.
pub async fn send(ipc: IpcConfig, from: &str, to: &str, body: &str) -> anyhow::Result<()> {
    let client = client(&ipc).await?;
    let reply = client
        .call(
            methods::MAIL_SEND,
            serde_json::json!({ "from": from, "to": to, "body": body }),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Mail.Send failed: {e}"))?;
    let ids = reply
        .get("ids")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let recipients = reply
        .get("recipients")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    println!("sent={} recipients={}", ids.len(), recipients.len());
    for (id, agent) in ids.iter().zip(recipients.iter()) {
        println!(
            "  {} → {}",
            id.as_str().unwrap_or("?"),
            agent.as_str().unwrap_or("?")
        );
    }
    Ok(())
}

/// `surya mail drain [--agent X]` — what is waiting, and what is in flight.
pub async fn drain(ipc: IpcConfig, agent: Option<&str>) -> anyhow::Result<()> {
    let client = client(&ipc).await?;
    let mut params = serde_json::Map::new();
    if let Some(agent) = agent {
        params.insert("agent".into(), serde_json::json!(agent));
    }
    let reply = client
        .call(methods::MAIL_LIST, serde_json::Value::Object(params))
        .await
        .map_err(|e| anyhow::anyhow!("Mail.List failed: {e}"))?;
    let messages = reply
        .get("messages")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let state = |m: &serde_json::Value| -> &'static str {
        if m.get("ackedAt").is_some_and(|v| !v.is_null()) {
            "acked"
        } else if m.get("deliveredAt").is_some_and(|v| !v.is_null()) {
            "delivered"
        } else {
            "queued"
        }
    };
    let count = |want: &str| messages.iter().filter(|m| state(m) == want).count();
    println!(
        "total={} queued={} delivered={} acked={}",
        messages.len(),
        count("queued"),
        count("delivered"),
        count("acked")
    );
    for message in &messages {
        let field = |k: &str| message.get(k).and_then(|v| v.as_str()).unwrap_or("?");
        println!(
            "  [{}] {} from {} → {}: {}",
            state(message),
            field("id"),
            field("from"),
            field("toAgent"),
            field("body")
        );
    }
    Ok(())
}

/// `surya mail ack <id>` — the manual "seen"; turn completion acks on its own.
pub async fn ack(ipc: IpcConfig, id: &str) -> anyhow::Result<()> {
    let client = client(&ipc).await?;
    let reply = client
        .call(methods::MAIL_ACK, serde_json::json!({ "id": id }))
        .await
        .map_err(|e| anyhow::anyhow!("Mail.Ack failed: {e}"))?;
    let acked = reply
        .get("acked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    println!(
        "{}",
        if acked {
            format!("acked {id}")
        } else {
            format!("already acked or unknown: {id}")
        }
    );
    Ok(())
}
