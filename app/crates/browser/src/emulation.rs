//! Device emulation: the page laid out for a phone or a tablet, in the pane.
//!
//! Chrome DevTools' device mode through `Emulation.setDeviceMetricsOverride`,
//! touch emulation and a user agent, sent in process (`devtools.rs`). Ported
//! from haktui's `device.rs` + `emulation.rs` (D48), with these changes for
//! comet's pane: four presets instead of one phone (the owner reviews sites,
//! not one staff app), the user agent is overridden too so a site that
//! sniffs it serves its phone page, and the touch circle and the phone
//! "fit" are not here (the surface is the lead seat's; the override changes
//! the layout viewport, which is what the readback proves).
//!
//! **The preset is one switch for every tab**, as in haktui: DevTools
//! emulates per target, so a pick walks every open browser, and a tab
//! opened later gets the preset on its first load end. A preset that
//! applied to one tab and not the next would read as a bug in the page.
//!
//! `deviceScaleFactor` is left at the window's own (0), as haktui found: a
//! forced 3x changes the size of the frame CEF paints under offscreen
//! rendering, and that path cost haktui three sessions. The layout width
//! is what a phone breakpoint reads.
//!
//! After applying, the page reloads once: `'ontouchstart' in window` and a
//! library's input-handler choice are decided when a document is created
//! (haktui, measured 2026-08-27), so DevTools shows a "reload to apply"
//! banner for the same reason.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;

use cef::ImplBrowser as _;
use serde_json::{Value, json};

use crate::devtools;

/// The picker's presets, in the order the bar cycles them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Device {
    Desktop = 0,
    IPhone15 = 1,
    Pixel8 = 2,
    IPad = 3,
}

pub const ALL: [Device; 4] = [Device::Desktop, Device::IPhone15, Device::Pixel8, Device::IPad];

/// One preset's viewport and identity. Widths and heights are CSS pixels,
/// portrait; the values Chrome DevTools ships for the same names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub width: i32,
    pub height: i32,
    pub dpr: f64,
    pub mobile: bool,
    pub user_agent: &'static str,
}

