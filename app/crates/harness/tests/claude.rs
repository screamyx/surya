//! ClaudeHarness integration tests against the fake CLI in
//! `tests/fixtures/fake-claude.sh` (no real `claude` binary involved).
//! A live smoke test against the real CLI lives at the bottom, `#[ignore]`d.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::{mpsc, oneshot};

use surya_harness::{
    CancellationToken, ClaudeHarness, Harness, HarnessError, RunControls, SteerMessage,
};
use surya_proto::{
    AgentEvent, DoneStatus, HarnessId, RunRequest, SandboxLevel, ToolCall, UserInputAnswer,
    UserInputQuestion,
};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("fake-claude.sh")
}

fn harness() -> ClaudeHarness {
    ClaudeHarness::new().with_executable(fixture_path())
}

fn request(prompt: &str) -> RunRequest {
    RunRequest {
        surya: None,
        prompt: prompt.into(),
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

/// Controls whose `request_input` answers every question with `answer_label`.
fn controls(
    answer_label: &'static str,
) -> (RunControls, mpsc::Sender<SteerMessage>, CancellationToken) {
    let (steer_tx, steer_rx) = mpsc::channel(8);
    let token = CancellationToken::new();
    let controls = RunControls {
        request_input: Box::new(move |questions| {
            let (tx, rx) = oneshot::channel();
            let answers: Vec<UserInputAnswer> = questions
                .iter()
                .map(|q| UserInputAnswer {
                    question_id: q.id.clone(),
                    labels: vec![answer_label.into()],
                })
                .collect();
            let _ = tx.send(answers);
            rx
        }),
        steering: steer_rx,
        interrupt: token.clone(),
        permission: surya_harness::permission::PermissionGate::auto_allow(),
    };
    (controls, steer_tx, token)
}

async fn run_to_end(
    harness: &ClaudeHarness,
    req: RunRequest,
    controls: RunControls,
) -> Vec<AgentEvent> {
    let stream = harness.run(req, controls).await.expect("run starts");
    tokio::time::timeout(
        surya_test_deadlines::WAIT,
        stream.map(|r| r.expect("stream event")).collect::<Vec<_>>(),
    )
    .await
    .expect("run finished in time")
}

/// The `scenario:permission` fixture blocks on a real host gate, so a
/// smoke run (and this test) can watch a permission fill the needs-you queue
/// and drain. Both decisions must reach the CLI: an allow runs the tool, a
/// deny ends the turn without it.
#[tokio::test]
async fn a_host_gate_decides_whether_the_tool_runs() {
    for (decision, expected) in [
        (surya_proto::PermissionDecision::Allow, "permission allowed"),
        (surya_proto::PermissionDecision::Deny, "permission denied"),
    ] {
        let asked = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = asked.clone();
        let (mut controls, _steer, _token) = controls("A");
        controls.permission =
            surya_harness::permission::PermissionGate::new(move |request| {
                seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                assert_eq!(request.tool_name, "Bash");
                assert_eq!(request.command, "php artisan migrate --seed");
                let (tx, rx) = tokio::sync::oneshot::channel();
                let _ = tx.send(decision);
                rx
            });
        let events = run_to_end(&harness(), request("scenario:permission"), controls).await;
        let result = events.iter().find_map(|e| match e {
            AgentEvent::Done { result, .. } => result.clone(),
            _ => None,
        });
        let ran_tool = events
            .iter()
            .any(|e| matches!(e, AgentEvent::ToolCall { id, .. } if id == "tool-perm"));
        println!(
            "{decision:?}: asked={} result={result:?} ran_tool={ran_tool}",
            asked.load(std::sync::atomic::Ordering::SeqCst)
        );
        assert_eq!(
            asked.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the gate is asked exactly once"
        );
        assert_eq!(result.as_deref(), Some(expected));
        assert_eq!(
            ran_tool,
            decision == surya_proto::PermissionDecision::Allow,
            "the tool runs only on an allow"
        );
    }
}

#[tokio::test]
async fn happy_path_normalizes_events_and_tags_subagents() {
    let (controls, _steer, _token) = controls("A");
    let events = run_to_end(&harness(), request("scenario:happy"), controls).await;

    // One SessionStarted despite the re-emitted init frame.
    let starts: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::SessionStarted {
                harness,
                model,
                tools,
                session_id,
                ..
            } => Some((harness, model, tools, session_id)),
            _ => None,
        })
        .collect();
    assert_eq!(starts.len(), 1, "init must be deduped: {events:?}");
    let (h, model, tools, session_id) = starts[0];
    assert_eq!(*h, HarnessId::ClaudeCode);
    assert_eq!(model, "claude-fable-5");
    assert_eq!(tools, &vec!["Bash".to_string(), "Read".to_string()]);
    assert_eq!(session_id, "sess-1");

    assert!(events.contains(&AgentEvent::ReasoningDelta {
        text: "pondering".into()
    }));
    assert!(events.contains(&AgentEvent::TextDelta {
        text: "Hello".into()
    }));

    // Subagent frames (parent_tool_use_id set) arrive TAGGED — never as bare
    // parent-feed events.
    assert!(
        !events.iter().any(|e| matches!(
            e,
            AgentEvent::TextDelta { text } if text.contains("SUBAGENT")
        )),
        "subagent delta leaked into the parent feed: {events:?}"
    );
    assert!(
        !events.iter().any(|e| matches!(
            e,
            AgentEvent::ToolCall { id, .. } | AgentEvent::ToolResult { id, .. } if id == "sub-tool"
        )),
        "subagent tool frames leaked into the parent feed: {events:?}"
    );
    assert!(events.contains(&AgentEvent::Subagent {
        parent_tool_use_id: "sub-1".into(),
        event: Box::new(AgentEvent::TextDelta {
            text: "SUBAGENT".into()
        }),
    }));
    assert!(events.contains(&AgentEvent::Subagent {
        parent_tool_use_id: "sub-1".into(),
        event: Box::new(AgentEvent::ToolCall {
            id: "sub-tool".into(),
            call: ToolCall::Exec {
                command: "echo sub".into()
            },
        }),
    }));
    assert!(events.contains(&AgentEvent::Subagent {
        parent_tool_use_id: "sub-1".into(),
        event: Box::new(AgentEvent::ToolResult {
            id: "sub-tool".into(),
            is_error: false,
            output: None,
            diff: None,
        }),
    }));

    // Typed tool decoding: Bash -> Exec, mcp__server__tool -> Mcp.
    assert!(events.contains(&AgentEvent::ToolCall {
        id: "tool-1".into(),
        call: ToolCall::Exec {
            command: "ls -la".into()
        },
    }));
    assert!(events.contains(&AgentEvent::ToolCall {
        id: "tool-2".into(),
        call: ToolCall::Mcp {
            server: "linear".into(),
            tool: "search".into(),
            input: Some(serde_json::json!({"q": "bug"})),
        },
    }));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::AssistantMessageCompleted { .. }))
    );
    assert!(events.contains(&AgentEvent::ToolResult {
        id: "tool-1".into(),
        is_error: false,
        output: None,
        diff: None,
    }));
    assert!(events.contains(&AgentEvent::ToolResult {
        id: "tool-2".into(),
        is_error: true,
        output: None,
        diff: None,
    }));

    // Informational rate-limit frames stay quiet.
    assert!(!events.iter().any(|e| matches!(e, AgentEvent::Error { .. })));

    assert!(events.contains(&AgentEvent::Usage {
        input_tokens: 10,
        output_tokens: 20
    }));
    assert_eq!(
        events.last(),
        Some(&AgentEvent::Done {
            status: DoneStatus::Completed,
            result: Some("done!".into()),
            error: None,
            session_id: Some("sess-1".into()),
        })
    );
}

