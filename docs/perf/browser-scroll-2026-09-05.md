# Browser scroll frame rate on Windows, 2026-09-05

Seat `surya-browser-perf`.
Owner order 20:21: "measure first (after cef browser gpu work)".
The record this builds on is haktui's `docs/spike-scroll-frame-rate-2026-08-28.md`, rounds one to three, all measured on the same box (dtry, RTX 4080, 120 Hz screen).

## The instruments (PR #86)

| variable | what it measures | line it prints |
| --- | --- | --- |
| `SURYA_SELFTEST_SCROLL=<secs>` | a 40,000 px page, sixty wheel events at 60 a second | `selftest: SCROLL ... cef_frames= app_frames= ... p2d ...` |
| `SURYA_SELFTEST_ANIM=<secs>` | a page that animates on its own, no input, two seconds | `selftest: ANIM ... cef_frames= app_frames= ... p2d ...` |
| `SURYA_SELFTEST_TIMER=<secs>` | thirty 16 ms timers back to back, gpui's then ours | `selftest: TIMER asked=16ms x30 gpui median= ... ours median= ...` |

`p2d` is paint to draw: microseconds from CEF's `on_paint` to the render that shows the frame, median, p90 and max over the test window.
`<secs>` is the delay before the test starts, so the page and the pane have settled.

## The switches (PR 2)

| variable | default | what |
| --- | --- | --- |
| `SURYA_PUMP_TIMER=pool` | clock | both pump waits on the old path: gpui's timer for the idle chain, a condvar for CEF's delayed asks (the control) |
| `SURYA_PUMP_MS=<n>` | 8 | the idle pump's base interval |
| `SURYA_CEF_FPS=<n>` | display rate, capped at 120 | CEF's `windowless_frame_rate` |
| `SURYA_COARSE_TIMER=1` | off | skip `timeBeginPeriod(1)` and the coalescing opt-out |

## Run recipe on dtry

Release build with the browser feature, one run at a time, GUI slot from surya-remote, session 1.
Each run is one self-test and one switch set:

```
$env:SURYA_SELFTEST_SCROLL=8; .\zeron.exe > scroll-clock.log
$env:SURYA_PUMP_TIMER='pool'; $env:SURYA_SELFTEST_SCROLL=8; .\zeron.exe > scroll-pool.log
```

Then `Select-String 'selftest:' *.log`.

## Rehearsal of the scripts, 21:49 to 21:52 (not the baseline)

Build: the PR 2 tree (e68ee56, clock default, without #85), release, dtry, session 1, one run at a time.
The screen reports 175 Hz (`browser: frame rate 120 (display reports 175 Hz)`), so CEF is capped at 120.
Lines from `docs/perf/dtry/lines.py runs/*.out.log`:

| run | switch | line |
| --- | --- | --- |
| timer | default | `TIMER asked=16ms x30 gpui median=30.9ms min=15.9ms max=31.6ms \| ours median=16.4ms min=16.1ms max=16.8ms pump_timer=clock` |
| scroll | default (clock) | `SCROLL loaded=1 wheel asked=60 sent=60 over 1652ms cef_frames=210 app_frames=210 pump_timer=clock pump_ms=8 p2d n=209 median=2.8ms p90=5.1ms max=5.6ms` |
| scroll | `SURYA_PUMP_TIMER=pool` | `SCROLL loaded=1 wheel asked=60 sent=60 over 1721ms cef_frames=223 app_frames=219 pump_timer=pool pump_ms=8 p2d n=219 median=2.9ms p90=4.9ms max=8.2ms` |

Two things to read with the numbers.
`app_frames` counts renders of the root view, which is what haktui's `renders` counted too; on surya every CEF frame got a render on both paths, so the timer did not decide frames here the way it did in haktui.
The self-test spaces its sixty wheel events with gpui's 16 ms timer, so on Windows they go out at about 36 a second (`over 1652ms`), the same as haktui's instrument; the two tables compare, and the real cadence is what the line says.

## Baseline, main with #85 (zero-copy) in, PR #86 instruments

Not measured yet.
Fill from the `selftest:` lines, one row per run, exact numbers.

| run | switch | cef frames | app frames | timer median (gpui / ours) | p2d median / p90 / max |
| --- | --- | --- | --- | --- | --- |
| scroll | default | | | | |
| scroll | `SURYA_PUMP_TIMER=pool` | | | | |
| anim | default | | | | |
| anim | `SURYA_PUMP_TIMER=pool` | | | | |
| timer | default | | | | |

## After each change

Not measured yet.

| change | scroll cef / app | anim cef / app | timer ours median | p2d median / p90 / max | idle % of a core, 40 s still page |
| --- | --- | --- | --- | --- | --- |
| (a) the clock, base 8 ms | | | | | |
| (b) CEF at the display's rate | | | | | |
| (c) DXGI frame latency 1 | needs the fork patch below | | | | |
| (d) idle cost, before and after | | | | | |

## (c) DXGI frame latency 1 needs a fork change

The pinned gpui fork (`f910653`) and the incoming `surya/external-texture` (`a07e9577`) have no `SetMaximumFrameLatency` call: checked by grep of `crates/gpui_windows/src/directx_renderer.rs` at both revisions.
haktui's fourth gpui patch adds one.
The same change, rebased onto `a07e9577` and renamed to `SURYA_FRAME_LATENCY`, is `docs/perf/gpui-surya-a07e9577-frame-latency.patch` beside this file: 22 added lines in `DirectXRendererDevices::new`, after `let annotation = device_context.cast().ok();`.
Default 1, `SURYA_FRAME_LATENCY=3` restores DXGI's.
raven 20:43: the Codex seat applies it as a second fork PR plus a pin bump, after this seat's baseline.

## What haktui found, for the comparison

| build | scroll cef / app | anim cef / app |
| --- | --- | --- |
| pool timer, CEF cap 60 | 108 / 64 | |
| own clock, base 8 ms, CEF cap 60 | 112 / 98 | |
| own clock, CEF at 120 | 210 / 106 | 242 / 110 |
| plus DXGI latency 1 | 221 / 125 | 240 / 137 |

Timer: gpui's 16 ms asked, 30.9 ms median; haktui's own clock 16.3 ms.
Idle: 0.5% of a core on a still page with either timer; 0.2% with CEF at 120.
