//! Zero-copy frames on Windows.
//!
//! With `SURYA_BROWSER_ZERO_COPY=1` CEF paints into a D3D11 texture in its
//! GPU process and calls `on_accelerated_paint` instead of `on_paint`. This
//! module copies that texture on the GPU into one this crate owns (see
//! `d3d11.rs` for why a copy, and why a fresh one per frame) and hands it to
//! gpui as an `ExternalTexture`, which the fork's DirectX renderer blits into
//! the swap chain. No pixel touches the CPU.
//!
//! A frame is published only once its GPU copy has finished (the fork's
//! `ProducerComplete` contract). Most copies finish inside the callback's
//! 1.5 ms budget; the rest wait in a short pending queue and are published
//! by the next callback or by the surface element's next paint, whichever
//! comes first. CEF's callback runs on gpui's main thread, so it never waits
//! on the GPU longer than that budget.
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
/// Published late, from the pending queue, because the GPU had not finished
/// the copy inside the callback's budget.
static DEFERRED: AtomicU64 = AtomicU64::new(0);
/// Dropped before the surface element had told us gpui's adapter, or
/// because a newer frame completed first.
static WAITED: AtomicU64 = AtomicU64::new(0);
/// Dropped by a D3D11 error; the first five and every 300th are printed.
static FAILED: AtomicU64 = AtomicU64::new(0);
static COPY_LAST_US: AtomicU64 = AtomicU64::new(0);
static COPY_TOTAL_US: AtomicU64 = AtomicU64::new(0);
static COPY_MAX_US: AtomicU64 = AtomicU64::new(0);
static ADAPTER: Mutex<Option<String>> = Mutex::new(None);

/// `(n, last_ms, avg_ms, max_ms)` of the callback's own cost (open, create,
/// submit, bounded wait), the number that stands beside
/// `render::copy_stats` for the CPU path.
pub(crate) fn copy_stats() -> (u64, f64, f64, f64) {
    let n = ACCEL.load(Ordering::Relaxed);
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
        "zero_copy=on accel={n} copied={} deferred={} waited={} failed={} \
         gpu_ms last={last:.2} avg={avg:.2} max={max:.2} adapter=\"{adapter}\"",
        COPIED.load(Ordering::Relaxed),
        DEFERRED.load(Ordering::Relaxed),
        WAITED.load(Ordering::Relaxed),
        FAILED.load(Ordering::Relaxed),
    )
}

/// Publish any pending frame whose GPU copy has finished. Called by the
/// surface element on every paint (cheap when the queue is empty), so a
/// frame deferred by the last callback of a burst still reaches the screen.
#[cfg(windows)]
pub(crate) fn publish_ready() {
    if let Ok(device) = win::DEVICE.lock()
        && let Some(Ok(device)) = device.as_ref()
    {
        win::drain(device);
    }
}

#[cfg(not(windows))]
pub(crate) fn publish_ready() {}

