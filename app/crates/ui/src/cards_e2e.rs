//! Headless end-to-end proof of the card path, one counter pair per hop:
//! the fake claude CLI calls `mcp__surya__show_card`, the harness reads the
//! card the sidecar recorded and emits a Card event, the doc fold turns it
//! into a Card part, the Loro session doc round-trips that part, the
//! transcript builds one Card row from it, and the A2UI parser accepts what
//! came out unchanged. Nothing here needs a window.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::{mpsc, oneshot};
use surya_doc::{
    MessagePart, MessageRole, MessageStatus, SessionDoc, SessionMessageEntry, fold_event_into_parts,
};
use surya_harness::{CancellationToken, ClaudeHarness, Harness, RunControls};
use surya_proto::{AgentEvent, RunRequest, SandboxLevel, SuryaOptions, UserInputAnswer};

use crate::transcript::{RowKind, rows_for_entry};

/// The tool_use id `fake-claude.sh` gives the show_card call that succeeds
/// (`scenario:card`). The sidecar stamps the same id on the store record.
const TOOL_USE_ID: &str = "toolu_card_ok";
/// `scenario:card` makes two show_card calls; one fails and keeps its chip.
const SHOW_CARD_CALLS: usize = 2;
const CARD_ID: &str = "card-01-vehicle";

fn fake_claude() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../harness/tests/fixtures/fake-claude.sh")
        .canonicalize()
        .expect("fake-claude fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
    }
    path
}

fn fixture_01() -> Vec<serde_json::Value> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../a2ui/fixtures/01-vehicle.json");
    let text = std::fs::read_to_string(&path).expect("fixture 01");
    match serde_json::from_str(&text).expect("fixture 01 is JSON") {
        serde_json::Value::Array(items) => items,
        other => vec![other],
    }
}

/// Exactly one line, shaped like the sidecar's own record.
fn seed_store(dir: &Path, a2ui: &[serde_json::Value]) -> PathBuf {
    let store = dir.join("cards.jsonl");
    let record = serde_json::json!({
        "card_id": CARD_ID,
        "surface_id": "vehicle_kss_0412",
        "tool_use_id": TOOL_USE_ID,
        "agent_id": "seat-1",
        "workspace": "demo",
        "at": "2026-09-05T00:00:00Z",
        "a2ui": a2ui,
    });
    std::fs::write(&store, record.to_string() + "\n").expect("seed store");
    store
}

fn request(store: &Path) -> RunRequest {
    RunRequest {
        surya: Some(SuryaOptions {
            agent_id: "seat-1".into(),
            workspace: "demo".into(),
            mcp_binary: None,
            card_store: Some(store.to_string_lossy().into()),
            mail_socket: None,
            mail_log: None,
            catalog_id: None,
        }),
        prompt: "scenario:card".into(),
        harness: None,
        model: None,
        reasoning: None,
        model_options: serde_json::Map::new(),
        cwd: String::new(),
        sandbox: SandboxLevel::DangerFullAccess,
        auto_approve: true,
        attachments: Vec::new(),
        worktree: None,
        resume: None,
    }
}

fn controls() -> RunControls {
    let (_steer_tx, steer_rx) = mpsc::channel(8);
    RunControls {
        request_input: Box::new(|questions| {
            let (tx, rx) = oneshot::channel();
            let answers: Vec<UserInputAnswer> = questions
                .iter()
                .map(|q| UserInputAnswer {
                    question_id: q.id.clone(),
                    labels: vec!["A".into()],
                })
                .collect();
            let _ = tx.send(answers);
            rx
        }),
        steering: steer_rx,
        interrupt: CancellationToken::new(),
        permission: surya_harness::permission::PermissionGate::auto_allow(),
    }
}

fn card_parts(parts: &[MessagePart]) -> Vec<&MessagePart> {
    parts
        .iter()
        .filter(|p| matches!(p, MessagePart::Card { .. }))
        .collect()
}

