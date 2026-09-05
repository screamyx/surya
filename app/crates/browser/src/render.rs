//! CEF's render handler: the view size it asks for, and the frames it paints.
//!
//! Linux takes CPU pixels through `on_paint`: one BGRA copy into a
//! `RenderImage` per paint, on CEF's thread, so the render that shows it
//! uploads once and copies nothing. The shared-texture paths (Windows D3D11,
//! Mac IOSurface) need haktui's gpui patch and are not ported.

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
static VIEW_RECT_ASKS: AtomicU64 = AtomicU64::new(0);
/// Last painted frame size, `w << 32 | h`, in device pixels.
static LAST_SIZE: AtomicU64 = AtomicU64::new(0);

/// A CEF frame: already a `RenderImage`, built once in the callback. `seq`
/// says which paint it is, so the element can drop the previous texture.
struct FrameBuf {
    seq: u64,
    img: Arc<RenderImage>,
}
static FRAME: Mutex<Option<FrameBuf>> = Mutex::new(None);

/// Frames CEF delivered so far.
pub(crate) fn frames() -> u64 {
    PAINTS.load(Ordering::Relaxed)
}

pub(crate) fn frame() -> Option<(u64, Arc<RenderImage>)> {
    FRAME.lock().ok()?.as_ref().map(|f| (f.seq, f.img.clone()))
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
            _browser: Option<&mut Browser>,
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
            let n = PAINTS.fetch_add(1, Ordering::Relaxed) + 1;
            crate::perf::on_paint();
            if let Ok(mut slot) = FRAME.lock() {
                *slot = Some(FrameBuf { seq: n, img });
            }
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
