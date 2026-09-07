//! Tests for the surya block: the generated files, the missing-sidecar
//! fallback, and the auto-allow list. What `apply` puts on the command line
//! is in `apply_tests.rs`.

#[cfg(test)]
mod prepare_tests {
    use super::super::*;

    fn options(binary: &Path) -> SuryaOptions {
        SuryaOptions {
            agent_id: "seat/one".into(),
            workspace: "demo".into(),
            mcp_binary: Some(binary.to_string_lossy().into()),
            card_store: Some("/var/surya/cards.jsonl".into()),
            mail_socket: None,
            catalog_id: None,
        }
    }

    #[test]
    fn the_config_names_the_server_surya_and_carries_the_paths() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("surya-mcp");
        std::fs::write(&binary, "").unwrap();
        let config = mcp_config(&binary, &options(&binary), "/repo");
        let server = &config["mcpServers"]["surya"];
        assert_eq!(server["type"], "stdio");
        assert_eq!(server["command"], binary.to_string_lossy().to_string());
        assert_eq!(server["env"]["SURYA_AGENT_ID"], "seat/one");
        assert_eq!(server["env"]["SURYA_WORKSPACE_ROOT"], "/repo");
        assert_eq!(server["env"]["SURYA_CARD_STORE"], "/var/surya/cards.jsonl");
        assert!(
            server["env"].get("SURYA_MAIL_SOCKET").is_none(),
            "an unset option leaves the sidecar on its own default"
        );
    }

    #[test]
    fn an_agent_id_with_a_slash_still_makes_one_directory() {
        let dir = run_dir(Path::new("/root"), "seat/one");
        assert_eq!(dir.file_name().unwrap(), "seat-one");
        assert_eq!(dir.parent().unwrap(), Path::new("/root"));
    }

    /// A fixed `/tmp/surya-mcp` is shared ground: the first user to create it
    /// owns it, the second cannot write there, and either could plant an
    /// `mcp.json` the other's agent loads. The root must be per-user.
    #[test]
    fn the_default_root_is_per_user() {
        let with_runtime = temp_env("XDG_RUNTIME_DIR", Some("/run/user/4242"), default_root);
        assert_eq!(with_runtime, PathBuf::from("/run/user/4242/surya-mcp"));

        let without = temp_env("XDG_RUNTIME_DIR", None, default_root);
        assert_ne!(
            without,
            std::env::temp_dir().join("surya-mcp"),
            "a bare shared name is exactly the collision to avoid"
        );
        assert!(
            without
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("surya-mcp-"),
            "{without:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_generated_directory_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("surya-mcp");
        std::fs::write(&binary, "").unwrap();
        let root = dir.path().join("root");
        let files = prepare_in(&root, &options(&binary), "").expect("both files are written");
        let mcp_config = files.mcp_config.as_ref().expect("the sidecar exists in this test");
        let mode = std::fs::metadata(mcp_config.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700, "got {mode:o}");
    }

    /// Set or clear one variable for the duration of `body`. Serialised by
    /// the caller being the only test that touches this variable.
    fn temp_env<T>(key: &str, value: Option<&str>, body: impl FnOnce() -> T) -> T {
        let previous = std::env::var_os(key);
        // SAFETY: single-threaded within this test; no other test reads it.
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        let out = body();
        unsafe {
            match previous {
                Some(previous) => std::env::set_var(key, previous),
                None => std::env::remove_var(key),
            }
        }
        out
    }

    #[test]
    fn prepare_writes_both_files() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("surya-mcp");
        std::fs::write(&binary, "").unwrap();
        let files = prepare_in(&dir.path().join("root"), &options(&binary), dir.path().to_str().unwrap())
            .expect("the binary exists, so both files are written");
        let config: Value =
            serde_json::from_str(
                &std::fs::read_to_string(files.mcp_config.as_ref().expect("config")).unwrap(),
            )
            .unwrap();
        assert!(config["mcpServers"]["surya"].is_object());
        let append = std::fs::read_to_string(&files.system_append).unwrap();
        assert!(append.contains("show_card"), "the append teaches show_card");

        // The skill ships as a plugin because that is what --plugin-dir reads.
        let plugin_dir = files.plugin_dir.as_ref().expect("the plugin is written");
        let manifest: Value = serde_json::from_str(
            &std::fs::read_to_string(plugin_dir.join(".claude-plugin/plugin.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["name"], "surya");
        assert_eq!(manifest["skills"], "./skills/");
        let skill =
            std::fs::read_to_string(plugin_dir.join("skills/surya-cards/SKILL.md")).unwrap();
        assert!(skill.starts_with("---"), "the skill keeps its frontmatter");
        assert!(skill.contains("diff-summary"), "and its shape table");
    }

    /// The bug this guards: `prepare` used to return `None` when the plugin
    /// write failed, which collapsed the whole surya block at the call site -
    /// the MCP config, the prompt append AND the denied built-in messaging
    /// tools went with it, silently handing the agent back Claude Code's own
    /// SendMessage that decision 19 exists to deny. A missing skill costs the
    /// card examples and nothing else.
    #[test]
    fn a_failed_plugin_write_costs_only_the_skill() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("surya-mcp");
        std::fs::write(&binary, "").unwrap();
        let root = dir.path().join("root");

        // A FILE where the plugin wants its directory, so that write fails
        // while the config and the append succeed.
        let run_dir = run_dir(&root, "seat/one");
        create_private_dir(&run_dir).unwrap();
        std::fs::write(run_dir.join("plugin"), "in the way").unwrap();

        let files = prepare_in(&root, &options(&binary), "")
            .expect("the run still gets its config and its append");
        assert!(files.plugin_dir.is_none(), "the skill is the only casualty");
        assert!(
            files.mcp_config.as_ref().is_some_and(|p| p.exists()),
            "the MCP config still lands"
        );
        assert!(
            std::fs::read_to_string(&files.system_append)
                .unwrap()
                .contains("show_card"),
            "and so does the append"
        );
    }

    #[test]
    fn a_card_is_found_by_its_tool_use_id_not_by_position() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("cards.jsonl");
        let lines = [
            json!({"card_id":"card_a","surface_id":"s_a","tool_use_id":"toolu_1","a2ui":[{"x":1}]}),
            json!({"card_id":"card_b","surface_id":"s_b","tool_use_id":"toolu_2","a2ui":[{"x":2}]}),
        ];
        std::fs::write(
            &store,
            lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n"),
        )
        .unwrap();

        let event = read_card(&store, "toolu_1").expect("the first card is still reachable");
        let surya_proto::AgentEvent::Card { card_id, a2ui, .. } = event else {
            panic!("read_card returns a Card");
        };
        assert_eq!(card_id, "card_a");
        assert_eq!(a2ui, vec![json!({"x": 1})]);

        assert!(read_card(&store, "toolu_missing").is_none());
        assert!(read_card(&store, "").is_none(), "no id, no lookup");
        assert!(read_card(&dir.path().join("absent.jsonl"), "toolu_1").is_none());
    }

    #[test]
    fn a_malformed_line_does_not_hide_the_cards_around_it() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("cards.jsonl");
        std::fs::write(
            &store,
            "{ truncated\n{\"card_id\":\"card_a\",\"surface_id\":\"s\",\"tool_use_id\":\"t1\",\"a2ui\":[]}\n",
        )
        .unwrap();
        assert!(read_card(&store, "t1").is_some());
    }

    /// `read_card` reads only the tail, so a long run does not pay for its own
    /// history on every card. The window must never hand back half a line.
    #[test]
    fn the_tail_read_returns_whole_lines_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cards.jsonl");
        let line = |n: usize| format!("{{\"n\":{n},\"pad\":\"{}\"}}", "x".repeat(200));
        let body: String = (0..50).map(|n| line(n) + "\n").collect();
        std::fs::write(&path, &body).unwrap();

        // A window smaller than the file drops the partial first line and
        // every line it returns still parses.
        let tail = read_tail(&path, 1000).unwrap();
        assert!(tail.len() < body.len(), "the window bounded the read");
        for line in tail.lines() {
            serde_json::from_str::<Value>(line).expect("whole lines only");
        }
        assert!(tail.ends_with('\n'));

        // A window larger than the file is the whole file.
        assert_eq!(read_tail(&path, 1 << 20).unwrap(), body);
    }

    /// A card past the tail window is simply not found - a missing card, which
    /// falls back to the tool chip, never a wrong one.
    #[test]
    fn a_card_outside_the_window_is_not_found_rather_than_confused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cards.jsonl");
        let old = json!({"card_id":"card_old","surface_id":"s","tool_use_id":"toolu_old","a2ui":[]});
        let filler = json!({"card_id":"c","surface_id":"s","tool_use_id":"t","a2ui":[],
                            "pad": "x".repeat(4096)});
        let mut body = old.to_string() + "\n";
        for _ in 0..(CARD_TAIL_BYTES / 4096 + 4) {
            body.push_str(&filler.to_string());
            body.push('\n');
        }
        std::fs::write(&path, body).unwrap();
        assert!(read_card(&path, "toolu_old").is_none(), "pushed out of the window");
        assert!(read_card(&path, "t").is_some(), "the recent ones are still found");
    }
}

