//! CEF's render handler: the view size it asks for, and the frames it paints.
//!
//! Linux takes CPU pixels through `on_paint`: one BGRA copy into a
//! `RenderImage` per paint, on CEF's thread, so the render that shows it
//! uploads once and copies nothing. Windows can take the GPU texture instead
//! through `on_accelerated_paint` (`zero_copy`, behind
//! `SURYA_BROWSER_ZERO_COPY=1`); the Mac IOSurface path is not ported.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cef::rc::Rc as _;
use cef::*;
use gpui::RenderImage;

/// Live view size in DIPs, written by the surface element each frame and
/// read by CEF through `view_rect`.
pub(crate) static VIEW_W: AtomicI32 = AtomicI32::new(800);
pub(crate) static VIEW_H: AtomicI32 = AtomicI32::new(600);
/// Device scale, times 1000.
pub(crate) static SCALE: AtomicU32 = AtomicU32::new(1000);

static PAINTS: AtomicU64 = AtomicU64::new(0);
static NEXT_FRAME: AtomicU64 = AtomicU64::new(1);
static VIEW_RECT_ASKS: AtomicU64 = AtomicU64::new(0);
/// Last painted frame size, `w << 32 | h`, in device pixels.
static LAST_SIZE: AtomicU64 = AtomicU64::new(0);

/// Where a frame's pixels are: in a `RenderImage` the element uploads once
/// (the CPU path), or already on the GPU as a texture gpui's renderer opens
/// by handle (the zero-copy path, Windows only).
#[derive(Clone)]
pub(crate) enum FrameSource {
    Cpu(Arc<RenderImage>),
    #[cfg(windows)]
    Shared(gpui::ExternalTexture),
}

/// A CEF frame, built once in the callback. `seq` says which paint it is,
/// so the element can drop the previous texture.
struct FrameBuf {
    seq: u64,
    src: FrameSource,
    arrived_us: u64,
}
/// The latest frame of every browser, by CEF identifier. A parked tab keeps
/// its last frame, so switching back to it shows something at once. Bounded
/// to [`KEPT_FRAMES`] entries: beyond that the oldest parked frame goes
/// (a frame is about 14 MB at 2560x1440, and a tab that is shown again
/// repaints within a frame anyway).
static FRAMES: Mutex<Option<HashMap<i32, FrameBuf>>> = Mutex::new(None);
const KEPT_FRAMES: usize = 8;
/// Paints by browsers that were not on screen (parked, or created and not
/// yet activated). They store a frame and count here, nothing else.
static BACKGROUND_PAINTS: AtomicU64 = AtomicU64::new(0);

/// Frames the active browser delivered so far. The pump's idle detection
/// reads this, so a parked tab's paint must not move it.
pub(crate) fn frames() -> u64 {
    PAINTS.load(Ordering::Relaxed)
}

pub(crate) fn background_paints() -> u64 {
    BACKGROUND_PAINTS.load(Ordering::Relaxed)
}

/// The active tab's latest frame.
pub(crate) fn frame() -> Option<(u64, FrameSource, u64)> {
    drain_frames();
    frame_of(crate::tabs::active_browser())
}

fn frame_of(browser: i32) -> Option<(u64, FrameSource, u64)> {
    if browser == 0 {
        return None;
    }
    FRAMES.lock().ok()?.as_ref()?.get(&browser).map(|f| (f.seq, f.src.clone(), f.arrived_us))
}

/// Only the gpui thread consumes owned paint messages. Close callbacks can
/// overtake a queued paint, so never reinstall a frame for a closed browser.
pub(crate) fn drain_frames() {
    if !crate::cef_thread::threaded() { return; }
    let started = Instant::now();
    let active = crate::tabs::active_browser();
    for frame in crate::frame_handoff::drain() {
        if crate::client::has_browser(frame.browser) {
            store_at(frame.browser, frame.seq, frame.source, active, frame.arrived_us);
            // A close can race the membership check above. If its forget
            // ran before our store, finish that removal again; if it runs
            // after this check, the close callback removes our store.
            if !crate::client::has_browser(frame.browser) { forget(frame.browser); }
        }
    }
    crate::frame_timing::main_handoff(started.elapsed());
}

fn publish(browser: i32, src: FrameSource, arrived_us: u64) {
    let seq = NEXT_FRAME.fetch_add(1, Ordering::Relaxed);
    if crate::cef_thread::threaded() {
        crate::frame_handoff::publish(crate::frame_handoff::PendingFrame {
            browser, seq, source: src, arrived_us,
        });
    } else {
        store_at(browser, seq, src, crate::tabs::active_browser(), arrived_us);
    }
}

