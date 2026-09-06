# surya-browser

haktui's offscreen Chromium (CEF 151) composited into comet's gpui window.
Ported 2026-09-05 from `haktui/crates/haktui/src/browser/` (`osr.rs`,
`osr_input.rs`, `chrome.rs`), split into files under 500 lines.

Off by default. Build with `cargo build -p surya --features browser`.
Tests: `cd app/crates/browser && cargo test` (the crate is its own workspace root; CI does not run it).

## How a frame gets on screen

```
CEF renderer process ──on_paint(BGRA, w, h)──▶ render.rs: copy into a RenderImage
                                                    │
gpui idle pump (pump.rs, main thread) ◀── wake ─────┘   cx.refresh() when a frame moved
                                                    │
surface.rs canvas: paint_image(bounds, frame) ──▶ gpui sprite atlas upload (measured per frame)
```

Linux takes the CPU path only: one BGRA copy in the callback, one atlas
upload per paint. Windows (D3D11 shared texture) and the Mac (IOSurface) need
haktui's gpui patch to `paint_surface`, which comet's gpui fork does not carry;
those paths are not ported here.

## Runtime layout

- `libcef.so`, `*.pak`, `locales/`, `icudtl.dat` sit next to the binary. The
  `cef-dll-sys` build script copies them into `target/<profile>/`.
- `surya-browser-helper` (apps/surya, feature `browser`) is CEF's subprocess.
  It must sit next to `surya`. Without it CEF re-executes `surya` itself, which
  works because `preflight()` is the first line of `main`.
- The binary carries an `$ORIGIN` rpath (apps/surya/build.rs) so `libcef.so`
  loads without `LD_LIBRARY_PATH`.
- Windows: the same layout with `libcef.dll`, `chrome_elf.dll` and the other
  DLLs, `*.pak`, `icudtl.dat`, `*.bin`, `locales\` and
  `surya-browser-helper.exe` next to `surya.exe`; Windows loads DLLs from the
  exe's own folder, so no rpath is needed. `deploy\windows\build.ps1 -Browser`
  builds with the feature and ships that set in the zip. The cef crate needs
  CMake and Ninja on the build box and downloads the CEF binary (about 250 MB)
  into `CEF_PATH` on the first build.

## Environment

| var | effect |
| --- | --- |
| `SURYA_NO_BROWSER=1` | leave CEF out of the process; the pane is a placeholder |
| `SURYA_BROWSER_URL` | first page (default `https://example.com`) |
| `SURYA_CEF_GPU=1` | do not pass `--disable-gpu`; default is software rendering |
| `SURYA_CEF_CACHE` | CEF profile dir (default `<data>/surya/cef`) |
| `SURYA_CDP_PORT` | remote debugging port for an out-of-process client during a diagnosis (off unless set; Chromium binds it to 127.0.0.1) |
| `SURYA_PUMP_MS` | idle pump base interval, default 8 |

## Agents, devices, theme: DevTools in process

Everything an agent does to the page goes over the Chrome DevTools Protocol
without a socket: `src/devtools.rs` sends `{id, method, params}` through
CEF's `BrowserHost::send_dev_tools_message` and reads replies in a
`DevToolsMessageObserver`, the route haktui's pin and phone mode take.
No port is opened and no token exists because nothing outside the process
can reach it; `SURYA_CDP_PORT` stays off by default and is not used by the
app.

- `src/agent.rs`: `browser_open`, `browser_snapshot` (`snapshot.js`, one
  line per control with an id), `browser_click` (a real mouse click at the
  element), `browser_type`, `browser_screenshot`, `browser_eval`. Reached
  from surya-mcp through the engine's `Browser.Call`; the app serves the
  pane through `Browser.Watch` / `Browser.Reply` (`surya-ui/browser_agent.rs`).
- `src/emulation.rs`: Desktop, iPhone 15, Pixel 8, iPad presets through
  `Emulation.setDeviceMetricsOverride`, touch and a user agent; picked from
  the bar (`surya-ui/browser_device.rs`). The scale factor stays the
  window's, as haktui found necessary under offscreen rendering.
- `src/scheme.rs`: `set_color_scheme` sends `Emulation.setEmulatedMedia`
  when the app's appearance changes mid-session; the start-up switch stays
  in `cef_app.rs`.

Proof scripts: `app/scripts/browser-agent-proof.sh`,
`browser-device-proof.sh`, `browser-scheme-flip-proof.sh`.

## Not wired (kept for the next seat)

`src/unwired/`: haktui's pins (`pin.rs`). Copied verbatim, not compiled.