#[tokio::test]
async fn eager_done_forwards_wake_turn_as_second_done() {
    // The background-subagent shape (live-verified 2.1.228): result #1 is
    // eager — the run must NOT hold the turn for the subagent — and the wake
    // turn's frames flow through the SAME stream, settling with result #2.
    let (controls, _steer, _token) = controls("A");
    let events = run_to_end(&harness(), request("scenario:wake"), controls).await;

    let done_positions: Vec<usize> = events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| matches!(e, AgentEvent::Done { .. }).then_some(i))
        .collect();
    assert_eq!(
        done_positions.len(),
        2,
        "eager done + wake done: {events:?}"
    );

    // One SessionStarted total — the wake init is deduped.
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, AgentEvent::SessionStarted { .. }))
            .count(),
        1
    );

    // The subagent's interior streams tagged BETWEEN the two dones.
    let tagged_position = events
        .iter()
        .position(|e| {
            matches!(e, AgentEvent::Subagent { parent_tool_use_id, .. } if parent_tool_use_id == "toolu_agent")
        })
        .expect("tagged subagent traffic present");
    assert!(
        done_positions[0] < tagged_position && tagged_position < done_positions[1],
        "subagent interior must stream between the eager done and the wake done: {events:?}"
    );

    // The wake turn's own (untagged) output precedes the second done.
    let wake_text = events
        .iter()
        .position(|e| matches!(e, AgentEvent::TextDelta { text } if text == "subagent finished"))
        .expect("wake-turn delta present");
    assert!(done_positions[0] < wake_text && wake_text < done_positions[1]);

    // Both dones settle Completed with the same session id.
    for i in done_positions {
        assert!(matches!(
            &events[i],
            AgentEvent::Done {
                status: DoneStatus::Completed,
                session_id: Some(id),
                ..
            } if id == "sess-wake"
        ));
    }
}

