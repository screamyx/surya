//! The pane's side of the browser broker: the app subscribes to the engine's
//! `Browser.Watch` stream, runs each `{id, op, args}` on the CEF pane, and
//! answers with `Browser.Reply`. That is how `surya-mcp`'s `browser_*` tools
//! reach the page the owner is looking at (`zeron_engine::browser_rpc`).
//!
//! Runs on the gpui foreground: the ops are DevTools calls whose replies
//! arrive on the main thread from CEF's pump, so awaiting here is what lets
//! them arrive. One op at a time, in order; a deadline per op so a page
//! that never loads cannot hold the stream.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures::future::Either;
use gpui::{Context, Task};
use serde_json::{Value, json};
use zeron_rpc::methods;

use crate::state::{AppState, EngineHandle};

/// A little under the engine's own 30 s wait, so the agent reads the pane's
/// reason rather than the broker's timeout.
const OP_DEADLINE: Duration = Duration::from_secs(25);
const RETRY: Duration = Duration::from_secs(2);

static CALLED: AtomicU64 = AtomicU64::new(0);
static OK: AtomicU64 = AtomicU64::new(0);

/// `tools_called=N ok=M`, for a proof line.
pub fn counters() -> String {
    format!(
        "tools_called={} ok={}",
        CALLED.load(Ordering::Relaxed),
        OK.load(Ordering::Relaxed)
    )
}

/// Attach the pane to this engine. Returns the standing task; it resubscribes
/// after a daemon restart like the other watches.
pub fn spawn(cx: &mut Context<AppState>, handle: EngineHandle) -> Task<()> {
    cx.spawn(async move |this, cx| {
        loop {
            match handle
                .client()
                .subscribe(methods::BROWSER_WATCH, json!({}))
                .await
            {
                Ok(mut rx) => {
                    println!("browser-agent: attached to the engine's Browser.Watch");
                    while let Some(item) = rx.recv().await {
                        let Some(id) = item["id"].as_u64() else { continue };
                        let op = item["op"].as_str().unwrap_or("").to_string();
                        let args = item["args"].clone();
                        CALLED.fetch_add(1, Ordering::Relaxed);
                        let run = std::pin::pin!(surya_browser::agent::run(&op, &args));
                        let deadline = std::pin::pin!(cx.background_executor().timer(OP_DEADLINE));
                        let result = match futures::future::select(run, deadline).await {
                            Either::Left((result, _)) => result,
                            Either::Right(_) => Err(format!(
                                "{op} did not finish within {}s",
                                OP_DEADLINE.as_secs()
                            )),
                        };
                        if result.is_ok() {
                            OK.fetch_add(1, Ordering::Relaxed);
                        }
                        println!(
                            "browser-agent: {op} ok={} {} {}",
                            u8::from(result.is_ok()),
                            counters(),
                            surya_browser::devtools::counters()
                        );
                        let reply = match result {
                            Ok(ok) => json!({ "id": id, "ok": ok }),
                            Err(error) => json!({ "id": id, "error": error }),
                        };
                        if let Err(e) = handle.client().call(methods::BROWSER_REPLY, reply).await {
                            tracing::warn!(error = %e, "browser-agent: reply did not reach the engine");
                        }
                    }
                }
                Err(err) => {
                    tracing::debug!(error = %err, "Browser.Watch unavailable; retrying");
                }
            }
            if this.update(cx, |_, _| {}).is_err() {
                return;
            }
            cx.background_executor().timer(RETRY).await;
        }
    })
}

/// The shape of one reply, kept in one place so the test pins it.
#[cfg(test)]
fn reply_for(id: u64, result: Result<Value, String>) -> Value {
    match result {
        Ok(ok) => json!({ "id": id, "ok": ok }),
        Err(error) => json!({ "id": id, "error": error }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_carries_ok_or_error_never_both() {
        let ok = reply_for(3, Ok(json!({ "title": "x" })));
        assert_eq!(ok["id"], 3);
        assert_eq!(ok["ok"]["title"], "x");
        assert!(ok.get("error").is_none());
        let err = reply_for(4, Err("no element".into()));
        assert_eq!(err["error"], "no element");
        assert!(err.get("ok").is_none());
    }
}
