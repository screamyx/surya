//! `SURYA_DEMO_TASK_ERROR=<message>`: start the board with its error banner
//! already painted. Display-only, the same shape as `SURYA_DEMO_CARDS` and
//! `SURYA_DEMO_UPLOAD` in `shell.rs`, and it exists for the same reason those
//! do: the state cannot be staged in front of a camera.
//!
//! `board.rs` sets the banner in exactly two places. One is a rejected
//! `Mutate`. A healthy engine rejects none that the board can send: an
//! unknown space is the only rejection `create_task` has, and losing the
//! space unmounts the pane before a card can be added. The other is a
//! `WatchTasks` subscribe that failed, and that only fails on a broken
//! transport, which puts the connection gate card over the whole window and
//! takes the board off screen with it.
//!
//! So there is no engine state and no click that shows this banner, and the
//! contrast fix in surya#192 had no way to be photographed. This knob gives
//! it one, through the real render path.

/// The banner a run starts with. `None` on any normal run.
///
/// Blank and whitespace-only text is `None`: exporting a variable empty is
/// how a shell leaves a knob switched off, and an empty banner would paint a
/// bordered box with nothing in it.
pub fn banner(raw: Option<String>) -> Option<String> {
    let text = raw?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

/// Read the knob. The only reader of this variable in the process.
pub fn from_env() -> Option<String> {
    banner(std::env::var("SURYA_DEMO_TASK_ERROR").ok())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gpui::{AppContext as _, TestAppContext};
    use tokio::sync::mpsc;

    use super::*;
    use crate::tasks::TasksPane;

    #[test]
    fn an_unset_or_blank_knob_leaves_the_board_clean() {
        assert_eq!(banner(None), None);
        assert_eq!(banner(Some(String::new())), None);
        assert_eq!(banner(Some("   \t ".into())), None);
    }

    #[test]
    fn the_knobs_text_is_the_banners_text() {
        assert_eq!(
            banner(Some("  task board unavailable: no engine  ".into())),
            Some("task board unavailable: no engine".to_string())
        );
    }

    /// The wiring, not the helper. `banner()` can be perfect and the board
    /// still start clean if nothing calls it, which is the whole defect this
    /// test guards: delete the `from_env()` line in `TasksPane::new` and the
    /// two tests above stay green while the knob does nothing.
    ///
    /// The client is a pair of idle channels, as in `inbox/watch.rs`: the
    /// board's `WatchTasks` subscribe never resolves on it, so the watch task
    /// parks and the pane under test is the one the app builds.
    #[gpui::test]
    async fn the_knob_reaches_the_board(cx: &mut TestAppContext) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let _guard = runtime.enter();

        // SAFETY: this variable has one reader in the process, `from_env()`,
        // and it is read only while a `TasksPane` is being constructed. No
        // other test in this crate builds one.
        unsafe { std::env::set_var("SURYA_DEMO_TASK_ERROR", "  the engine said no  ") };

        let (out, _out_rx) = mpsc::channel::<String>(1);
        let (_in_tx, inbound) = mpsc::channel::<String>(1);
        let client = Arc::new(surya_rpc::RpcClient::new(out, inbound));
        let pane = cx.new(|cx| TasksPane::new(client, "space-1", "Space one", cx));

        // SAFETY: same reader, and the pane that reads it is already built.
        unsafe { std::env::remove_var("SURYA_DEMO_TASK_ERROR") };

        pane.read_with(cx, |pane, _| {
            assert_eq!(
                pane.error.as_deref(),
                Some("the engine said no"),
                "the knob did not reach the board's banner"
            );
        });
    }
}