/// Keep `browser`'s latest frame, dropping the oldest other one past the
/// bound. `active` is never the one dropped.
fn store_at(browser: i32, seq: u64, src: FrameSource, active: i32, arrived_us: u64) {
    let Ok(mut guard) = FRAMES.lock() else { return };
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(browser, FrameBuf { seq, src, arrived_us });
    while map.len() > KEPT_FRAMES {
        let oldest = map
            .iter()
            .filter(|(id, _)| **id != active && **id != browser)
            .min_by_key(|(_, f)| f.seq)
            .map(|(id, _)| *id);
        match oldest {
            Some(id) => {
                map.remove(&id);
            }
            None => break,
        }
    }
}

/// The browser is gone; so is its frame.
pub(crate) fn forget(browser: i32) {
    if let Ok(mut guard) = FRAMES.lock()
        && let Some(map) = guard.as_mut()
    {
        map.remove(&browser);
    }
}

/// Every frame goes: CEF is shutting down and `on_before_close` may not
/// have reached every browser in time.
pub(crate) fn forget_all() {
    if let Ok(mut guard) = FRAMES.lock() {
        *guard = None;
    }
}

pub(crate) fn kept_frames() -> usize {
    FRAMES.lock().ok().and_then(|g| g.as_ref().map(|m| m.len())).unwrap_or(0)
}

pub(crate) fn last_size() -> (i32, i32) {
    let packed = LAST_SIZE.load(Ordering::Relaxed);
    ((packed >> 32) as i32, packed as u32 as i32)
}

/// The callback's own cost: `to_vec` plus the `RenderImage`, per paint.
static COPY_N: AtomicU64 = AtomicU64::new(0);
static COPY_LAST_US: AtomicU64 = AtomicU64::new(0);
static COPY_TOTAL_US: AtomicU64 = AtomicU64::new(0);
static COPY_MAX_US: AtomicU64 = AtomicU64::new(0);

/// `(n, last_ms, avg_ms, max_ms)`.
pub(crate) fn copy_stats() -> (u64, f64, f64, f64) {
    let n = COPY_N.load(Ordering::Relaxed);
    let total = COPY_TOTAL_US.load(Ordering::Relaxed) as f64;
    (
        n,
        COPY_LAST_US.load(Ordering::Relaxed) as f64 / 1000.0,
        if n == 0 { 0.0 } else { total / n as f64 / 1000.0 },
        COPY_MAX_US.load(Ordering::Relaxed) as f64 / 1000.0,
    )
}

