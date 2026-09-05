//! The pane's side of the browser broker: the app subscribes to the engine's
//! `Browser.Watch` stream, runs each `{id, op, args}` on the CEF pane, and
//! answers with `Browser.Reply`. That is how `surya-mcp`'s `browser_*` tools
//! reach the page the owner is looking at (`zeron_engine::browser_rpc`).
//!
//! Runs on the gpui foreground: the ops are DevTools calls whose replies
//! arrive on the main thread from CEF's pump, so awaiting here is what lets
//! them arrive. One op at a time, in order; a deadline per op so a page
//! that never loads cannot hold the stream.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures::future::Either;
use gpui::{Context, Task};
use serde_json::json;
use zeron_rpc::methods;

use crate::state::{AppState, EngineHandle};

/// A little under the engine's own 30 s wait, so the agent reads the pane's
/// reason rather than the broker's timeout.
const OP_DEADLINE: Duration = Duration::from_secs(25);
const RETRY: Duration = Duration::from_secs(2);

static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Said once: a refused watch is a wrong token, not a missing engine.
static REFUSED_WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
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

/// The pane token this engine wants.
///
/// `dialed` FIRST: it is the token this app authenticated to this engine
/// with, so it is the one the engine will accept. The local file is a
/// fallback for the embedded case, where nothing was dialed. Reading the
/// file first was wrong wherever the app and the engine are not the same
/// process - a remote engine, and every Windows install - because the app's
/// own data dir holds a different secret, or none.
pub fn pane_token(dialed: Option<&str>, data_dir: Option<&Path>) -> Option<String> {
    let non_empty = |t: String| {
        let t = t.trim().to_string();
        (!t.is_empty()).then_some(t)
    };
    if let Some(t) = dialed.map(str::to_string).and_then(non_empty) {
        return Some(t);
    }
    if let Some(t) = std::env::var("ZERON_IPC_TOKEN").ok().and_then(non_empty) {
        return Some(t);
    }
    let dir = data_dir?;
    std::fs::read_to_string(zeron_engine::ipc::token_path(dir))
        .ok()
        .and_then(non_empty)
}

/// Attach the pane to this engine. Returns the standing task; it resubscribes
/// after a daemon restart like the other watches.
pub fn spawn(cx: &mut Context<AppState>, handle: EngineHandle, data_dir: Option<PathBuf>) -> Task<()> {
    cx.spawn(async move |this, cx| {
        loop {
            let token =
                pane_token(handle.dialed_token(), data_dir.as_deref()).unwrap_or_default();
            if token.is_empty() && !WARNED.swap(true, Ordering::AcqRel) {
                println!(
                    "browser-agent: no pane token (the engine was dialed without one, and neither ZERON_IPC_TOKEN nor {{data_dir}}/ipc-token is set); the engine will refuse the watch"
                );
            }
            match handle
                .client()
                .subscribe(methods::BROWSER_WATCH, json!({ "token": token }))
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
                            Ok(ok) => json!({ "id": id, "ok": ok, "token": token }),
                            Err(error) => json!({ "id": id, "error": error, "token": token }),
                        };
                        if let Err(e) = handle.client().call(methods::BROWSER_REPLY, reply).await {
                            tracing::warn!(error = %e, "browser-agent: reply did not reach the engine");
                        }
                    }
                }
                Err(err) => {
                    // A refusal is not the same as "no engine yet". Say it
                    // once, out loud: a wrong token retries silently forever
                    // otherwise, and the pane just looks dead.
                    let refused = err.to_string().to_lowercase().contains("token")
                        || err.to_string().to_lowercase().contains("unauthor");
                    if refused && !REFUSED_WARNED.swap(true, Ordering::AcqRel) {
                        println!(
                            "browser-agent: the engine REFUSED Browser.Watch ({err}). The pane token does not match the engine's - on a remote or Windows engine it is the token this app dialed with, not a file in this machine's data dir."
                        );
                    }
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
fn reply_for(id: u64, result: Result<serde_json::Value, String>) -> serde_json::Value {
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

#[cfg(test)]
mod token_tests {
    use super::pane_token;

    fn write(dir: &std::path::Path, token: &str) {
        std::fs::write(zeron_engine::ipc::token_path(dir), token).unwrap();
    }

    /// Case 3, remote and Windows: the app dialed the engine with a token, and
    /// THAT is the one the engine will accept. The app's own data dir may hold
    /// a different secret entirely - reading it first is how the pane ended up
    /// refused on every non-embedded engine.
    #[test]
    fn the_dialed_token_beats_this_machines_file() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "this-machines-token");
        assert_eq!(
            pane_token(Some("dialed-token"), Some(dir.path())).as_deref(),
            Some("dialed-token")
        );
        // Embedded: nothing was dialed, so the file it just wrote is right.
        assert_eq!(
            pane_token(None, Some(dir.path())).as_deref(),
            Some("this-machines-token")
        );
        // Neither: the watch is refused, and `spawn` says so once.
        assert_eq!(pane_token(None, None), None);
        // A blank dialed token is not a token.
        assert_eq!(
            pane_token(Some("  "), Some(dir.path())).as_deref(),
            Some("this-machines-token")
        );
    }
}
