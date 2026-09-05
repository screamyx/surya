//! The Windows half of zero-copy: a D3D11 device of this crate's own and the
//! GPU copy from CEF's pooled texture into a fresh texture this crate owns.
//! No gpui and no cef types here, so the file checks on any host with
//! `cargo check --target x86_64-pc-windows-msvc`.
//!
//! Why a copy at all: CEF 151's `include/cef_render_handler.h` (lines 161 to
//! 167) says the pooled texture "cannot be cached and cannot be accessed
//! outside of this callback. It should be reopened each time this callback
//! is executed and the contents should be copied to a texture owned by the
//! client application." haktui's spike stored the raw handle and opened it a
//! frame later, and every open returned E_INVALIDARG: the handle was already
//! closed, not on the wrong adapter (haktui `docs/handoff-browser-round2-2026-08-25.md`
//! line 56).
//!
//! Why a fresh texture per frame: gpui's renderer may still have GPU reads
//! queued on the last frame's texture after its scene dropped it, and only a
//! keyed mutex or a fence could prove otherwise. A new texture per frame is
//! immutable by construction; its NT handle keeps the allocation alive until
//! the last holder drops it. Slot reuse under a keyed mutex is the next step
//! once this is proven.

use std::ffi::c_void;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::time::{Duration, Instant};

use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HMODULE, S_OK};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11Device1, ID3D11DeviceContext, ID3D11Query,
    ID3D11Texture2D, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BOX,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_QUERY_DESC, D3D11_QUERY_EVENT,
    D3D11_RESOURCE_MISC_SHARED, D3D11_RESOURCE_MISC_SHARED_NTHANDLE, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter, IDXGIAdapter1, IDXGIFactory1, IDXGIResource1,
    DXGI_ADAPTER_DESC1, DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_SHARED_RESOURCE_READ,
    DXGI_SHARED_RESOURCE_WRITE,
};

/// The adapter LUID the way the fork's `ExternalTexture::new` takes it:
/// `(LowPart, HighPart)`.
pub(crate) type Luid = (u32, i32);

/// The callback waits for the GPU to finish reading the pooled texture, with
/// no upper bound: returning earlier would let CEF recycle a texture a
/// queued copy still reads (cef_render_handler.h 161-167, and D3D11 gives
/// no cross-process ordering). Past this much waiting a warning is printed
/// once, so a stalled GPU is visible in the log.
const SLOW_WAIT: Duration = Duration::from_millis(100);

/// This crate's own D3D11 device. One per process, made before the first
/// browser, on the same adapter as gpui's renderer. Clone is a COM refcount
/// bump, so a caller takes a clone out of the lock and never holds the lock
/// across a GPU wait.
#[derive(Clone)]
pub(crate) struct Device {
    device: ID3D11Device,
    device1: ID3D11Device1,
    ctx: ID3D11DeviceContext,
    luid: Luid,
    name: String,
}

/// One frame, copied and finished on the GPU. `handle` is the NT handle gpui
/// opens the texture with; dropping it releases this side's claim on the
/// allocation.
pub(crate) struct Snapshot {
    pub(crate) handle: OwnedHandle,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Wall time of open + create + submit + wait, the callback's own cost.
    pub(crate) took: Duration,
    /// The same, split: open CEF's handle; create the texture and its
    /// handle; submit the copy; wait for the GPU. Microseconds.
    pub(crate) split: [u64; 4],
    /// `SURYA_BROWSER_ZERO_COPY_PROBE=1`: the wait, in microseconds, for a
    /// second copy between two textures of this device alone. If this stays
    /// small while `split[3]` bursts, the burst is the GPU process still
    /// writing the pooled texture, not this device's queue.
    pub(crate) probe_wait_us: Option<u64>,
}

fn err(e: windows::core::Error, what: &str) -> String {
    format!("{what}: {e}")
}

/// What went wrong with one frame. `fatal` means the device itself is gone
/// (a removed or reset device): every later frame would fail the same way,
/// so the caller retires the device and the pane keeps its last good frame.
pub(crate) struct SnapshotError {
    pub(crate) msg: String,
    pub(crate) fatal: bool,
}

