//! surya-ui — the gpui viewport. Shell, sidebar, conversation, composer, terminal,
//! diff pane.
//!
//! Design: ARCHITECTURE.md §4; animation catalog docs/research/feature-inventory.md
//! §1.12; virtualization/markdown techniques docs/research/mugen-pretext.md.
//!
//! M3a foundation:
//! - [`theme`] — always-dark monochrome theme (oklch-derived neutrals), a gpui Global;
//! - [`motion`] — the surya animation catalog over gpui `Animation` + cubic-bezier;
//! - [`state`] — `AppState` entity + `EngineHandle` (connect-or-embed engine);
//! - [`settings`] — persisted pane widths/collapse flags;
//! - [`shell`] — sidebar + main panel + right-pane scaffold + gate;
//! - [`loaders`] — surya pulse loader, gradient spinner, boot splash.

pub mod app_menus;
pub mod appearance;
pub mod attachments;
pub mod badges;
#[cfg(feature = "browser")]
pub mod browser_agent;
#[cfg(feature = "browser")]
pub mod browser_device;
#[cfg(feature = "browser")]
pub mod browser_pane;
#[cfg(feature = "browser")]
mod browser_proof;
pub mod cards;
#[cfg(test)]
mod cards_e2e;
pub mod change_requests;
pub mod changes;
pub mod comments;
pub mod composer;
pub mod demo_bootstrap;
pub mod edge_fade;
pub mod files;
pub mod frost;
pub mod history;
pub mod icons;
pub mod inbox;
pub mod key_chips;
pub mod links;
pub mod loaders;
pub mod markdown;
pub mod motion;
pub mod nav_rail;
pub mod notify;
pub mod permission_options;
pub mod pickers;
pub mod popover;
pub mod rail;
pub mod settings;
pub mod shell;
pub mod sound;
pub mod state;
pub mod surya;
pub mod syntax_cache;
pub mod tasks;
pub mod terminal;
pub mod theme;
pub mod theme_library;
pub mod transcript;
pub mod typography;
pub mod yolo;

use std::path::PathBuf;

use futures::StreamExt as _;
use gpui::{App, AppContext as _, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};

pub use state::{EngineBootConfig, RemoteEngineTarget};
pub use surya_proto::HarnessId;

/// Everything the headed binary passes in (config/env resolution lives in
/// `apps/surya`, not here).
#[derive(Debug, Clone)]
pub struct UiConfig {
    /// Data directory — engine stores + `ui-settings.json`.
    pub data_dir: PathBuf,
    /// IPC port: connect if an engine daemon is listening, embed if not.
    pub ipc_port: u16,
    /// Bind address the embedded engine serves other viewports on.
    pub ipc_bind: std::net::IpAddr,
    /// Explicit IPC token for the embedded engine's socket.
    pub ipc_token: Option<String>,
    /// `--engine`: dial this remote engine and never embed. Beats the saved
    /// active server in `ui-settings.json`.
    pub engine: Option<RemoteEngineTarget>,
    /// Edge base URL for the embedded engine.
    pub edge_url: String,
    /// Edge bearer; `None` runs offline.
    pub edge_token: Option<String>,
    /// Workspace org override for explicit dev-mode runs.
    pub org_id: Option<String>,
    /// WorkOS client id; `Some` makes the embedded headed engine require a
    /// production session before opening identity-scoped stores.
    pub workos_client_id: Option<String>,
    /// Harness for doc-command runs until per-chat config lands (M4).
    pub default_harness: HarnessId,
    /// Conversation URL passed by the OS on a cold launch.
    pub initial_url: Option<String>,
}

impl UiConfig {
    /// The boot configuration with the engine choice resolved by the caller
    /// (`--engine`, the saved active server, or local).
    fn boot_with(&self, remote: Option<RemoteEngineTarget>) -> EngineBootConfig {
        EngineBootConfig {
            data_dir: self.data_dir.clone(),
            ipc_port: self.ipc_port,
            ipc_bind: self.ipc_bind,
            ipc_token: self.ipc_token.clone(),
            remote,
            edge_url: self.edge_url.clone(),
            edge_token: self.edge_token.clone(),
            org_id: self.org_id.clone(),
            workos_client_id: self.workos_client_id.clone(),
            default_harness: self.default_harness,
        }
    }
}

/// What a dock-icon reopen needs to rebuild the main window after ⌘W closed it
/// (macOS keeps the process alive with just the menu bar, like zed).
struct ReopenState {
    state: gpui::Entity<state::AppState>,
    boot: EngineBootConfig,
}

impl gpui::Global for ReopenState {}

