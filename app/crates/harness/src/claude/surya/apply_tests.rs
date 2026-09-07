//! What `surya::apply` puts on the command line and in the environment.
//!
//! Its own file because `tests.rs` would cross the 500-line rule with it.
use super::*;

fn options(binary: &Path) -> SuryaOptions {
    SuryaOptions {
        agent_id: "seat/one".into(),
        workspace: "demo".into(),
        mcp_binary: Some(binary.to_string_lossy().into()),
        card_store: None,
        mail_socket: None,
        mail_log: None,
        catalog_id: None,
    }
}

/// The command as it would be spawned: its arguments, and the variables
/// it would carry into the child.
///
/// Goes through `apply_files` with files built under `root`, never
/// `apply`. `apply` writes under the real runtime root and resolves the
/// sidecar from PATH, so calling it here would litter a shared directory
/// and would find an installed sidecar on a box that has one - which is
/// exactly what the first version of this test did.
fn applied(
    root: &Path,
    options: &SuryaOptions,
    binary: Option<PathBuf>,
) -> (Vec<String>, Vec<(String, String)>) {
    let files = prepare_with(root, options, "/repo", binary)
        .expect("a missing binary must not collapse the block");
    let mut cmd = Command::new("/bin/true");
    apply_files(&mut cmd, options, Some(files));
    let std = cmd.as_std();
    let args = std
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let envs = std
        .get_envs()
        .filter_map(|(key, value)| {
            Some((
                key.to_string_lossy().into_owned(),
                value?.to_string_lossy().into_owned(),
            ))
        })
        .collect();
    (args, envs)
}

/// The marker is the agent's identity, so it does not depend on the
/// sidecar being installed. A run with no sidecar is the case that used
/// to leave the child indistinguishable from a person at a terminal.
#[test]
fn the_marker_rides_the_environment_even_with_no_sidecar_on_the_box() {
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("surya-mcp");
    let (args, envs) = applied(&dir.path().join("root"), &options(&binary), None);
    assert!(
        !args.iter().any(|a| a == "--mcp-config"),
        "no sidecar, no config: {args:?}"
    );
    assert!(
        !args.iter().any(|a| a == "--strict-mcp-config"),
        "nothing to pin: {args:?}"
    );
    assert!(
        envs.contains(&("SURYA_AGENT_ID".to_string(), "seat/one".to_string())),
        "{envs:?}"
    );
    assert!(
        envs.contains(&("SURYA_WORKSPACE".to_string(), "demo".to_string())),
        "{envs:?}"
    );
}

/// The config surya hands over is the only one the run loads. Asserted as
/// an ordered triple rather than "both flags are present somewhere",
/// because the pin belongs to that config and a reader has to see it
/// next to the path it pins.
#[test]
fn the_mcp_config_is_pinned_to_the_one_surya_wrote() {
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("surya-mcp");
    std::fs::write(&binary, "").unwrap();
    let (args, envs) = applied(
        &dir.path().join("root"),
        &options(&binary),
        Some(binary.clone()),
    );

    let at = args
        .iter()
        .position(|a| a == "--mcp-config")
        .expect("--mcp-config is passed");
    assert!(!args[at + 1].is_empty(), "a config path follows: {args:?}");
    assert_eq!(
        args[at + 2],
        "--strict-mcp-config",
        "the pin follows the config it pins: {args:?}"
    );
    // The marker is there in the ordinary case too.
    assert!(
        envs.contains(&("SURYA_AGENT_ID".to_string(), "seat/one".to_string())),
        "{envs:?}"
    );
}
