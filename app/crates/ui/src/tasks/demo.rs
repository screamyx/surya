//! `surya --tasks-demo`: a window with only the Tasks pane, against the local
//! engine (`surya headless` on the IPC port). No shell, no sidebar. Set
//! `SURYA_DEMO_EXIT_SECS` (shared with files-demo) to quit by itself and print `started=1 panics=0`
//! (the headless proof on a box without a display).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gpui::{App, AppContext as _, Bounds, WindowBounds, WindowOptions, px, size};
use surya_proto::Space;
use surya_rpc::methods;

use super::TasksPane;
use crate::{icons, typography};

pub struct DemoConfig {
    pub data_dir: PathBuf,
    pub ipc_port: u16,
    /// Board to open; default = the first space the engine lists.
    pub space: Option<String>,
    pub exit_after: Option<Duration>,
}

pub fn run(config: DemoConfig) {
    let app = gpui_platform::application().with_assets(icons::Assets);
    app.run(move |cx: &mut App| {
        crate::demo_bootstrap::init_app(&config.data_dir, cx);

        let url = format!("ws://127.0.0.1:{}", config.ipc_port);
        let wanted = config.space.clone();
        let dial = gpui_tokio::Tokio::spawn(cx, async move { surya_rpc::connect_ws(&url).await });
        cx.spawn(async move |cx| {
            let client = match dial.await {
                Ok(Ok(client)) => Arc::new(client),
                Ok(Err(err)) => {
                    eprintln!("tasks-demo: no engine on the IPC port: {err}");
                    let _ = cx.update(|cx| cx.quit());
                    return;
                }
                Err(err) => {
                    eprintln!("tasks-demo: dial task failed: {err}");
                    let _ = cx.update(|cx| cx.quit());
                    return;
                }
            };
            let (space_id, space_name) = pick_space(&client, wanted.as_deref()).await;
            let opened = cx.update(|cx| {
                let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        app_id: Some("surya-tasks-demo".into()),
                        ..Default::default()
                    },
                    move |window, cx| {
                        window.set_rem_size(px(typography::font_size(cx).pixels()));
                        cx.new(|cx| TasksPane::new(client, space_id, space_name, cx))
                    },
                )
            });
            match opened {
                Ok(_) => println!("tasks-demo: started=1 window=1"),
                Err(err) => eprintln!("tasks-demo: open_window failed: {err}"),
            }
        })
        .detach();

        crate::demo_bootstrap::schedule_exit("tasks-demo", config.exit_after, cx);
        cx.activate(true);
    });
}

/// The requested space, else the first one the engine has, else a
/// placeholder board that stays empty (the pane still runs).
async fn pick_space(client: &surya_rpc::RpcClient, wanted: Option<&str>) -> (String, String) {
    let spaces: Vec<Space> = match client
        .subscribe(methods::WATCH_SPACES, serde_json::json!({}))
        .await
    {
        Ok(mut rx) => rx
            .recv()
            .await
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let chosen = match wanted {
        Some(id) => spaces.iter().find(|s| s.id == id),
        None => spaces.first(),
    };
    match chosen {
        Some(space) => (space.id.clone(), space.display_name().to_string()),
        None => (
            wanted.unwrap_or("demo").to_string(),
            wanted.unwrap_or("demo").to_string(),
        ),
    }
}
