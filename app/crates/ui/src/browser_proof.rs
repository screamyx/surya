//! Proof knobs for the browser pane on a box with no keyboard (display :7).
//! Each reads one environment variable at start-up and does what a person
//! would do, on a timer, printing counters as pairs. Nothing here runs
//! unless the variable is set.
//!
//! - `SURYA_PROOF_FLIP_SCHEME=<seconds>`: after that long, switch the app's
//!   appearance to the opposite mode, the way the theme menu would. The
//!   page's `prefers-color-scheme` follows through `set_color_scheme`;
//!   `app/scripts/browser-scheme-flip-proof.sh` reads the frame dumps.
//! - `SURYA_PROOF_DEVICE=<seconds>`: after that long, read the page's
//!   `innerWidth`, pick the iPhone 15 preset, wait for the reload, read it
//!   again, and print `proof: device asked=2 matched=N`.

use std::time::Duration;

use gpui::App;
use serde_json::json;

use crate::appearance::{self, AppearanceMode};
use crate::theme::Theme;

fn seconds(key: &str) -> Option<u64> {
    std::env::var(key).ok()?.trim().parse().ok()
}

pub fn install(cx: &mut App) {
    if let Some(after) = seconds("SURYA_PROOF_FLIP_SCHEME") {
        cx.spawn(async move |cx| {
            cx.background_executor().timer(Duration::from_secs(after)).await;
            let _ = cx.update(|cx| {
                let dark = Theme::of(cx).appearance.is_dark();
                let to = if dark { AppearanceMode::Light } else { AppearanceMode::Dark };
                appearance::set_mode(to, cx);
                println!("proof: scheme flipped asked=1 done=1 to={to:?}");
            });
        })
        .detach();
    }
    if let Some(after) = seconds("SURYA_PROOF_DEVICE") {
        cx.spawn(async move |cx| {
            cx.background_executor().timer(Duration::from_secs(after)).await;
            // `innerWidth` is the layout viewport: a page with a viewport
            // meta lays out at the device width (the probe page has one);
            // one without falls back to Chromium's 980px mobile layout.
            let width = json!({ "js": "innerWidth" });
            let before = surya_browser::agent::run("browser_eval", &width).await;
            surya_browser::emulation::set_device(surya_browser::emulation::Device::IPhone15);
            // The preset reloads the page once; give the reload its time.
            cx.background_executor().timer(Duration::from_secs(8)).await;
            let after_w = surya_browser::agent::run("browser_eval", &width).await;
            let read = |r: &Result<serde_json::Value, String>| {
                r.as_ref().ok().and_then(|v| v["value"].as_i64()).unwrap_or(-1)
            };
            let (b, a) = (read(&before), read(&after_w));
            let wanted = surya_browser::emulation::Device::IPhone15
                .metrics()
                .map(|m| m.width as i64)
                .unwrap_or(0);
            let matched = u8::from(b > 0 && b != wanted) + u8::from(a == wanted);
            let screen = surya_browser::agent::run("browser_eval", &json!({ "js": "screen.width" })).await;
            println!(
                "proof: device asked=2 matched={matched} before={b} after={a} wanted={wanted} screen={} {}",
                read(&screen),
                surya_browser::devtools::counters()
            );
        })
        .detach();
    }
}
