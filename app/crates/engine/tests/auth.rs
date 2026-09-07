//! Auth service tests: dev mode, and the WorkOS flows (headless paste-code
//! exchange, loopback callback, refresh rotation + revocation, org onboarding)
//! against the stub edge in `auth_support`.

use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::net::TcpListener;

use surya_engine::{Auth, AuthConfig, AuthState};
use surya_rpc::TokenSource;

mod auth_support;
use auth_support::*;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dev_mode_is_signed_in_with_configured_bearer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut config = AuthConfig::new("http://127.0.0.1:1", dir.path());
    config.dev_user_id = "wing-dev".into();
    let auth = Auth::new(config);
    assert!(!auth.workos_enabled());
    assert!(!auth.loaded_workos_session());
    assert!(matches!(auth.state(), AuthState::SignedIn { user, .. } if user.id == "wing-dev"));
    assert_eq!(auth.access_token().await.as_deref(), Some("wing-dev"));
    // Dev sign-in mirrors the TS service: a no-op URL, CompleteSignIn accepted.
    assert_eq!(auth.start_sign_in().await.expect("dev sign-in"), "");
    auth.complete_sign_in("whatever")
        .await
        .expect("dev complete is a no-op");
}

#[tokio::test]
async fn headless_flow_exchanges_pasted_code_and_gates_on_org() {
    let edge = StubEdge::start().await;
    let dir = tempfile::tempdir().expect("tempdir");
    let auth = Auth::new(workos_config(&edge.url(), dir.path()));
    assert!(auth.workos_enabled());
    assert!(!auth.loaded_workos_session());
    assert_eq!(auth.state(), AuthState::SignedOut);
    assert_eq!(auth.access_token().await, None, "signed out: no token");

    let url = auth.start_headless_sign_in();
    assert!(url.starts_with("https://authkit.example/user_management/authorize?"));
    assert_eq!(
        query_param(&url, "client_id").as_deref(),
        Some("client_test")
    );
    let redirect = query_param(&url, "redirect_uri").expect("redirect");
    assert!(
        redirect.contains("auth%2Fcli%2Fcallback"),
        "hosted paste-code page: {redirect}"
    );
    let state = query_param(&url, "state").expect("state param");

    // A code minted for someone else's flow (unknown state) is rejected — CSRF check.
    assert!(auth.complete_sign_in("bogus-state.code123").await.is_err());

    // The real paste: `state.code`. The exchange-minted token carries no org claim, so
    // the session lands in NeedsOrganization (the org gate).
    auth.complete_sign_in(&format!("{state}.code123"))
        .await
        .expect("paste-code sign-in");
    assert!(
        !auth.loaded_workos_session(),
        "sign-in does not rewrite the startup fact"
    );
    assert_eq!(edge.state.exchanges.load(Ordering::SeqCst), 1);
    assert!(
        matches!(auth.state(), AuthState::NeedsOrganization { user } if user.email == "w@example.com")
    );

    // Session persisted 0600 with the exchange's refresh token.
    let session_file = dir.path().join("session.json");
    let raw = std::fs::read_to_string(&session_file).expect("session persisted");
    assert!(raw.contains("refresh-1"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&session_file)
            .expect("meta")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "session file must be private");
    }

    // Org onboarding: list, then select — an org-scoped refresh; state follows the
    // returned token's org claim.
    let orgs = auth.list_orgs().await.expect("list orgs");
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].organization_id, "org_1");
    let mut token_changes = auth.subscribe().expect("auth token signal");
    auth.select_org("org_1").await.expect("select org");
    token_changes
        .changed()
        .await
        .expect("org-scoped token should wake transport supervisors");
    assert!(
        matches!(auth.state(), AuthState::SignedIn { org_id: Some(org), .. } if org == "org_1")
    );
    assert!(auth.token().await.is_some());
    assert_eq!(
        edge.state
            .refresh_tokens
            .lock()
            .expect("lock")
            .first()
            .map(String::as_str),
        Some("refresh-1"),
        "org refresh presents the stored refresh token"
    );
    // Rotation persisted.
    let raw = std::fs::read_to_string(&session_file).expect("session persisted");
    assert!(
        raw.contains("rotated-1"),
        "rotated refresh token stored: {raw}"
    );

    // Sign-out clears state and removes the persisted session.
    auth.sign_out();
    assert_eq!(auth.state(), AuthState::SignedOut);
    assert!(!session_file.exists());
}

