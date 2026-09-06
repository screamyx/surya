//! The browser broker: how an agent's `browser_*` tool reaches the pane the
//! owner is looking at.
//!
//! The pane lives in the app process (CEF is process state there), and the
//! app is a *client* of the engine, so the engine cannot call into it. The
//! app therefore publishes its pane by subscribing to `Browser.Watch`; each
//! command the engine wants run arrives as one stream item `{id, op, args}`
//! and the app answers with `Browser.Reply {id, ok | error}`. An agent's
//! tool call is `Browser.Call {op, args}`: it waits for that reply, up to
//! [`CALL_DEADLINE`], and returns it as the tool result.
//!
//! One pane at a time: the newest `Browser.Watch` wins, and an older one is
//! ended (its receiver is dropped, the stream closes). No app attached means
//! `Browser.Call` fails at once with a message the agent can read; nothing
//! is queued for a pane that may never come.
//!
//! **Every Browser.* call carries the pane token.** The engine's loopback
//! socket is open (no handshake token, `ipc.rs`), and on a shared box any
//! local process could otherwise drive the owner's cookie-persistent
//! Chromium profile or take over as the pane and read every typed text
//! (Opus review of #87). So the broker is built with the engine's IPC
//! token, `{data_dir}/ipc-token` (0600, the same secret a non-loopback bind
//! enforces at the handshake), and refuses any call, watch or reply whose
//! `token` does not match it. The app reads the file from its data dir; the
//! sidecar reads it from the engine's; a process of another user cannot.
//!
//! Same sub-service shape as [`crate::mail::MailRpc`]: `handles` + `handle`,
//! one dispatch line in `rpc.rs`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};
use surya_rpc::{RpcError, RpcReply, RpcService, methods, parse_params};

/// How long a tool call waits for the pane. A page load on a slow site is
/// the longest op; the pane's own DevTools deadline is the same.
pub const CALL_DEADLINE: Duration = Duration::from_secs(30);

/// Commands queued for the watcher before it reads them.
const WATCH_QUEUE: usize = 32;

#[derive(Default)]
struct State {
    next_id: u64,
    watcher: Option<mpsc::Sender<Value>>,
    pending: HashMap<u64, oneshot::Sender<Result<Value, String>>>,
}

#[derive(Clone, Default)]
pub struct BrowserRpc {
    state: Arc<Mutex<State>>,
    /// The pane token every call must present. `None` refuses everything:
    /// a broker without a secret is closed, not open.
    token: Option<Arc<str>>,
}

#[derive(Debug, Deserialize)]
struct CallParams {
    op: String,
    #[serde(default)]
    args: Value,
}

#[derive(Debug, Deserialize)]
struct ReplyParams {
    id: u64,
    #[serde(default)]
    ok: Option<Value>,
    #[serde(default)]
    error: Option<String>,
}

impl BrowserRpc {
    /// A broker that admits calls carrying `token`.
    pub fn new(token: Option<String>) -> Self {
        Self {
            state: Arc::default(),
            token: token
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .map(Arc::from),
        }
    }

    /// Whether `params.token` is the pane token. Constant-time on the
    /// bytes, so a wrong token costs the same as a right one.
    fn authorized(&self, params: &Value) -> Result<(), RpcError> {
        let Some(wanted) = self.token.as_deref() else {
            return Err(RpcError::Failed(
                "browser: this engine has no pane token; Browser.* is closed".into(),
            ));
        };
        let given = params.get("token").and_then(Value::as_str).unwrap_or("");
        let mut diff = usize::from(given.len() != wanted.len());
        for (a, b) in given.bytes().zip(wanted.bytes()) {
            diff |= usize::from(a != b);
        }
        if diff != 0 {
            return Err(RpcError::Failed(
                "browser: not authorized; pass the engine's pane token ({data_dir}/ipc-token, or SURYA_IPC_TOKEN)".into(),
            ));
        }
        Ok(())
    }

    pub fn handles(method: &str) -> bool {
        matches!(
            method,
            methods::BROWSER_CALL | methods::BROWSER_WATCH | methods::BROWSER_REPLY
        )
    }

    /// Whether an app with a pane is attached right now.
    pub fn attached(&self) -> bool {
        self.state
            .lock()
            .map(|s| s.watcher.as_ref().is_some_and(|w| !w.is_closed()))
            .unwrap_or(false)
    }

    /// Queue one command for the pane and wait for its answer.
    pub async fn call(&self, op: &str, args: Value) -> Result<Value, String> {
        let (tx, rx) = oneshot::channel();
        let (id, watcher) = {
            let mut s = self.state.lock().map_err(|_| "browser broker poisoned".to_string())?;
            let watcher = s
                .watcher
                .clone()
                .filter(|w| !w.is_closed())
                .ok_or_else(|| {
                    "no surya app with a browser pane is attached to this engine; open the app and its Browser tab"
                        .to_string()
                })?;
            s.next_id += 1;
            let id = s.next_id;
            s.pending.insert(id, tx);
            (id, watcher)
        };
        let item = json!({ "id": id, "op": op, "args": args });
        if watcher.try_send(item).is_err() {
            self.forget(id);
            return Err("the browser pane is busy or has gone away; try again".into());
        }
        match tokio::time::timeout(CALL_DEADLINE, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                self.forget(id);
                Err("the browser pane went away before it answered".into())
            }
            Err(_) => {
                self.forget(id);
                Err(format!(
                    "the browser pane did not answer {op} within {}s",
                    CALL_DEADLINE.as_secs()
                ))
            }
        }
    }

    fn forget(&self, id: u64) {
        if let Ok(mut s) = self.state.lock() {
            s.pending.remove(&id);
        }
    }

    /// The app is here: it takes over as the one pane.
    fn watch(&self) -> mpsc::Receiver<Value> {
        let (tx, rx) = mpsc::channel(WATCH_QUEUE);
        if let Ok(mut s) = self.state.lock() {
            let replaced = s.watcher.replace(tx).is_some();
            if replaced {
                tracing::info!("browser broker: a new app pane replaced the previous one");
            }
        }
        rx
    }

    /// The app answered one command. `true` when someone was waiting.
    fn reply(&self, id: u64, result: Result<Value, String>) -> bool {
        let Some(tx) = self.state.lock().ok().and_then(|mut s| s.pending.remove(&id)) else {
            return false;
        };
        tx.send(result).is_ok()
    }
}

