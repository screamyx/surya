//! HostRelay + ClientLink end-to-end over the in-memory fake device room in
//! `device_room_support`.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use surya_rpc::{
    DeviceLink, HostRelay, HostRelayConfig, LinkCache, LinkCacheConfig, RpcError, StaticToken,
    device_room_ws_url,
};

mod device_room_support;
use device_room_support::*;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn token_recovery_wakes_a_signed_out_host_relay_immediately() {
    let relay = FakeRelay::start().await;
    let token = Arc::new(RecoveringToken::new(None));
    let mut config = HostRelayConfig::new(relay.edge_url(), "dev-a", token.clone());
    // Without the token signal this test would remain asleep for two minutes.
    config.retry = Duration::from_secs(120);
    let _host = HostRelay::spawn(config, TestService::new("host-a"), noop_nudge());
    tokio::task::yield_now().await;
    assert!(!relay.host_connected());

    token.replace("test-user");

    relay.wait_host_connected().await;
}

#[tokio::test]
async fn sign_out_closes_a_live_host_relay_immediately() {
    let relay = FakeRelay::start().await;
    let token = Arc::new(RecoveringToken::new(Some("test-user")));
    let mut config = HostRelayConfig::new(relay.edge_url(), "dev-a", token.clone());
    // A sign-out signal must close the authenticated socket, not wait for retry.
    config.retry = Duration::from_secs(120);
    let _host = HostRelay::spawn(config, TestService::new("host-a"), noop_nudge());
    relay.wait_host_connected().await;

    token.clear();

    wait_until(|| !relay.host_connected()).await;
}

#[tokio::test]
async fn token_rotation_does_not_interrupt_a_live_peer_link() {
    let relay = FakeRelay::start().await;
    let _host = HostRelay::spawn(
        relay_config(&relay.edge_url(), 100),
        TestService::new("host-a"),
        noop_nudge(),
    );
    relay.wait_host_connected().await;

    let token = Arc::new(RecoveringToken::new(Some("old-token")));
    let links = LinkCache::new(LinkCacheConfig::new(relay.edge_url(), token.clone()));
    let client = links.client("dev-a").await.expect("client dials");
    token.replace("new-token");
    tokio::time::sleep(Duration::from_millis(20)).await;

    let echoed = client
        .call("Echo", serde_json::json!({ "after": "refresh" }))
        .await
        .expect("live link survives token rotation");
    assert_eq!(echoed["params"]["after"], "refresh");
}