#[tokio::test]
async fn short_lived_tokens_refresh_on_demand() {
    let edge = StubEdge::start().await;
    // Tokens live 20s < the 30s slack → every access_token() call refreshes.
    edge.state.token_ttl.store(20, Ordering::SeqCst);
    *edge.state.exchange_org.lock().expect("lock") = "org_1".into();
    let dir = tempfile::tempdir().expect("tempdir");
    let auth = Auth::new(workos_config(&edge.url(), dir.path()));

    let url = auth.start_headless_sign_in();
    let state = query_param(&url, "state").expect("state");
    auth.complete_sign_in(&format!("{state}.codeX"))
        .await
        .expect("sign in");
    assert!(auth.state().is_signed_in());

    let first = auth.access_token().await.expect("token after refresh");
    assert_eq!(
        edge.state.refreshes.load(Ordering::SeqCst),
        1,
        "stale exchange token refreshed"
    );
    let second = auth.access_token().await.expect("token again");
    assert_eq!(
        edge.state.refreshes.load(Ordering::SeqCst),
        2,
        "still under slack → refreshed"
    );
    assert_eq!(first, second, "same claims → same fake token bytes");
    // Rotated refresh tokens are chained: refresh N presents rotation N-1's token.
    let seen = edge.state.refresh_tokens.lock().expect("lock").clone();
    assert_eq!(seen, vec!["refresh-1".to_string(), "rotated-1".to_string()]);
}

#[tokio::test]
async fn revoked_refresh_token_signs_out() {
    let edge = StubEdge::start().await;
    let dir = tempfile::tempdir().expect("tempdir");
    // A persisted session whose refresh token the edge rejects with a definitive 4xx.
    std::fs::write(
        dir.path().join("session.json"),
        r#"{"refreshToken":"dead","user":{"id":"user_1","email":"w@example.com"},"orgId":"org_1"}"#,
    )
    .expect("seed session");
    let auth = Auth::new(workos_config(&edge.url(), dir.path()));
    assert!(
        auth.state().is_signed_in(),
        "boots from the persisted session"
    );
    assert!(auth.loaded_workos_session());

    // The refresh is doomed → the session degrades to SignedOut and the file is gone.
    assert_eq!(auth.access_token().await, None);
    assert_eq!(auth.state(), AuthState::SignedOut);
    assert!(
        auth.loaded_workos_session(),
        "revocation must not rewrite the captured startup fact"
    );
    assert!(!dir.path().join("session.json").exists());
}

#[tokio::test]
async fn offline_refresh_loop_backs_off_without_revoking_session() {
    let edge = StubEdge::start().await;
    edge.state.drop_refresh.store(true, Ordering::SeqCst);
    let dir = tempfile::tempdir().expect("tempdir");
    let session_file = dir.path().join("session.json");
    std::fs::write(
        &session_file,
        r#"{"refreshToken":"offline","user":{"id":"user_1","email":"w@example.com"},"orgId":"org_1"}"#,
    )
    .expect("seed session");
    let auth = Auth::new(workos_config(&edge.url(), dir.path()));

    let refresh_loop = auth.spawn_refresh_loop();
    // Wait on the attempt landing, not on the clock. The old fixed 200 ms sleep
    // asserted the loop had already reached the edge, and on a loaded runner it
    // had not: the test read 0 refreshes and failed while being correct.
    wait_for_refreshes(&edge.state, 1).await;
    // Quiet window: the retry pause after a failed refresh is 30 s
    // (`spawn_refresh_loop`, crates/engine/src/auth.rs), so a second attempt
    // inside 200 ms would mean the loop is spinning instead of backing off.
    // This one stays a fixed sleep on purpose - it asserts that nothing
    // arrives, so a longer deadline would only slow the suite down.
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(
        edge.state.refreshes.load(Ordering::SeqCst),
        1,
        "a transport failure must enter the background retry delay"
    );
    assert!(auth.state().is_signed_in());
    assert!(session_file.exists(), "offline must not revoke the session");
    refresh_loop.abort();
}