#[tokio::test]
async fn ask_user_question_round_trips_through_the_control_channel() {
    // The questions must reach the ENGINE's input bridge (`request_input`) —
    // and the harness must NOT emit its own `InputRequested`/`InputResolved`
    // twins: the bridge owns that lifecycle (it mints the request id the
    // resolver is parked under; a harness-emitted copy folded an unanswerable
    // duplicate chip into the doc).
    let asked: Arc<Mutex<Vec<UserInputQuestion>>> = Arc::new(Mutex::new(Vec::new()));
    let (steer_tx, steer_rx) = mpsc::channel(8);
    let _steer = steer_tx;
    let token = CancellationToken::new();
    let seen = asked.clone();
    let controls = RunControls {
        request_input: Box::new(move |questions| {
            seen.lock().unwrap().extend(questions.iter().cloned());
            let (tx, rx) = oneshot::channel();
            let answers: Vec<UserInputAnswer> = questions
                .iter()
                .map(|q| UserInputAnswer {
                    question_id: q.id.clone(),
                    labels: vec!["B".into()],
                })
                .collect();
            let _ = tx.send(answers);
            rx
        }),
        steering: steer_rx,
        interrupt: token.clone(),
        permission: surya_harness::permission::PermissionGate::auto_allow(),
    };
    let events = run_to_end(&harness(), request("scenario:askuser"), controls).await;

    let asked = asked.lock().unwrap();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].header, "Choice");
    assert_eq!(asked[0].question, "Pick one");
    assert_eq!(asked[0].options, vec!["A".to_string(), "B".to_string()]);
    assert!(
        !events.iter().any(|e| matches!(
            e,
            AgentEvent::InputRequested { .. } | AgentEvent::InputResolved { .. }
        )),
        "harness must not emit input lifecycle events itself: {events:?}"
    );

    // "answered" proves both control round-trips: the plain Bash can_use_tool
    // was auto-allowed AND the answers reached the CLI as updatedInput.answers
    // keyed by question text.
    assert_eq!(
        events.last(),
        Some(&AgentEvent::Done {
            status: DoneStatus::Completed,
            result: Some("answered".into()),
            error: None,
            session_id: Some("sess-ask".into()),
        })
    );
}