/// Run the headed app: tokio bridge up, engine bootstrap kicked off (probe →
/// connect-or-embed), 1320×880 window (min 900×600) with [`shell::Shell`] as the
/// root view, boot splash overlaid until the engine reports ready.
pub fn run_app(config: UiConfig) {
    let app = gpui_platform::application().with_assets(icons::Assets);
    let (url_tx, mut url_rx) = futures::channel::mpsc::unbounded::<String>();
    let callback_tx = url_tx.clone();
    app.on_open_urls(move |urls| {
        for url in urls {
            let _ = callback_tx.unbounded_send(url);
        }
    });
    if let Some(url) = config.initial_url.clone() {
        let _ = url_tx.unbounded_send(url);
    }
    // Dock-icon click with no window (⌘W closed it): rebuild the main window
    // around the still-running engine — zed does the same via `on_reopen`
    // (crates/zed/src/main.rs `app.on_reopen`).
    app.on_reopen(|cx| {
        if cx.windows().is_empty()
            && let Some(reopen) = cx.try_global::<ReopenState>()
        {
            let (state, boot) = (reopen.state.clone(), reopen.boot.clone());
            open_main_window(state, boot, cx);
        }
    });
    app.run(move |cx: &mut App| {
        // NB: pinned-rev API — `gpui_tokio::init(cx)` free function (not `Tokio::init`).
        gpui_tokio::init(cx);
        let data_dir = config.data_dir.clone();
        let ui_settings = settings::UiSettings::load(&data_dir);
        settings::init(ui_settings.clone(), data_dir.clone(), cx);
        // `--engine` wins; otherwise the saved active server; otherwise local.
        let boot = config.boot_with(
            config
                .engine
                .clone()
                .or_else(|| ui_settings.active_server_entry().map(settings::ServerEntry::target)),
        );
        let font_availability = typography::register_fonts(cx);
        // Typography first: theme installation reads the effective family, so
        // the first frame has the final font and palette without a flash.
        typography::init(
            ui_settings.ui_font_family.clone(),
            ui_settings.ui_font_size,
            font_availability,
            cx,
        );
        theme_library::init(data_dir.clone(), cx);
        // Before any window: gpui snaps `with_animation` elements when the
        // flag is set, so installing it after the first paint would play the
        // boot splash at full travel for a user who asked for none.
        motion::apply_motion_mode(ui_settings.motion, cx);
        appearance::init(
            ui_settings.appearance,
            ui_settings.theme_selection,
            ui_settings.accent,
            ui_settings.surface,
            cx,
        );
        // Keys (composer, files editor, terminal toggle, shortcuts) are bound
        // inside shell::apply_keymap, which clears and rebuilds the whole keymap
        // when the shell mounts; a boot-time bind_keys here would only be
        // discarded. app_menus::init registers actions, not keys.
        app_menus::init(cx);
        // CEF before the window: one browser per process, pumped from the
        // shell's render and an idle chain (surya-browser). After
        // `appearance::init` so the page's `prefers-color-scheme` matches
        // the theme the shell resolved.
        #[cfg(feature = "browser")]
        surya_browser::start(
            cx,
            if theme::Theme::of(cx).appearance.is_dark() {
                surya_browser::ColorScheme::Dark
            } else {
                surya_browser::ColorScheme::Light
            },
        );
        cx.register_url_scheme("surya").detach();
        // One release: the OS still routes links minted before the rename.
        cx.register_url_scheme("zeron").detach(); // surya-rename: legacy

        let state = cx.new(|_| state::AppState::new());
        let url_state = state.clone();
        cx.spawn(async move |cx| {
            while let Some(url) = url_rx.next().await {
                url_state.update(cx, |state, cx| state.open_deep_link(&url, cx));
            }
        })
        .detach();
        state::AppState::bootstrap(state.clone(), boot.clone(), cx);

        // Graceful teardown: an in-process engine drains live runs and flushes
        // doc snapshots before the process exits (remote engines outlive us).
        let quit_state = state.clone();
        cx.on_app_quit(move |cx| {
            settings::flush(cx);
            let shutdown =
                quit_state.read(cx).engine().cloned().map(|handle| {
                    gpui_tokio::Tokio::spawn(cx, async move { handle.shutdown().await })
                });
            async move {
                if let Some(task) = shutdown {
                    let _ = task.await;
                }
            }
        })
        .detach();
        // CEF comes down with the app: close the browser, then cef_shutdown,
        // so no helper process outlives a clean quit (surya-browser).
        #[cfg(feature = "browser")]
        cx.on_app_quit(|_| {
            surya_browser::shutdown();
            async {}
        })
        .detach();
        #[cfg(feature = "browser")]
        browser_proof::install(cx);
        // `SURYA_QUIT_AFTER=<seconds>`: a clean quit on a timer, for proofs
        // with no keyboard (the helper-count check after exit).
        #[cfg(feature = "browser")]
        if let Some(secs) = std::env::var("SURYA_QUIT_AFTER")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
        {
            cx.spawn(async move |cx: &mut gpui::AsyncApp| {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(secs))
                    .await;
                let _ = cx.update(|cx| cx.quit());
            })
            .detach();
        }

        cx.set_global(ReopenState {
            state: state.clone(),
            boot: boot.clone(),
        });
        open_main_window(state, boot, cx);
        // Native menu bar — macOS gets the standard app menu (About/Services/
        // Hide/Quit ⌘Q), Edit clipboard verbs routed to the focused input, and
        // a Window menu (⌘M/⌘W). Without this, `NSApp.mainMenu` stays nil: no
        // Cmd+Q, and nothing for the system menu bar to show. Set after
        // `open_main_window` because `Shell::new` ran `apply_keymap`
        // synchronously, so `set_menus` reads the final bindings for the ⌘-key
        // equivalents (gpui snapshots the keymap at set time).
        cx.set_menus(app_menus::app_menus());
        cx.activate(true);
    });
}

