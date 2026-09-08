//! What surya puts on a run's command line. Moved out of `surya.rs` under
//! the 500-line rule (decision 13); the code itself is unchanged.
//!
//! `surya.rs` keeps the half that WRITES the run folder, this file is the
//! half that spends it. `surya::apply` and `surya::apply_files` still resolve
//! through the re-export there, so no caller changes.

use tokio::process::Command;

use surya_proto::SuryaOptions;

use super::{DENIED_TOOLS, SuryaFiles, prepare};

/// Put surya's abilities on the command line of a run that asked for them.
///
/// Everything here is conditional on the caller passing `SuryaOptions`, so a
/// run without them spawns byte-for-byte the command it always did.
pub fn apply(cmd: &mut Command, options: &SuryaOptions, cwd: &str) {
    let files = prepare(options, cwd);
    apply_files(cmd, options, files);
}

/// [`apply`], with the files already prepared.
///
/// Split for the same reason [`super::prepare_with`] is: `prepare` writes under the
/// real runtime root and resolves the sidecar from PATH, so a test that
/// called `apply` would write into a shared directory and would find an
/// installed sidecar on a box that has one. Tests hand this the files they
/// built in a temp dir, and "no sidecar" means it.
pub fn apply_files(cmd: &mut Command, options: &SuryaOptions, files: Option<SuryaFiles>) {
    // A surya agent says so in its own environment, with the same two values
    // the sidecar's `env` block gets. Anything the child runs can then tell a
    // surya agent from a person at a terminal and behave accordingly.
    //
    // Set on the options alone, not on the files below: an agent is a surya
    // agent whether or not its config could be written.
    cmd.env("SURYA_AGENT_ID", &options.agent_id);
    cmd.env("SURYA_WORKSPACE", &options.workspace);
    // Surya's own MCP config and nothing else. Without this flag the CLI also
    // loads the MCP servers configured on the box, so the agent gets foreign
    // tools beside surya's own, and an agent told to message a peer can reach
    // one of those instead of `mcp__surya__send_message`. Surya hands the
    // agent its abilities (decision 5); the box does not add to them.
    //
    // Unconditional, like the two variables above, because the case with no
    // `--mcp-config` to pin is the leaky one: measured on the CLI, a plain
    // run loaded every MCP server the host machine had configured, and the
    // same run with this flag alone loaded none and exposed no `mcp__` tool
    // at all.
    cmd.arg("--strict-mcp-config");

    let Some(files) = files else {
        return;
    };
    // The tools are conditional on the sidecar being installed; the prompt
    // append and the denial below are NOT. A missing binary costs show_card,
    // never decision 19's single mail channel.
    if let Some(mcp_config) = &files.mcp_config {
        cmd.arg("--mcp-config");
        cmd.arg(mcp_config);
    }
    cmd.arg("--append-system-prompt-file");
    cmd.arg(&files.system_append);
    // The cards skill, as a one-skill plugin. Without this the agent is told
    // a card beats prose and given no reference for writing one; with it the
    // skill shows up as `surya:surya-cards`. Only these two args are gated on
    // it - everything above still applies when the skill could not be
    // written.
    if let Some(plugin_dir) = &files.plugin_dir {
        cmd.arg("--plugin-dir");
        cmd.arg(plugin_dir);
    }
    // surya is the host of every agent, so surya owns mail (decision 19). The
    // CLI's own cross-session messaging talks to Claude Code sessions surya
    // does not know about, and an agent told to message a peer reaches for it
    // first - measured on 2026-09-05, the model called the built-in
    // SendMessage and never touched ours. Deny both so there is one mail
    // channel.
    cmd.args(["--disallowed-tools", DENIED_TOOLS]);
}
