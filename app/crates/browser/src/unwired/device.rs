//! Mobile view mode: the decisions, with no CEF in them.
//!
//! The owner reviews a phone app in column 3. In desktop mode he is driving a
//! phone app with a mouse at desktop width, and every pin he takes there
//! records `pointer: mouse` and a breakpoint the phone never hits. This is
//! Chrome DevTools' device mode, in our own browser: a phone-sized viewport,
//! touch instead of a mouse, and a circle that shows where the touch lands.
//!
//! Built on every platform, like `input` and `chrome`, so the suite can check
//! the arithmetic and the CDP payloads without a browser. `emulation.rs` is the
//! Windows half that sends them.

/// The phone. One preset, on purpose: he has one phone, and a picker is a
/// decision he would have to make every time he toggles.
///
/// 390x844 is the iPhone 14 class viewport in CSS pixels, which is also what
/// the staff PWA's `sm` breakpoint (640px) is measured against.
pub const PHONE_W: i32 = 390;
pub const PHONE_H: i32 = 844;

/// The touch circle's radius in DIPs. DevTools draws about this size.
pub const TOUCH_R: f32 = 12.0;

/// Where the phone viewport sits inside the column, in DIPs from the column's
/// own origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fit {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Fit the phone into the column: never wider or taller than the column, and
/// centred across it. Top-aligned rather than centred vertically, because a
/// phone app puts its navigation at the top and the bottom, and a viewport
/// that starts part way down the column reads as a page that has scrolled.
pub fn fit(column_w: i32, column_h: i32) -> Fit {
    let w = PHONE_W.min(column_w).max(1);
    let h = PHONE_H.min(column_h).max(1);
    Fit { x: (column_w - w).max(0) / 2, y: 0, w, h }
}

/// The CDP calls that turn the mode on or off, in the order to send them.
///
/// Every call is one Chromium already accepts over the remote debugging port,
/// so an agent on the 9333 tunnel can read the same state back.
///
/// `deviceScaleFactor: 0` keeps the window's own scale. A forced 3x would make
/// `devicePixelRatio` honest for the phone, but under offscreen rendering it
/// also changes the size of the frame CEF paints, and that path has cost three
/// sessions already. Layout width is what the pins need.
pub fn cdp_calls(on: bool, w: i32, h: i32) -> Vec<(&'static str, serde_json::Value)> {
    if on {
        vec![
            (
                "Emulation.setDeviceMetricsOverride",
                serde_json::json!({
                    "width": w,
                    "height": h,
                    "deviceScaleFactor": 0,
                    "mobile": true,
                }),
            ),
            (
                "Emulation.setTouchEmulationEnabled",
                serde_json::json!({ "enabled": true, "maxTouchPoints": 1 }),
            ),
        ]
    } else {
        vec![
            ("Emulation.clearDeviceMetricsOverride", serde_json::json!({})),
            (
                "Emulation.setTouchEmulationEnabled",
                serde_json::json!({ "enabled": false }),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phone_is_centred_in_a_wide_column() {
        // 1200 wide: 390 in the middle leaves 405 either side.
        assert_eq!(fit(1200, 1000), Fit { x: 405, y: 0, w: 390, h: 844 });
    }

    #[test]
    fn a_narrow_column_clamps_the_phone_rather_than_overflowing() {
        // A viewport wider than the surface would be painted off the edge and
        // every click past it would land on nothing.
        assert_eq!(fit(300, 600), Fit { x: 0, y: 0, w: 300, h: 600 });
    }

    #[test]
    fn the_phone_is_top_aligned() {
        assert_eq!(fit(1000, 2000).y, 0);
    }

    #[test]
    fn a_collapsed_column_still_gives_cef_a_real_size() {
        // CEF's view_rect must never be 0x0; the surface clamps to 64 for the
        // same reason.
        let f = fit(0, 0);
        assert!(f.w >= 1 && f.h >= 1);
    }

    #[test]
    fn turning_on_asks_for_mobile_and_touch() {
        let calls = cdp_calls(true, 390, 844);
        assert_eq!(calls[0].0, "Emulation.setDeviceMetricsOverride");
        assert_eq!(calls[0].1["mobile"], true);
        assert_eq!(calls[0].1["width"], 390);
        assert_eq!(calls[0].1["height"], 844);
        assert_eq!(calls[1].0, "Emulation.setTouchEmulationEnabled");
        assert_eq!(calls[1].1["enabled"], true);
    }

    #[test]
    fn turning_off_clears_both() {
        let calls = cdp_calls(false, 390, 844);
        assert_eq!(calls[0].0, "Emulation.clearDeviceMetricsOverride");
        assert_eq!(calls[1].1["enabled"], false);
    }

    #[test]
    fn the_scale_is_left_alone() {
        // A forced DPR changes the frame CEF paints. See the doc comment.
        assert_eq!(cdp_calls(true, 390, 844)[0].1["deviceScaleFactor"], 0);
    }
}