#[tokio::test]
async fn steering_lines_are_written_to_stdin_mid_run() {
    let (controls, steer, _token) = controls("A");
    steer
        .send(SteerMessage {
            prompt: "redirect please".into(),
            message_id: None,
        })
        .await
        .expect("steer queued");
    let events = run_to_end(&harness(), request("scenario:steer"), controls).await;

    let steered = events
        .iter()
        .find_map(|e| match e {
            AgentEvent::Steered {
                assistant_message_id,
                next_assistant_message_id,
            } => Some((
                assistant_message_id.clone(),
                next_assistant_message_id.clone(),
            )),
            _ => None,
        })
        .expect("Steered emitted");
    assert!(steered.0.is_some() && steered.1.is_some());
    assert_ne!(steered.0, steered.1);

    // The fake CLI echoes the steer line's content back as a delta.
    assert!(events.contains(&AgentEvent::TextDelta {
        text: "steered:redirect please".into()
    }));
    assert!(matches!(
        events.last(),
        Some(AgentEvent::Done {
            status: DoneStatus::Completed,
            ..
        })
    ));
}

#[tokio::test]
async fn interrupt_escalates_to_sigterm_and_ends_with_interrupted_done() {
    let harness = ClaudeHarness::new()
        .with_executable(fixture_path())
        .with_graces(Duration::from_millis(100), Duration::from_millis(500));
    let (controls, _steer, token) = controls("A");
    let mut stream = harness
        .run(request("scenario:interrupt"), controls)
        .await
        .expect("run starts");

    let events = tokio::time::timeout(surya_test_deadlines::WAIT, async move {
        let mut events = Vec::new();
        while let Some(ev) = stream.next().await {
            let ev = ev.expect("stream event");
            if matches!(ev, AgentEvent::SessionStarted { .. }) {
                token.cancel(); // interrupt as soon as the session is up
            }
            events.push(ev);
        }
        events
    })
    .await
    .expect("interrupt completed in time");

    assert_eq!(
        events.last(),
        Some(&AgentEvent::Done {
            status: DoneStatus::Interrupted,
            result: None,
            error: None,
            session_id: Some("sess-int".into()),
        })
    );
}

#[tokio::test]
async fn error_codes_map_to_readable_messages() {
    let (controls, _steer, _token) = controls("A");
    let events = run_to_end(&harness(), request("scenario:error"), controls).await;

    let errors: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::Error { message } => Some(message.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        errors.contains(&"Claude usage limit reached — try again after the limit resets."),
        "assistant error code not mapped: {errors:?}"
    );
    assert!(
        errors.contains(
            &"Claude 5-hour limit reached — the turn was blocked. Try again after it resets."
        ),
        "rejected rate_limit_event not mapped: {errors:?}"
    );

    // Empty `errors` array on the result falls back to subtype wording.
    assert_eq!(
        events.last(),
        Some(&AgentEvent::Done {
            status: DoneStatus::Errored,
            result: None,
            error: Some("The run hit the maximum number of turns.".into()),
            session_id: Some("sess-err".into()),
        })
    );
}

#[tokio::test]
async fn missing_binary_is_not_installed() {
    let harness = ClaudeHarness::new().with_executable("/nonexistent/claude-nowhere");
    let (controls, _steer, _token) = controls("A");
    let err = harness
        .run(request("scenario:happy"), controls)
        .await
        .err()
        .expect("spawn fails");
    assert!(matches!(err, HarnessError::NotInstalled(_)), "{err:?}");
}

