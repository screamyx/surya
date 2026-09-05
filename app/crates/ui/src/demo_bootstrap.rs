//! Shared bootstrap for the single-pane demo entries (`zeron files-demo`,
//! `zeron --tasks-demo`): the gpui globals a pane needs outside the shell,
//! and the self-quit used as the headless proof on a box with no display.
//!
//! `SURYA_DEMO_EXIT_SECS=<n>` quits after n seconds and prints
//! `<label>: started=1 panics=0 ran_secs=<n>`. A demo may name one legacy
//! variable that keeps working as an alias for one release.

use std::path::Path;
use std::time::Duration;

use gpui::App;

use crate::{appearance, composer, settings, theme_library, typography};

/// Tokio bridge, settings, fonts, theme, appearance, composer keymap: the
/// same order `run_app` uses, so the pane renders with the real palette.
pub fn init_app(data_dir: &Path, cx: &mut App) {
    gpui_tokio::init(cx);
    let ui_settings = settings::UiSettings::load(data_dir);
    settings::init(ui_settings.clone(), data_dir.to_path_buf(), cx);
    let fonts = typography::register_fonts(cx);
    typography::init(
        ui_settings.ui_font_family.clone(),
        ui_settings.ui_font_size,
        fonts,
        cx,
    );
    theme_library::init(data_dir.to_path_buf(), cx);
    appearance::init(
        ui_settings.appearance,
        ui_settings.theme_selection,
        ui_settings.accent,
        ui_settings.surface,
        cx,
    );
    composer::init(cx);
}

/// The env var every demo shares.
pub const EXIT_ENV: &str = "SURYA_DEMO_EXIT_SECS";

/// `SURYA_DEMO_EXIT_SECS`, else the demo's legacy variable when given.
pub fn exit_after(legacy_env: Option<&str>) -> Option<Duration> {
    let read = |name: &str| {
        std::env::var(name)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(Duration::from_secs)
    };
    read(EXIT_ENV).or_else(|| legacy_env.and_then(read))
}

/// Quit after `after`, printing the proof line first. `None` = run forever.
pub fn schedule_exit(label: &'static str, after: Option<Duration>, cx: &mut App) {
    let Some(after) = after else {
        return;
    };
    cx.spawn(async move |cx| {
        cx.background_executor().timer(after).await;
        println!("{label}: started=1 panics=0 ran_secs={}", after.as_secs());
        let _ = cx.update(|cx| cx.quit());
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_after_parses_seconds_and_ignores_junk() {
        // Process env is shared across tests; exercise the parser through a
        // legacy name nobody else sets.
        const LEGACY: &str = "SURYA_TEST_DEMO_EXIT_SECS_UNIQUE";
        unsafe { std::env::set_var(LEGACY, " 7 ") };
        assert_eq!(exit_after(Some(LEGACY)), Some(Duration::from_secs(7)));
        unsafe { std::env::set_var(LEGACY, "soon") };
        assert_eq!(exit_after(Some(LEGACY)), None);
        unsafe { std::env::remove_var(LEGACY) };
        assert_eq!(exit_after(Some(LEGACY)), None);
    }
}
