//! Unit tests for the Claude harness. Moved out of `mod.rs` under the
//! 500-line rule (issue #159); the tests themselves are unchanged.

use super::control::{parse_questions, updated_input_with_answers};
use super::*;
use serde_json::json;

// Not through `use super::*`: the move took `UserInputAnswer` to
// `control.rs`, so it no longer arrives here via `mod.rs`'s imports.
use surya_proto::{SandboxLevel, SuryaOptions, UserInputAnswer};

fn request() -> RunRequest {
    RunRequest {
        prompt: "hi".into(),
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
        surya: None,
    }
}

fn args_of(request: &RunRequest) -> Vec<String> {
    ClaudeHarness::new()
        .build_command(&PathBuf::from("/bin/true"), request)
        .as_std()
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

/// Without the option the command is byte-for-byte the one every other
/// caller and every existing test already spawns.
#[test]
fn no_surya_option_means_no_extra_flags() {
    let args = args_of(&request());
    assert!(!args.iter().any(|a| a == "--mcp-config"), "{args:?}");
    assert!(!args.iter().any(|a| a == "--strict-mcp-config"), "{args:?}");
    assert!(
        !args.iter().any(|a| a == "--append-system-prompt-file"),
        "{args:?}"
    );
    // And nothing marks the child as a surya agent, because it is not one.
    let marked: Vec<String> = ClaudeHarness::new()
        .build_command(&PathBuf::from("/bin/true"), &request())
        .as_std()
        .get_envs()
        .filter_map(|(key, _)| {
            let key = key.to_string_lossy().into_owned();
            key.starts_with("SURYA_").then_some(key)
        })
        .collect();
    assert!(marked.is_empty(), "{marked:?}");
}

#[test]
fn the_surya_option_adds_the_mcp_config_and_the_prompt_append() {
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("surya-mcp");
    std::fs::write(&binary, "").unwrap();
    let mut request = request();
    request.surya = Some(SuryaOptions {
        agent_id: "seat-1".into(),
        workspace: "demo".into(),
        mcp_binary: Some(binary.to_string_lossy().into()),
        card_store: None,
        mail_socket: None,
        mail_log: None,
        catalog_id: None,
    });
    let args = args_of(&request);

    let config = args
        .iter()
        .position(|a| a == "--mcp-config")
        .map(|i| args[i + 1].clone())
        .expect("--mcp-config is passed");
    let body: Value = serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(body["mcpServers"]["surya"]["command"], binary.to_string_lossy().to_string());

    let append = args
        .iter()
        .position(|a| a == "--append-system-prompt-file")
        .map(|i| args[i + 1].clone())
        .expect("--append-system-prompt-file is passed");
    assert!(std::fs::read_to_string(&append).unwrap().contains("show_card"));

    // surya owns mail, so the CLI's own cross-session tools are denied.
    let plugin = args
        .iter()
        .position(|a| a == "--plugin-dir")
        .map(|i| args[i + 1].clone())
        .expect("--plugin-dir is passed");
    assert!(
        std::path::Path::new(&plugin)
            .join("skills/surya-cards/SKILL.md")
            .exists(),
        "the plugin dir carries the cards skill"
    );

    let denied = args
        .iter()
        .position(|a| a == "--disallowed-tools")
        .map(|i| args[i + 1].clone())
        .expect("--disallowed-tools is passed");
    assert_eq!(denied, "SendMessage,ListAgents");
}

#[test]
fn parses_questions_tolerantly() {
    let input = json!({
        "questions": [
            {
                "header": "Choice",
                "question": "Pick one",
                "options": ["A", {"label": "B", "description": "second"}],
                "multiSelect": false
            },
            { "title": "Alt", "prompt": "Pick many", "multi_select": true }
        ]
    });
    let qs = parse_questions(&input);
    assert_eq!(qs.len(), 2);
    assert_eq!(qs[0].header, "Choice");
    assert_eq!(qs[0].options, vec!["A".to_string(), "B".to_string()]);
    assert!(!qs[0].multi_select);
    assert_eq!(qs[1].header, "Alt");
    assert_eq!(qs[1].question, "Pick many");
    assert!(qs[1].multi_select);
}

#[test]
fn answers_key_by_question_text() {
    let input =
        json!({"questions": [{"header": "H", "question": "Pick one", "options": ["A", "B"]}]});
    let qs = parse_questions(&input);
    let answers = vec![UserInputAnswer {
        question_id: qs[0].id.clone(),
        labels: vec!["B".into()],
    }];
    let updated = updated_input_with_answers(&input, &qs, &answers);
    assert_eq!(updated["answers"]["Pick one"], json!("B"));
    // Original input is preserved alongside the answers.
    assert!(updated["questions"].is_array());
}