#[tokio::test]
async fn captured_live_background_subagent_frames_replay_correctly() {
    // Frames captured VERBATIM from claude 2.1.228 (2026-08-17): a turn that
    // spawns a background Agent subagent — eager result while it runs, tagged
    // subagent traffic, then the wake turn (second init, same session id,
    // second result). Replayed through the fake-CLI transport so the whole
    // driver path (wire parse → normalize → run loop) is exercised, not just
    // the normalizer.
    //
    // The capture is named to the CHECKED-IN fake CLI through the prompt,
    // `scenario:replay:<path>`, and nothing is written at run time. This test
    // used to write its own replayer script and exec it, which is what made
    // the suite flaky on Linux: the write fd is inherited by a sibling test's
    // fork, and execve answers ETXTBSY until that child reaches its own exec.
    // Renaming into place does not help, the block is on the inode. See
    // `tests/exec_race.rs`, which pins both facts.
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("claude")
        .join("live-2.1.228-background-subagent.jsonl");
    assert!(fixture.is_file(), "capture missing: {fixture:?}");
    let cli = harness();
    let (controls, _steer, _token) = controls("A");
    let prompt = format!("scenario:replay:{}", fixture.display());
    let events = run_to_end(&cli, request(&prompt), controls).await;

    // One SessionStarted (the wake init dedupes), two Dones (eager + wake).
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, AgentEvent::SessionStarted { .. }))
            .count(),
        1,
        "{events:?}"
    );
    let dones: Vec<usize> = events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| matches!(e, AgentEvent::Done { .. }).then_some(i))
        .collect();
    assert_eq!(dones.len(), 2, "eager done + wake done: {events:?}");

    // The parent-feed Agent spawn is a plain tool call; the subagent's own
    // Bash call arrives tagged with the spawning tool-use id, between the
    // two dones, and never as a bare parent event.
    let spawn_id = events
        .iter()
        .find_map(|e| match e {
            AgentEvent::ToolCall { id, call } => match call {
                ToolCall::Unknown { name, .. } if name.starts_with("Agent") => Some(id.clone()),
                _ => None,
            },
            _ => None,
        })
        .expect("Agent spawn tool call in the parent feed");
    // The synthesized opening user message rides WITH the spawn (before the
    // eager done); the child's own interior streams between the two dones.
    let opening: Vec<usize> = events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            matches!(
                e,
                AgentEvent::Subagent { parent_tool_use_id, event }
                    if *parent_tool_use_id == spawn_id
                        && matches!(event.as_ref(), AgentEvent::UserMessage { .. })
            )
            .then_some(i)
        })
        .collect();
    assert_eq!(opening.len(), 1, "one seeded opening prompt: {events:?}");
    assert!(opening[0] < dones[0], "opening rides with the spawn");
    let tagged: Vec<usize> = events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            matches!(
                e,
                AgentEvent::Subagent { parent_tool_use_id, event }
                    if *parent_tool_use_id == spawn_id
                        && !matches!(event.as_ref(), AgentEvent::UserMessage { .. })
            )
            .then_some(i)
        })
        .collect();
    assert!(!tagged.is_empty(), "tagged subagent traffic: {events:?}");
    assert!(
        tagged.iter().all(|i| dones[0] < *i && *i < dones[1]),
        "subagent interior streams between the eager and wake dones"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            AgentEvent::Subagent { event, .. }
                if matches!(event.as_ref(), AgentEvent::ToolCall { call: ToolCall::Exec { .. }, .. })
        )),
        "subagent Bash call arrives tagged: {events:?}"
    );
}

/// Live smoke against the REAL claude CLI (2.1.x, must be installed + authed):
/// one trivial turn through the stdio permission channel, ending on the
/// result frame. `cargo test -p surya-harness --test claude -- --ignored`.
#[tokio::test]
#[ignore = "spawns the real claude CLI; needs install + auth + network"]
async fn live_real_cli_single_turn() {
    let harness = ClaudeHarness::new();
    let mut req = request("Reply with exactly the word: pong");
    req.model = Some("haiku".into());
    req.cwd = std::env::temp_dir().display().to_string();
    req.auto_approve = false; // exercise --permission-prompt-tool stdio
    let (controls, _steer, _token) = controls("A");
    let mut stream = harness.run(req, controls).await.expect("run starts");
    // The session PARKS after the turn (steering mailbox still open, the CLI
    // waits for more stdin) — collect up to the first Done, not stream end.
    let events = tokio::time::timeout(Duration::from_secs(120), async {
        let mut events = Vec::new();
        while let Some(ev) = stream.next().await {
            let ev = ev.expect("stream event");
            let done = matches!(ev, AgentEvent::Done { .. });
            events.push(ev);
            if done {
                break;
            }
        }
        events
    })
    .await
    .expect("live turn finished in time");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::SessionStarted { .. })),
        "{events:?}"
    );
    assert!(matches!(
        events.last(),
        Some(AgentEvent::Done {
            status: DoneStatus::Completed,
            ..
        })
    ));
}