#[tokio::test]
async fn loopback_callback_completes_headed_sign_in() {
    let edge = StubEdge::start().await;
    *edge.state.exchange_org.lock().expect("lock") = "org_1".into();
    let dir = tempfile::tempdir().expect("tempdir");
    let auth = Auth::new(workos_config(&edge.url(), dir.path()));

    let url = auth.start_sign_in().await.expect("authorize url");
    let redirect = query_param(&url, "redirect_uri").expect("redirect");
    assert!(
        redirect.starts_with("http%3A%2F%2F127.0.0.1%3A"),
        "loopback redirect: {redirect}"
    );
    let state = query_param(&url, "state").expect("state");
    let callback: String = redirect.replace("%3A", ":").replace("%2F", "/");

    // A wrong/expired state is rejected without touching the exchange endpoint.
    let bad = reqwest::get(format!("{callback}?code=abc&state=wrong"))
        .await
        .expect("bad cb");
    assert_eq!(bad.status().as_u16(), 400);
    assert_eq!(edge.state.exchanges.load(Ordering::SeqCst), 0);

    // The browser hits the loopback callback → the engine exchanges the code with the
    // edge and the session lands org-scoped.
    let ok = reqwest::get(format!("{callback}?code=abc&state={state}"))
        .await
        .expect("cb");
    assert_eq!(ok.status().as_u16(), 200);
    let mut state_rx = auth.watch_state();
    wait_for(&mut state_rx, |s| s.is_signed_in()).await;
    assert_eq!(edge.state.exchanges.load(Ordering::SeqCst), 1);
    assert!(
        matches!(auth.state(), AuthState::SignedIn { org_id: Some(org), user } if org == "org_1" && user.name.as_deref() == Some("Wing Test"))
    );
}

#[tokio::test]
async fn sign_out_invalidates_pending_and_in_flight_oauth_callbacks() {
    let edge = StubEdge::start().await;
    *edge.state.exchange_org.lock().expect("lock") = "org_1".into();
    let dir = tempfile::tempdir().expect("tempdir");
    let auth = Auth::new(workos_config(&edge.url(), dir.path()));

    let first_url = auth.start_sign_in().await.expect("authorize url");
    let callback = query_param(&first_url, "redirect_uri")
        .expect("redirect")
        .replace("%3A", ":")
        .replace("%2F", "/");
    let first_state = query_param(&first_url, "state").expect("state");
    auth.sign_out();
    let canceled_pending = reqwest::get(format!("{callback}?code=old&state={first_state}"))
        .await
        .expect("pending callback response");
    assert_eq!(canceled_pending.status().as_u16(), 400);
    assert_eq!(edge.state.exchanges.load(Ordering::SeqCst), 0);

    edge.state.block_exchange.store(true, Ordering::SeqCst);
    let second_url = auth.start_sign_in().await.expect("second authorize url");
    let second_state = query_param(&second_url, "state").expect("second state");
    let callback_request = tokio::spawn(reqwest::get(format!(
        "{callback}?code=in-flight&state={second_state}"
    )));
    tokio::time::timeout(
        surya_test_deadlines::WAIT,
        edge.state.exchange_started.notified(),
    )
    .await
    .expect("exchange did not start");

    auth.sign_out();
    edge.state.release_exchange.notify_one();
    let canceled_exchange = callback_request
        .await
        .expect("callback task")
        .expect("in-flight callback response");

    assert_eq!(canceled_exchange.status().as_u16(), 409);
    assert_eq!(auth.state(), AuthState::SignedOut);
    assert!(!dir.path().join("session.json").exists());
    assert_eq!(edge.state.exchanges.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn detect_probes_edge_dev_mode() {
    // A stub that reports auth:"dev" forces dev mode even with a client id configured.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let task = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            if read_request(&mut stream).await.is_some() {
                respond(&mut stream, "200 OK", r#"{"ok":true,"auth":"dev"}"#).await;
            }
        }
    });
    let dir = tempfile::tempdir().expect("tempdir");
    let mut config = workos_config(&format!("http://127.0.0.1:{port}"), dir.path());
    config.dev_user_id = "dev-w".into();
    let auth = Auth::detect(config).await;
    assert!(!auth.workos_enabled(), "edge dev mode wins");
    assert_eq!(auth.access_token().await.as_deref(), Some("dev-w"));
    task.abort();
}
