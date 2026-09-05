//! The mail rulings that are not the happy path: a replayed delivery id, a
//! turn that errored, an address nobody is behind, and the surya-mcp seat's
//! own id surviving the trip.

mod mail_support;

use std::sync::Arc;
use std::time::Duration;

use mail_support::{CHAT_B, EchoHarness, FailingHarness, request, settled, user_texts, wait_for};
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_proto::HarnessId;

#[tokio::test(flavor = "multi_thread")]
async fn a_repeated_delivery_id_does_not_redeliver() {
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
        .send_with_id("seat-1", CHAT_B, "once", None, Some("d_same"))
        .await
        .expect("first send");
    wait_until!("the first copy to be acked", {
        core.mail
            .list(Some(CHAT_B))
            .await
            .unwrap()
            .iter()
            .any(|m| m.id == "d_same" && m.acked_at.is_some())
    });

    // The replay, with the same id and a different body.
    core.mail
        .send_with_id("seat-1", CHAT_B, "twice", None, Some("d_same"))
        .await
        .expect("replay is accepted");

    let rows = core.mail.list(Some(CHAT_B)).await.unwrap();
    assert_eq!(rows.len(), 1, "one row, not two");
    assert_eq!(rows[0].body, "once", "the replay did not overwrite the row");
    assert!(rows[0].acked_at.is_some(), "the ack survived the replay");
    let (sent, delivered, acked) = core.mail.counts().await.unwrap();
    println!("replay: sent={sent} delivered={delivered} acked={acked}");
    assert_eq!((sent, delivered, acked), (1, 1, 1));

    let carried = user_texts(&core, CHAT_B)
        .iter()
        .filter(|t| t.contains("[MAIL d_same"))
        .count();
    assert_eq!(carried, 1, "the agent saw it once: carried=1");

    core.shutdown().await;
}

/// A run that errors did not read the mail. The row goes back to the queue.

#[tokio::test(flavor = "multi_thread")]
async fn an_errored_turn_requeues_its_mail() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = HarnessRegistry::new();
    registry.register(Arc::new(FailingHarness));
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
    wait_for(|| settled(&core, CHAT_B), "B settled after a failing turn").await;

    core.mail
        .send("owner", CHAT_B, "did you get this", None)
        .await
        .expect("send");

    // It went out, the run errored, and it is back in the queue unacked.
    wait_until!("the mail to be requeued after the errored run", {
        core.mail
            .list(Some(CHAT_B))
            .await
            .unwrap()
            .iter()
            .any(|m| m.delivered_at.is_none() && m.acked_at.is_none() && m.run_id.is_none())
    });
    let (sent, delivered, acked) = core.mail.counts().await.unwrap();
    println!("errored turn: sent={sent} delivered={delivered} acked={acked}");
    assert_eq!(
        (sent, delivered, acked),
        (1, 0, 0),
        "an errored turn never counts as delivered or acked"
    );

    core.shutdown().await;
}

/// A `#workspace` nobody is in is an error, not an empty success.

#[tokio::test(flavor = "multi_thread")]
async fn an_empty_workspace_address_is_an_error() {
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

    let err = core
        .mail
        .send("owner", "#nobody-is-here", "anyone?", None)
        .await
        .expect_err("an empty fan-out must not report success");
    let message = err.to_string();
    assert!(
        message.contains("#nobody-is-here"),
        "the error names the address: {message}"
    );
    let (sent, delivered, acked) = core.mail.counts().await.unwrap();
    println!("empty workspace: sent={sent} delivered={delivered} acked={acked}");
    assert_eq!((sent, delivered, acked), (0, 0, 0));

    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_seats_delivery_id_becomes_the_row_id() {
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

    let receipt = core
        .mail
        .send_with_id(
            "seat-1",
            "seat-2",
            "the migration is ready",
            None,
            Some("d_abc"),
        )
        .await
        .expect("send");
    assert_eq!(receipt.ids, vec!["d_abc".to_string()]);
    let row = core
        .mail
        .list(Some("seat-2"))
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("row");
    assert_eq!(row.id, "d_abc");
    assert_eq!(row.body, "the migration is ready");
    assert!(
        core.mail.ack("d_abc").await.expect("ack"),
        "the seat's id acks"
    );

    core.shutdown().await;
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
    // SAFETY: single-threaded setup, before any engine task reads the env.
    unsafe { std::env::set_var("ZERON_MAIL_SET_RUN_DELAY_MS", "400") };
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

    unsafe { std::env::remove_var("ZERON_MAIL_SET_RUN_DELAY_MS") };
    core.shutdown().await;
}
