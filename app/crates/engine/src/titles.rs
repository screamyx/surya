//! Chat auto-titling — after the first user+assistant exchange completes on an
//! untitled chat, name it with the harness's cheapest model (port of surya's
//! `generateTitle` in `sessions.ts`).
//!
//! Flow (fire-and-forget from the run task; every failure is a silent skip with
//! tracing — a title must never fail or delay a run):
//! 1. skip when the chat already has a title (or has no workspace row);
//! 2. pick the run harness's cheapest model (small-tier name heuristic, else the
//!    last listed model — surya's `cheapestModel`);
//! 3. run a one-shot, non-streaming-collected titling prompt through the
//!    [`Harness`] trait (read-only sandbox, minimal reasoning, auto-approve),
//!    retrying on surya's short backoff ladder; fall back to the prompt's first
//!    words when every attempt produces nothing;
//! 4. re-check the title (a user rename during generation wins);
//! 5. when the chat sits in a surya worktree (`surya/<name>` branch), rename the
//!    branch from the title and update the chat's branch row;
//! 6. `rename_chat` in the workspace doc.

use std::sync::Arc;

use futures::StreamExt;

use surya_harness::{CancellationToken, RunControls, SteerMessage};
use surya_proto::{
    AgentEvent, DoneStatus, HarnessId, Model, ReasoningLevel, RunRequest, SandboxLevel,
    UserInputAnswer, UserInputQuestion,
};

use crate::EngineError;
use crate::registry::HarnessRegistry;
use crate::repos::Repos;
use crate::workspace_host::WorkspaceHost;

/// Throwaway title runs are cheap but still cross a process boundary — retry a
/// couple of times with a short backoff before falling back (surya's ladder).
const RETRY_DELAYS_MS: &[u64] = &[250, 1_000];

struct Inner {
    workspace: WorkspaceHost,
    registry: Arc<HarnessRegistry>,
    repos: Repos,
}

#[derive(Clone)]
pub struct TitleGenerator {
    inner: Arc<Inner>,
}

impl TitleGenerator {
    pub fn new(workspace: WorkspaceHost, registry: Arc<HarnessRegistry>, repos: Repos) -> Self {
        Self {
            inner: Arc::new(Inner {
                workspace,
                registry,
                repos,
            }),
        }
    }

    /// Fire-and-forget: title `chat_id` if it's still untitled. Called by the run
    /// task after a completed exchange; runs detached so it never delays anything.
    pub fn maybe_generate(&self, chat_id: &str, harness: HarnessId, prompt: &str, cwd: &str) {
        let this = self.clone();
        let chat_id = chat_id.to_string();
        let prompt = prompt.to_string();
        let cwd = cwd.to_string();
        tokio::spawn(async move {
            if let Err(err) = this.generate(&chat_id, harness, &prompt, &cwd).await {
                tracing::debug!(chat = %chat_id, error = %err, "chat auto-titling skipped");
            }
        });
    }

    async fn generate(
        &self,
        chat_id: &str,
        harness_id: HarnessId,
        prompt: &str,
        cwd: &str,
    ) -> Result<(), EngineError> {
        let chat = self
            .inner
            .workspace
            .chat(chat_id)?
            .ok_or_else(|| EngineError::Other("chat has no workspace row".into()))?;
        if chat.title.as_deref().is_some_and(|t| !t.trim().is_empty()) {
            return Ok(()); // already named
        }

        let generated = self.run_title_model(harness_id, prompt, cwd).await;
        // Fallback so a chat is always named even if the model run produced nothing.
        let fallback: String = prompt
            .split_whitespace()
            .take(7)
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(48)
            .collect();
        let title = generated.unwrap_or(fallback);
        if title.is_empty() {
            return Ok(());
        }

        // Re-read after the model call: a user may have named the chat or checked
        // out another branch while the throwaway generation was live.
        let latest = self.inner.workspace.chat(chat_id)?.unwrap_or(chat);
        if latest
            .title
            .as_deref()
            .is_some_and(|t| !t.trim().is_empty())
        {
            return Ok(());
        }

        // Rename the worktree branch when the chat still sits on its original
        // surya/<name> branch (guards live inside rename_worktree_branch).
        if let (Some(chat_cwd), Some(branch)) = (&latest.cwd, &latest.branch)
            && branch.starts_with("surya/")
        {
            match self
                .inner
                .repos
                .rename_worktree_branch(std::path::Path::new(chat_cwd), branch, &title)
                .await
            {
                Ok(renamed) if &renamed != branch => {
                    if let Err(err) = self.inner.workspace.set_chat_branch(chat_id, &renamed) {
                        tracing::warn!(chat = %chat_id, error = %err, "chat branch update failed");
                    }
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::warn!(chat = %chat_id, error = %err, "automatic worktree branch rename failed");
                }
            }
        }

        self.inner.workspace.rename_chat(chat_id, &title)?;
        tracing::info!(chat = %chat_id, title = %title, "chat auto-titled");
        Ok(())
    }

