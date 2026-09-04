//! Live smoke for the surya wiring: generate the same `--mcp-config` and
//! `--append-system-prompt-file` the Claude harness generates, print both
//! paths, and let a real `claude -p` run against them.
//!
//! ```text
//! cargo run -p zeron-harness --example surya_mcp_probe -- \
//!     <path to surya-mcp> <card store>
//! ```
//!
//! Prints two lines, `mcp-config=<path>` and `append=<path>`.

use zeron_proto::SuryaOptions;

fn main() {
    let mut args = std::env::args().skip(1);
    let binary = args.next().expect("argument 1: path to the surya-mcp binary");
    let card_store = args.next().expect("argument 2: card store path");
    // The probe has no daemon, so the socket points at a path that cannot
    // exist and mail takes its log fallback. Set SURYA_MAIL_LOG in the shell
    // that runs `claude`; the sidecar inherits it.
    let options = SuryaOptions {
        agent_id: "probe-seat".into(),
        workspace: "surya".into(),
        mcp_binary: Some(binary),
        card_store: Some(card_store),
        mail_socket: Some("/nonexistent/surya-probe.sock".into()),
        catalog_id: None,
    };
    let cwd = std::env::current_dir().unwrap();
    let files = zeron_harness::claude::surya::prepare(&options, cwd.to_str().unwrap())
        .expect("surya-mcp resolves and both files are written");
    println!("mcp-config={}", files.mcp_config.display());
    println!("append={}", files.system_append.display());
}