// ---------------------------------------------------------------------------
// Slash-command discovery
// ---------------------------------------------------------------------------

#[tokio::test]
async fn commands_come_from_the_initialize_control_request() {
    let h = harness();
    let commands = h.commands().await.expect("discovery succeeds");
    assert_eq!(commands.len(), 2, "nameless entries are dropped: {commands:?}");
    assert_eq!(commands[0].name, "review");
    assert_eq!(commands[0].description, "Review a pull request");
    assert_eq!(commands[0].input_hint.as_deref(), Some("[pr number]"));
    assert_eq!(commands[1].name, "compact");
    assert_eq!(commands[1].input_hint, None, "empty hint reads as None");

    // Cached: the second call reuses the first probe's result (the fake has
    // exited; a re-probe against a dead binary path would still work here,
    // but object identity of the cached list is the cheap assertion).
    let again = h.commands().await.expect("cache hit");
    assert_eq!(again, commands);
}

/// Live smoke against the real CLI: `cargo test -p surya-harness --test
/// claude -- --ignored live_commands`. No model turn, no API cost.
#[tokio::test]
#[ignore]
async fn live_commands_discovery() {
    let h = ClaudeHarness::new();
    let commands = h.commands().await.expect("live discovery");
    assert!(!commands.is_empty());
    eprintln!("{} commands, first: {:?}", commands.len(), commands.first());
}

/// A `mcp__surya__show_card` call whose result lands produces one Card event,
/// read out of the card store by the tool_use id the sidecar stamped on it.
/// A failed show_card produces none, and an ordinary tool never produces one.
#[tokio::test]
async fn a_resolved_show_card_call_emits_one_card_event_in_transcript_order() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("cards.jsonl");
    // Exactly what surya-mcp writes: one JSON line per shown card. The line
    // for the FAILED call is absent, as it would be - a rejected card writes
    // nothing.
    std::fs::write(
        &store,
        serde_json::json!({
            "card_id": "card_abc",
            "surface_id": "card_abc",
            "tool_use_id": "toolu_card_ok",
            "agent_id": "seat-1",
            "workspace": "demo",
            "at": "2026-09-05T00:00:00Z",
            "a2ui": [
                {"version": "v0.9.1", "createSurface": {"surfaceId": "card_abc", "catalogId": "c"}},
                {"version": "v0.9.1", "updateComponents": {"surfaceId": "card_abc", "components": [
                    {"id": "root", "component": "Card", "child": "body"}
                ]}}
            ]
        })
        .to_string()
            + "\n",
    )
    .unwrap();

    let mut req = request("scenario:card");
    req.surya = Some(surya_proto::SuryaOptions {
        agent_id: "seat-1".into(),
        workspace: "demo".into(),
        mcp_binary: None,
        card_store: Some(store.to_string_lossy().into()),
        mail_socket: None,
        mail_log: None,
        catalog_id: None,
    });
    let (controls, _steer, _token) = controls("A");
    let events = run_to_end(&harness(), req, controls).await;

    // A drawn card replaces its own tool chip: no ToolCall and no ToolResult
    // for the call that succeeded, so the transcript shows the card alone.
    let card_chips = events
        .iter()
        .filter(|e| {
            matches!(e, AgentEvent::ToolCall { id, .. } if id == "toolu_card_ok")
                || matches!(e, AgentEvent::ToolResult { id, .. } if id == "toolu_card_ok")
        })
        .count();
    let cards: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::Card {
                card_id,
                surface_id,
                tool_use_id,
                a2ui,
            } => Some((card_id, surface_id, tool_use_id, a2ui)),
            _ => None,
        })
        .collect();
    assert_eq!(
        (cards.len(), card_chips),
        (1, 0),
        "tool_uses=2 card_events={} chips_for_the_drawn_card={card_chips}",
        cards.len()
    );
    let (card_id, surface_id, tool_use_id, a2ui) = cards[0];
    assert_eq!(card_id, "card_abc");
    assert_eq!(surface_id, "card_abc");
    assert_eq!(tool_use_id, "toolu_card_ok");
    assert_eq!(a2ui.len(), 2, "the whole envelope list rides the event");

    // A show_card that FAILED keeps its pair, or the user would never learn
    // the agent tried to draw something.
    let failed_call = events
        .iter()
        .any(|e| matches!(e, AgentEvent::ToolCall { id, .. } if id == "toolu_card_bad"));
    let failed_result = events.iter().any(
        |e| matches!(e, AgentEvent::ToolResult { id, is_error, .. } if id == "toolu_card_bad" && *is_error),
    );
    assert!(failed_call && failed_result, "a failed card stays visible");

    // An ordinary tool is untouched.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::ToolCall { id, .. } if id == "toolu_bash"))
    );

    // The card lands where the call was: after the preceding frame's events
    // and before the next assistant frame's tool call.
    let position = |predicate: &dyn Fn(&AgentEvent) -> bool| {
        events.iter().position(|e| predicate(e)).expect("event present")
    };
    let card = position(&|e| matches!(e, AgentEvent::Card { .. }));
    let later_call =
        position(&|e| matches!(e, AgentEvent::ToolCall { id, .. } if id == "toolu_bash"));
    assert!(card < later_call, "card={card} later_call={later_call}");
}

