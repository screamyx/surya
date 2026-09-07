//! The page's pixels as a gpui element.
//!
//! A `canvas`, because it is the only element that both knows its final
//! bounds and can paint an image without going through an `img` asset
//! source. The paint closure also feeds the element's size and DPI back to
//! CEF, which is what makes the pane resizable.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cef::ImplBrowserHost as _;
use gpui::{canvas, div, prelude::*, px, Corners, HitboxBehavior, RenderImage, Window};

use crate::render::{FrameSource, SCALE, VIEW_H, VIEW_W};

static RESIZES: AtomicU64 = AtomicU64::new(0);
static UIPAINTS: AtomicU64 = AtomicU64::new(0);

/// Where the surface sits in the window, in DIPs, for the input path. CEF's
/// own view starts at 0,0 and the pane does not.
static ORIGIN: Mutex<(f32, f32)> = Mutex::new((0.0, 0.0));

pub fn surface_origin() -> (f32, f32) {
    ORIGIN.lock().map(|o| *o).unwrap_or((0.0, 0.0))
}

/// Per-frame upload cost: the `paint_image` call on a frame gpui has not
/// seen yet, which is where the atlas upload happens.
static UPLOAD_N: AtomicU64 = AtomicU64::new(0);
static UPLOAD_LAST_US: AtomicU64 = AtomicU64::new(0);
static UPLOAD_TOTAL_US: AtomicU64 = AtomicU64::new(0);
static UPLOAD_MAX_US: AtomicU64 = AtomicU64::new(0);

/// `(n, last_ms, avg_ms, max_ms)`.
pub(crate) fn upload_stats() -> (u64, f64, f64, f64) {
    let n = UPLOAD_N.load(Ordering::Relaxed);
    let total = UPLOAD_TOTAL_US.load(Ordering::Relaxed) as f64;
    (
        n,
        UPLOAD_LAST_US.load(Ordering::Relaxed) as f64 / 1000.0,
        if n == 0 { 0.0 } else { total / n as f64 / 1000.0 },
        UPLOAD_MAX_US.load(Ordering::Relaxed) as f64 / 1000.0,
    )
}

/// Tell CEF its view changed size, then make it produce a frame at it.
/// CEF only paints damage and a resize is not damage: `was_hidden(1)` then
/// `was_hidden(0)` forces a full repaint (haktui, 2026-08-26).
fn notify_resized() {
    let id = crate::tabs::active_browser();
    crate::cef_thread::on_ui(move || {
        let Some(host) = crate::client::host_of(id) else { return };
        host.was_resized();
        host.was_hidden(1);
        host.was_hidden(0);
    });
}

/// The last frame on screen, so the one before it can leave the atlas.
static LAST: Mutex<Option<(u64, Arc<RenderImage>)>> = Mutex::new(None);

/// The surface plus every listener that carries a person's mouse and
/// keyboard into the page. Clicking it takes the keyboard, as clicking a page
/// in Chrome does; `focus` is the pane's handle for that.
pub fn panel(focus: &gpui::FocusHandle) -> gpui::AnyElement {
    use gpui::{InteractiveElement as _, MouseButton};
    if crate::disabled() {
        return surface();
    }
    let taker = focus.clone();
    let mut panel = div()
        .id("browser-surface")
        .size_full()
        .track_focus(focus)
        .on_any_mouse_down(move |event, window, cx| {
            if !taker.is_focused(window) {
                window.focus(&taker, cx);
                crate::events::set_focus(true);
            }
            crate::events::mouse_down(event);
        })
        .on_mouse_move(|event, _, _| crate::events::mouse_moved(event.position, &event.modifiers))
        .on_scroll_wheel(|event, _, _| crate::events::scroll(event))
        .on_key_down(|event, _, cx| {
            // A chord the pane acted on itself, F5 or alt+left, must not also
            // reach the page.
            if crate::events::key_down(event) {
                cx.stop_propagation();
            }
        })
        .on_key_up(|event, _, _| crate::events::key_up(event));
    for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
        panel = panel
            .on_mouse_up(button, |event, _, _| crate::events::mouse_up(event))
            // A drag that ends outside the pane still has to end, or the page
            // keeps the button down forever.
            .on_mouse_up_out(button, |event, _, _| crate::events::mouse_up(event));
    }
    panel.child(surface()).into_any_element()
}