#[cfg(test)]
mod missing_sidecar_tests {
    use super::super::*;

    fn options_with(binary: &str) -> SuryaOptions {
        SuryaOptions {
            agent_id: "seat-1".into(),
            workspace: "demo".into(),
            mcp_binary: Some(binary.to_string()),
            card_store: None,
            mail_socket: None,
            catalog_id: None,
        }
    }

    /// A sidecar that is not installed costs `show_card`. It must not cost the
    /// prompt append or the denied built-ins: measured on 2026-09-06, a real
    /// run with the binary absent came back holding Claude Code's own
    /// `SendMessage` and `ListAgents`, because `prepare` returned `None` and
    /// the whole block was skipped at the call site.
    #[test]
    fn a_missing_binary_keeps_the_append_and_the_denial() {
        let dir = tempfile::tempdir().unwrap();
        let files = prepare_with(
            &dir.path().join("root"),
            &options_with("/nowhere/surya-mcp"),
            dir.path().to_str().unwrap(),
            None,
        )
        .expect("a missing binary must NOT collapse the block");

        assert_eq!(files.mcp_config, None, "no tools without the sidecar");
        assert_eq!(files.plugin_dir, None, "the skill documents a tool that is absent");
        assert!(
            files.system_append.exists(),
            "the prompt append does not need the binary"
        );
    }