/// Without a store, a SHORT-FORM card lifts to nothing: its A2UI lives in the
/// sidecar's record, not in the tool input. Better the tool chip than a blank
/// card where the user expected an answer.
#[tokio::test]
async fn without_a_store_a_short_form_card_shows_its_chip_not_a_blank_card() {
    let (controls, _steer, _token) = controls("A");
    let events = run_to_end(&harness(), request("scenario:card"), controls).await;
    let cards = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::Card { .. }))
        .count();
    let chip = events
        .iter()
        .any(|e| matches!(e, AgentEvent::ToolCall { id, .. } if id == "toolu_card_ok"));
    assert_eq!((cards, chip), (0, true), "card_events={cards} chip={chip}");
}

/// Raw A2UI in the call's input still draws with no store at all: that is what
/// the input lift is for.
#[tokio::test]
async fn without_a_store_raw_a2ui_input_still_draws() {
    let (controls, _steer, _token) = controls("A");
    let events = run_to_end(&harness(), request("scenario:card-raw"), controls).await;
    let cards: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::Card { .. }))
        .collect();
    let chips = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::ToolCall { id, .. } if id == "toolu_raw"))
        .count();
    assert_eq!(
        (cards.len(), chips),
        (1, 0),
        "card_events={} chips={chips}",
        cards.len()
    );
}

/// A card whose store record cannot be found is a VISIBLE failure, not an
/// empty card: with a store configured, its record is the only truth, so the
/// call falls back to its tool chip rather than to the call's own input.
#[tokio::test]
async fn a_configured_store_with_no_record_shows_the_chip_not_an_empty_card() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("cards.jsonl");
    // The store exists but holds a record for a different call.
    std::fs::write(
        &store,
        serde_json::json!({
            "card_id": "card_other",
            "surface_id": "s",
            "tool_use_id": "toolu_someone_else",
            "a2ui": []
        })
        .to_string()
            + "\n",
    )
    .unwrap();

    let mut req = request("scenario:card");
    req.surya = Some(surya_proto::SuryaOptions {
        agent_id: "seat-1".into(),
        workspace: "demo".into(),
        mcp_binary: None,
        card_store: Some(store.to_string_lossy().into()),
        mail_socket: None,
        mail_log: None,
        catalog_id: None,
    });
    let (controls, _steer, _token) = controls("A");
    let events = run_to_end(&harness(), req, controls).await;

    let cards = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::Card { .. }))
        .count();
    let chip = events
        .iter()
        .any(|e| matches!(e, AgentEvent::ToolCall { id, .. } if id == "toolu_card_ok"));
    let result = events
        .iter()
        .any(|e| matches!(e, AgentEvent::ToolResult { id, .. } if id == "toolu_card_ok"));
    assert_eq!(
        (cards, chip, result),
        (0, true, true),
        "card_events={cards} chip={chip} result={result}"
    );
}

