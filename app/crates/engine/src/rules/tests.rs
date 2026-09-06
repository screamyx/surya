//! Tests for the always-allow table: what a remembered answer may widen to,
//! who may write the file, and what survives a restart.

use super::*;
use surya_proto::AllowRule;

fn request(tool: &str, command: &str) -> PermissionRequest {
    PermissionRequest {
        request_id: "req-1".into(),
        tool_name: tool.into(),
        command: command.into(),
        input: None,
    }
}

#[test]
fn a_remembered_answer_matches_the_identical_next_request() {
    let dir = tempfile::tempdir().unwrap();
    let rules = AllowRules::open(dir.path());
    let req = request("Bash", "php artisan migrate");
    assert!(rules.matching(&req, "/repos/project-jag").is_none());

    let remember = RememberRule {
        scope: RuleScope::Workspace,
        pattern: "php artisan migrate*".into(),
        name: None,
    };
    let rule = AllowRules::from_remember(&remember, &req, "/repos/project-jag").expect("the pattern covers the command");
    assert_eq!(rule.name, "Bash php artisan migrate* in project-jag");
    rules.add(rule).unwrap();

    assert!(rules.matching(&req, "/repos/project-jag").is_some());
    assert!(
        rules
            .matching(&request("Bash", "php artisan migrate --seed"), "/repos/project-jag")
            .is_some()
    );
    // Another checkout is not covered by a workspace rule.
    assert!(rules.matching(&req, "/repos/other").is_none());
    // Another tool is not covered either.
    assert!(
        rules
            .matching(&request("Edit", "php artisan migrate"), "/repos/project-jag")
            .is_none()
    );
}

#[test]
fn an_empty_remember_pattern_pins_the_exact_command() {
    let req = request("Bash", "git push");
    let rule = AllowRules::from_remember(
        &RememberRule {
            scope: RuleScope::Global,
            pattern: "  ".into(),
            name: None,
        },
        &req,
        "/repos/x",
    )
    .expect("an empty pattern pins the exact command");
    assert_eq!(rule.pattern, "git push");
    assert_eq!(rule.workspace_path, None);
    assert_eq!(rule.name, "Bash git push");
}

#[test]
fn rules_survive_a_reopen_and_delete_removes_them() {
    let dir = tempfile::tempdir().unwrap();
    let rules = AllowRules::open(dir.path());
    let rule = AllowRules::from_remember(
        &RememberRule {
            scope: RuleScope::Global,
            pattern: "git status*".into(),
            name: Some("read-only git".into()),
        },
        &request("Bash", "git status"),
        "/repos/x",
    )
    .expect("the pattern covers the command");
    let stored = rules.add(rule).unwrap();

    let reopened = AllowRules::open(dir.path());
    assert_eq!(reopened.list().len(), 1);
    assert_eq!(reopened.list()[0].name, "read-only git");

    reopened.delete(&stored.id).unwrap();
    assert!(reopened.list().is_empty());
    assert!(AllowRules::open(dir.path()).list().is_empty());
    assert!(matches!(
        reopened.delete(&stored.id),
        Err(RulesError::NotFound(_))
    ));
}

#[test]
fn adding_the_same_rule_twice_does_not_grow_the_table() {
    let dir = tempfile::tempdir().unwrap();
    let rules = AllowRules::open(dir.path());
    let make = || {
        AllowRules::from_remember(
            &RememberRule {
                scope: RuleScope::Global,
                pattern: "ls*".into(),
                name: None,
            },
            &request("Bash", "ls"),
            "/repos/x",
        )
        .expect("the pattern covers the command")
    };
    let first = rules.add(make()).unwrap();
    let second = rules.add(make()).unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(rules.list().len(), 1);
}

#[test]
fn a_workspace_rule_without_a_workspace_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let rules = AllowRules::open(dir.path());
    // A pattern anchored to the command, so this test is about the missing
    // workspace path and nothing else.
    let mut rule = AllowRules::from_remember(
        &RememberRule {
            scope: RuleScope::Workspace,
            pattern: "ls*".into(),
            name: None,
        },
        &request("Bash", "ls"),
        "/repos/x",
    )
    .expect("ls* is anchored to ls");
    rule.workspace_path = None;
    assert!(matches!(
        rules.add(rule),
        Err(RulesError::WorkspaceRuleNeedsPath)
    ));
    assert!(rules.list().is_empty());
}

/// The escalation this check exists to stop: the card said "Bash: ls",
/// so the rule it creates must not silently mean "Bash: anything".
#[test]
fn a_pattern_that_does_not_cover_the_shown_command_is_refused() {
    let shown = request("Bash", "ls");
    let widened = AllowRules::from_remember(
        &RememberRule {
            scope: RuleScope::Global,
            // Matches `ls`, but also `rm -rf /` and everything else.
            pattern: "rm*".into(),
            name: None,
        },
        &shown,
        "/repos/x",
    );
    assert!(matches!(
        widened,
        Err(RulesError::PatternMissesRequest { .. })
    ));

    // A wider pattern that still covers what the user saw is fine — that
    // is the whole point of "always allow migrations", not just this one.
    assert!(
        AllowRules::from_remember(
            &RememberRule {
                scope: RuleScope::Global,
                pattern: "ls*".into(),
                name: None,
            },
            &shown,
            "/repos/x",
        )
        .is_ok()
    );
}