/// The page pixels, filling whatever bounds the parent gives them.
pub fn surface() -> gpui::AnyElement {
    if let Some(off) = crate::off_reason() {
        return placeholder(&crate::off::note(off)).into_any_element();
    }
    canvas(
        // The page area as a hitbox, so the page's own pointer shape applies
        // there and nowhere else: off it, gpui shows the shell's.
        |bounds, window: &mut Window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
        |bounds, hitbox, window: &mut Window, _cx: &mut gpui::App| {
            UIPAINTS.fetch_add(1, Ordering::Relaxed);
            window.set_cursor_style(crate::cursor::active(), &hitbox);
            let w = (f32::from(bounds.size.width).round() as i32).max(1);
            let h = (f32::from(bounds.size.height).round() as i32).max(1);
            let sf = (window.scale_factor() * 1000.0).round() as u32;
            if let Ok(mut o) = ORIGIN.lock() {
                *o = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            }
            crate::events::set_origin(
                f32::from(bounds.origin.x).round() as i32,
                f32::from(bounds.origin.y).round() as i32,
            );
            // Three separate swaps, then the OR: `||` short-circuits and a
            // DPI change on an unchanged size would be dropped.
            let dw = VIEW_W.swap(w, Ordering::AcqRel) != w;
            let dh = VIEW_H.swap(h, Ordering::AcqRel) != h;
            let ds = SCALE.swap(sf, Ordering::AcqRel) != sf;
            if dw || dh || ds {
                let n = RESIZES.fetch_add(1, Ordering::Relaxed) + 1;
                println!("browser: view -> {w}x{h} scale={} resizes={n}", sf as f32 / 1000.0);
                notify_resized();
            }
            let Some((seq, src)) = crate::render::frame() else { return };
            let img = match src {
                FrameSource::Cpu(img) => img,
                // Already on the GPU: the fork's renderer opens the texture
                // by handle and blits it. Nothing to upload, nothing to drop.
                #[cfg(windows)]
                FrameSource::Shared(texture) => {
                    window.paint_external_texture(bounds, &texture);
                    return;
                }
            };
            let fresh = LAST.lock().ok().map(|l| l.as_ref().map(|(s, _)| *s) != Some(seq)).unwrap_or(true);
            let t0 = Instant::now();
            // comet's fork (e2ddcc6): `paint_image(bounds, radii, image, frame, grayscale)`;
            // haktui's gpui took a second `image_bounds` here.
            let _ = window.paint_image(bounds, Corners::default(), img.clone(), 0, false);
            if fresh {
                let us = t0.elapsed().as_micros() as u64;
                UPLOAD_N.fetch_add(1, Ordering::Relaxed);
                UPLOAD_LAST_US.store(us, Ordering::Relaxed);
                UPLOAD_TOTAL_US.fetch_add(us, Ordering::Relaxed);
                UPLOAD_MAX_US.fetch_max(us, Ordering::Relaxed);
            }
            // The previous paint's texture leaves the atlas once a newer one
            // is on screen; without this every CEF frame stays uploaded.
            let prev = LAST.lock().ok().and_then(|mut last| match last.as_ref() {
                Some((s, _)) if *s == seq => None,
                _ => last.replace((seq, img)).map(|(_, p)| p),
            });
            if let Some(prev) = prev {
                let _ = window.drop_image(prev);
            }
        },
    )
    .size_full()
    .into_any_element()
}

/// What the pane shows when there is no browser in it. The words come from
/// `off.rs`, which knows which of the three reasons this is.
fn placeholder(note: &crate::off::Note) -> gpui::Div {
    let mut pane = div()
        .size_full()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(14.0))
        .text_size(px(12.0))
        .child(div().child(note.label))
        .child(div().opacity(0.6).child(note.line));
    // The way out sits under the sentence, quieter again, so the eye reads
    // what happened before it reads what to do about it.
    if let Some(hint) = note.hint {
        pane = pane.child(div().opacity(0.4).child(hint));
    }
    pane
}
