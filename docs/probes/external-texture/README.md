# Windows external D3D11 texture entry

Date: 2026-09-05. Fork PR: https://github.com/screamyx/gpui-surya/pull/1.
Pinned revision: `a07e9577ec788feb73c06fe7e307a3df8adaa895`, branch
`surya/external-texture`, cut from the existing `f910653` HLSL stride fix.
The pinned revision includes review fixes for resize cropping and owner-identity caching.
Windows check/build and all scoped tests were repeated after those fixes.

This pin adds an additive Windows-only renderer API. It does not turn on CEF
accelerated painting: that belongs to the browser integration's runtime switch
`SURYA_BROWSER_ZERO_COPY=1`, off by default. Existing callers still paint CPU
images. The integration must be reviewed and proven separately.

## Measured proof

Machine: dtry, Windows Rust/Cargo 1.97.1. Isolated source, Cargo home, and target
paths were `E:\surya-gpufork`, `E:\surya-gpufork-cargo`, and
`E:\surya-gpufork-target`. No release build cache was modified.

| Check | Actual output |
| --- | --- |
| Full app with browser feature | `cargo check --locked -p surya --features browser`, `exit=0` in [browser-check.log](browser-check.log) |
| Windows standalone check/build | `check_exit=0`, `exit=0` in [build.log](build.log) |
| GPUI external surface submissions | `asked=59 frames=59 clipped=0 dropped=0` in [run.log](run.log) |
| Scoped renderer tests | `5 passed; 0 failed` in [test.log](test.log) |
| GPU clipping and owner lifetime | `asked=32 frames=32 dropped=0 pixels_asked=256 pixels_matched=256` |
| Native screenshot | `PrintWindow asked=1 done=1`, inspected [dtry.png](dtry.png) |

The screenshot shows the GPU-cleared green shared texture inside an ordinary
GPUI dark background. The standalone producer drops its D3D texture/device
wrappers before the scene renders, exercising handle ownership. The pixel test
checks an offset content mask, a negative surface origin, and bounds larger than
the actual source against a GPU readback, including pixels outside the copied
rectangle. It also asserts cache reuse and retirement after the final owner drops. The mutex test leaves
key 1 available and proves that a key-0 timeout is rejected.

Reproduce from the fork's interactive Windows session:

```powershell
$env:GPUI_EXTERNAL_TEXTURE_STATS = '1'
cargo check --manifest-path tools/external_texture_probe/Cargo.toml
cargo build --manifest-path tools/external_texture_probe/Cargo.toml
cargo run --manifest-path tools/external_texture_probe/Cargo.toml
cargo test --manifest-path tools/external_texture_probe/Cargo.toml -p gpui_windows --lib external_texture -- --nocapture
```

Repository-wide instruction gate: `node scripts/accuracy_report.mjs` returned
`RESULT: 5/37 checks passed`. Its unchanged design-kit gates reference absent
`examples/golden`, `examples/sample-app`, and other fixtures, and Playwright is
not installed in this checkout. This is not a passing repository accuracy
claim. The no-emoji gate separately reported `Scanned 217 file(s)` and passed.
No HTML, theme, token, or component-spec files changed in this pin PR.

The dependency `proc-macro-error2` emitted a future-incompatibility warning.
The full app check also reported existing warnings in sync, harness, engine, and
UI files; none of the changed code emitted a compiler warning. The renderer timing counter
measures CPU submission cost, not completion of GPU execution.

## Why the Haktui raw-handle patch was insufficient

Haktui's `spike-cef-zedmain-2026-08-26.md` and
`handoff-browser-round2-2026-08-25.md` identify the earlier `E_INVALIDARG` as a
handle-lifetime problem. Its callback adapter probe could open the texture on
the same NVIDIA adapter as GPUI. Deferred opening used a stale handle.

The later duplicate-handle retirement ring keeps a handle alive but cannot
freeze Chromium's pooled content. CEF 151.3.24 `cef_render_handler.h` lines
161-167 require reopening the texture and copying its content into a texture
owned by the client during each `OnAcceleratedPaint` callback. The current
[CEF header](https://github.com/chromiumembedded/cef/blob/master/include/cef_render_handler.h)
carries the same rule. This entry therefore accepts a client-owned snapshot,
never CEF's borrowed pool handle.

## API and ownership

```rust
use gpui::{ExternalTexture, ExternalTextureSync};
// SAFETY: fresh opaque BGRA8 allocation, GPU writes completed, immutable;
// handle is an owned NT share from the stated adapter and dimensions.
let texture = unsafe {
    ExternalTexture::new(handle, (luid_low, luid_high), (width, height),
                         ExternalTextureSync::ProducerComplete)
};
// In a canvas paint closure:
window.paint_external_texture(bounds, &texture);
```

`ExternalTexture` is cloneable with an internal `Arc<OwnedHandle>`. The final
clone closes the NT handle. The scene retains its own clone across browser
resize/close, so there is no fixed retirement-ring length or handle-value cache. Opened resources
and descriptors are cached by the internal Arc allocation identity with a weak
owner; frame-start pruning and renderer-device changes retire stale entries.
The renderer compares the producer LUID with its adapter before
`OpenSharedResource1` and validates the actual descriptor and dimensions.

`ProducerComplete` snapshots use `SHARED | SHARED_NTHANDLE`, with all GPU writes
complete before publication and no later mutation. CEF must observe a GPU event
before returning from its callback; `Flush` alone only submits commands. CPU
reference counts do not prove queued consumer GPU reads have finished, so the
initial producer must not recycle these allocations.

`KeyedMutex` supports `SHARED_KEYEDMUTEX | SHARED_NTHANDLE`, acquiring and releasing
key 0 for each access. The renderer uses a nonblocking acquire and accepts only
exact `S_OK`; positive `WAIT_TIMEOUT` and `WAIT_ABANDONED` HRESULTs must not pass
through a generic success check. [Microsoft documents this distinction](https://learn.microsoft.com/en-us/windows/win32/api/dxgi/nf-dxgi-idxgikeyedmutex-acquiresync).

## Scope

This is an opaque rectangular GPU copy with device pixels mapped 1:1, clipped
against the source dimensions, content mask, and render target. During fractional
DPI rounding or resize, differing bounds copy the available intersection instead
of rejecting the entire surface. Missing renderer resources log and skip the
surface instead of returning a frame-aborting error. Scaling, rounded corners,
opacity, edge fades, and transparent browser surfaces are unsupported. No
GPU-facing structured-buffer layout or shader changes were made. CEF must provide
A=255 content: its existing OPAQUE_WHITE browser background requests opaque
backing. Transparent backing is not supported by this direct-copy path.

The synthetic proof does not prove CEF page rendering, real-page resize,
callback-cost improvement, or fallback behavior after accelerated painting
fails. Those are the browser GPU integration's separate acceptance checks.
