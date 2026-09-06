//! The pane's side of the browser broker: the app subscribes to the engine's
//! `Browser.Watch` stream, runs each `{id, op, args}` on the CEF pane, and
//! answers with `Browser.Reply`. That is how `surya-mcp`'s `browser_*` tools
//! reach the page the owner is looking at (`surya_engine::browser_rpc`).
//!
//! Runs on the gpui foreground: the ops are DevTools calls whose replies
//! arrive on the main thread from CEF's pump, so awaiting here is what lets
//! them arrive. One op at a time, in order; a deadline per op so a page
//! that never loads cannot hold the stream.
//!
//! When the watch runs (e2e ENGINE-01, 2026-09-06): only once this app's
//! Browser tab has been shown (`AppState::browser_pane_shown`), and then it
//! is held. The engine keeps one pane and the newest `Browser.Watch` wins,
//! so two apps on one engine that both retried every 2 s displaced each
//! other forever: 1690 "a new app pane replaced the previous one" engine
//! lines in 62 min from a chat-only session that never opened the pane. Now
//! a displaced app stops (its engine says `{displaced}` before ending the
//! stream) until its tab is shown again; a lost connection stops too, the
//! reconnect re-attaches; a failed subscribe backs off, 2 s doubling to a
//! minute. [`next_step`] is that policy, as data.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use futures::future::Either;
use gpui::{Context, Task};
use serde_json::json;
use surya_rpc::methods;

use crate::state::{AppState, EngineHandle};

/// A little under the engine's own 30 s wait, so the agent reads the pane's
/// reason rather than the broker's timeout.
const OP_DEADLINE: Duration = Duration::from_secs(25);
/// First wait after a failed subscribe; doubles per attempt up to [`RETRY_MAX`].
pub const RETRY: Duration = Duration::from_secs(2);
pub const RETRY_MAX: Duration = Duration::from_secs(60);

/// True while a watch is attached to the engine. `AppState` reads it to
/// decide whether showing the Browser tab has to start a new loop.
static ATTACHED: AtomicBool = AtomicBool::new(false);
static WARNED: AtomicBool = AtomicBool::new(false);
/// Said once: a refused watch is a wrong token, not a missing engine.
static REFUSED_WARNED: AtomicBool = AtomicBool::new(false);
static CALLED: AtomicU64 = AtomicU64::new(0);
static OK: AtomicU64 = AtomicU64::new(0);

/// Whether a watch is attached right now.
pub fn attached() -> bool {
    ATTACHED.load(Ordering::Acquire)
}

/// Why one pass of the watch loop ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// The engine gave the pane to a newer app: its last item said so.
    Displaced,
    /// This app's connection to the engine is gone.
    Disconnected,
    /// The stream ended while the connection is up (an older engine ending
    /// a displaced watch without saying so).
    Dropped,
    /// The engine refused the subscribe: the pane token is wrong.
    Refused,
    /// The subscribe failed for another reason (engine still assembling, or
    /// one that does not serve the method yet).
    Unavailable,
}

/// What the loop does after a pass ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Leave the loop; the text is the log line's reason.
    Stop(&'static str),
    /// Subscribe again after this long.
    RetryAfter(Duration),
}

/// The wait before retry number `attempt` (0-based): 2 s, 4, 8, 16, 32, then
/// a minute, and a minute from there on.
pub fn backoff(attempt: u32) -> Duration {
    RETRY.saturating_mul(1u32 << attempt.min(5)).min(RETRY_MAX)
}

/// The reconnect policy. A displaced pane does not fight back: the newer
/// app has it until this one's Browser tab is shown again. A dead
/// connection is the reconnect's job, which re-attaches through
/// `AppState::attach_engine`. Everything else retries, backing off.
pub fn next_step(ended: Ended, attempt: u32) -> Step {
    match ended {
        Ended::Displaced => Step::Stop(
            "another app took the browser pane; this one re-attaches when its Browser tab is shown again",
        ),
        Ended::Disconnected => Step::Stop("the engine connection closed; the reconnect re-attaches"),
        Ended::Dropped | Ended::Refused | Ended::Unavailable => Step::RetryAfter(backoff(attempt)),
    }
}

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
    if let Some(t) = surya_proto::env_compat::var("IPC_TOKEN").ok().and_then(non_empty) {
        return Some(t);
    }
    let dir = data_dir?;
    std::fs::read_to_string(surya_engine::ipc::token_path(dir))
        .ok()
        .and_then(non_empty)
}

