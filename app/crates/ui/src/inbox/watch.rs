//! One engine subscription, kept alive across engine restarts.
//!
//! Every inbox and agent feed runs through here. The rule they all need is
//! the same, and it is not "retry on stream end" - all four loops already did
//! that. It is **re-read the engine handle before every attempt**.

use gpui::{Context, Entity, Task};
use serde::de::DeserializeOwned;
use tokio::sync::mpsc::Receiver;

use crate::state::{AppState, EngineHandle};

/// Two seconds between attempts, matching the chats watch in `state.rs`.
const RETRY: std::time::Duration = std::time::Duration::from_secs(2);

/// Subscribe to `method`, hand every frame to `apply`, and never stop until
/// the view goes away.
///
/// The handle is read from [`AppState`] on each attempt rather than captured
/// once. A reconnect builds a **new** `EngineHandle` and stores it
/// (`AppState::bootstrap`), and the previous one's client is finished for
/// good - `crates/rpc/src/client.rs` says it plainly: "Calls and subscribes
/// on a closed client fail with `RpcError::Closed`; nothing reconnects on its
/// own."
///
/// A loop holding that old handle therefore retries every two seconds for the
/// rest of the session and never receives another frame. That is what left
/// Needs you, the rail badge and the agent tree frozen on a client that
/// stayed open through an engine restart, while the session list - whose
/// watch the bootstrap re-spawns with the new handle - kept updating.
pub(crate) fn spawn_engine_watch<V, T>(
    cx: &mut Context<V>,
    state: Entity<AppState>,
    method: &'static str,
    apply: impl Fn(&mut V, T, &mut Context<V>) + 'static,
) -> Task<()>
where
    V: 'static,
    T: DeserializeOwned + 'static,
{
    spawn_engine_watch_with(cx, state, method, apply, |engine, method| async move {
        engine
            .client()
            .subscribe(method, serde_json::json!({}))
            .await
            .ok()
    })
}

/// [`spawn_engine_watch`] with the subscribe step injected.
///
/// The seam exists for the tests: gpui's test scheduler panics the moment a
/// tokio worker wakes a gpui task ("Your test is not deterministic"), and
/// `RpcClient` is tokio all the way down. Handing the loop a subscriber the
/// test drives from its own thread keeps the part worth proving - *which
/// handle the next attempt uses* - testable without a live socket.
fn spawn_engine_watch_with<V, T, Fut>(
    cx: &mut Context<V>,
    state: Entity<AppState>,
    method: &'static str,
    apply: impl Fn(&mut V, T, &mut Context<V>) + 'static,
    subscribe: impl Fn(EngineHandle, &'static str) -> Fut + 'static,
) -> Task<()>
where
    V: 'static,
    T: DeserializeOwned + 'static,
    Fut: std::future::Future<Output = Option<Receiver<serde_json::Value>>>,
{
    cx.spawn(async move |this, cx| {
        loop {
            // An error here means the view is gone, and with it any reason to
            // keep watching.
            let Ok(engine) = this.update(cx, |_, cx| state.read(cx).engine().cloned()) else {
                return;
            };
            // No engine yet, or the subscribe failed: fall through to the
            // timer and ask again with whatever handle is current by then.
            if let Some(engine) = engine
                && let Some(mut rx) = subscribe(engine, method).await
            {
                while let Some(value) = rx.recv().await {
                    let frame = match serde_json::from_value::<T>(value) {
                        Ok(frame) => frame,
                        Err(error) => {
                            tracing::warn!(method, %error, "dropping malformed frame");
                            continue;
                        }
                    };
                    if this.update(cx, |view, cx| apply(view, frame, cx)).is_err() {
                        return;
                    }
                }
                tracing::debug!(method, "stream ended; resubscribing");
            }
            if this.update(cx, |_, _| {}).is_err() {
                return;
            }
            cx.background_executor().timer(RETRY).await;
        }
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use gpui::{AppContext, TestAppContext};
    use tokio::sync::mpsc;

    use super::*;

    /// The view under test: it only records what the watch delivered.
    #[derive(Default)]
    struct Seen {
        frames: Vec<String>,
    }

    /// A handle whose client is a pair of idle channels. Nothing is ever sent
    /// on it, so its tokio reader never wakes a gpui task and the test
    /// scheduler stays happy; the injected subscriber below is what the loop
    /// actually talks to.
    fn handle(device_id: &str) -> EngineHandle {
        let (out, _out_rx) = mpsc::channel::<String>(1);
        let (_in_tx, inbound) = mpsc::channel::<String>(1);
        EngineHandle::for_test(Arc::new(zeron_rpc::RpcClient::new(out, inbound)), device_id)
    }

    /// The regression, stated as a counter pair: a reconnect replaces the
    /// handle on `AppState`, and the watch has to notice. Before the fix the
    /// loop captured one handle at spawn, so `asked` was `["A", "A", ...]`
    /// forever and Needs you, the badge and the agent tree never moved again.
    #[gpui::test]
    async fn a_watch_follows_the_engine_across_a_reconnect(cx: &mut TestAppContext) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let _guard = runtime.enter();

        // Which engine each attempt subscribed to, in order.
        let asked: Arc<Mutex<Vec<String>>> = Arc::default();
        // The sender for the CURRENT attempt's stream, so the test can push a
        // frame and then end the stream.
        let stream: Arc<Mutex<Option<mpsc::Sender<serde_json::Value>>>> = Arc::default();

        let state = cx.new(|_| AppState::new());
        let view = cx.new(|_| Seen::default());

        state.update(cx, |state, _| {
            state.set_engine_for_test(handle("engine-a"))
        });

        let watch = view.update(cx, |_, cx| {
            let asked = asked.clone();
            let stream = stream.clone();
            spawn_engine_watch_with(
                cx,
                state.clone(),
                "watch.test",
                |seen: &mut Seen, frame: String, _| seen.frames.push(frame),
                move |engine, _method| {
                    asked
                        .lock()
                        .expect("asked")
                        .push(engine.engine_info().device_id.clone());
                    let (tx, rx) = mpsc::channel(4);
                    *stream.lock().expect("stream") = Some(tx);
                    async move { Some(rx) }
                },
            )
        });
        cx.run_until_parked();

        let send = |value: &str, cx: &mut TestAppContext| {
            let tx = stream.lock().expect("stream").clone().expect("subscribed");
            tx.try_send(serde_json::Value::String(value.to_string()))
                .expect("frame accepted");
            cx.run_until_parked();
        };
        send("from-a", cx);
        view.read_with(cx, |seen, _| {
            assert_eq!(seen.frames, vec!["from-a".to_string()]);
        });

        // The engine restarts: the stream ends, and the bootstrap puts a NEW
        // handle on the state before the loop's next attempt.
        stream.lock().expect("stream").take();
        cx.run_until_parked();
        state.update(cx, |state, _| {
            state.set_engine_for_test(handle("engine-b"))
        });
        cx.executor().advance_clock(RETRY + Duration::from_secs(1));
        cx.run_until_parked();

        let engines = asked.lock().expect("asked").clone();
        assert_eq!(
            engines,
            vec!["engine-a".to_string(), "engine-b".to_string()],
            "asked={engines:?} - the second attempt must use the handle the reconnect stored"
        );

        send("from-b", cx);
        view.read_with(cx, |seen, _| {
            assert_eq!(seen.frames, vec!["from-a".to_string(), "from-b".to_string()]);
        });
        drop(watch);
    }
}