/// What the OS calls the window: the taskbar, Alt+Tab, window switchers and
/// screen readers all read this. Matches `app_id`.
const WINDOW_TITLE: &str = "surya";

/// The window's titlebar options.
///
/// The title is the OS-level name, not text the app paints. Left unset it
/// reaches the platform as an empty string (gpui_windows window.rs:455-462
/// hands `titlebar.title.unwrap_or("")` to `CreateWindowEx` as the window
/// name; x11 window.rs:545 and wayland window.rs:578 read the same field), so
/// the window had no name anywhere outside itself: `Get-Process surya |
/// Select-Object MainWindowTitle` came back empty (E2E-WIN-01).
///
/// Setting it does not put text on the custom-drawn strip. macOS:
/// frameless-inset chrome like the original Electron app (`titleBarStyle:
/// "hiddenInset"`, traffic lights at 14,15 - feature-inventory §1.1), and
/// with `appears_transparent` gpui calls
/// `setTitleVisibility_(NSWindowTitleHidden)` (gpui_macos window.rs:966-967),
/// so the title names the window for Mission Control and the Window menu and
/// paints nothing. On Linux/Windows `appears_transparent` hides the system
/// titlebar for our own chrome; harmless where unsupported.
fn titlebar() -> TitlebarOptions {
    TitlebarOptions {
        title: Some(WINDOW_TITLE.into()),
        appears_transparent: true,
        // Centered on the titlebar's content line (40px bar, content
        // shifted 4px down, lights ~12px tall -> center 22).
        traffic_light_position: Some(gpui::point(px(14.), px(14.))),
    }
}

