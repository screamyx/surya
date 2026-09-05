# Probe: zero-copy browser frames on Windows, 2026-09-05

Seat surya-browser-gpu, branch `feat/browser-d3d11`, with surya-browser-gpu-fork on the gpui side (screamyx/gpui-surya PR 1, rev a07e957).
Box: dtry, RTX 4080, release build with the browser feature, pane 518x786 device pixels.

## What was asked

Paint CEF's frames without the CPU copy that `on_paint` does today (BGRA bytes into a `RenderImage`, then an atlas upload), behind a switch, and measure it against the CPU path.
haktui had tried CEF's shared-texture path and stopped at `E_INVALIDARG` when opening the handle.

## What E_INVALIDARG was

CEF 151.3.24 `include/cef_render_handler.h` lines 161 to 167, on `OnAcceleratedPaint`:

> "The underlying implementation uses a pool to deliver frames. As a result, the handle may differ every frame depending on how many frames are in-progress. The handle's resource cannot be cached and cannot be accessed outside of this callback. It should be reopened each time this callback is executed and the contents should be copied to a texture owned by the client application. The contents of |info| will be released back to the pool after this callback returns."

haktui's spike stored the raw handle in the scene and opened it in the renderer a frame later.
By then CEF had closed it.
A closed handle value is what `OpenSharedResource1` calls an invalid argument.
Not an adapter mismatch: haktui's own later probe opened the handle fine on the NVIDIA card (`docs/handoff-browser-round2-2026-08-25.md` line 56 in the haktui repo).
Not a keyed mutex: `cef-dll-sys` says the texture "is instantiated without a keyed mutex" (`bindings/x86_64_pc_windows_msvc.rs:17956`).

## What was built

```
CEF GPU process ──pooled texture──> on_accelerated_paint (main thread)          app/crates/browser/src/zero_copy/
                                        │ OpenSharedResource1 on the crate's own D3D11 device
                                        │ CopyResource into a fresh SHARED | SHARED_NTHANDLE BGRA8 texture
                                        │ Flush, D3D11_QUERY_EVENT, wait until the GPU is done
                                        ▼
                            gpui::ExternalTexture (owns the NT handle, ProducerComplete)
                                        ▼
                    surface.rs: Window::paint_external_texture(bounds, &texture)    fork
                                        ▼
                                   swap chain
```

`SURYA_BROWSER_ZERO_COPY=1`, Windows only, default off.
Unset, the CPU path runs unchanged.

## Measured

Same page both runs: a CSS spinner plus a `requestAnimationFrame` counter, served from pc-ajim, 50 seconds each.

| | flag off (CPU path) | flag on (zero-copy) |
| --- | --- | --- |
| paints in 50 s | frames=2935 | accel=3120 copied=3120 waited=0 failed=0 (frames=2937) |
| callback cost per paint, avg | copy 0.06 ms | 1.45 ms |
| callback cost per paint, max | copy 0.41 ms | 11.29 ms |
| element cost per paint, avg | upload 0.07 ms | 0 (the renderer opens the handle; the heartbeat's `upload_ms` stays at 0 with the flag on, the zero-copy numbers follow it as `zero_copy=on ...`) |
| gpui renders in 50 s | 3118 | 3013 |
| page on screen | `docs/images/browser-zero-copy-off.png` | `docs/images/browser-zero-copy-on.png` |

Where the zero-copy callback's time goes, from 55 sampled paints (every 60th):

| step | typical | worst sampled |
| --- | --- | --- |
| open CEF's handle | 40 to 90 us | 93 us |
| create the texture and its NT handle | 100 to 210 us | 477 us |
| submit the copy and flush | 60 to 160 us | 375 us |
| wait for the GPU to finish the copy | 0.5 to 0.8 ms | 7.98 ms |

The wait is the cost.
A probe (`SURYA_BROWSER_ZERO_COPY_PROBE=1`) times a second copy right after, between two textures of the crate's own device, on the same context.
Pairs from the same run, same sampled paints:

| | source copy wait | own-texture copy wait |
| --- | --- | --- |
| average | 1.09 ms | 0.44 ms |
| over 2 ms | 5 of 55 | 3 of 55 |
| worst | 7.98 ms (paint 1200) | 11.1 ms (paint 1320) |

Reading: the source copy waits longer on average because WDDM orders it after the GPU process's write to the pooled texture, but the own-texture copy bursts too, so the bursts are the GPU's scheduling between three contexts at 60 frames a second (Chromium's GPU process, gpui, this crate), not one thing this crate can remove.
The callback cannot return before the wait ends: the header above forbids reading the pooled texture after the callback, and D3D11 orders nothing across processes.
A bounded wait with deferred publishing was tried and withdrawn for that reason.

## Verdict

The path works: 3120 of 3120 paints copied, zero failures, the page correct on screen, no pixel through the CPU.
At this pane size it costs the main thread ten times more per paint than the CPU path (1.45 ms against 0.13 ms), with bursts near a frame.
The switch stays off.
It earns its keep only where the CPU copy grows with the pixels and the GPU copy does not: 4K panes and video.
Not measured here; the pane was 518x786.

Next, if anyone wants the flag on by default: measure at 2560x1440 and 4K, and then try moving the wait off the main thread by running CEF's message loop on its own thread (`multi_threaded_message_loop`), which is a change to the pump, not to this module.