/// A run that dies before the `result` frame still shows the card it was
/// drawing. The synthetic Done is sent straight down the channel and never
/// passes through the normalizer, so the held call has to be flushed there.
#[tokio::test]
async fn a_crash_mid_card_still_shows_the_held_call() {
    let (controls, _steer, _token) = controls("A");
    let events = run_to_end(&harness(), request("scenario:card-crash"), controls).await;

    let chip = events
        .iter()
        .any(|e| matches!(e, AgentEvent::ToolCall { id, .. } if id == "toolu_card_ok"));
    let done_is_last = matches!(events.last(), Some(AgentEvent::Done { .. }));
    assert!(chip, "the held call is flushed on the crash path: {events:?}");
    assert!(done_is_last, "and it lands before the Done: {events:?}");
}

/// A subagent's `show_card` is held back like the parent's, so a short-form
/// card never renders as an empty one on a nested transcript.
#[tokio::test]
async fn a_subagent_show_card_is_held_back_too() {
    let (controls, _steer, _token) = controls("A");
    let events = run_to_end(&harness(), request("scenario:card-subagent"), controls).await;

    // Everything a subagent emits stays wrapped and attributed.
    let inner: Vec<&AgentEvent> = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::Subagent { event, .. } => Some(event.as_ref()),
            _ => None,
        })
        .collect();
    let chips = inner
        .iter()
        .filter(|e| matches!(e, AgentEvent::ToolCall { id, .. } if id == "toolu_sub_card"))
        .count();
    let cards = inner
        .iter()
        .filter(|e| matches!(e, AgentEvent::Card { .. }))
        .count();
    // No store here, so the input lift draws it - the point is that the call
    // is held and resolved on its result, not emitted as a chip at call time.
    assert_eq!(
        (cards, chips),
        (1, 0),
        "subagent card_events={cards} chips={chips}; inner={inner:?}"
    );
}

/// Drawing a card must not cost the user a permission prompt (decisions 8 and
/// 11), while everything else stays gated.
///
/// Two runs with the same recorder. The first is the control: it proves the
/// recorder is wired and does see a gated tool. Only then does the second
/// run's empty list mean anything - without the control, a gate that was
/// never consulted at all would pass just as quietly.
#[tokio::test]
async fn show_card_never_reaches_the_permission_gate() {
    fn recorder(
        seen: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    ) -> surya_harness::permission::PermissionGate {
        surya_harness::permission::PermissionGate::new(move |request| {
            seen.lock().expect("the recorder is not poisoned").push(request.tool_name.clone());
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = tx.send(surya_proto::PermissionDecision::Allow);
            rx
        })
    }

    let control = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let (mut control_controls, _steer, _token) = controls("A");
    control_controls.permission = recorder(control.clone());
    run_to_end(&harness(), request("scenario:permission"), control_controls).await;
    let control_saw = control.lock().expect("not poisoned").clone();
    assert_eq!(
        control_saw,
        vec!["Bash".to_string()],
        "the recorder sees a gated tool: {control_saw:?}"
    );

    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let (mut card_controls, _steer, _token) = controls("B");
    card_controls.permission = recorder(seen.clone());
    let events = run_to_end(&harness(), request("scenario:card-perm"), card_controls).await;
    let card_saw = seen.lock().expect("not poisoned").clone();
    let result = events.iter().find_map(|e| match e {
        AgentEvent::Done { result, .. } => result.clone(),
        _ => None,
    });
    println!(
        "control asked={} card asked={} tools={card_saw:?} result={result:?}",
        control_saw.len(),
        card_saw.len()
    );
    assert_eq!(
        card_saw.len(),
        0,
        "control={control_saw:?} card asked={} tools={card_saw:?}",
        card_saw.len()
    );
    // The fixture blocks on the answer and reports "show_card was not
    // auto-allowed" if it never came, so a completed run is the other half.
    assert_eq!(result.as_deref(), Some("done!"), "the run still completed");
}
