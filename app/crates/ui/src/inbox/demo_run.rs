//! `surya inbox-demo`: the three inbox views in a window, against fixtures.
//!
//! No engine and no ports: the views are fed [`super::demo`] rows directly,
//! so this starts anywhere and exercises exactly the layout code. The
//! engine-backed path is the same constructors with a live `AppState`.
//!
//! `SURYA_INBOX_DEMO_EXIT_SECS=<n>` quits after n seconds and prints
//! `inbox-demo: started=1 panics=0`, the headless proof on a box with no
//! usable display.

use std::path::PathBuf;

use gpui::{
    App, AppContext as _, Bounds, Context, Entity, Render, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, size,
};

use super::{AgentsRail, NeedsYouPane, RulesPane, demo};
use crate::theme::Theme;
use crate::{appearance, settings, theme_library, typography};

/// The three views side by side: the agent rail, the needs-you list, and the
/// approval-policy page.
struct DemoInbox {
    agents: Entity<AgentsRail>,
    needs_you: Entity<NeedsYouPane>,
    rules: Entity<RulesPane>,
}

impl Render for DemoInbox {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx);
        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(theme.bg)
            .child(
                div()
                    .w(px(260.0))
                    .h_full()
                    .flex_none()
                    .border_r_1()
                    .border_color(theme.border)
                    .child(self.agents.clone()),
            )
            .child(div().flex_1().min_w_0().h_full().child(self.needs_you.clone()))
            .child(
                div()
                    .w(px(320.0))
                    .h_full()
                    .flex_none()
                    .border_l_1()
                    .border_color(theme.border)
                    .child(self.rules.clone()),
            )
    }
}

pub fn run_demo(data_dir: PathBuf) {
    let exit_after = std::env::var("SURYA_INBOX_DEMO_EXIT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok());
    let app = gpui_platform::application().with_assets(crate::icons::Assets);
    app.run(move |cx: &mut App| {
        gpui_tokio::init(cx);
        let ui_settings = settings::UiSettings::load(&data_dir);
        settings::init(ui_settings.clone(), data_dir.clone(), cx);
        let fonts = typography::register_fonts(cx);
        typography::init(
            ui_settings.ui_font_family.clone(),
            ui_settings.ui_font_size,
            fonts,
            cx,
        );
        theme_library::init(data_dir.clone(), cx);
        appearance::init(
            ui_settings.appearance,
            ui_settings.theme_selection,
            ui_settings.accent,
            ui_settings.surface,
            cx,
        );

        let bounds = Bounds::centered(None, size(px(1180.), px(720.)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(600.), px(400.))),
                app_id: Some("surya-inbox-demo".into()),
                ..Default::default()
            },
            move |window, cx| {
                window.set_rem_size(px(typography::font_size(cx).pixels()));
                cx.new(|cx| DemoInbox {
                    agents: cx.new(|_| AgentsRail::demo(demo::agent_states(), demo::chats())),
                    needs_you: cx.new(|_| NeedsYouPane::demo(demo::needs_you(), demo::chats())),
                    rules: cx.new(|_| RulesPane::demo(demo::rules())),
                })
            },
        );
        match opened {
            Ok(_) => println!(
                "inbox-demo: window=1 inbox_rows={} agent_rows={} rules={}",
                demo::needs_you().len(),
                demo::agent_states().len(),
                demo::rules().len()
            ),
            Err(err) => {
                eprintln!("inbox-demo: window: {err}");
                cx.quit();
            }
        }

        if let Some(secs) = exit_after {
            cx.spawn(async move |cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(secs))
                    .await;
                println!("inbox-demo: started=1 panics=0 ran_secs={secs}");
                let _ = cx.update(|cx| cx.quit());
            })
            .detach();
        }
        cx.activate(true);
    });
}
