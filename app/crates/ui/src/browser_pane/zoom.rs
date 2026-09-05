//! Page zoom: the chords, the per-host memory, and the reading in the bar.
//!
//! CEF's zoom is a continuous level where the scale is `1.2^level`. The pane
//! steps along Chrome's own ladder of percentages instead (see
//! [`state::ZOOM_LADDER`]) and converts at the edge, so the bar reads 125%
//! rather than the 131.5% half-level stepping would land on.

use gpui::Context;

use super::{backend, keys, state, BrowserPane};
use crate::settings::SavePolicy;

impl BrowserPane {
    /// A zoom the person asked for: apply it, remember it against the host,
    /// and write the memory to `ui-settings.json`.
    pub(super) fn step_zoom(&mut self, percent: u32, cx: &mut Context<Self>) {
        self.zoom_percent = percent;
        backend::set_zoom(state::zoom_level(percent));
        keys::zoomed();
        let url = backend::page().url;
        if self.zoom.set(&url, percent) {
            let saved = self.zoom.as_settings();
            crate::settings::update(SavePolicy::Debounced, cx, |settings| {
                settings.browser_zoom = saved;
            });
        }
        self.zoom_url = url;
        keys::report(&format!("zoom {percent}%"));
        cx.notify();
    }

    /// The page moved to another address: pick that host's remembered zoom
    /// up, as Chrome does. Called from render, which runs every frame, so it
    /// returns immediately unless the address actually changed.
    pub(super) fn follow_page_zoom(&mut self, url: &str, cx: &mut Context<Self>) {
        if url.is_empty() || url == self.zoom_url {
            return;
        }
        self.zoom_url = url.to_string();
        let percent = self.zoom.get(url);
        if percent != self.zoom_percent {
            self.zoom_percent = percent;
            backend::set_zoom(state::zoom_level(percent));
            cx.notify();
        }
    }
}