impl SnapshotError {
    fn frame(msg: String) -> Self {
        Self { msg, fatal: false }
    }
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

fn adapter_name(desc: &DXGI_ADAPTER_DESC1) -> String {
    String::from_utf16_lossy(&desc.Description)
        .trim_end_matches('\0')
        .trim()
        .to_string()
}

/// The first hardware adapter, in DXGI's enumeration order. That is the
/// adapter gpui's renderer takes too (the fork's `directx_devices.rs`
/// `get_adapter` walks `EnumAdapters(0..)` and keeps the first that makes a
/// D3D11 device), so both halves of the handoff sit on one GPU. Software
/// adapters are skipped: WARP can open nothing the GPU process shares.
fn pick_adapter() -> Result<(IDXGIAdapter1, DXGI_ADAPTER_DESC1), String> {
    let factory: IDXGIFactory1 =
        unsafe { CreateDXGIFactory1() }.map_err(|e| err(e, "CreateDXGIFactory1"))?;
    let mut i = 0;
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(i) } {
        i += 1;
        let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
            continue;
        };
        if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }
        return Ok((adapter, desc));
    }
    Err("no hardware adapter".to_string())
}

impl Device {
    pub(crate) fn new() -> Result<Self, String> {
        let (adapter, desc) = pick_adapter()?;
        let adapter: IDXGIAdapter = adapter.cast().map_err(|e| err(e, "IDXGIAdapter"))?;
        let mut device = None;
        let mut ctx = None;
        unsafe {
            D3D11CreateDevice(
                &adapter,
                D3D_DRIVER_TYPE_UNKNOWN,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut ctx),
            )
        }
        .map_err(|e| err(e, "D3D11CreateDevice"))?;
        let device: ID3D11Device = device.ok_or("D3D11CreateDevice returned no device")?;
        let ctx = ctx.ok_or("D3D11CreateDevice returned no context")?;
        let device1: ID3D11Device1 = device.cast().map_err(|e| err(e, "ID3D11Device1"))?;
        Ok(Self {
            device,
            device1,
            ctx,
            luid: (desc.AdapterLuid.LowPart, desc.AdapterLuid.HighPart),
            name: adapter_name(&desc),
        })
    }

    pub(crate) fn luid(&self) -> Luid {
        self.luid
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Has the GPU passed `fence`? `GetData` answers S_OK once it has and
    /// S_FALSE before that; windows-rs folds S_FALSE into Ok, so this reads
    /// the raw HRESULT through the vtable. Never blocks.
    fn passed(&self, fence: &ID3D11Query) -> Result<bool, String> {
        let code = unsafe {
            (Interface::vtable(&self.ctx).GetData)(
                Interface::as_raw(&self.ctx),
                Interface::as_raw(fence),
                std::ptr::null_mut(),
                0,
                0,
            )
        };
        if code == S_OK {
            Ok(true)
        } else if code.is_err() {
            Err(format!("GetData: {code:?}"))
        } else {
            Ok(false)
        }
    }

    /// Block until the GPU has passed `fence`. Only a device error ends the
    /// wait early: the fence guards a read of CEF's pooled texture, which
    /// must be finished before the callback returns.
    ///
    /// On that error the device is gone (`DXGI_ERROR_DEVICE_REMOVED` or
    /// `DEVICE_RESET`: the adapter was reset, or the driver crashed or was
    /// updated). D3D11 offers no way to cancel or wait on the queued copy
    /// then, and a removed device's context runs nothing further: every
    /// call on it, `GetData` included, answers with the removal error, and
    /// the only way on is a new device. So the copy either finished before
    /// the reset or was discarded with the context, the frame is dropped,
    /// and the caller retires the device (`SnapshotError::fatal`).
    fn wait(&self, fence: &ID3D11Query, since: Instant) -> Result<(), String> {
        static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        while !self.passed(fence)? {
            if since.elapsed() > SLOW_WAIT && !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                println!("browser: zero_copy: a GPU copy has taken over {SLOW_WAIT:?}; still waiting");
            }
            std::thread::yield_now();
        }
        Ok(())
    }

    fn make_fence(&self) -> Result<ID3D11Query, String> {
        let fence_desc = D3D11_QUERY_DESC { Query: D3D11_QUERY_EVENT, MiscFlags: 0 };
        let mut fence = None;
        unsafe { self.device.CreateQuery(&fence_desc, Some(&mut fence)) }
            .map_err(|e| err(e, "CreateQuery"))?;
        fence.ok_or_else(|| "CreateQuery returned nothing".to_string())
    }

    /// Open CEF's pooled texture, copy its visible part into a new texture
    /// of this device, and return once the GPU has finished the copy. Only
    /// then may the callback return and the snapshot be published.
    ///
    /// `visible` is CEF's `visible_rect` (`x, y, width, height`, device
    /// pixels): the page's pixels inside the pooled texture, whose
    /// `coded_size` may be larger (cef_types_osr.h, `visible_rect`).
    pub(crate) fn snapshot(
        &self,
        cef_handle: *mut c_void,
        visible: [i32; 4],
        probe: bool,
    ) -> Result<Snapshot, SnapshotError> {
        let t0 = Instant::now();
        let src: ID3D11Texture2D = unsafe { self.device1.OpenSharedResource1(HANDLE(cef_handle)) }
            .map_err(|e| SnapshotError::frame(err(e, "OpenSharedResource1")))?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { src.GetDesc(&mut desc) };
        let t_open = t0.elapsed();
        if desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM {
            return Err(SnapshotError::frame(format!("format {:?}, not B8G8R8A8_UNORM", desc.Format)));
        }
        let [vx, vy, vw, vh] = visible;
        let inside = vx >= 0
            && vy >= 0
            && vw > 0
            && vh > 0
            && (vx as u32).saturating_add(vw as u32) <= desc.Width
            && (vy as u32).saturating_add(vh as u32) <= desc.Height;
        if !inside {
            return Err(SnapshotError::frame(format!(
                "visible_rect {vx},{vy} {vw}x{vh} outside the {}x{} texture",
                desc.Width, desc.Height
            )));
        }
        let (width, height) = (vw as u32, vh as u32);

        let dst_desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
            CPUAccessFlags: 0,
            // SHARED alone would hand out a legacy handle; NTHANDLE alone is
            // refused. Together: an NT handle, no keyed mutex (the fork seat
            // proved this pair on dtry, 2026-09-05 20:08).
            MiscFlags: (D3D11_RESOURCE_MISC_SHARED.0 | D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0) as u32,
        };
        let mut dst = None;
        unsafe { self.device.CreateTexture2D(&dst_desc, None, Some(&mut dst)) }
            .map_err(|e| SnapshotError::frame(err(e, "CreateTexture2D")))?;
        let dst: ID3D11Texture2D =
            dst.ok_or_else(|| SnapshotError::frame("CreateTexture2D returned nothing".into()))?;
        let shared: IDXGIResource1 =
            dst.cast().map_err(|e| SnapshotError::frame(err(e, "IDXGIResource1")))?;
        let handle: HANDLE = unsafe {
            shared.CreateSharedHandle(
                None,
                DXGI_SHARED_RESOURCE_READ.0 | DXGI_SHARED_RESOURCE_WRITE.0,
                PCWSTR::null(),
            )
        }
        .map_err(|e| SnapshotError::frame(err(e, "CreateSharedHandle")))?;
        // SAFETY: a fresh NT handle this process owns; OwnedHandle closes it.
        let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };
        let fence = self.make_fence().map_err(SnapshotError::frame)?;
        let t_create = t0.elapsed();

        // The visible part only. A whole-texture CopyResource when the two
        // sizes agree, else a boxed region copy into the (0,0) corner.
        let region = D3D11_BOX {
            left: vx as u32,
            top: vy as u32,
            front: 0,
            right: vx as u32 + width,
            bottom: vy as u32 + height,
            back: 1,
        };
        unsafe {
            if width == desc.Width && height == desc.Height {
                self.ctx.CopyResource(&dst, &src);
            } else {
                self.ctx.CopySubresourceRegion(&dst, 0, 0, 0, 0, &src, 0, Some(&region));
            }
            self.ctx.End(&fence);
            self.ctx.Flush();
        }
        let t_submit = t0.elapsed();
        // Past here an error is the device, not the frame.
        let fatal = |msg: String| SnapshotError { msg, fatal: true };
        self.wait(&fence, t0).map_err(fatal)?;
        let took = t0.elapsed();
        let us = |d: Duration| d.as_micros() as u64;

        // The probe: the same copy again, between two textures this device
        // owns, timed the same way. Nothing else has ever touched either.
        let probe_wait_us = if probe {
            let mut scratch_desc = dst_desc;
            scratch_desc.MiscFlags = 0;
            let mut scratch = None;
            unsafe { self.device.CreateTexture2D(&scratch_desc, None, Some(&mut scratch)) }
                .map_err(|e| fatal(err(e, "CreateTexture2D (probe)")))?;
            let scratch: ID3D11Texture2D =
                scratch.ok_or_else(|| fatal("CreateTexture2D (probe) returned nothing".into()))?;
            let probe_fence = self.make_fence().map_err(fatal)?;
            let p0 = Instant::now();
            unsafe {
                self.ctx.CopyResource(&scratch, &dst);
                self.ctx.End(&probe_fence);
                self.ctx.Flush();
            }
            let p_submit = p0.elapsed();
            self.wait(&probe_fence, p0).map_err(fatal)?;
            Some(us(p0.elapsed() - p_submit))
        } else {
            None
        };

        Ok(Snapshot {
            handle,
            width,
            height,
            took,
            probe_wait_us,
            split: [
                us(t_open),
                us(t_create - t_open),
                us(t_submit - t_create),
                us(took - t_submit),
            ],
        })
    }
}