wrap_render_handler! {
    struct Render;
    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            if let Some(r) = rect {
                r.x = 0;
                r.y = 0;
                r.width = VIEW_W.load(Ordering::Acquire).max(1);
                r.height = VIEW_H.load(Ordering::Acquire).max(1);
                let n = VIEW_RECT_ASKS.fetch_add(1, Ordering::Relaxed) + 1;
                if n <= 3 {
                    println!("browser: view_rect #{n} -> {}x{}", r.width, r.height);
                }
            }
        }

        fn screen_info(
            &self,
            _browser: Option<&mut Browser>,
            screen_info: Option<&mut ScreenInfo>,
        ) -> ::std::os::raw::c_int {
            let Some(si) = screen_info else { return 0 };
            si.device_scale_factor = SCALE.load(Ordering::Acquire) as f32 / 1000.0;
            si.rect = Rect {
                x: 0,
                y: 0,
                width: VIEW_W.load(Ordering::Acquire).max(1),
                height: VIEW_H.load(Ordering::Acquire).max(1),
            };
            si.available_rect = si.rect.clone();
            1
        }

        fn on_paint(
            &self,
            browser: Option<&mut Browser>,
            type_: PaintElementType,
            _dirty: Option<&[Rect]>,
            buffer: *const u8,
            width: ::std::os::raw::c_int,
            height: ::std::os::raw::c_int,
        ) {
            // A dropdown's popup widget paints through the same callback at
            // its own size; it is not the page.
            if type_ != PaintElementType::VIEW {
                return;
            }
            if buffer.is_null() || width <= 0 || height <= 0 {
                return;
            }
            // A browser no tab owns never gets a frame slot (0 is nobody).
            let id = browser.map(|b| b.identifier()).unwrap_or(0);
            if id == 0 {
                return;
            }
            let active = crate::tabs::active_browser();
            let arrived_us = crate::frame_timing::now_us();
            let t0 = Instant::now();
            let len = (width as usize) * (height as usize) * 4;
            // SAFETY: CEF owns `buffer` for the duration of the callback and
            // it holds `width * height` BGRA pixels.
            let bytes = unsafe { std::slice::from_raw_parts(buffer, len) };
            // CEF's buffer is BGRA, which is what `RenderImage` holds.
            let Some(buf): Option<image::RgbaImage> =
                image::ImageBuffer::from_raw(width as u32, height as u32, bytes.to_vec())
            else {
                return;
            };
            let img = Arc::new(RenderImage::new(vec![image::Frame::new(buf)]));
            if id != active {
                // Parked, or created and not activated yet: keep the frame
                // for when it is shown, count it apart, move nothing else.
                BACKGROUND_PAINTS.fetch_add(1, Ordering::Relaxed);
                publish(id, FrameSource::Cpu(img), arrived_us);
                return;
            }
            let n = PAINTS.fetch_add(1, Ordering::Relaxed) + 1;
            crate::perf::on_paint();
            publish(id, FrameSource::Cpu(img), arrived_us);
            LAST_SIZE.store(((width as u64) << 32) | (height as u32 as u64), Ordering::Relaxed);
            let us = t0.elapsed().as_micros() as u64;
            COPY_N.fetch_add(1, Ordering::Relaxed);
            COPY_LAST_US.store(us, Ordering::Relaxed);
            COPY_TOTAL_US.fetch_add(us, Ordering::Relaxed);
            COPY_MAX_US.fetch_max(us, Ordering::Relaxed);
            if n <= 3 || n % 60 == 0 {
                println!("browser: on_paint #{n}: {width}x{height}, {len} bytes, copy {:.2}ms", us as f64 / 1000.0);
            }
            dump_frame(n, width as u32, height as u32, bytes);
        }

        /// The GPU twin of `on_paint`, called instead of it when the browser
        /// was made with `shared_texture_enabled`. The texture behind `info`
        /// is CEF's for the duration of this call only; `zero_copy` copies it
        /// on the GPU before returning. Same bookkeeping as `on_paint`: no
        /// slot for browser 0, a parked browser's paint is stored and counted
        /// apart and moves nothing else.
        fn on_accelerated_paint(
            &self,
            browser: Option<&mut Browser>,
            type_: PaintElementType,
            _dirty: Option<&[Rect]>,
            info: Option<&AcceleratedPaintInfo>,
        ) {
            if type_ != PaintElementType::VIEW {
                return;
            }
            let id = browser.map(|b| b.identifier()).unwrap_or(0);
            if id == 0 {
                return;
            }
            #[cfg(windows)]
            if let Some(info) = info {
                let arrived_us = crate::frame_timing::now_us();
                let Some(texture) = crate::zero_copy::on_accelerated_paint(info) else { return };
                let active = crate::tabs::active_browser();
                if id != active {
                    BACKGROUND_PAINTS.fetch_add(1, Ordering::Relaxed);
                    publish(id, FrameSource::Shared(texture), arrived_us);
                    return;
                }
                // The texture is the visible part of the paint, not its coded size.
                let (width, height) = texture.size();
                PAINTS.fetch_add(1, Ordering::Relaxed);
                crate::perf::on_paint();
                publish(id, FrameSource::Shared(texture), arrived_us);
                LAST_SIZE.store(((width as u64) << 32) | (height as u32 as u64), Ordering::Relaxed);
            }
            #[cfg(not(windows))]
            {
                let _ = info;
            }
        }
    }
}

/// `SURYA_BROWSER_DUMP=<dir>`: write paints 1, 2, 3 and then every 30th as
/// PNG, for a picture of what CEF produced on a box with no display to
/// screenshot. BGRA in, RGBA on disk.
fn dump_frame(n: u64, width: u32, height: u32, bgra: &[u8]) {
    static DIR: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    let Some(dir) = DIR.get_or_init(|| std::env::var_os("SURYA_BROWSER_DUMP").map(Into::into)) else {
        return;
    };
    if !(n <= 3 || n % 30 == 0) {
        return;
    }
    let mut rgba = bgra.to_vec();
    for px in rgba.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join(format!("frame-{n:04}-{width}x{height}.png"));
    match image::save_buffer(&path, &rgba, width, height, image::ColorType::Rgba8) {
        Ok(()) => println!("browser: dumped {}", path.display()),
        Err(e) => println!("browser: dump failed {e}"),
    }
}

pub(crate) fn render_handler() -> RenderHandler {
    Render::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img() -> FrameSource {
        let buf: image::RgbaImage = image::ImageBuffer::from_raw(1, 1, vec![0, 0, 0, 255]).unwrap();
        FrameSource::Cpu(Arc::new(RenderImage::new(vec![image::Frame::new(buf)])))
    }

    #[test]
    fn frames_are_kept_per_browser_and_bounded_without_dropping_the_active_one() {
        forget_all();
        let active = 1;
        for id in 1..=(KEPT_FRAMES as i32 + 3) {
            store_at(id, id as u64, img(), active, 0);
        }
        assert_eq!(kept_frames(), KEPT_FRAMES);
        assert!(frame_of(active).is_some(), "the active frame is never the one dropped");
        assert!(frame_of(2).is_none(), "the oldest parked frame went first");
        assert!(frame_of(KEPT_FRAMES as i32 + 3).is_some());
        assert!(frame_of(0).is_none(), "0 names no browser");
        forget(active);
        assert!(frame_of(active).is_none());
        forget_all();
        assert_eq!(kept_frames(), 0);
    }
}