    /// The normal case still gets all four.
    #[test]
    fn an_installed_binary_gets_the_tools_too() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("surya-mcp");
        std::fs::write(&binary, "").unwrap();
        let files = prepare_with(
            &dir.path().join("root"),
            &options_with(binary.to_str().unwrap()),
            dir.path().to_str().unwrap(),
            Some(binary.clone()),
        )
        .expect("installed");
        assert!(files.mcp_config.is_some(), "the sidecar is here, so the tools are");
        assert!(files.system_append.exists());
    }
}

#[cfg(test)]
mod auto_allow_tests {
    use super::super::*;

    /// Every tool id the CLI will show the model, taken from the sidecar
    /// itself rather than typed out again here.
    ///
    /// `surya-mcp` is a dev-dependency for exactly this. The old version of
    /// this test asserted `AUTO_ALLOWED.len() == 2` under a comment promising
    /// it would fail "if a fourth arrives". Nine more arrived, three task
    /// tools and six browser ones, and it stayed green the whole time,
    /// because it was counting its own list and had never looked at the
    /// sidecar.
    fn sidecar_tool_ids() -> Vec<String> {
        surya_mcp::protocol::tool_definitions()
            .as_array()
            .expect("tool_definitions is an array")
            .iter()
            .map(|tool| {
                let name = tool["name"].as_str().expect("every tool has a name");
                format!("{TOOL_PREFIX}{name}")
            })
            .collect()
    }

    /// The whole sidecar runs without a prompt, and nothing else does.
    ///
    /// Stated as set equality in both directions on purpose. "Every sidecar
    /// tool is allowed" alone would pass with a typo sitting in the list, and
    /// "everything listed is real" alone would pass with the list empty.
    #[test]
    fn the_allowlist_is_exactly_what_the_sidecar_serves() {
        let mut served = sidecar_tool_ids();
        let mut allowed: Vec<String> = AUTO_ALLOWED.iter().map(|t| t.to_string()).collect();
        served.sort();
        allowed.sort();
        assert_eq!(
            served, allowed,
            "the auto-allow list and the sidecar have drifted apart"
        );
    }

    /// The same claim from the caller's side: the harness answers yes for
    /// every id the CLI will actually hand it.
    #[test]
    fn every_tool_the_sidecar_serves_runs_without_asking() {
        let served = sidecar_tool_ids();
        assert!(
            served.len() >= 12,
            "the sidecar served {}, which is fewer than the tools this list was written for: {served:?}",
            served.len()
        );
        let gated: Vec<&String> = served.iter().filter(|id| !is_auto_allowed(id)).collect();
        assert!(
            gated.is_empty(),
            "these sidecar tools would still park the run in needs-you: {gated:?}"
        );
    }

    /// Exact match, so a lookalike is a different tool. This is what the
    /// list buys over a `mcp__surya__` prefix test, and it is why widening
    /// the list did not widen the rule.
    #[test]
    fn a_lookalike_or_an_outsider_still_asks() {
        let cases = [
            // A real name with something appended, prepended, or clipped.
            "mcp__surya__show_card_evil",
            "mcp__surya__browser_eval_evil",
            "mcp__surya__show_car",
            "mcp__surya__browser_ope",
            "evil_mcp__surya__show_card",
            // Right name, wrong server.
            "mcp__evil__show_card",
            "mcp__surya_evil__show_card",
            // The bare tool name, without the CLI's prefix.
            "show_card",
            "list_tasks",
            // Whitespace is not a name.
            "mcp__surya__show_card ",
            " mcp__surya__show_card",
            "",
            // Claude Code's own tools are not surya's to allow.
            "Bash",
            "AskUserQuestion",
        ];
        let leaked: Vec<&str> = cases
            .into_iter()
            .filter(|tool| is_auto_allowed(tool))
            .collect();
        assert!(
            leaked.is_empty(),
            "these ran without asking and should not have: {leaked:?}"
        );
    }

    /// The tool the app's transcript detection depends on is still in the
    /// list, under the exact string the normalizer watches for.
    #[test]
    fn the_card_tool_the_normalizer_watches_is_allowed() {
        assert!(AUTO_ALLOWED.contains(&SHOW_CARD_TOOL));
        assert!(is_auto_allowed(SHOW_CARD_TOOL));
    }
}
