//! Shared scaffolding for the agent-mail integration tests: two mock
//! harnesses, the poll helpers, and the transcript reader.
#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;

use zeron_doc::{MessageRole, SessionMessageEntry};
use zeron_engine::{EngineCore, HarnessRegistry};
use zeron_harness::{Harness, HarnessError, RunControls};
use zeron_proto::{
    AgentEvent, DoneStatus, HarnessId, Model, ReasoningLevel, RunRequest, SandboxLevel,
    SessionStatus, SteeringMode,
};

pub const CHAT_A: &str = "chat-a";
pub const CHAT_B: &str = "chat-b";
pub const SPACE: &str = "space-mail";

/// Completes a one-line turn for any request. Not steerable: every mail
/// arrives as its own turn, which is the shape the proof asserts on.
pub struct EchoHarness;

#[async_trait]
impl Harness for EchoHarness {
    fn id(&self) -> HarnessId {
        HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Echo"
    }
    fn supports_steering(&self) -> bool {
        false
    }
    fn steering_mode(&self) -> SteeringMode {
        SteeringMode::TurnBoundary
    }
    fn reasoning_levels(&self) -> &[ReasoningLevel] {
        &[ReasoningLevel::Medium]
    }
    async fn models(&self) -> Result<Vec<Model>, HarnessError> {
        Ok(vec![])
    }
    async fn run(
        &self,
        request: RunRequest,
        _controls: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        let events: Vec<Result<AgentEvent, HarnessError>> = vec![
            Ok(AgentEvent::SessionStarted {
                harness: HarnessId::Mock,
                model: "mock-1".into(),
                tools: vec![],
                cwd: request.cwd.clone(),
                session_id: "sess-mail".into(),
                assistant_message_id: format!("a-{}", request.prompt.len()),
            }),
            Ok(AgentEvent::TextDelta {
                text: format!("read: {}", request.prompt),
            }),
            Ok(AgentEvent::Done {
                status: DoneStatus::Completed,
                result: None,
                error: None,
                session_id: Some("sess-mail".into()),
            }),
        ];
        Ok(futures::stream::iter(events).boxed())
    }
}

/// Fails every turn: the run reaches Errored, never Idle.
pub struct FailingHarness;

#[async_trait]
impl Harness for FailingHarness {
    fn id(&self) -> HarnessId {
        HarnessId::Mock
    }
    fn display_name(&self) -> &str {
        "Failing"
    }
    fn supports_steering(&self) -> bool {
        false
    }
    fn steering_mode(&self) -> SteeringMode {
        SteeringMode::TurnBoundary
    }
    fn reasoning_levels(&self) -> &[ReasoningLevel] {
        &[ReasoningLevel::Medium]
    }
    async fn models(&self) -> Result<Vec<Model>, HarnessError> {
        Ok(vec![])
    }
    async fn run(
        &self,
        _request: RunRequest,
        _controls: RunControls,
    ) -> Result<BoxStream<'static, Result<AgentEvent, HarnessError>>, HarnessError> {
        let events: Vec<Result<AgentEvent, HarnessError>> = vec![Ok(AgentEvent::Done {
            status: DoneStatus::Errored,
            result: None,
            error: Some("the harness fell over".into()),
            session_id: None,
        })];
        Ok(futures::stream::iter(events).boxed())
    }
}

pub async fn wait_for<F>(mut predicate: F, what: &str)
where
    F: FnMut() -> bool,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while !predicate() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for {what}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// [`wait_for`] for a condition that has to await — the mail reads do.
#[macro_export]
macro_rules! wait_until {
    ($what:expr, $cond:block) => {{
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        loop {
            if $cond {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "timed out waiting for {}",
                $what
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }};
}

pub fn user_texts(core: &EngineCore, chat: &str) -> Vec<String> {
    let entries: Vec<SessionMessageEntry> = core
        .doc_host
        .open(chat)
        .ok()
        .and_then(|h| h.doc().read_entries().ok())
        .unwrap_or_default();
    entries
        .iter()
        .filter(|e| e.role == MessageRole::User)
        .map(|e| {
            e.parts
                .iter()
                .filter_map(|p| match p {
                    zeron_doc::MessagePart::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .collect()
}

pub fn request(prompt: &str) -> RunRequest {
    RunRequest {
        prompt: prompt.into(),
        harness: Some(HarnessId::Mock),
        model: None,
        reasoning: None,
        model_options: Default::default(),
        cwd: "~".into(),
        sandbox: SandboxLevel::WorkspaceWrite,
        auto_approve: true,
        attachments: Vec::new(),
        worktree: None,
        resume: None,
        surya: None,
    }
}

pub fn settled(core: &EngineCore, chat: &str) -> bool {
    core.sessions
        .session_status(chat)
        .is_some_and(|s| matches!(s.status, SessionStatus::Idle | SessionStatus::Errored))
}
