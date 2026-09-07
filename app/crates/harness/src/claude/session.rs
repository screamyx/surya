//! The per-run event loop and the task that owns the child's stdin. Moved
//! out of `mod.rs` under the 500-line rule (issue #159); the code itself is
//! unchanged.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::mpsc;

use surya_proto::{AgentEvent, DoneStatus, ReasoningLevel};

use super::catalog::apply_ultrathink;
use super::control::handle_control_request;
use super::normalize::Normalizer;
use super::wire::{self, Frame};
use crate::{HarnessError, RunControls, Signal, send_signal, shutdown_child};

pub(super) enum StdinMsg {
    Line(String),
    /// Close stdin (end of steering input): the CLI finishes the current turn
    /// and exits, which ends the run stream at stdout EOF.
    Close,
}

/// Owns the child's stdin; a write failure (EPIPE after the child died) is
/// tolerated and logged.
pub(super) async fn stdin_writer(mut stdin: ChildStdin, mut rx: mpsc::UnboundedReceiver<StdinMsg>) {
    while let Some(msg) = rx.recv().await {
        match msg {
            StdinMsg::Line(line) => {
                let write = async {
                    stdin.write_all(line.as_bytes()).await?;
                    stdin.write_all(b"\n").await?;
                    stdin.flush().await
                };
                if let Err(e) = write.await {
                    tracing::debug!(target: "surya_harness::claude", "stdin write failed (tolerated): {e}");
                    return;
                }
            }
            StdinMsg::Close => {
                let _ = stdin.shutdown().await;
                return;
            }
        }
    }
}

pub(super) struct Session {
    pub(super) child: Child,
    pub(super) stdout_lines: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    pub(super) stdin_tx: mpsc::UnboundedSender<StdinMsg>,
    pub(super) event_tx: mpsc::Sender<Result<AgentEvent, HarnessError>>,
    pub(super) controls: RunControls,
    pub(super) reasoning: Option<ReasoningLevel>,
    pub(super) interrupt_grace: Duration,
    pub(super) kill_grace: Duration,
    /// Rolling stderr tail for the crash message on an unexpected exit.
    pub(super) stderr_tail: crate::StderrTail,
    /// Where the surya sidecar records cards, when this run has one.
    pub(super) card_store: Option<PathBuf>,
}

/// The per-run event loop: one task multiplexing stdout frames, the steering
/// mailbox, the interrupt token, and consumer liveness.
pub(super) async fn run_session(session: Session) {
    let Session {
        mut child,
        mut stdout_lines,
        stdin_tx,
        event_tx,
        controls,
        reasoning,
        interrupt_grace,
        kill_grace,
        stderr_tail,
        card_store,
    } = session;
    let RunControls {
        request_input,
        mut steering,
        interrupt,
        permission,
    } = controls;
    let request_input = Arc::new(request_input);

    let mut norm = Normalizer::new().with_card_store(card_store);
    let mut steering_open = true;
    let mut interrupted = false;
    let mut interrupt_sent = false;
    let mut any_done = false;
    let mut done_after_interrupt = false;
    let mut escalation: Option<tokio::task::JoinHandle<()>> = None;

    'main: loop {
        tokio::select! {
            line = stdout_lines.next_line() => match line {
                Ok(Some(line)) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let frame = match wire::parse_frame(line) {
                        Ok(frame) => frame,
                        Err(e) => {
                            tracing::debug!(target: "surya_harness::claude", "unparseable frame (skipped): {e}");
                            continue;
                        }
                    };
                    if let Frame::ControlRequest(req) = frame {
                        handle_control_request(req, &request_input, &permission, &stdin_tx);
                        continue;
                    }
                    for ev in norm.normalize(frame, interrupted) {
                        let is_done = matches!(ev, AgentEvent::Done { .. });
                        if event_tx.send(Ok(ev)).await.is_err() {
                            break 'main; // consumer gone - reap below
                        }
                        if is_done {
                            any_done = true;
                            if interrupted {
                                done_after_interrupt = true;
                                break 'main;
                            }
                        }
                    }
                }
                Ok(None) => break 'main, // stdout EOF: the CLI exited
                Err(e) => {
                    let _ = event_tx.send(Err(HarnessError::Io(e))).await;
                    break 'main;
                }
            },

            steer = steering.recv(), if steering_open && !interrupted => match steer {
                Some(msg) => {
                    let line = wire::user_message_line(&apply_ultrathink(reasoning, &msg.prompt));
                    let _ = stdin_tx.send(StdinMsg::Line(line));
                    // The CLI consumes the queued line at its own step
                    // boundary; rotate the assistant message id so post-steer
                    // output folds into a fresh message.
                    let (prev, next) = norm.rotate_for_steer();
                    let ev = AgentEvent::Steered {
                        assistant_message_id: Some(prev),
                        next_assistant_message_id: Some(next),
                    };
                    if event_tx.send(Ok(ev)).await.is_err() {
                        break 'main;
                    }
                }
                None => {
                    // Mailbox closed: end the input so the run can finish
                    // after the current turn.
                    steering_open = false;
                    let _ = stdin_tx.send(StdinMsg::Close);
                }
            },

            _ = interrupt.cancelled(), if !interrupt_sent => {
                interrupt_sent = true;
                interrupted = true;
                let _ = stdin_tx.send(StdinMsg::Line(wire::interrupt_request_line("int_1")));
                // Escalate if the CLI doesn't wind down within the grace
                // periods: SIGTERM (kills bash trees, runs SessionEnd hooks),
                // then SIGKILL. Aborted once the child is reaped.
                if let Some(pid) = child.id() {
                    escalation = Some(tokio::spawn(async move {
                        tokio::time::sleep(interrupt_grace).await;
                        send_signal(pid, Signal::Term);
                        tokio::time::sleep(kill_grace).await;
                        send_signal(pid, Signal::Kill);
                    }));
                }
            },

            _ = event_tx.closed() => break 'main,
        }
    }

    // Terminal bookkeeping: never end the stream without a Done unless the
    // consumer already hung up.
    if !event_tx.is_closed() {
        // A crash or an EOF skips the `result` frame, so nothing has flushed
        // the held-back `show_card` calls. Do it here, ahead of the synthetic
        // Done: an agent that died drawing a card must still show that it
        // tried. (These Dones are sent straight down the channel and never
        // pass through the normalizer, so the flush cannot ride along.)
        for event in norm.flush_card_calls() {
            let _ = event_tx.send(Ok(event)).await;
        }
        if interrupted && !done_after_interrupt {
            let _ = event_tx
                .send(Ok(AgentEvent::Done {
                    status: DoneStatus::Interrupted,
                    result: None,
                    error: None,
                    session_id: norm.session_id.clone(),
                }))
                .await;
        } else if !interrupted && !any_done {
            let status = child.try_wait().ok().flatten();
            let _ = event_tx
                .send(Ok(AgentEvent::Done {
                    status: DoneStatus::Errored,
                    result: None,
                    error: Some(crate::crash_message("claude", status, &stderr_tail)),
                    session_id: norm.session_id.clone(),
                }))
                .await;
        }
    }

    shutdown_child(&mut child, kill_grace).await;
    if let Some(handle) = escalation {
        handle.abort();
    }
}