#[tokio::test]
async fn sign_out_closes_cached_peer_links() {
    let relay = FakeRelay::start().await;
    let _host = HostRelay::spawn(
        relay_config(&relay.edge_url(), 100),
        TestService::new("host-a"),
        noop_nudge(),
    );
    relay.wait_host_connected().await;

    let token = Arc::new(RecoveringToken::new(Some("test-user")));
    let links = LinkCache::new(LinkCacheConfig::new(relay.edge_url(), token.clone()));
    let client = links.client("dev-a").await.expect("client dials");
    client
        .call("Echo", serde_json::json!({ "before": "sign-out" }))
        .await
        .expect("link is live before sign-out");

    token.clear();

    // The close is an expected event, so wait for it on the shared deadline. The
    // old 5 s ceiling was the only thing failing here on a loaded runner
    // (main run 34055129484: "cached peer link survived sign-out: Elapsed(())").
    tokio::time::timeout(surya_test_deadlines::WAIT, async {
        loop {
            if client
                .call("Echo", serde_json::json!({ "after": "sign-out" }))
                .await
                .is_err()
            {
                return;
            }
            // Pace the retry. On a real regression this loop now runs for
            // the full WAIT, and a bare yield_now would spend that minute
            // hammering the runner with RPC calls.
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cached peer link survived sign-out");
    assert!(
        links.client("dev-a").await.is_err(),
        "signed-out cache must not redial"
    );
}

#[tokio::test]
async fn relay_serves_multiple_clients_end_to_end() {
    let relay = FakeRelay::start().await;
    let service = TestService::new("host-a");
    let _host = HostRelay::spawn(relay_config(&relay.edge_url(), 100), service, noop_nudge());
    relay.wait_host_connected().await;

    let links = cache(&relay.edge_url());
    let a = links.client("dev-a").await.expect("client a dials");
    let b = links.client("dev-a").await.expect("client b reuses/dials");

    let echoed = a
        .call("Echo", serde_json::json!({ "who": "a" }))
        .await
        .expect("echo a");
    assert_eq!(echoed["host"], "host-a");
    assert_eq!(echoed["params"]["who"], "a");

    // Streaming through the relay: items arrive in order, stream terminates.
    let mut items = b
        .subscribe("Count", serde_json::json!({ "n": 3 }))
        .await
        .expect("count");
    let mut seen = Vec::new();
    while let Some(v) = items.recv().await {
        seen.push(v);
    }
    assert_eq!(
        seen,
        vec![
            serde_json::json!(0),
            serde_json::json!(1),
            serde_json::json!(2)
        ]
    );

    // Concurrent calls from the same cached link multiplex fine.
    let (x, y) = tokio::join!(
        a.call("Echo", serde_json::json!(1)),
        a.call("Echo", serde_json::json!(2))
    );
    assert_eq!(x.expect("x")["params"], serde_json::json!(1));
    assert_eq!(y.expect("y")["params"], serde_json::json!(2));
}

#[tokio::test]
async fn client_disconnect_tears_down_virtual_conn() {
    let relay = FakeRelay::start().await;
    let service = TestService::new("host-a");
    let active = service.active_streams.clone();
    let _host = HostRelay::spawn(relay_config(&relay.edge_url(), 100), service, noop_nudge());
    relay.wait_host_connected().await;

    let url = device_room_ws_url(&relay.edge_url(), "dev-a", "client", Some("conn-x"), "t");
    let link = DeviceLink::connect(&url).await.expect("link connects");
    let client = link.client();
    let _items = client
        .subscribe("Never", serde_json::Value::Null)
        .await
        .expect("subscribe");
    wait_until(|| active.load(Ordering::SeqCst) == 1).await;

    // Dropping the link closes the relay socket → the DO tells the host client_closed →
    // the host drops the virtual conn, aborting the client's server-side streams.
    drop(link);
    wait_until(|| active.load(Ordering::SeqCst) == 0).await;
}

#[tokio::test]
async fn host_offline_fails_fast_and_cools_down() {
    let relay = FakeRelay::start().await;
    let links = cache(&relay.edge_url());

    // No host connected: the readiness probe is bounced with host_offline → link-down →
    // the dial fails quickly instead of hanging.
    let Err(err) = links.client("dev-a").await else {
        panic!("dial must fail with no host")
    };
    let message = err.to_string();
    assert!(
        message.contains("readiness check"),
        "expected readiness failure, got: {message}"
    );

    // Immediately after, the cooldown makes callers fail fast without redialing.
    let Err(err) = links.client("dev-a").await else {
        panic!("must fail fast while cooling")
    };
    assert!(err.to_string().contains("backing off"), "got: {err}");

    // After the cooldown a host is up — dial succeeds and clears the slate.
    let service = TestService::new("host-a");
    let _host = HostRelay::spawn(relay_config(&relay.edge_url(), 100), service, noop_nudge());
    relay.wait_host_connected().await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let client = links.client("dev-a").await.expect("dials after cooldown");
    assert_eq!(
        client
            .call("Echo", serde_json::json!({}))
            .await
            .expect("echo")["host"],
        "host-a"
    );
}

/// The data-driven cooldown reset (fresh workspace presence → peer is alive):
/// with a long backoff engaged, `reset_cooldown` lets the next call dial
/// immediately instead of waiting the window out.
#[tokio::test]
async fn presence_reset_clears_cooldown_immediately() {
    let relay = FakeRelay::start().await;
    let mut config =
        LinkCacheConfig::new(relay.edge_url(), Arc::new(StaticToken("test-user".into())));
    // Long enough that only a reset (never elapsed time) can explain success.
    config.cooldown_base = Duration::from_secs(120);
    config.cooldown_max = Duration::from_secs(120);
    config.probe_timeout = Duration::from_millis(1_500);
    let links = LinkCache::new(config);

    // No host: the dial fails and the two-minute cooldown engages.
    assert!(links.client("dev-a").await.is_err(), "no host: dial fails");
    let Err(err) = links.client("dev-a").await else {
        panic!("must fail fast while cooling")
    };
    assert!(err.to_string().contains("backing off"), "got: {err}");

    // Host comes up and its presence heartbeat clears the backoff — the next
    // call dials immediately.
    let service = TestService::new("host-a");
    let _host = HostRelay::spawn(relay_config(&relay.edge_url(), 100), service, noop_nudge());
    relay.wait_host_connected().await;
    links.reset_cooldown("dev-a");
    let client = links.client("dev-a").await.expect("dials after reset");
    assert_eq!(
        client
            .call("Echo", serde_json::json!({}))
            .await
            .expect("echo")["host"],
        "host-a"
    );
}

#[tokio::test]
async fn host_supersede_drops_old_links_and_recovers() {
    let relay = FakeRelay::start().await;
    let service = TestService::new("host-a");
    let _host = HostRelay::spawn(relay_config(&relay.edge_url(), 100), service, noop_nudge());
    relay.wait_host_connected().await;

    let links = cache(&relay.edge_url());
    let client = links.client("dev-a").await.expect("dials");
    assert!(client.call("Echo", serde_json::json!({})).await.is_ok());

    // A rogue host joins: the relay closes the HostRelay's socket (supersede) and, when
    // the rogue's socket later closes, clients see host_closed. The HostRelay backs off
    // and reclaims the room; old links are down and the cache re-dials.
    let rogue_url = device_room_ws_url(&relay.edge_url(), "dev-a", "host", None, "t");
    let (rogue_ws, _) = tokio_tungstenite::connect_async(&rogue_url)
        .await
        .expect("rogue joins");
    // Wait for the supersede to land, then let the rogue die: the HostRelay's reconnect
    // supersedes it right back (proved by its socket closing).
    let (_, mut rogue_stream) = rogue_ws.split();
    loop {
        match rogue_stream.next().await {
            Some(Ok(WsMessage::Close(_))) | Some(Err(_)) | None => break,
            Some(Ok(_)) => {}
        }
    }
    relay.wait_host_connected().await;

    // The pre-supersede link died; in-flight/new calls on it fail rather than hang.
    let err = client
        .call("Echo", serde_json::json!({}))
        .await
        .expect_err("old link dead");
    assert!(
        matches!(err, RpcError::Closed | RpcError::Transport(_)),
        "got: {err}"
    );

    // The cache notices the dead link and re-dials the reclaimed host (allowing for a
    // cooldown window if a re-dial raced the reclaim).
    let recovered = loop {
        match links.client("dev-a").await {
            Ok(c) => break c,
            Err(_) => tokio::time::sleep(Duration::from_millis(120)).await,
        }
    };
    let echoed = recovered
        .call("Echo", serde_json::json!({}))
        .await
        .expect("echo");
    assert_eq!(echoed["host"], "host-a");
}

#[tokio::test]
async fn nudges_reach_the_host_callback() {
    let relay = FakeRelay::start().await;
    let service = TestService::new("host-a");
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let on_nudge: surya_rpc::NudgeHandler = Arc::new(move |chat_id| {
        let _ = tx.send(chat_id);
    });
    let _host = HostRelay::spawn(relay_config(&relay.edge_url(), 100), service, on_nudge);
    relay.wait_host_connected().await;

    relay.nudge("chat-42");
    let got = tokio::time::timeout(surya_test_deadlines::WAIT, rx.recv())
        .await
        .expect("nudge delivered")
        .expect("channel open");
    assert_eq!(got, "chat-42");
}

/// Live-edge variant: run the same host+client path through a real DeviceRoom DO.
/// `SURYA_EDGE_WS=http://127.0.0.1:26640 cargo test -p surya-rpc -- --ignored live_edge`
/// (dev-mode edge; SURYA_EDGE_TOKEN defaults to a fixed dev user id).
#[tokio::test]
#[ignore = "needs a running edge (set SURYA_EDGE_WS)"]
async fn live_edge_relay_round_trip() {
    let Ok(edge_url) = std::env::var("SURYA_EDGE_WS") else {
        panic!("set SURYA_EDGE_WS to the edge base URL (e.g. http://127.0.0.1:26640)");
    };
    let token = std::env::var("SURYA_EDGE_TOKEN").unwrap_or_else(|_| "relay-live-test".into());
    let device_id = format!("relay-live-{}", uuid::Uuid::new_v4());

    let service = TestService::new("live-host");
    let mut config = HostRelayConfig::new(
        edge_url.clone(),
        device_id.clone(),
        Arc::new(StaticToken(token.clone())),
    );
    config.retry = Duration::from_millis(500);
    let _host = HostRelay::spawn(config, service, noop_nudge());

    let mut cache_config = LinkCacheConfig::new(edge_url, Arc::new(StaticToken(token)));
    cache_config.probe_timeout = Duration::from_secs(5);
    let links = LinkCache::new(cache_config);

    // The host claims the room asynchronously; retry the dial until it answers.
    let client = loop {
        match links.client(&device_id).await {
            Ok(client) => break client,
            Err(err) => {
                eprintln!("dial retry: {err}");
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    };
    let echoed = client
        .call("Echo", serde_json::json!({ "live": true }))
        .await
        .expect("echo");
    assert_eq!(echoed["host"], "live-host");
    assert_eq!(echoed["params"]["live"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn zombie_relay_path_trips_the_echo_deadline() {
    // 2026-08-19 incident shape: the client↔edge leg stays healthy (pongs
    // flow), the edge↔host leg is dead — the link used to sit "open" for
    // minutes, retrying frames into the void, until an unrelated host-session
    // cycle exposed it. The app-level echo must rule the link dead within its
    // deadline instead.
    surya_rpc::device_room::set_client_liveness_for_tests(
        Duration::from_millis(100),
        Duration::from_millis(600),
    );
    let relay = FakeRelay::start().await;
    let service = TestService::new("host-a");
    let _host = HostRelay::spawn(relay_config(&relay.edge_url(), 100), service, noop_nudge());
    relay.wait_host_connected().await;

    let url = device_room_ws_url(&relay.edge_url(), "dev-a", "client", Some("c-echo"), "tok");
    let link = DeviceLink::connect(&url).await.expect("client link dials");
    // Healthy phase: echoes flow, the deadline never trips.
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(!link.is_closed(), "echoing link must stay open");

    relay.set_blackhole_host_bound(true);
    wait_until(|| link.is_closed()).await;
    assert_eq!(
        link.closed().borrow().as_deref(),
        Some("host echo silent"),
        "the zombie path must be ruled dead by the echo deadline"
    );
    // Restore the production clocks for the rest of the process's tests.
    surya_rpc::device_room::set_client_liveness_for_tests(Duration::ZERO, Duration::ZERO);
}
