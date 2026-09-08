//! What `surya::apply` puts on the command line and in the environment.
//!
//! Its own file because `tests.rs` would cross the 500-line rule with it.
use super::*;
// Not through `use super::*`: this is tokio's Command, the one `apply_files`
// takes, and it used to arrive here only because `surya.rs` happened to
// import it for its own use.
use tokio::process::Command;

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

/// [`applied`], for a run whose files could not be prepared at all.
fn applied_without_files(options: &SuryaOptions) -> Vec<String> {
    let mut cmd = Command::new("/bin/true");
    apply_files(&mut cmd, options, None);
    cmd.as_std()
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
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
        envs.contains(&("SURYA_AGENT_ID".to_string(), "seat/one".to_string())),
        "{envs:?}"
    );
    assert!(
        envs.contains(&("SURYA_WORKSPACE".to_string(), "demo".to_string())),
        "{envs:?}"
    );
}

/// With no sidecar there is no `--mcp-config`, and without the pin the CLI
/// falls back to the MCP servers configured on the box: measured on 2.1.263,
/// a plain run loaded 8 of them. The pin is what makes that number zero, so
/// it is exactly the no-sidecar run that needs it most.
#[test]
fn a_run_with_no_sidecar_is_still_pinned_to_no_servers() {
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("surya-mcp");
    let (args, _) = applied(&dir.path().join("root"), &options(&binary), None);
    assert!(
        args.iter().any(|a| a == "--strict-mcp-config"),
        "no config to load means load none: {args:?}"
    );
}

/// The same hole one step earlier: `prepare` gives back nothing when it
/// cannot write its directory, and the run still spawns. It carries the
/// marker and it carries the pin, because neither depends on a file.
#[test]
fn a_run_whose_files_could_not_be_written_is_still_pinned() {
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("surya-mcp");
    let args = applied_without_files(&options(&binary));
    assert_eq!(
        args,
        vec!["--strict-mcp-config".to_string()],
        "the pin is the whole command line here: {args:?}"
    );
}

/// The config surya hands over is the only one the run loads. Asserted as
/// an ordered triple rather than "both flags are present somewhere", so the
/// pin cannot drift away from the path it pins.
///
/// The pin leads because it is unconditional now. That order is honored:
/// probed on CLI 2.1.263, `--strict-mcp-config --mcp-config <file>` listed
/// only the server in the file. `--mcp-config` takes a list, so a flag has
/// to follow its path or it swallows the next argument - here that flag is
/// `--append-system-prompt-file`.
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
        .position(|a| a == "--strict-mcp-config")
        .expect("the pin is passed");
    assert_eq!(
        args[at + 1],
        "--mcp-config",
        "the pin leads the config it pins: {args:?}"
    );
    assert!(!args[at + 2].is_empty(), "a config path follows: {args:?}");
    assert!(
        args.get(at + 3).is_some_and(|a| a.starts_with("--")),
        "a flag ends the config list: {args:?}"
    );
    // The marker is there in the ordinary case too.
    assert!(
        envs.contains(&("SURYA_AGENT_ID".to_string(), "seat/one".to_string())),
        "{envs:?}"
    );
}
