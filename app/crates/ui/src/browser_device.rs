//! The device picker in the browser bar: one 24px control that names the
//! current preset (Desktop, iPhone 15, Pixel 8, iPad) and cycles to the
//! next on click. No menu and no floating box (decision 26): four presets
//! are one click each at most, and the label says where you are.
//!
//! The choice is process state in `surya_browser::emulation`, like the page
//! itself, so this is a function of the theme and nothing else; the bar
//! calls it at the end of its row.

use gpui::{SharedString, div, prelude::*, px};

use crate::icons::{self, icon};
use crate::theme::Theme;
use surya_browser::emulation::{self, Device};

fn glyph(device: Device) -> &'static str {
    match device {
        Device::Desktop => icons::MONITOR,
        Device::IPhone15 | Device::Pixel8 => icons::SMARTPHONE,
        Device::IPad => icons::LAPTOP,
    }
}

/// The control. Click cycles the preset and repaints the window.
pub fn picker(theme: &Theme) -> impl IntoElement {
    let device = emulation::device();
    div()
        .id("browser-device")
        .h(px(24.0))
        .px(px(8.0))
        .flex_none()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(theme.border)
        .cursor_pointer()
        .hover(|s| s.bg(crate::theme::ink(0.06)))
        .child(icon(glyph(device)).size(px(14.0)).text_color(theme.text_muted))
        .child(
            div()
                .text_size(crate::typography::ui_rems(12.0))
                .text_color(theme.text)
                .whitespace_nowrap()
                .child(SharedString::from(device.label())),
        )
        .on_click(|_, window, _| {
            let next = emulation::cycle();
            println!("browser-device: {}", next.key());
            window.refresh();
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_has_a_glyph_and_a_short_label() {
        for d in emulation::ALL {
            assert!(!glyph(d).is_empty());
            // The control is at most about 140px wide at 12px type.
            assert!(d.label().len() <= 10, "{}", d.label());
        }
    }
}
