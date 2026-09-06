//! `zeron files-demo <checkout>`: the [`FilesPane`] alone in a window against
//! a local engine, so the tree and editor can be driven without the shell.
//! The engine gets its own data dir and IPC port so the demo never attaches
//! to, or writes a space into, the real app's workspace.
//!
//! `SURYA_DEMO_EXIT_SECS=<n>` (or the older `SURYA_FILES_DEMO_EXIT_SECS`) quits after n seconds and prints
//! `files-demo: started=1 panics=0`, the headless proof on a box with no
//! usable display.

use std::path::PathBuf;

use gpui::{App, AppContext as _, Bounds, WindowBounds, WindowOptions, px, size};
use gpui_tokio::Tokio;
use zeron_proto::HarnessId;
use zeron_rpc::methods;

use super::FilesPane;
use crate::state::{EngineBootConfig, EngineHandle};
use crate::typography;

pub fn run_demo(checkout: PathBuf, data_dir: PathBuf, ipc_port: u16, edge_url: String) {
    let checkout = match std::fs::canonicalize(&checkout) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("files-demo: {}: {err}", checkout.display());
            std::process::exit(2);
        }
    };
    let exit_after = crate::demo_bootstrap::exit_after(Some("SURYA_FILES_DEMO_EXIT_SECS"));
    let app = gpui_platform::application().with_assets(crate::icons::Assets);
    app.run(move |cx: &mut App| {
        crate::demo_bootstrap::init_app(&data_dir, cx);
        super::init(cx);
        // The demo never runs the shell's apply_keymap, which is where the
        // app binds the save chord: bind it here or Ctrl+S does nothing.
        super::bind_save_keys(cx);

        let boot = EngineBootConfig {
            data_dir: data_dir.clone(),
            ipc_port,
            ipc_bind: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            ipc_token: None,
            remote: None,
            edge_url,
            edge_token: None,
            org_id: None,
            workos_client_id: None,
            default_harness: HarnessId::ClaudeCode,
        };
        let booted = Tokio::spawn(cx, EngineHandle::bootstrap(boot));
        let checkout_for_space = checkout.clone();
        cx.spawn(async move |cx| {
            let handle = match booted.await {
                Ok(Ok(handle)) => handle,
                Ok(Err(err)) => {
                    eprintln!("files-demo: engine: {err}");
                    let _ = cx.update(|cx| cx.quit());
                    return;
                }
                Err(err) => {
                    eprintln!("files-demo: engine task: {err:?}");
                    let _ = cx.update(|cx| cx.quit());
                    return;
                }
            };
            let space_id = match register_space(&handle, &checkout_for_space).await {
                Ok(id) => id,
                Err(err) => {
                    eprintln!("files-demo: space: {err}");
                    let _ = cx.update(|cx| cx.quit());
                    return;
                }
            };
            let pane_engine = handle.clone();
            let _ = cx.update(|cx| {
                let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
                let opened = cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        window_min_size: Some(size(px(600.), px(400.))),
                        app_id: Some("zeron-files-demo".into()),
                        ..Default::default()
                    },
                    move |window, cx| {
                        window.set_rem_size(px(typography::font_size(cx).pixels()));
                        cx.new(|cx| FilesPane::new(pane_engine, space_id, cx))
                    },
                );
                match opened {
                    Ok(_) => println!(
                        "files-demo: window=1 checkout={}",
                        checkout_for_space.display()
                    ),
                    Err(err) => {
                        eprintln!("files-demo: window: {err}");
                        cx.quit();
                    }
                }
            });
        })
        .detach();

        crate::demo_bootstrap::schedule_exit("files-demo", exit_after, cx);
        cx.activate(true);
    });
}

/// Create (or reuse) a space for `checkout` on this device and return its id.
async fn register_space(
    handle: &EngineHandle,
    checkout: &std::path::Path,
) -> anyhow::Result<String> {
    let client = handle.client();
    let device = client
        .call(methods::LOCAL_DEVICE, serde_json::json!({}))
        .await?;
    let device_id = device
        .get("deviceId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("LocalDevice gave no deviceId"))?
        .to_string();
    let path = checkout.to_string_lossy().to_string();
    // Reuse a space already pointing at this folder, if the workspace has one.
    let mut spaces = client
        .subscribe(methods::WATCH_SPACES, serde_json::json!({}))
        .await?;
    if let Some(first) = spaces.recv().await
        && let Some(existing) = first
            .as_array()
            .into_iter()
            .flatten()
            .find(|s| s.get("path").and_then(|p| p.as_str()) == Some(path.as_str()))
        && let Some(id) = existing.get("id").and_then(|v| v.as_str())
    {
        return Ok(id.to_string());
    }
    drop(spaces);
    let space_id = uuid::Uuid::new_v4().to_string();
    client
        .call(
            methods::MUTATE,
            serde_json::json!({
                "op": "createSpace",
                "spaceId": space_id,
                "deviceId": device_id,
                "path": path,
                "gitDetected": true,
            }),
        )
        .await?;
    Ok(space_id)
}