/// Attach the pane to this engine and hold the watch. Returns the standing
/// task; dropping it detaches (the receiver drop cancels the stream). Runs
/// only after the Browser tab was shown: `AppState::browser_pane_shown`.
pub fn spawn(cx: &mut Context<AppState>, handle: EngineHandle, data_dir: Option<PathBuf>) -> Task<()> {
    cx.spawn(async move |this, cx| {
        let _guard = AttachedGuard;
        let mut attempt = 0u32;
        loop {
            let token =
                pane_token(handle.dialed_token(), data_dir.as_deref()).unwrap_or_default();
            if token.is_empty() && !WARNED.swap(true, Ordering::AcqRel) {
                println!(
                    "browser-agent: no pane token (the engine was dialed without one, and neither SURYA_IPC_TOKEN nor {{data_dir}}/ipc-token is set); the engine will refuse the watch"
                );
            }
            let ended = match handle
                .client()
                .subscribe(methods::BROWSER_WATCH, json!({ "token": token }))
                .await
            {
                Ok(mut rx) => {
                    println!("browser-agent: attached to the engine's Browser.Watch");
                    ATTACHED.store(true, Ordering::Release);
                    attempt = 0;
                    let mut displaced = false;
                    while let Some(item) = rx.recv().await {
                        if item.get("displaced").is_some() {
                            displaced = true;
                            continue;
                        }
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
                    ATTACHED.store(false, Ordering::Release);
                    if displaced {
                        Ended::Displaced
                    } else if handle.client().is_closed() {
                        Ended::Disconnected
                    } else {
                        Ended::Dropped
                    }
                }
                Err(err) if handle.client().is_closed() => {
                    tracing::debug!(error = %err, "Browser.Watch: connection closed");
                    Ended::Disconnected
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
                    if refused { Ended::Refused } else { Ended::Unavailable }
                }
            };
            match next_step(ended, attempt) {
                Step::Stop(why) => {
                    println!("browser-agent: detached: {why}");
                    return;
                }
                Step::RetryAfter(wait) => {
                    if this.update(cx, |_, _| {}).is_err() {
                        return;
                    }
                    attempt = attempt.saturating_add(1);
                    cx.background_executor().timer(wait).await;
                }
            }
        }
    })
}

/// Clears the attached flag however the task ends, a drop included.
struct AttachedGuard;

impl Drop for AttachedGuard {
    fn drop(&mut self) {
        ATTACHED.store(false, Ordering::Release);
    }
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
mod policy_tests {
    use super::*;

    #[test]
    fn a_displaced_pane_stops_instead_of_fighting_back() {
        // Two apps on one engine, each retrying in 2 s, displaced each other
        // every 2 s for an hour (ENGINE-01). The loser now stays out.
        assert!(matches!(next_step(Ended::Displaced, 0), Step::Stop(why) if why.contains("another app")));
        assert!(matches!(next_step(Ended::Displaced, 7), Step::Stop(_)));
    }

    #[test]
    fn a_lost_connection_is_the_reconnects_job() {
        assert!(matches!(next_step(Ended::Disconnected, 0), Step::Stop(why) if why.contains("reconnect")));
    }

    #[test]
    fn a_failed_subscribe_backs_off_to_a_minute() {
        let waits: Vec<u64> = (0..8)
            .map(|attempt| match next_step(Ended::Unavailable, attempt) {
                Step::RetryAfter(d) => d.as_secs(),
                Step::Stop(why) => panic!("stopped: {why}"),
            })
            .collect();
        assert_eq!(waits, [2, 4, 8, 16, 32, 60, 60, 60]);
        assert_eq!(backoff(u32::MAX), RETRY_MAX, "no overflow at the top");
    }

    #[test]
    fn a_refusal_and_a_silent_end_retry_too() {
        assert_eq!(next_step(Ended::Refused, 0), Step::RetryAfter(RETRY));
        assert_eq!(next_step(Ended::Dropped, 1), Step::RetryAfter(RETRY * 2));
    }
}

#[cfg(test)]
mod token_tests {
    use super::pane_token;

    fn write(dir: &std::path::Path, token: &str) {
        std::fs::write(surya_engine::ipc::token_path(dir), token).unwrap();
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