#[async_trait]
impl RpcService for BrowserRpc {
    async fn handle(&self, method: &str, params: Value) -> Result<RpcReply, RpcError> {
        self.authorized(&params)?;
        match method {
            methods::BROWSER_CALL => {
                let p: CallParams = parse_params(params)?;
                let result = self.call(&p.op, p.args).await.map_err(RpcError::Failed)?;
                RpcReply::value(&result)
            }
            methods::BROWSER_WATCH => {
                let rx = self.watch();
                let stream = futures::stream::unfold(rx, |mut rx| async move {
                    rx.recv().await.map(|item| (item, rx))
                });
                Ok(RpcReply::Stream(stream.boxed()))
            }
            methods::BROWSER_REPLY => {
                let p: ReplyParams = parse_params(params)?;
                let result = match (p.ok, p.error) {
                    (_, Some(error)) => Err(error),
                    (Some(ok), None) => Ok(ok),
                    (None, None) => Ok(Value::Null),
                };
                let delivered = self.reply(p.id, result);
                RpcReply::value(&json!({ "delivered": delivered }))
            }
            _ => Err(RpcError::UnknownMethod(method.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn broker() -> BrowserRpc {
        BrowserRpc::new(Some("pane-secret".into()))
    }

    #[tokio::test]
    async fn a_call_with_no_pane_fails_at_once_and_says_so() {
        let rpc = broker();
        let err = rpc.call("browser_snapshot", json!({})).await.unwrap_err();
        assert!(err.contains("no surya app"), "{err}");
        assert!(!rpc.attached());
    }

    #[tokio::test]
    async fn a_call_reaches_the_watcher_and_its_reply_comes_back() {
        let rpc = broker();
        let mut rx = rpc.watch();
        assert!(rpc.attached());
        let pane = rpc.clone();
        let answer = tokio::spawn(async move {
            let item = rx.recv().await.unwrap();
            assert_eq!(item["op"], "browser_open");
            assert_eq!(item["args"]["url"], "https://example.com");
            let id = item["id"].as_u64().unwrap();
            pane.handle(
                methods::BROWSER_REPLY,
                json!({ "id": id, "ok": { "title": "Example Domain" }, "token": "pane-secret" }),
            )
            .await
            .unwrap();
        });
        let result = rpc
            .call("browser_open", json!({ "url": "https://example.com" }))
            .await
            .unwrap();
        assert_eq!(result["title"], "Example Domain");
        answer.await.unwrap();
    }

    #[tokio::test]
    async fn an_error_reply_is_the_tool_error() {
        let rpc = broker();
        let mut rx = rpc.watch();
        let pane = rpc.clone();
        tokio::spawn(async move {
            let item = rx.recv().await.unwrap();
            let id = item["id"].as_u64().unwrap();
            assert!(pane.reply(id, Err("no element with id 9".into())));
        });
        let err = rpc.call("browser_click", json!({ "id": 9 })).await.unwrap_err();
        assert_eq!(err, "no element with id 9");
    }

    #[tokio::test]
    async fn a_new_watcher_replaces_the_old_and_a_dropped_one_detaches() {
        let rpc = broker();
        let old = rpc.watch();
        let _new = rpc.watch();
        drop(old);
        assert!(rpc.attached());
        drop(_new);
        assert!(!rpc.attached());
        assert!(!rpc.reply(42, Ok(Value::Null)), "a reply nobody waits for is not delivered");
    }

    #[tokio::test]
    async fn a_reply_frame_reports_whether_anyone_waited() {
        let rpc = broker();
        let RpcReply::Value(v) = rpc
            .handle(methods::BROWSER_REPLY, json!({ "id": 1, "ok": {}, "token": "pane-secret" }))
            .await
            .unwrap()
        else {
            panic!("unary")
        };
        assert_eq!(v["delivered"], false);
    }

    #[tokio::test]
    async fn without_the_pane_token_every_method_is_refused() {
        let rpc = broker();
        for method in [methods::BROWSER_CALL, methods::BROWSER_WATCH, methods::BROWSER_REPLY] {
            for params in [json!({}), json!({ "token": "wrong" }), json!({ "token": "pane-secre" })] {
                let err = rpc.handle(method, params.clone()).await.err().map(|e| e.to_string());
                assert!(
                    err.as_deref().is_some_and(|e| e.contains("not authorized")),
                    "{method} {params}: {err:?}"
                );
            }
        }
        assert!(!rpc.attached(), "a refused watch attaches nothing");
    }

    #[tokio::test]
    async fn a_broker_without_a_token_is_closed_not_open() {
        let rpc = BrowserRpc::default();
        let err = rpc
            .handle(methods::BROWSER_CALL, json!({ "op": "browser_snapshot", "token": "" }))
            .await
            .err()
            .map(|e| e.to_string());
        assert!(err.as_deref().is_some_and(|e| e.contains("no pane token")), "{err:?}");
    }
}
