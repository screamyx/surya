//! Zero-copy frames on Windows.
//!
//! With `SURYA_BROWSER_ZERO_COPY=1` CEF paints into a D3D11 texture in its
//! GPU process and calls `on_accelerated_paint` instead of `on_paint`. This
//! module copies that texture on the GPU into one this crate owns (see
//! `d3d11.rs` for why a copy, and why a fresh one per frame) and hands it to
//! gpui as an `ExternalTexture`, which the fork's DirectX renderer blits into
//! the swap chain. No pixel touches the CPU.
//!
//! The device is made before the first browser ([`ready`]); only if that
//! worked does the browser get `shared_texture_enabled`, so a box where D3D11
//! sharing is not available keeps the CPU path with the flag set. Once a
//! browser has shared textures CEF never calls `on_paint` for it again, so a
//! frame that fails later is dropped and the pane keeps its last good frame;
//! a device that dies is retired and counted, and the pane keeps that frame
//! until the browser is reopened.
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

/// Accelerated paints CEF delivered.
static ACCEL: AtomicU64 = AtomicU64::new(0);
/// Of those, copied and published.
static COPIED: AtomicU64 = AtomicU64::new(0);
/// Dropped by a D3D11 error on that frame; the first five and every 300th
/// are printed.
static FAILED: AtomicU64 = AtomicU64::new(0);
/// Dropped because the device is gone (retired after a fatal error).
static DEAD: AtomicU64 = AtomicU64::new(0);
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
        "zero_copy=on accel={} copied={n} failed={} dead={} \
         gpu_ms last={last:.2} avg={avg:.2} max={max:.2} adapter=\"{adapter}\"",
        ACCEL.load(Ordering::Relaxed),
        FAILED.load(Ordering::Relaxed),
        DEAD.load(Ordering::Relaxed),
    )
}

/// The device, made once by [`ready`]. `Err` after a fatal error: the
/// device is retired and not remade (a reopened browser starts over).
#[cfg(windows)]
static DEVICE: Mutex<Option<Result<d3d11::Device, String>>> = Mutex::new(None);

/// Whether a browser made now may use shared textures: the switch is on and
/// this crate's D3D11 device exists. Makes the device on the first call, on
/// the main thread, before any browser. Off Windows: never.
#[cfg(windows)]
pub(crate) fn ready() -> bool {
    if !enabled() {
        return false;
    }
    let Ok(mut device) = DEVICE.lock() else { return false };
    if device.is_none() {
        let made = d3d11::Device::new();
        match &made {
            Ok(d) => {
                println!("browser: zero_copy device on \"{}\" luid={:?}", d.name(), d.luid());
                if let Ok(mut a) = ADAPTER.lock() {
                    *a = Some(d.name().to_string());
                }
            }
            Err(e) => println!("browser: zero_copy device failed, browsers take the CPU path: {e}"),
        }
        *device = Some(made);
    }
    matches!(device.as_ref(), Some(Ok(_)))
}

#[cfg(not(windows))]
pub(crate) fn ready() -> bool {
    false
}

/// Turn one accelerated paint into a texture gpui can draw. Runs inside
/// CEF's callback on the main thread; everything it touches on the pooled
/// texture is finished before it returns, as `cef_render_handler.h` demands.
/// `None` drops the frame and the pane keeps the last one.
#[cfg(windows)]
pub(crate) fn on_accelerated_paint(info: &cef::AcceleratedPaintInfo) -> Option<gpui::ExternalTexture> {
    let n = ACCEL.fetch_add(1, Ordering::Relaxed) + 1;
    // A clone (COM refcount) out of the lock: the GPU wait below must not
    // hold the lock, and `ready` may be asked from a tab opening meanwhile.
    let device = match DEVICE.lock().ok()?.as_ref() {
        Some(Ok(d)) => d.clone(),
        _ => {
            DEAD.fetch_add(1, Ordering::Relaxed);
            return None;
        }
    };

    let handle: *mut std::ffi::c_void = info.shared_texture_handle;
    if handle.is_null() {
        FAILED.fetch_add(1, Ordering::Relaxed);
        return None;
    }
    let r = &info.extra.visible_rect;
    let visible = [r.x, r.y, r.width, r.height];
    static PROBE: OnceLock<bool> = OnceLock::new();
    let probe = *PROBE.get_or_init(|| std::env::var_os("SURYA_BROWSER_ZERO_COPY_PROBE").is_some());
    match device.snapshot(handle, visible, probe) {
        Ok(snap) => {
            let us = snap.took.as_micros() as u64;
            COPIED.fetch_add(1, Ordering::Relaxed);
            COPY_LAST_US.store(us, Ordering::Relaxed);
            COPY_TOTAL_US.fetch_add(us, Ordering::Relaxed);
            COPY_MAX_US.fetch_max(us, Ordering::Relaxed);
            if n <= 3 || n % 60 == 0 {
                let [open, create, submit, wait] = snap.split;
                let probe = snap.probe_wait_us.map(|p| format!(" probe_wait {p}us")).unwrap_or_default();
                println!(
                    "browser: accelerated_paint #{n}: {}x{} of {}x{}, gpu copy {:.2}ms \
                     (open {open}us create {create}us submit {submit}us wait {wait}us{probe}); {}",
                    snap.width,
                    snap.height,
                    info.extra.coded_size.width,
                    info.extra.coded_size.height,
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
        Err(e) if e.fatal => {
            DEAD.fetch_add(1, Ordering::Relaxed);
            println!("browser: zero_copy device lost at paint #{n}, retired; the pane keeps its last frame: {e}");
            if let Ok(mut slot) = DEVICE.lock() {
                *slot = Some(Err(e.msg));
            }
            None
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
