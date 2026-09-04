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
fn switches() -> Vec<&'static str> {
    let mut s = Vec::new();
    if std::env::var_os("SURYA_CEF_GPU").is_none() {
        s.push("disable-gpu");
        s.push("disable-gpu-compositing");
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
                cl.append_switch(Some(&CefString::from(switch)));
            }
            println!("browser: switches {:?}", switches());
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(ProcessHandler::new())
        }
    }
}