#[tokio::test]
async fn a_shown_card_reaches_the_transcript_row_and_parses_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = fixture_01();
    let store = seed_store(dir.path(), &fixture);

    // Hop 1: fake claude -> harness Card event.
    let harness = ClaudeHarness::new().with_executable(fake_claude());
    let stream = harness
        .run(request(&store), controls())
        .await
        .expect("run starts");
    let events: Vec<AgentEvent> = tokio::time::timeout(
        Duration::from_secs(10),
        stream.map(|r| r.expect("stream event")).collect::<Vec<_>>(),
    )
    .await
    .expect("run finished in time");
    let card_events: Vec<&AgentEvent> = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::Card { .. }))
        .collect();
    eprintln!(
        "hop1 harness: show_card_calls={SHOW_CARD_CALLS} card_events={}",
        card_events.len()
    );
    assert_eq!(card_events.len(), 1, "one drawn card, one failed one");
    match card_events[0] {
        AgentEvent::Card {
            tool_use_id,
            card_id,
            a2ui,
            ..
        } => {
            assert_eq!(tool_use_id, TOOL_USE_ID);
            assert_eq!(card_id, CARD_ID);
            assert_eq!(a2ui, &fixture, "the whole envelope list rides the event");
        }
        other => panic!("not a card: {other:?}"),
    }

    // Hop 2: events -> parts through the doc fold.
    let mut parts = Vec::new();
    for event in &events {
        fold_event_into_parts(&mut parts, event);
    }
    let folded = card_parts(&parts);
    eprintln!(
        "hop2 fold: card_events={} card_parts={}",
        card_events.len(),
        folded.len()
    );
    assert_eq!(folded.len(), 1);
    match folded[0] {
        MessagePart::Card { id, a2ui, .. } => {
            assert_eq!(id, TOOL_USE_ID, "the part keys on the tool_use id");
            assert_eq!(a2ui.as_ref(), Some(&fixture));
        }
        other => panic!("not a card part: {other:?}"),
    }

    // Hop 3: parts -> Loro session doc -> parts.
    let doc = SessionDoc::init("chat-e2e").expect("doc");
    doc.push_message(&SessionMessageEntry {
        id: "m1".into(),
        role: MessageRole::Assistant,
        parts: parts.clone(),
        created_at: 0,
        device_id: "test".into(),
        status: Some(MessageStatus::Complete),
        continuation_of: None,
        source: None,
    })
    .expect("push");
    let entries = doc.read_entries().expect("read");
    let read: Vec<&MessagePart> = entries.iter().flat_map(|e| card_parts(&e.parts)).collect();
    eprintln!(
        "hop3 doc: card_parts_written={} card_parts_read={}",
        folded.len(),
        read.len()
    );
    assert_eq!(read.len(), 1);
    assert_eq!(read[0], folded[0], "the doc hands back the same part");

    // Hop 4: doc entry -> transcript rows.
    let mut parse = |_: &str, text: &str| Arc::new(crate::markdown::parser::parse_full(text));
    let rows = rows_for_entry(&entries[0], false, &mut parse);
    let card_rows: Vec<_> = rows
        .iter()
        .filter_map(|r| match &r.kind {
            RowKind::Card { card_id, card } => Some((card_id.clone(), card.clone())),
            _ => None,
        })
        .collect();
    eprintln!(
        "hop4 rows: card_parts_read={} card_rows={}",
        read.len(),
        card_rows.len()
    );
    assert_eq!(card_rows.len(), 1);
    let (row_card_id, card) = &card_rows[0];
    assert_eq!(row_card_id.as_ref(), CARD_ID);

    // Hop 5: what the row holds parses exactly like the fixture read directly.
    let direct = surya_a2ui::parse_card(&serde_json::Value::Array(fixture.clone()));
    eprintln!(
        "hop5 parse: components_asked={} components_parsed={} errors={}",
        direct.components.len(),
        card.components.len(),
        card.errors.len()
    );
    assert!(card.root().is_some(), "root present");
    assert!(card.errors.is_empty(), "diagnostics: {:?}", card.errors);
    assert!(!direct.components.is_empty(), "the fixture is not empty");
    assert_eq!(card.components.len(), direct.components.len());
}