/// Turn one accelerated paint into a texture gpui can draw, for `browser`.
/// Runs inside CEF's callback on the main thread; the copy out of the
/// pooled texture is queued before it returns, as `cef_render_handler.h`
/// demands, and the frame is published through `render::store_shared` as
/// soon as that copy has finished.
#[cfg(windows)]
pub(crate) fn on_accelerated_paint(info: &cef::AcceleratedPaintInfo, browser: i32) {
    let n = ACCEL.fetch_add(1, Ordering::Relaxed) + 1;
    let Ok(mut device) = win::DEVICE.lock() else { return };
    if device.is_none() {
        // Until the surface has painted once we do not know gpui's adapter.
        // Drop the frame: the surface's first paint resizes the view, and
        // that resize makes CEF paint again.
        let Some(prefer) = GPU_NAME.lock().ok().and_then(|g| g.clone()) else {
            WAITED.fetch_add(1, Ordering::Relaxed);
            return;
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
    let Some(Ok(device)) = device.as_ref() else {
        FAILED.fetch_add(1, Ordering::Relaxed);
        return;
    };

    let handle: *mut std::ffi::c_void = info.shared_texture_handle;
    if handle.is_null() {
        FAILED.fetch_add(1, Ordering::Relaxed);
        return;
    }
    match device.snapshot(handle) {
        Ok(snap) => {
            let us = snap.took.as_micros() as u64;
            COPY_LAST_US.store(us, Ordering::Relaxed);
            COPY_TOTAL_US.fetch_add(us, Ordering::Relaxed);
            COPY_MAX_US.fetch_max(us, Ordering::Relaxed);
            if n <= 3 || n % 60 == 0 {
                let [open, create, submit, wait] = snap.split;
                println!(
                    "browser: accelerated_paint #{n}: {}x{}, gpu copy {:.2}ms \
                     (open {open}us create {create}us submit {submit}us wait {wait}us) \
                     complete={}; {}",
                    snap.width,
                    snap.height,
                    us as f64 / 1000.0,
                    u8::from(snap.complete()),
                    counters()
                );
            }
            win::enqueue(browser, snap);
            win::drain(device);
        }
        Err(e) => {
            let f = FAILED.fetch_add(1, Ordering::Relaxed) + 1;
            if f <= 5 || f % 300 == 0 {
                println!("browser: accelerated_paint #{n} dropped: {e}");
            }
        }
    }
}

#[cfg(windows)]
mod win {
    use std::collections::VecDeque;
    use std::sync::atomic::Ordering;
    use std::sync::Mutex;

    use super::d3d11::{Device, Snapshot};

    /// Made on the first paint. An error is kept so a broken device is
    /// reported once and not retried sixty times a second.
    pub(super) static DEVICE: Mutex<Option<Result<Device, String>>> = Mutex::new(None);

    /// A frame waiting for its GPU copy to finish: whose it is, the frame,
    /// and whether the callback already had to hand it over unfinished.
    struct Queued {
        browser: i32,
        snap: Snapshot,
        late: bool,
    }

    /// Oldest first.
    static PENDING: Mutex<VecDeque<Queued>> = Mutex::new(VecDeque::new());

    /// More than this many frames in flight means the GPU is far behind;
    /// the oldest is dropped rather than left to pile up.
    const MAX_PENDING: usize = 4;

    pub(super) fn enqueue(browser: i32, snap: Snapshot) {
        let Ok(mut pending) = PENDING.lock() else { return };
        let late = !snap.complete();
        pending.push_back(Queued { browser, snap, late });
        while pending.len() > MAX_PENDING {
            pending.pop_front();
            super::WAITED.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Publish the newest complete frame of each browser and drop the older
    /// complete ones (a later frame supersedes them). Incomplete frames
    /// stay queued. Never blocks on the GPU.
    pub(super) fn drain(device: &Device) {
        let Ok(mut pending) = PENDING.lock() else { return };
        if pending.is_empty() {
            return;
        }
        let mut ready: Vec<Queued> = Vec::new();
        let mut keep: VecDeque<Queued> = VecDeque::new();
        for mut q in pending.drain(..) {
            match device.poll(&mut q.snap) {
                Ok(true) => {
                    if let Some(slot) = ready.iter_mut().find(|r| r.browser == q.browser) {
                        super::WAITED.fetch_add(1, Ordering::Relaxed);
                        *slot = q;
                    } else {
                        ready.push(q);
                    }
                }
                Ok(false) => keep.push_back(q),
                Err(e) => {
                    let f = super::FAILED.fetch_add(1, Ordering::Relaxed) + 1;
                    if f <= 5 || f % 300 == 0 {
                        println!("browser: zero_copy fence failed: {e}");
                    }
                }
            }
        }
        *pending = keep;
        drop(pending);

        for q in ready {
            if q.late {
                super::DEFERRED.fetch_add(1, Ordering::Relaxed);
            }
            super::COPIED.fetch_add(1, Ordering::Relaxed);
            let (width, height) = (q.snap.width, q.snap.height);
            // SAFETY: the handle is an NT handle to a BGRA8 texture of
            // exactly `width` x `height` on the adapter `device.luid()`. Its
            // copy has finished on the GPU (`poll` said so), and nothing
            // writes it again: the next paint gets a new texture.
            let texture = unsafe {
                gpui::ExternalTexture::new(
                    q.snap.handle,
                    device.luid(),
                    (width, height),
                    gpui::ExternalTextureSync::ProducerComplete,
                )
            };
            crate::render::store_shared(q.browser, texture, width as i32, height as i32);
        }
    }
}