/// The escalation `glob_match` alone does NOT stop: `*` matches the shown
/// command trivially, so a client that lies in its JSON could turn "allow
/// this one `ls`" into blanket Bash approval. The threat is a dishonest
/// client, not a user typing a star, so the pattern must stay anchored to the
/// command the card showed.
#[test]
fn a_wildcard_pattern_cannot_widen_a_single_answer() {
    let remember = |pattern: &str| RememberRule {
        scope: RuleScope::Global,
        pattern: pattern.into(),
        name: None,
    };
    let migrate = request("Bash", "php artisan migrate");
    for pattern in ["*", "**", "?*", "*migrate"] {
        assert!(
            matches!(
                AllowRules::from_remember(&remember(pattern), &migrate, "/repos/x"),
                Err(RulesError::PatternMissesRequest { .. })
            ),
            "{pattern:?} starts with a wildcard and must be refused"
        );
    }
    // Stopping at the first word would approve every php subcommand.
    assert!(matches!(
        AllowRules::from_remember(&remember("php *"), &migrate, "/repos/x"),
        Err(RulesError::PatternMissesRequest { .. })
    ));
    // Reaching past it is the shape decision 20 asks for.
    assert!(
        AllowRules::from_remember(&remember("php artisan migrate*"), &migrate, "/repos/x").is_ok()
    );
    assert!(AllowRules::from_remember(&remember("php artisan *"), &migrate, "/repos/x").is_ok());

    // A one-word command: the pattern spells the whole thing out, so the
    // first-token rule is satisfied by equality rather than by length.
    let ls = request("Bash", "ls");
    assert!(AllowRules::from_remember(&remember("ls*"), &ls, "/repos/x").is_ok());
    assert!(AllowRules::from_remember(&remember("ls"), &ls, "/repos/x").is_ok());
    assert!(matches!(
        AllowRules::from_remember(&remember("l*"), &ls, "/repos/x"),
        Err(RulesError::PatternMissesRequest { .. })
    ));
}

/// Leading blanks must not buy a pattern any slack: `"   r*"` for
/// `"   rm -rf x"` is still just `r*`, which stops at the first word.
#[test]
fn leading_whitespace_does_not_widen_a_pattern() {
    let padded = request("Bash", "   rm -rf x");
    assert!(matches!(
        AllowRules::from_remember(
            &RememberRule {
                scope: RuleScope::Global,
                pattern: "   r*".into(),
                name: None,
            },
            &padded,
            "/repos/x",
        ),
        Err(RulesError::PatternMissesRequest { .. })
    ));
    // Reaching past the first word is still fine, padding and all.
    assert!(
        AllowRules::from_remember(
            &RememberRule {
                scope: RuleScope::Global,
                pattern: "rm -rf *".into(),
                name: None,
            },
            &padded,
            "/repos/x",
        )
        .is_ok()
    );
}

/// "Remember this one command" on a command that CONTAINS a wildcard must
/// pin the literal, not store a glob that approves far more.
#[test]
fn remembering_an_exact_command_containing_a_wildcard_pins_only_that_command() {
    let dir = tempfile::tempdir().unwrap();
    let rules = AllowRules::open(dir.path());
    let shown = request("Bash", "ls *.txt");
    let rule = AllowRules::from_remember(
        &RememberRule {
            scope: RuleScope::Global,
            // Empty: "just this command".
            pattern: String::new(),
            name: None,
        },
        &shown,
        "/repos/x",
    )
    .expect("an exact pin is never refused by the anchoring check");
    assert!(rule.exact, "stored as a literal, not as a glob");
    assert_eq!(rule.pattern, "ls *.txt");
    rules.add(rule).unwrap();

    // The same command again is auto-allowed…
    assert!(rules.matching(&shown, "/repos/x").is_some());
    // …and nothing the glob would have swept up is.
    for other in ["ls notes.txt", "ls a.txt b.txt", "ls "] {
        assert!(
            rules.matching(&request("Bash", other), "/repos/x").is_none(),
            "{other:?} was never approved"
        );
    }
}

/// The settings page is not filtered: there the user is authoring the rule
/// with their eyes open, and a broad pattern is the whole point.
#[test]
fn a_broad_rule_written_in_settings_is_still_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let rules = AllowRules::open(dir.path());
    rules
        .add(AllowRule {
            id: "r-1".into(),
            name: "anything git".into(),
            scope: RuleScope::Global,
            workspace_path: None,
            tool_name: "Bash".into(),
            pattern: "*".into(),
            exact: false,
            created_at: Utc::now(),
        })
        .expect("AddAllowRule is explicit authorship, not an answer");
    assert_eq!(rules.list().len(), 1);
}

#[cfg(unix)]
#[test]
fn the_rules_file_is_not_readable_by_other_local_accounts() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let rules = AllowRules::open(dir.path());
    rules
        .add(
            AllowRules::from_remember(
                &RememberRule {
                    scope: RuleScope::Global,
                    pattern: "ls*".into(),
                    name: None,
                },
                &request("Bash", "ls"),
                "/repos/x",
            )
            .unwrap(),
        )
        .unwrap();
    let mode = std::fs::metadata(dir.path().join("allow-rules.json"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "the table says what runs without asking");
}

#[test]
fn a_corrupt_file_starts_empty_rather_than_allowing_anything() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("allow-rules.json"), "{ not json").unwrap();
    let rules = AllowRules::open(dir.path());
    assert!(rules.list().is_empty());
    assert!(
        rules
            .matching(&request("Bash", "anything"), "/repos/x")
            .is_none()
    );
}
