//! The CEF app: command-line switches for the browser process, and the
//! process handler that hands CEF's pump requests to `pump.rs`.

use cef::rc::Rc as _;
use cef::*;

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
fn switches() -> Vec<String> {
    // Chrome's first-run flow on a fresh profile makes ContentMainRun return
    // 28 (RESULT_CODE_EULA_REFUSED) on Linux and CEF's initialize fails
    // (measured 2026-09-05 under Xvfb; a second launch on the same profile
    // failed the same way). Skipping first run and the default-browser
    // prompt is what an embedded browser wants anyway.
    let mut s = vec!["no-first-run".to_string(), "no-default-browser-check".to_string()];
    if std::env::var_os("SURYA_CEF_GPU").is_none() {
        s.push("disable-gpu".to_string());
        s.push("disable-gpu-compositing".to_string());
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
            for switch in switches() {
                match switch.split_once('=') {
                    Some((name, value)) => cl.append_switch_with_value(
                        Some(&CefString::from(name)),
                        Some(&CefString::from(value)),
                    ),
                    None => cl.append_switch(Some(&CefString::from(switch.as_str()))),
                }
            }
            println!("browser: switches {:?}", switches());
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(ProcessHandler::new())
        }
    }
}
