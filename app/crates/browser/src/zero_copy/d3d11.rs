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
    ID3D11Texture2D, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE,
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

/// How long the callback waits for the GPU to finish the copy before handing
/// the frame to the pending queue. The copy itself is far under a
/// millisecond; what the wait really covers is the GPU process finishing
/// its own writes to the pooled texture (WDDM serialises the two), and
/// under an animating page that ran to 12 ms in bursts on an RTX 4080.
/// CEF's callback is on gpui's main thread, so it must not block that long.
const CALLBACK_WAIT: Duration = Duration::from_micros(1500);

/// This crate's own D3D11 device. One per process, made on the first
/// accelerated paint, on the same adapter as gpui's renderer.
pub(crate) struct Device {
    device: ID3D11Device,
    device1: ID3D11Device1,
    ctx: ID3D11DeviceContext,
    luid: Luid,
    name: String,
}

/// One frame, copied on the GPU. `handle` is the NT handle gpui opens the
/// texture with; dropping it releases this side's claim on the allocation.
/// `fence` is `None` once the GPU has finished the copy; until then the
/// frame must not be published (see [`Device::poll`]).
pub(crate) struct Snapshot {
    pub(crate) handle: OwnedHandle,
    pub(crate) width: u32,
    pub(crate) height: u32,
    fence: Option<ID3D11Query>,
    /// Wall time of open + create + submit + the bounded wait, the
    /// callback's own cost.
    pub(crate) took: Duration,
    /// The same, split: open CEF's handle; create the texture and its
    /// handle; submit the copy; wait for the GPU. Microseconds.
    pub(crate) split: [u64; 4],
}

impl Snapshot {
    /// The GPU has finished writing this texture.
    pub(crate) fn complete(&self) -> bool {
        self.fence.is_none()
    }
}

fn err(e: windows::core::Error, what: &str) -> String {
    format!("{what}: {e}")
}

fn adapter_name(desc: &DXGI_ADAPTER_DESC1) -> String {
    String::from_utf16_lossy(&desc.Description)
        .trim_end_matches('\0')
        .trim()
        .to_string()
}

/// The adapter called `prefer` (gpui's own, from `Window::gpu_specs`), else
/// the first hardware adapter. Software adapters are skipped: WARP can open
/// nothing the GPU process shares.
fn pick_adapter(prefer: Option<&str>) -> Result<(IDXGIAdapter1, DXGI_ADAPTER_DESC1), String> {
    let factory: IDXGIFactory1 =
        unsafe { CreateDXGIFactory1() }.map_err(|e| err(e, "CreateDXGIFactory1"))?;
    let mut first: Option<(IDXGIAdapter1, DXGI_ADAPTER_DESC1)> = None;
    let mut i = 0;
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(i) } {
        i += 1;
        let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
            continue;
        };
        if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }
        if prefer.is_some_and(|p| p.trim() == adapter_name(&desc)) {
            return Ok((adapter, desc));
        }
        if first.is_none() {
            first = Some((adapter, desc));
        }
    }
    first.ok_or_else(|| "no hardware adapter".to_string())
}

impl Device {
    pub(crate) fn new(prefer: Option<&str>) -> Result<Self, String> {
        let (adapter, desc) = pick_adapter(prefer)?;
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

    /// Check a snapshot the callback handed over unfinished. Returns true
    /// once the copy is complete; the snapshot may then be published.
    pub(crate) fn poll(&self, snap: &mut Snapshot) -> Result<bool, String> {
        let Some(fence) = snap.fence.as_ref() else { return Ok(true) };
        if self.passed(fence)? {
            snap.fence = None;
        }
        Ok(snap.complete())
    }

    /// Open CEF's pooled texture and copy it into a new texture of this
    /// device. The copy is queued on the GPU and given [`CALLBACK_WAIT`]
    /// to finish; a snapshot that comes back with `complete() == false`
    /// goes to the pending queue and is polled from there. The pooled
    /// texture is only ever read by that queued copy, and the copy has
    /// been submitted before this returns, which is what CEF's header
    /// requires of the callback.
    pub(crate) fn snapshot(&self, cef_handle: *mut c_void) -> Result<Snapshot, String> {
        let t0 = Instant::now();
        let src: ID3D11Texture2D = unsafe { self.device1.OpenSharedResource1(HANDLE(cef_handle)) }
            .map_err(|e| err(e, "OpenSharedResource1"))?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { src.GetDesc(&mut desc) };
        let t_open = t0.elapsed();
        if desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM {
            return Err(format!("format {:?}, not B8G8R8A8_UNORM", desc.Format));
        }
        if desc.Width == 0 || desc.Height == 0 {
            return Err(format!("empty texture {}x{}", desc.Width, desc.Height));
        }

        let dst_desc = D3D11_TEXTURE2D_DESC {
            Width: desc.Width,
            Height: desc.Height,
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
            .map_err(|e| err(e, "CreateTexture2D"))?;
        let dst: ID3D11Texture2D = dst.ok_or("CreateTexture2D returned nothing")?;
        let shared: IDXGIResource1 = dst.cast().map_err(|e| err(e, "IDXGIResource1"))?;
        let handle: HANDLE = unsafe {
            shared.CreateSharedHandle(
                None,
                DXGI_SHARED_RESOURCE_READ.0 | DXGI_SHARED_RESOURCE_WRITE.0,
                PCWSTR::null(),
            )
        }
        .map_err(|e| err(e, "CreateSharedHandle"))?;
        // SAFETY: a fresh NT handle this process owns; OwnedHandle closes it.
        let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };
        let fence_desc = D3D11_QUERY_DESC { Query: D3D11_QUERY_EVENT, MiscFlags: 0 };
        let mut fence = None;
        unsafe { self.device.CreateQuery(&fence_desc, Some(&mut fence)) }
            .map_err(|e| err(e, "CreateQuery"))?;
        let fence: ID3D11Query = fence.ok_or("CreateQuery returned nothing")?;
        let t_create = t0.elapsed();

        unsafe {
            self.ctx.CopyResource(&dst, &src);
            self.ctx.End(&fence);
            self.ctx.Flush();
        }
        let t_submit = t0.elapsed();
        let mut fence = Some(fence);
        while let Some(f) = fence.as_ref() {
            if self.passed(f)? {
                fence = None;
            } else if t0.elapsed() - t_submit > CALLBACK_WAIT {
                break;
            } else {
                std::thread::yield_now();
            }
        }
        let took = t0.elapsed();
        let us = |d: Duration| d.as_micros() as u64;
        Ok(Snapshot {
            handle,
            width: desc.Width,
            height: desc.Height,
            fence,
            took,
            split: [
                us(t_open),
                us(t_create - t_open),
                us(t_submit - t_create),
                us(took - t_submit),
            ],
        })
    }
}