/// Open the 1320×880 main window (min 900×600) with [`shell::Shell`] as the
/// root view. Called at boot and again from `on_reopen` if the dock icon is
/// clicked after ⌘W closed the window.
fn open_main_window(state: gpui::Entity<state::AppState>, boot: EngineBootConfig, cx: &mut App) {
    // surya window geometry: 1320×880, min 900×600 (feature-inventory §1.1).
    //
    // `SURYA_WINDOW_SIZE=1100x700` overrides it. A capture knob, in the same
    // family as `SURYA_OPEN_DIALOG` and `SURYA_MOTION_SCALE`: a floating-panel
    // layout fails at the SMALL end, where the margins and seams eat the
    // content, and there is no other way to photograph that. Ignored unless it
    // parses; never read outside boot.
    let forced = std::env::var("SURYA_WINDOW_SIZE")
        .ok()
        .and_then(|value| {
            let (w, h) = value.split_once(['x', 'X'])?;
            Some((w.trim().parse::<f32>().ok()?, h.trim().parse::<f32>().ok()?))
        })
        .filter(|(w, h)| w.is_finite() && h.is_finite() && *w >= 400.0 && *h >= 300.0);
    // Where the window was last left wins over the centred default, but not
    // over `SURYA_WINDOW_SIZE`: that knob exists to photograph a set size, and
    // a gallery frame must not come out at whatever rect the last session
    // happened to end on. A saved rect that no display can still show is
    // refused inside `settings::window::restore`, which falls back here
    // (E2E-WIN-02).
    let displays: Vec<_> = cx.displays().iter().map(|display| display.bounds()).collect();
    let saved = forced
        .is_none()
        .then(|| settings::window::restore(settings::current(cx).window_placement, &displays))
        .flatten();
    let window_bounds = saved.unwrap_or_else(|| {
        let (w, h) = forced.unwrap_or((1320.0, 880.0));
        WindowBounds::Windowed(Bounds::centered(None, size(px(w), px(h)), cx))
    });
    let handle = cx.open_window(
        WindowOptions {
            window_bounds: Some(window_bounds),
            window_min_size: Some(size(px(900.), px(600.))),
            // `kind` is deliberately left at its default `WindowKind::Normal`
            // (gpui platform.rs WindowOptions::default), which on macOS maps
            // to `NSNormalWindowLevel` (gpui_macos window.rs) — same as zed's
            // main window. Nothing here raises the window level or touches
            // presentation options; the "menu bar never appears" symptom came
            // from the missing `set_menus` call (nil `NSApp.mainMenu`), not
            // from window kind/level, and `appears_transparent` only affects
            // the titlebar, not the menu bar.
            titlebar: Some(titlebar()),
            // Our own titlebar strip drags the window (WindowControlArea::
            // Drag + start_window_move) — mark the content view app-owned
            // so AppKit neither dead-zones the strip nor delays clicks.
            app_owns_titlebar_drag: true,
            // Linux: request client-side decorations — surya draws its own
            // unified titlebar and (under CSD) its own caption buttons
            // (shell.rs `render_linux_caption_controls`). Leaving this unset
            // requests SERVER decorations, which stacked a compositor
            // titlebar on top of the app's chrome under sway/KDE, while
            // compositors without SSD support (GNOME) went client-side
            // anyway — frameless, and before the shell drew caption buttons,
            // with no window controls at all. The compositor can still
            // override via xdg-decoration negotiation; the shell re-resolves
            // what to draw every frame.
            window_decorations: cfg!(target_os = "linux")
                .then_some(gpui::WindowDecorations::Client),
            // Frosted shell (macOS): blur the desktop behind the window; the
            // shell paints its frost surface translucent so the sidebar reads
            // as glass (shell.rs root). Elsewhere blur support is compositor
            // roulette — stay opaque.
            // One source of truth with the re-apply loop in `appearance::apply`
            // — if these two ever disagree, vibrancy dies on the first theme
            // change and never comes back.
            window_background: theme::Theme::of(cx).window_background_appearance(),
            app_id: Some("surya".into()),
            ..Default::default()
        },
        move |window, cx| {
            window.set_rem_size(px(typography::font_size(cx).pixels()));
            // React to the user flipping macOS between light and dark. Detached:
            // the subscription lives as long as the window does, and the window
            // owns nothing that would drop it early.
            appearance::observe_window(window, cx).detach();
            cx.new(|cx| shell::Shell::new(state, boot, cx))
        },
    )
    .expect("failed to open window");
    // Remember where the window is put, so the next launch opens there.
    // Debounced, because a mouse resize fires this on every frame of the drag
    // and the settings file is not somewhere to write at the frame rate; the
    // `on_app_quit` handler above calls `settings::flush`, so the last rect
    // still reaches disk on a clean close. Nothing reads the window while it
    // is being destroyed, which is the part that is awkward to get right on
    // Windows.
    handle
        .update(cx, |_, window, cx| {
            cx.observe_window_bounds(window, |_, window, cx| {
                let placement = settings::window::WindowPlacement::of(window.window_bounds());
                settings::update(settings::SavePolicy::Debounced, cx, |settings| {
                    settings.window_placement = Some(placement);
                });
            })
            .detach();
        })
        .expect("failed to watch window bounds");
    // Belt and braces: assert the blur once the window actually exists. The
    // `WindowOptions` value is applied during creation, before the view is
    // attached; re-pushing it here means a window is never left opaque.
    appearance::reapply_window_background(cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// E2E-WIN-01: the window must carry a name for the OS, or the taskbar,
    /// Alt+Tab, window switchers and screen readers have nothing to call it.
    /// This is the whole fix, so it is the whole test: put `None` back and it
    /// fails.
    #[test]
    fn the_window_is_named_for_the_os() {
        assert_eq!(titlebar().title.as_deref(), Some("surya"));
    }

    /// Naming the window must not have disturbed the frameless chrome: the
    /// custom strip needs the system titlebar transparent, and the traffic
    /// lights stay where the layout puts them.
    #[test]
    fn naming_the_window_leaves_the_custom_chrome_alone() {
        let bar = titlebar();
        assert!(bar.appears_transparent);
        assert_eq!(bar.traffic_light_position, Some(gpui::point(px(14.), px(14.))));
    }
}
