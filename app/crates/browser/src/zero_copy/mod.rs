//! Zero-copy frames on Windows.
//!
//! With `SURYA_BROWSER_ZERO_COPY=1` CEF paints into a D3D11 texture in its
//! GPU process and calls `on_accelerated_paint` instead of `on_paint`. This
//! module copies that texture on the GPU into one this crate owns (see
//! `d3d11.rs` for why a copy, and why a fresh one per frame) and hands it to
//! gpui as an `ExternalTexture`, which the fork's DirectX renderer blits into
//! the swap chain. No pixel touches the CPU.
//!
//! Unset, nothing here runs and the CPU path in `render.rs` is byte for byte
//! what it was. Off Windows the switch is inert.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

#[cfg(windows)]
mod d3d11;

/// `SURYA_BROWSER_ZERO_COPY=1`, read once. Windows only; anywhere else the
/// answer is no whatever the environment says.
pub(crate) fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        cfg!(windows) && std::env::var("SURYA_BROWSER_ZERO_COPY").is_ok_and(|v| v == "1")
    })
}

/// gpui's adapter name (`Window::gpu_specs().device_name`, empty when gpui
/// does not say), stored by the surface element on its first paint so the
/// device below lands on the same GPU as gpui's renderer: a shared texture
/// opens only on the adapter that made it.
static GPU_NAME: Mutex<Option<String>> = Mutex::new(None);

pub(crate) fn set_gpu_name(name: &str) {
    if let Ok(mut slot) = GPU_NAME.lock() {
        if slot.is_none() {
            *slot = Some(name.to_string());
        }
    }
}

/// Accelerated paints CEF delivered.
static ACCEL: AtomicU64 = AtomicU64::new(0);
/// Of those, copied and published.
static COPIED: AtomicU64 = AtomicU64::new(0);
/// Dropped before the surface element had told us gpui's adapter.
static WAITED: AtomicU64 = AtomicU64::new(0);
/// Dropped by a D3D11 error; the first five and every 300th are printed.
static FAILED: AtomicU64 = AtomicU64::new(0);
static COPY_LAST_US: AtomicU64 = AtomicU64::new(0);
static COPY_TOTAL_US: AtomicU64 = AtomicU64::new(0);
static COPY_MAX_US: AtomicU64 = AtomicU64::new(0);
static ADAPTER: Mutex<Option<String>> = Mutex::new(None);

/// `(n, last_ms, avg_ms, max_ms)` of the callback's GPU copy, the number
/// that stands beside `render::copy_stats` for the CPU path.
pub(crate) fn copy_stats() -> (u64, f64, f64, f64) {
    let n = COPIED.load(Ordering::Relaxed);
    let total = COPY_TOTAL_US.load(Ordering::Relaxed) as f64;
    (
        n,
        COPY_LAST_US.load(Ordering::Relaxed) as f64 / 1000.0,
        if n == 0 { 0.0 } else { total / n as f64 / 1000.0 },
        COPY_MAX_US.load(Ordering::Relaxed) as f64 / 1000.0,
    )
}

/// The counters as pairs, for the proof line.
pub fn counters() -> String {
    if !enabled() {
        return "zero_copy=off".to_string();
    }
    let (n, last, avg, max) = copy_stats();
    let adapter = ADAPTER
        .lock()
        .ok()
        .and_then(|a| a.clone())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "zero_copy=on accel={} copied={n} waited={} failed={} \
         gpu_ms last={last:.2} avg={avg:.2} max={max:.2} adapter=\"{adapter}\"",
        ACCEL.load(Ordering::Relaxed),
        WAITED.load(Ordering::Relaxed),
        FAILED.load(Ordering::Relaxed),
    )
}

/// Turn one accelerated paint into a texture gpui can draw. Runs inside
/// CEF's callback on the main thread; everything it touches on the pooled
/// texture is finished before it returns, as `cef_render_handler.h` demands.
#[cfg(windows)]
pub(crate) fn on_accelerated_paint(info: &cef::AcceleratedPaintInfo) -> Option<gpui::ExternalTexture> {
    /// Made on the first paint. An error is kept so a broken device is
    /// reported once and not retried sixty times a second.
    static DEVICE: Mutex<Option<Result<d3d11::Device, String>>> = Mutex::new(None);

    let n = ACCEL.fetch_add(1, Ordering::Relaxed) + 1;
    let mut device = DEVICE.lock().ok()?;
    if device.is_none() {
        // Until the surface has painted once we do not know gpui's adapter.
        // Drop the frame: the surface's first paint resizes the view, and
        // that resize makes CEF paint again.
        let Some(prefer) = GPU_NAME.lock().ok().and_then(|g| g.clone()) else {
            WAITED.fetch_add(1, Ordering::Relaxed);
            return None;
        };
        let made = d3d11::Device::new(if prefer.is_empty() { None } else { Some(&prefer) });
        match &made {
            Ok(d) => {
                println!(
                    "browser: zero_copy device on \"{}\" luid={:?} (gpui reports \"{prefer}\")",
                    d.name(),
                    d.luid()
                );
                if let Ok(mut a) = ADAPTER.lock() {
                    *a = Some(d.name().to_string());
                }
            }
            Err(e) => println!("browser: zero_copy device failed: {e}"),
        }
        *device = Some(made);
    }
    let device = match device.as_ref()? {
        Ok(d) => d,
        Err(_) => {
            FAILED.fetch_add(1, Ordering::Relaxed);
            return None;
        }
    };

    let handle: *mut std::ffi::c_void = info.shared_texture_handle;
    if handle.is_null() {
        FAILED.fetch_add(1, Ordering::Relaxed);
        return None;
    }
    match device.snapshot(handle) {
        Ok(snap) => {
            let us = snap.took.as_micros() as u64;
            COPIED.fetch_add(1, Ordering::Relaxed);
            COPY_LAST_US.store(us, Ordering::Relaxed);
            COPY_TOTAL_US.fetch_add(us, Ordering::Relaxed);
            COPY_MAX_US.fetch_max(us, Ordering::Relaxed);
            if n <= 3 || n % 60 == 0 {
                println!(
                    "browser: accelerated_paint #{n}: {}x{}, gpu copy {:.2}ms; {}",
                    snap.width,
                    snap.height,
                    us as f64 / 1000.0,
                    counters()
                );
            }
            // SAFETY: `snap.handle` is an NT handle to a BGRA8 texture of
            // exactly `width` x `height` on the adapter `device.luid()`. Its
            // copy finished on the GPU inside `snapshot`, and nothing writes
            // it again: the next paint gets a new texture.
            let texture = unsafe {
                gpui::ExternalTexture::new(
                    snap.handle,
                    device.luid(),
                    (snap.width, snap.height),
                    gpui::ExternalTextureSync::ProducerComplete,
                )
            };
            Some(texture)
        }
        Err(e) => {
            let f = FAILED.fetch_add(1, Ordering::Relaxed) + 1;
            if f <= 5 || f % 300 == 0 {
                println!("browser: accelerated_paint #{n} dropped: {e}");
            }
            None
        }
    }
}
