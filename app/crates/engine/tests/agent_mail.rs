//! Agent mail end to end (decision 19), on two mock-harness sessions.
//!
//! Proof 1: A mails B. B's next turn carries the envelope block, the row is
//! marked delivered, and the ack lands when that turn ends —
//! `sent=1 delivered=1 acked=1`.
//! Proof 2: one `#workspace` send fans out to both live agents —
//! `sent=1 delivered=2`.

mod mail_support;

use std::sync::Arc;
use std::time::Duration;

use mail_support::{CHAT_A, CHAT_B, EchoHarness, SPACE, request, settled, user_texts, wait_for};
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_proto::HarnessId;

#[tokio::test(flavor = "multi_thread")]
async fn mail_rides_the_recipients_next_turn_and_acks_when_it_ends() {
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
        .create_space(SPACE, &core.device_id, "/tmp/mail-space", None, false)
        .expect("space");
    for chat in [CHAT_A, CHAT_B] {
        core.workspace
            .create_chat(chat, Some(SPACE), Some(&core.device_id), None, None)
            .expect("chat");
        core.workspace
            .rename_chat(chat, "Pre-titled")
            .expect("title");
    }

    // Both agents run once, so each has a live session and a run
    // configuration a mail turn can borrow.
    for chat in [CHAT_A, CHAT_B] {
        core.sessions
            .dispatch(chat, HarnessId::Mock, request("first turn"), None)
            .await
            .expect("first turn");
    }
    wait_for(
        || settled(&core, CHAT_A) && settled(&core, CHAT_B),
        "both agents idle after their first turn",
    )
    .await;

    // ---- Proof 1: one message, one recipient.
    let receipt = core
        .mail
        .send_verified(CHAT_A, CHAT_B, "please check the diff", None)
        .await
        .expect("send");
    assert_eq!(receipt.ids.len(), 1, "one recipient, one delivery id");
    assert_eq!(receipt.recipients, vec![CHAT_B.to_string()]);
    let id = receipt.ids[0].clone();
    let expected = format!("[MAIL {id} from {CHAT_A}]\n  please check the diff\n[/MAIL {id}]");

    wait_for(
        || user_texts(&core, CHAT_B).iter().any(|t| t == &expected),
        "B's turn to carry the envelope",
    )
    .await;
    wait_until!("the carrying turn to complete and ack the mail", {
        core.mail
            .list(Some(CHAT_B))
            .await
            .unwrap()
            .iter()
            .any(|m| m.id == id && m.acked_at.is_some())
    });

    let (sent, delivered, acked) = core.mail.counts().await.expect("counts");
    println!("proof 1: sent={sent} delivered={delivered} acked={acked}");
    assert_eq!((sent, delivered, acked), (1, 1, 1));

    // The delivered row records the run that carried it.
    let row = core
        .mail
        .list(Some(CHAT_B))
        .await
        .unwrap()
        .into_iter()
        .find(|m| m.id == id)
        .expect("row");
    assert!(row.run_id.is_some(), "the carrying run is recorded");
    assert_eq!(row.from_device, core.device_id);
    assert_eq!(row.to_device, core.device_id);

    // ---- Proof 2: one #workspace send, two live agents.
    wait_for(|| settled(&core, CHAT_B), "B idle again").await;
    let fanout = core
        .mail
        .send("owner", &format!("#{SPACE}"), "standup in five", None)
        .await
        .expect("fan-out send");
    let mut recipients = fanout.recipients.clone();
    recipients.sort();
    assert_eq!(recipients, vec![CHAT_A.to_string(), CHAT_B.to_string()]);
    println!(
        "proof 2: sent=1 recipients={} ids={}",
        fanout.recipients.len(),
        fanout.ids.len()
    );

    let ids = fanout.ids.clone();
    wait_until!("both fan-out copies delivered", {
        let all = core.mail.list(None).await.unwrap();
        ids.iter()
            .all(|id| all.iter().any(|m| &m.id == id && m.delivered_at.is_some()))
    });

    let delivered_fanout = {
        let all = core.mail.list(None).await.unwrap();
        ids.iter()
            .filter(|id| all.iter().any(|m| &&m.id == id && m.delivered_at.is_some()))
            .count()
    };
    println!("proof 2: sent=1 delivered={delivered_fanout}");
    assert_eq!(delivered_fanout, 2);

    for (chat, id) in [CHAT_A, CHAT_B].iter().zip(fanout.ids.iter()) {
        let line =
            format!("[MAIL {id} from unverified:owner]\n  standup in five\n[/MAIL {id}]");
        assert!(
            user_texts(&core, chat).iter().any(|t| t == &line),
            "{chat} must carry its own copy: {line}"
        );
    }

    core.shutdown().await;
}


#[tokio::test(flavor = "multi_thread")]
async fn mail_to_an_unknown_agent_queues_instead_of_failing() {
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

    // Reserve-on-spawn: the address exists before the agent does.
    let receipt = core
        .mail
        .send("owner", "not-created-yet", "welcome", None)
        .await
        .expect("send to an unknown agent is accepted");
    assert_eq!(receipt.ids.len(), 1);
    let (sent, delivered, acked) = core.mail.counts().await.expect("counts");
    println!("unknown recipient: sent={sent} delivered={delivered} acked={acked}");
    assert_eq!((sent, delivered, acked), (1, 0, 0));

    core.shutdown().await;
}

/// The surya-mcp seat's record shape, straight through the engine: its
/// `delivery_id` becomes the row id, and `text` is the body.