impl Device {
    pub fn label(self) -> &'static str {
        match self {
            Device::Desktop => "Desktop",
            Device::IPhone15 => "iPhone 15",
            Device::Pixel8 => "Pixel 8",
            Device::IPad => "iPad",
        }
    }

    /// The bar's short id, for a log line or an env knob.
    pub fn key(self) -> &'static str {
        match self {
            Device::Desktop => "desktop",
            Device::IPhone15 => "iphone15",
            Device::Pixel8 => "pixel8",
            Device::IPad => "ipad",
        }
    }

    pub fn from_key(key: &str) -> Option<Device> {
        ALL.iter().copied().find(|d| d.key() == key.trim().to_ascii_lowercase())
    }

    pub fn next(self) -> Device {
        let i = ALL.iter().position(|d| *d == self).unwrap_or(0);
        ALL[(i + 1) % ALL.len()]
    }

    /// `None` for the desktop: no override at all.
    pub fn metrics(self) -> Option<Metrics> {
        match self {
            Device::Desktop => None,
            Device::IPhone15 => Some(Metrics {
                width: 393,
                height: 852,
                dpr: 3.0,
                mobile: true,
                user_agent: "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
            }),
            Device::Pixel8 => Some(Metrics {
                width: 412,
                height: 915,
                dpr: 2.625,
                mobile: true,
                user_agent: "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36",
            }),
            Device::IPad => Some(Metrics {
                width: 820,
                height: 1180,
                dpr: 2.0,
                mobile: true,
                user_agent: "Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
            }),
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);
/// The browser's own user agent, read once before the first override so
/// Desktop restores it exactly (CDP has no "clear user agent override").
static DEFAULT_UA: Mutex<Option<String>> = Mutex::new(None);
/// Browsers that carry the current preset, by CEF id: one apply and one
/// reload each per pick, never a loop.
static APPLIED: Mutex<Vec<i32>> = Mutex::new(Vec::new());

pub fn device() -> Device {
    ALL[(CURRENT.load(Ordering::Acquire) as usize).min(ALL.len() - 1)]
}

/// The DevTools calls that put a page into a preset, in order. Pure, so the
/// suite checks the payloads without a browser.
pub fn cdp_calls(device: Device, default_ua: &str) -> Vec<(&'static str, Value)> {
    match device.metrics() {
        Some(m) => vec![
            (
                "Emulation.setDeviceMetricsOverride",
                json!({
                    "width": m.width,
                    "height": m.height,
                    "deviceScaleFactor": 0,
                    "mobile": m.mobile,
                }),
            ),
            (
                "Emulation.setTouchEmulationEnabled",
                json!({ "enabled": true, "maxTouchPoints": 1 }),
            ),
            ("Emulation.setUserAgentOverride", json!({ "userAgent": m.user_agent })),
        ],
        None => vec![
            ("Emulation.clearDeviceMetricsOverride", json!({})),
            ("Emulation.setTouchEmulationEnabled", json!({ "enabled": false })),
            ("Emulation.setUserAgentOverride", json!({ "userAgent": default_ua })),
        ],
    }
}

/// Choose a preset for the pane: every open browser gets it and reloads
/// once; a browser opened later gets it on its first load end.
pub fn set_device(device: Device) {
    let before = self::device();
    CURRENT.store(device as u8, Ordering::Release);
    println!("emulation: {} -> {}", before.key(), device.key());
    if crate::disabled() {
        return;
    }
    let open = devtools::browsers();
    if let Ok(mut applied) = APPLIED.lock() {
        applied.clear();
        applied.extend(open.iter().copied());
    }
    for id in open {
        apply_and_reload(id);
    }
}

pub fn cycle() -> Device {
    let next = device().next();
    set_device(next);
    next
}

/// Put one browser into the current preset and reload it.
fn apply_and_reload(browser: i32) {
    let device = device();
    let ua = DEFAULT_UA.lock().ok().and_then(|g| g.clone()).unwrap_or_default();
    for (method, params) in cdp_calls(device, &ua) {
        devtools::fire(browser, method, params);
    }
    crate::cef_thread::on_ui(move || {
        if let Some(b) = devtools::browser(browser) {
            b.reload();
        }
    });
}

/// A main frame finished loading in browser `id`. Learns the default user
/// agent on the first load of the process, and gives a browser that does
/// not carry the current preset yet the preset once (then reloads it once).
pub(crate) fn on_load_end(id: i32) {
    if DEFAULT_UA.lock().map(|g| g.is_none()).unwrap_or(false) {
        devtools::send(
            id,
            "Runtime.evaluate",
            json!({ "expression": "navigator.userAgent", "returnByValue": true }),
            Box::new(|r| {
                if let Ok(v) = r
                    && let Some(ua) = v["result"]["value"].as_str()
                    && let Ok(mut g) = DEFAULT_UA.lock()
                    && g.is_none()
                {
                    *g = Some(ua.to_string());
                }
            }),
        );
    }
    if device() == Device::Desktop {
        return;
    }
    let Ok(mut applied) = APPLIED.lock() else { return };
    if applied.contains(&id) {
        return;
    }
    applied.push(id);
    drop(applied);
    println!("emulation: browser {id} loaded, applying {} then reloading once", device().key());
    apply_and_reload(id);
}

/// The browser went away; its id may come back for another one.
pub(crate) fn forget(id: i32) {
    if let Ok(mut applied) = APPLIED.lock() {
        applied.retain(|b| *b != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phone_asks_for_its_width_touch_and_a_phone_user_agent() {
        let calls = cdp_calls(Device::IPhone15, "Desktop UA");
        assert_eq!(calls[0].0, "Emulation.setDeviceMetricsOverride");
        assert_eq!(calls[0].1["width"], 393);
        assert_eq!(calls[0].1["height"], 852);
        assert_eq!(calls[0].1["mobile"], true);
        assert_eq!(calls[1].1["enabled"], true);
        assert!(calls[2].1["userAgent"].as_str().unwrap().contains("iPhone"));
    }

    #[test]
    fn the_scale_is_left_to_the_window() {
        // A forced DPR changes the frame CEF paints; see the module doc.
        for d in [Device::IPhone15, Device::Pixel8, Device::IPad] {
            assert_eq!(cdp_calls(d, "")[0].1["deviceScaleFactor"], 0, "{d:?}");
        }
    }

    #[test]
    fn desktop_clears_everything_and_restores_the_real_user_agent() {
        let calls = cdp_calls(Device::Desktop, "Mozilla/5.0 real");
        assert_eq!(calls[0].0, "Emulation.clearDeviceMetricsOverride");
        assert_eq!(calls[1].1["enabled"], false);
        assert_eq!(calls[2].1["userAgent"], "Mozilla/5.0 real");
    }

    #[test]
    fn the_bar_cycles_through_every_preset_and_back() {
        let mut d = Device::Desktop;
        let mut seen = vec![d];
        for _ in 0..3 {
            d = d.next();
            seen.push(d);
        }
        assert_eq!(seen, ALL.to_vec());
        assert_eq!(d.next(), Device::Desktop);
    }

    #[test]
    fn keys_round_trip() {
        for d in ALL {
            assert_eq!(Device::from_key(d.key()), Some(d));
        }
        assert_eq!(Device::from_key(" iPhone15 "), Some(Device::IPhone15));
        assert_eq!(Device::from_key("nokia"), None);
    }

    #[test]
    fn a_pick_forgets_who_had_the_old_preset() {
        APPLIED.lock().unwrap().extend([7, 8]);
        // No browsers are observed in a test, so the pick marks none.
        set_device(Device::Pixel8);
        assert!(APPLIED.lock().unwrap().is_empty(), "a background tab must not keep the old preset");
        set_device(Device::Desktop);
    }
}
