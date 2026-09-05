//! The CEF app: command-line switches for the browser process, and the
//! process handler that hands CEF's pump requests to `pump.rs`.

use std::sync::atomic::{AtomicBool, Ordering};

use cef::rc::Rc as _;
use cef::*;

use crate::ColorScheme;

/// The app's appearance at start-up, read when Chromium builds its command
/// line. A switch is start-up only: a mid-session theme change needs CDP
/// `Emulation.setEmulatedMedia`, which waits on the CDP wiring (post-RC).
static DARK: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_color_scheme(scheme: ColorScheme) {
    DARK.store(matches!(scheme, ColorScheme::Dark), Ordering::Release);
}

fn color_scheme() -> ColorScheme {
    if DARK.load(Ordering::Acquire) {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    }
}

wrap_browser_process_handler! {
    struct ProcessHandler;
    impl BrowserProcessHandler {
        fn on_schedule_message_pump_work(&self, delay_ms: i64) {
            crate::pump::schedule_pump(delay_ms);
        }
    }
}

/// Switches for the browser process. Software rendering by default: the
/// pane takes CPU pixels anyway, and a GPU process under a virtual display
/// is the first thing to fail. `SURYA_CEF_GPU=1` leaves the GPU on.
///
/// `force-dark-mode` flips Chromium's native theme to dark, which is what
/// the page's `prefers-color-scheme: dark` media query reads (critique
/// round 4, R1: a dark app showed pages their light styling).
fn switches(scheme: ColorScheme) -> Vec<String> {
    // Chrome's first-run flow on a fresh profile makes ContentMainRun return
    // 28 (RESULT_CODE_EULA_REFUSED) on Linux and CEF's initialize fails
    // (measured 2026-09-05 under Xvfb; a second launch on the same profile
    // failed the same way). Skipping first run and the default-browser
    // prompt is what an embedded browser wants anyway.
    let mut s = vec!["no-first-run".to_string(), "no-default-browser-check".to_string()];
    // Zero-copy needs the GPU process: a shared texture comes from its
    // compositor, so the two switches below would leave the pane blank.
    if std::env::var_os("SURYA_CEF_GPU").is_none() && !crate::zero_copy::enabled() {
        s.push("disable-gpu".to_string());
        s.push("disable-gpu-compositing".to_string());
    }
    if matches!(scheme, ColorScheme::Dark) {
        s.push("force-dark-mode".to_string());
    }
    // `SURYA_CEF_SWITCHES=a,b=c`: extra Chromium switches for a diagnosis,
    // e.g. `enable-logging=stderr,v=1`.
    if let Ok(extra) = std::env::var("SURYA_CEF_SWITCHES") {
        s.extend(extra.split(',').map(str::trim).filter(|x| !x.is_empty()).map(String::from));
    }
    s
}

wrap_app! {
    pub(crate) struct SuryaApp;
    impl App {
        fn on_before_command_line_processing(
            &self,
            process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            let browser_process = process_type.map(|p| p.to_string().is_empty()).unwrap_or(true);
            if !browser_process {
                return;
            }
            let Some(cl) = command_line else { return };
            let scheme = color_scheme();
            for switch in switches(scheme) {
                match switch.split_once('=') {
                    Some((name, value)) => cl.append_switch_with_value(
                        Some(&CefString::from(name)),
                        Some(&CefString::from(value)),
                    ),
                    None => cl.append_switch(Some(&CefString::from(switch.as_str()))),
                }
            }
            println!("browser: switches {:?} scheme={scheme:?}", switches(scheme));
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(ProcessHandler::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_scheme_adds_force_dark_mode_and_light_does_not() {
        let dark = switches(ColorScheme::Dark);
        let light = switches(ColorScheme::Light);
        assert!(dark.iter().any(|s| s == "force-dark-mode"), "{dark:?}");
        assert!(!light.iter().any(|s| s == "force-dark-mode"), "{light:?}");
        // Everything else is the same in both.
        let rest: Vec<_> = dark.iter().filter(|s| *s != "force-dark-mode").collect();
        assert_eq!(rest, light.iter().collect::<Vec<_>>());
    }

    #[test]
    fn scheme_is_light_until_set() {
        assert_eq!(color_scheme(), ColorScheme::Light);
        set_color_scheme(ColorScheme::Dark);
        assert_eq!(color_scheme(), ColorScheme::Dark);
        set_color_scheme(ColorScheme::Light);
        assert_eq!(color_scheme(), ColorScheme::Light);
    }
}
