//! The page's `prefers-color-scheme`, mid-session.
//!
//! At start-up `cef_app.rs` passes `force-dark-mode` on Chromium's command
//! line, which is start-up only. When the app's appearance changes while it
//! runs (the OS flips at sunset, or the owner picks a theme), every open
//! tab is told through `Emulation.setEmulatedMedia`, the DevTools call
//! Chrome's own rendering panel uses for the same switch (critique round 4,
//! R1 follow-up). A browser opened after the change gets the current answer
//! on its first load end, once; each tab is its own DevTools target.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;

use serde_json::json;

use crate::{ColorScheme, devtools};

/// 0 = never set (the start-up switch rules), 1 = light, 2 = dark.
static WANTED: AtomicU8 = AtomicU8::new(0);
/// Browsers that have heard the current answer, by CEF id.
static APPLIED: Mutex<Vec<i32>> = Mutex::new(Vec::new());

fn wanted() -> Option<ColorScheme> {
    match WANTED.load(Ordering::Acquire) {
        1 => Some(ColorScheme::Light),
        2 => Some(ColorScheme::Dark),
        _ => None,
    }
}

/// The `Emulation.setEmulatedMedia` params for a scheme.
pub fn params(scheme: ColorScheme) -> serde_json::Value {
    let value = match scheme {
        ColorScheme::Light => "light",
        ColorScheme::Dark => "dark",
    };
    json!({ "features": [{ "name": "prefers-color-scheme", "value": value }] })
}

/// The app's appearance changed: tell every open tab. Idempotent; a repeat
/// of the current scheme sends nothing.
pub fn set_color_scheme(scheme: ColorScheme) {
    let code = match scheme {
        ColorScheme::Light => 1,
        ColorScheme::Dark => 2,
    };
    if WANTED.swap(code, Ordering::AcqRel) == code {
        return;
    }
    println!("scheme: prefers-color-scheme -> {scheme:?}");
    if crate::disabled() {
        return;
    }
    let open = devtools::browsers();
    if let Ok(mut applied) = APPLIED.lock() {
        applied.clear();
        applied.extend(open.iter().copied());
    }
    for id in open {
        devtools::fire(id, "Emulation.setEmulatedMedia", params(scheme));
    }
}

/// A main frame loaded in browser `id`: a browser made after the switch
/// has never heard it. Only that browser is told.
pub(crate) fn on_load_end(id: i32) {
    let Some(scheme) = wanted() else { return };
    let Ok(mut applied) = APPLIED.lock() else { return };
    if applied.contains(&id) {
        return;
    }
    applied.push(id);
    drop(applied);
    devtools::fire(id, "Emulation.setEmulatedMedia", params(scheme));
}

pub(crate) fn forget(id: i32) {
    if let Ok(mut applied) = APPLIED.lock() {
        applied.retain(|b| *b != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_name_the_media_feature() {
        assert_eq!(params(ColorScheme::Dark)["features"][0]["name"], "prefers-color-scheme");
        assert_eq!(params(ColorScheme::Dark)["features"][0]["value"], "dark");
        assert_eq!(params(ColorScheme::Light)["features"][0]["value"], "light");
    }

    #[test]
    fn a_closed_browser_is_forgotten_and_a_load_end_marks_only_that_one() {
        APPLIED.lock().unwrap().clear();
        WANTED.store(2, Ordering::Release);
        // No browser 41 is observed, so the fire fails quietly; the mark is
        // what this checks.
        on_load_end(41);
        on_load_end(41);
        assert_eq!(APPLIED.lock().unwrap().iter().filter(|b| **b == 41).count(), 1);
        forget(41);
        assert!(!APPLIED.lock().unwrap().contains(&41));
        WANTED.store(0, Ordering::Release);
    }
}