    /// One-shot titling run: collect TextDeltas until Done; retries on failure.
    async fn run_title_model(
        &self,
        harness_id: HarnessId,
        prompt: &str,
        cwd: &str,
    ) -> Option<String> {
        let harness = match self.inner.registry.resolve(harness_id) {
            Ok(harness) => harness,
            Err(err) => {
                tracing::debug!(error = %err, "titling harness unavailable");
                return None;
            }
        };
        let cheap = cheapest_model(&harness.models().await.unwrap_or_default());
        let title_prompt = format!(
            "Reply with ONLY a concise 3-5 word title in Title Case (no quotes, no punctuation) \
             for a coding session that begins with this request:\n\n{prompt}"
        );
        for attempt in 0..=RETRY_DELAYS_MS.len() {
            let request = RunRequest {
                surya: None,
                prompt: title_prompt.clone(),
                harness: Some(harness_id),
                model: cheap.clone(),
                reasoning: Some(ReasoningLevel::Minimal),
                model_options: serde_json::Map::new(),
                cwd: cwd.to_string(),
                sandbox: SandboxLevel::ReadOnly,
                auto_approve: true,
                attachments: Vec::new(),
                resume: None,
                worktree: None,
            };
            match collect_text(harness.as_ref(), request).await {
                Ok(raw) => {
                    let candidate = clean_title(&raw);
                    if !candidate.is_empty() {
                        return Some(candidate);
                    }
                }
                Err(err) => {
                    tracing::warn!(attempt = attempt + 1, error = %err,
                        "automatic chat title generation attempt failed");
                }
            }
            if let Some(delay) = RETRY_DELAYS_MS.get(attempt) {
                tokio::time::sleep(std::time::Duration::from_millis(*delay)).await;
            }
        }
        None
    }
}

/// The cheapest model a harness offers (surya's `cheapestModel` heuristic):
/// prefer a small-tier name (haiku/mini/nano/flash/small/lite), else the last
/// listed model; `None` when the catalog is empty (harness picks its default).
fn cheapest_model(models: &[Model]) -> Option<String> {
    if models.is_empty() {
        return None;
    }
    let small = models.iter().find(|m| {
        let haystack = format!("{} {}", m.id, m.label).to_lowercase();
        ["haiku", "mini", "nano", "flash", "small", "lite"]
            .iter()
            .any(|tier| haystack.contains(tier))
    });
    small.or(models.last()).map(|m| m.id.clone())
}

/// The longest title we keep. A model asked for a short title sometimes
/// answers with a whole sentence, so there has to be a cap.
const TITLE_MAX: usize = 60;

/// First line, stripped of quote/heading dressing, cut to [`TITLE_MAX`].
///
/// A title longer than the cap ends at a WORD boundary and gets an ellipsis.
/// The cut used to be a bare `.take(60)`, which stopped mid-word and said
/// nothing: a session header read "Before I wire the reconciliation path I
/// need two decisions f" - exactly 60 characters, the word "from" beheaded -
/// while the sidebar and the agent tree elided the same title properly with
/// "…". The header was not too narrow; the string arrived already cut
/// (E2E-UI-06).
///
/// The ellipsis is one character, so the whole result still fits the cap.
fn clean_title(raw: &str) -> String {
    let first = raw.trim().lines().next().unwrap_or("");
    let cleaned = first
        .trim_start_matches(['"', '\'', '#', ' ', '\t'])
        .trim_end_matches(['"', '\'', ' ', '\t']);
    if cleaned.chars().count() <= TITLE_MAX {
        return cleaned.to_string();
    }
    // One char of the budget belongs to the ellipsis.
    let head: String = cleaned.chars().take(TITLE_MAX - 1).collect();
    // Back up to the last space, so the title ends on a whole word. A single
    // word longer than the cap has no space to back up to; that one is cut
    // where it stands rather than thrown away.
    let trimmed = match head.rsplit_once(' ') {
        Some((upto, _)) if !upto.trim().is_empty() => upto,
        _ => head.as_str(),
    };
    format!("{}…", trimmed.trim_end_matches([' ', ',', ';', ':', '-']))
}

