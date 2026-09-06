//! One test, one binary, one env knob.
//!
//! `SURYA_MAIL_SET_RUN_DELAY_MS` is process-global, and CI runs
//! `cargo test --jobs 2` - sibling tests in the same binary run as threads in
//! one process, so setting and unsetting the knob around a test would change
//! delivery timing under whichever test happened to be running beside it.
//! Its own binary, set once, never removed: the value is true for the whole
//! process, which is the only way it is true at all.

mod mail_support;

use std::sync::{Arc, Once};
use std::time::Duration;

use mail_support::{CHAT_B, EchoHarness, request, settled, wait_for};
use surya_engine::{EngineCore, HarnessRegistry};
use surya_proto::HarnessId;

/// How far the run id lags `dispatch` returning, in this process.
const SET_RUN_DELAY_MS: u64 = 400;

fn init_delay_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // SAFETY: called before any engine (and thus any reader of the var)
        // exists in this test process; the one value holds for every test here.
        unsafe {
            std::env::set_var("SURYA_MAIL_SET_RUN_DELAY_MS", SET_RUN_DELAY_MS.to_string())
        };
    });
}

/// The turn can finish before the engine records which run carried the mail.
///
/// The ack pass ignores rows with no run id - that is what stops a status tick
/// during dispatch from acking a turn nobody read. So if Idle arrives while the
/// id is still unset, that transition is spent, and nothing else would ever
/// wake the pump for this agent: the row would stay delivered-and-unacked for
/// good. Main went red on exactly this, as a 15s timeout on a loaded runner.
///
/// The delay makes the window certain rather than lucky.
#[tokio::test(flavor = "multi_thread")]
async fn the_ack_survives_a_turn_that_ends_before_its_run_is_recorded() {
    init_delay_env();
    let tmp = tempfile::tempdir().unwrap();
    let registry = HarnessRegistry::new();
    registry.register(Arc::new(EchoHarness));
    let core = EngineCore::assemble(
        &tmp.path().join("data"),
        Arc::new(registry),
        HarnessId::Mock,
        None,
    )
    .expect("engine core assembles");

    core.workspace
        .create_chat(CHAT_B, None, Some(&core.device_id), None, None)
        .expect("chat");
    core.workspace
        .rename_chat(CHAT_B, "Pre-titled")
        .expect("title");
    core.sessions
        .dispatch(CHAT_B, HarnessId::Mock, request("first turn"), None)
        .await
        .expect("first turn");
    wait_for(|| settled(&core, CHAT_B), "B idle").await;

    core.mail
        .send("owner", CHAT_B, "read me", None)
        .await
        .expect("send");

    wait_until!("the ack to land despite the late run id", {
        core.mail
            .list(Some(CHAT_B))
            .await
            .unwrap()
            .iter()
            .any(|m| m.acked_at.is_some())
    });
    let (sent, delivered, acked) = core.mail.counts().await.unwrap();
    println!("late run id: sent={sent} delivered={delivered} acked={acked}");
    assert_eq!((sent, delivered, acked), (1, 1, 1));

    core.shutdown().await;
}