/// Drive one titling run through the harness: no steering, questions resolved
/// empty immediately (a titling prompt must never block on input).
async fn collect_text(
    harness: &dyn surya_harness::Harness,
    request: RunRequest,
) -> Result<String, EngineError> {
    let (steer_tx, steer_rx) = tokio::sync::mpsc::channel::<SteerMessage>(1);
    let controls = RunControls {
        request_input: Box::new(|_questions: Vec<UserInputQuestion>| {
            let (tx, rx) = tokio::sync::oneshot::channel::<Vec<UserInputAnswer>>();
            let _ = tx.send(Vec::new());
            rx
        }),
        steering: steer_rx,
        interrupt: CancellationToken::new(),
        permission: surya_harness::permission::PermissionGate::auto_allow(),
    };
    let mut stream = harness.run(request, controls).await?;
    let mut text = String::new();
    while let Some(event) = stream.next().await {
        match event? {
            AgentEvent::TextDelta { text: delta } => text.push_str(&delta),
            AgentEvent::Error { message } => {
                return Err(EngineError::Other(format!("titling run error: {message}")));
            }
            AgentEvent::Done { status, error, .. } => {
                if status == DoneStatus::Completed {
                    break;
                }
                return Err(EngineError::Other(format!(
                    "titling run ended {status:?}: {}",
                    error.unwrap_or_default()
                )));
            }
            _ => {}
        }
    }
    drop(steer_tx); // keep the mailbox open for the run's whole lifetime
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use surya_proto::Model;

    fn model(id: &str, label: &str) -> Model {
        Model {
            id: id.into(),
            label: label.into(),
            description: None,
            reasoning_levels: vec![],
            options: vec![],
        }
    }

    #[test]
    fn cheapest_prefers_small_tier_then_last() {
        let models = vec![
            model("opus-4", "Opus"),
            model("haiku-3", "Haiku"),
            model("sonnet-4", "Sonnet"),
        ];
        assert_eq!(cheapest_model(&models).as_deref(), Some("haiku-3"));
        let no_small = vec![model("opus-4", "Opus"), model("sonnet-4", "Sonnet")];
        assert_eq!(cheapest_model(&no_small).as_deref(), Some("sonnet-4"));
        assert_eq!(cheapest_model(&[]), None);
    }

    #[test]
    fn titles_are_cleaned() {
        assert_eq!(clean_title("\"Fix Login Flow\"\nextra"), "Fix Login Flow");
        assert_eq!(clean_title("# Add Dark Mode  "), "Add Dark Mode");
        assert_eq!(clean_title("   "), "");
    }

    /// E2E-UI-06: the header showed "Before I wire the reconciliation path I
    /// need two decisions f" - the bare `.take(60)` beheaded "from" and left
    /// no ellipsis, so the title read as a sentence that simply stopped.
    #[test]
    fn a_long_title_ends_on_a_whole_word_with_an_ellipsis() {
        let cut = clean_title("Before I wire the reconciliation path I need two decisions from you.");
        assert_eq!(cut, "Before I wire the reconciliation path I need two decisions…");
        assert!(
            !cut.ends_with("decisions f"),
            "the old cut stopped mid-word: {cut:?}"
        );
        assert!(cut.chars().count() <= TITLE_MAX);
    }

    #[test]
    fn a_title_that_fits_is_left_exactly_as_it_is() {
        let short = "Fix the login flow";
        assert_eq!(clean_title(short), short, "no ellipsis on a title that fits");
        // Exactly at the cap is still untouched - the cap is inclusive.
        let exact: String = std::iter::repeat_n('a', TITLE_MAX).collect();
        assert_eq!(clean_title(&exact), exact);
    }

    #[test]
    fn one_unbroken_word_longer_than_the_cap_is_still_cut() {
        // No space to back up to. Better a hard cut with an ellipsis than
        // throwing the whole title away.
        let word: String = std::iter::repeat_n('x', 80).collect();
        let out = clean_title(&word);
        assert!(out.ends_with('…'));
        assert_eq!(out.chars().count(), TITLE_MAX);
    }
}
