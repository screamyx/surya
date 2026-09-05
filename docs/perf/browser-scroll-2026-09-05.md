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
| `SURYA_CEF_FPS=<n>` | display rate, capped at 120 | CEF's `windowless_frame_rate`; a value over 120 is clamped and the log says so, a non-number is refused and the display decides |
| `SURYA_COARSE_TIMER=1` | off | skip `timeBeginPeriod(1)`. The 1 ms tick is a declared default on both pump paths (see `clock.rs` `fine_timer`): never released, and Windows returns the process to its default tick while the window is in the background, since PR 3 dropped the power-throttling opt-out that used to prevent that |

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

Measured 01:10 to 01:14 on 2026-09-06 by seat `surya-browser-perf2`.
Build: tree `d5234ca` (main after #85, with #86 merged in; osprey's merge commit on `feat/browser-perf`), release, `E:\surya-perf-target`, dtry session 1, one run at a time, GUI slot from surya-cef3.
On this tree the pump's default is the pool timer (`browser: pump base=8ms timer=pool`), `SURYA_PUMP_TIMER=clock` selects #86's clock, and CEF is capped at 60 (`client.rs` line 49, `set_windowless_frame_rate(60)`; PR 2's `display.rs` is not in this tree, so no `browser: frame rate` line is printed).
Logs: `E:\surya-perf-runs\<run>.out.log` on dtry, copies in `/tmp/perf2-runs/` on the build box.
Lines from `docs/perf/dtry/lines.py`, exact:

| run | switch | line |
| --- | --- | --- |
| scroll | default (pool) | `SCROLL loaded=1 wheel asked=60 sent=60 over 1817ms cef_frames=117 app_frames=120 pump_timer=pool pump_ms=8 p2d n=117 median=4.2ms p90=8.4ms max=9.5ms` |
| anim | default (pool) | `ANIM loaded=1 over 2000ms cef_frames=121 app_frames=122 pump_timer=pool pump_ms=8 p2d n=120 median=4.2ms p90=9.1ms max=10.1ms` |
| timer | default (pool) | `TIMER asked=16ms x30 gpui median=30.8ms min=16.1ms max=31.5ms \| ours median=30.9ms min=29.9ms max=31.7ms pump_timer=pool` |
| timer | `SURYA_PUMP_TIMER=clock` | `TIMER asked=16ms x30 gpui median=30.9ms min=16.0ms max=31.7ms \| ours median=16.3ms min=16.0ms max=16.7ms pump_timer=clock` |
| idle | default (pool), still page | `idle: pid=27412 cpu_ms=594 over 40s = 1.5% of one core` (from `idle-base.launch.log`) |

The same numbers in the comparison's shape:

| run | switch | cef frames | app frames | timer median (gpui / ours) | p2d median / p90 / max |
| --- | --- | --- | --- | --- | --- |
| scroll | default (pool) | 117 | 120 | | 4.2 / 8.4 / 9.5 ms |
| anim | default (pool) | 121 | 122 | | 4.2 / 9.1 / 10.1 ms |
| timer | default (pool) | | | 30.8 / 30.9 ms | |
| timer | `SURYA_PUMP_TIMER=clock` | | | 30.9 / 16.3 ms | |
| idle | default (pool) | | | | 1.5% of one core over 40 s |

Read against haktui's first row (pool timer, CEF cap 60: 108 / 64): surya's baseline paints at the same CEF rate and renders every paint (120 app frames for 117 CEF frames), so the app side is already ahead of haktui's before any lever moves.
The 30.9 ms timer is the Windows thread-pool tick haktui measured; #86's clock brings it to 16.3 ms on this box, the same as haktui's own clock.
The dtry screen reports 175 Hz (PR 2's rehearsal line above), not the 120 Hz written at the top of this file.

## After each change

Measured 02:06 to 02:15 on 2026-09-06 by seat `surya-browser-perf2`.
Build: the PR 2 tree rebased on `d5234ca` (`0b3e1a05`: clock default, CEF at the display rate capped at 120, `timeBeginPeriod(1)` plus the coalescing opt-out), release, `E:\surya-perf-target`, dtry session 1, one run at a time, GUI slot from surya-cef3.
Every run printed `browser: frame rate 120 (display reports 175 Hz)` except the `SURYA_CEF_FPS=60` run (`browser: frame rate 60 (SURYA_CEF_FPS)`).
Logs: `E:\surya-perf-runs\<run>.out.log` on dtry, copies in `/tmp/perf2-runs/` on the build box.
Lines from `docs/perf/dtry/lines.py`, exact:

| run | switches | line |
| --- | --- | --- |
| scroll | default (clock, CEF 120) | `SCROLL loaded=1 wheel asked=60 sent=60 over 1666ms cef_frames=214 app_frames=213 shown=212 pump_timer=clock pump_ms=8 p2d n=212 median=2.9ms p90=4.8ms p95=5.2ms max=7.4ms` |
| scroll | `SURYA_PUMP_TIMER=pool` | `SCROLL loaded=1 wheel asked=60 sent=60 over 1741ms cef_frames=224 app_frames=223 shown=221 pump_timer=pool pump_ms=8 p2d n=221 median=2.7ms p90=4.9ms p95=5.3ms max=8.8ms` |
| scroll | `SURYA_CEF_FPS=60` (clock) | `SCROLL loaded=1 wheel asked=60 sent=60 over 1690ms cef_frames=110 app_frames=114 shown=110 pump_timer=clock pump_ms=8 p2d n=110 median=3.1ms p90=5.1ms p95=5.2ms max=5.9ms` |
| anim | default (clock, CEF 120) | `ANIM loaded=1 over 2000ms cef_frames=241 app_frames=243 shown=242 pump_timer=clock pump_ms=8 p2d n=242 median=2.9ms p90=5.0ms p95=5.2ms max=5.6ms` |
| anim | `SURYA_PUMP_TIMER=pool` | `ANIM loaded=1 over 2000ms cef_frames=241 app_frames=241 shown=241 pump_timer=pool pump_ms=8 p2d n=241 median=2.6ms p90=4.9ms p95=5.2ms max=5.7ms` |
| timer | default (clock) | `TIMER asked=16ms x30 gpui median=30.9ms min=15.8ms max=31.7ms \| ours median=16.5ms min=16.1ms max=16.8ms pump_timer=clock` |
| idle | default (clock) | `idle: pid=2080 cpu_ms=531 over 40s = 1.3% of one core` |
| idle | `SURYA_PUMP_TIMER=pool` | `idle: pid=36812 cpu_ms=609 over 40s = 1.5% of one core` |

The comparison, baseline against each switch set. The "shown / s" column divides by `over Nms`, the wheel-send interval; the counters in this build ran through a further 500 ms settle after the last wheel event (`selftest/scroll.rs`, found by surya-browser-astra 03:27), so the true counting window is about 500 ms longer than the divisor and these are per-wheel-second figures, a workload-normalised proxy, not the frame cadence. Every row shares the same settle, so the ratio between rows stands (212 shown against 120 with the true windows 2166 ms against 2317 ms is still 1.9x). The raw counts are the evidence. PR 3 prints `window=` (measured, after the settle) on both self-tests; rates from then on divide by it.

| build | scroll shown / s | anim cef / app / shown (2 s) | timer ours median | scroll p2d median / p90 / max | idle % of a core, 40 s still page |
| --- | --- | --- | --- | --- | --- |
| baseline: pool timer, CEF 60 | 66 (120 app frames over 1817 ms) | 121 / 122 / n.a. | 30.9 ms | 4.2 / 8.4 / 9.5 ms | 1.5% |
| (a) the clock, CEF 60 (`SURYA_CEF_FPS=60`) | 65 (110 over 1690 ms) | not run | 16.5 ms | 3.1 / 5.1 / 5.9 ms | not run |
| (a)+(b) the clock, CEF at the display rate (PR 2 default) | 127 (212 over 1666 ms) | 241 / 243 / 242 | 16.5 ms | 2.9 / 4.8 / 7.4 ms | 1.3% |
| (b) only: pool timer, CEF at the display rate (`SURYA_PUMP_TIMER=pool`) | 127 (221 over 1741 ms) | 241 / 241 / 241 | 30.9 ms (from the baseline row; the pool timer is the same code) | 2.7 / 4.9 / 8.8 ms | 1.5% |
| (c) DXGI frame latency 1 | needs the fork patch below (astra's fork PR 2) | | | | |

What the numbers say:

1. CEF at the display rate is the lever that doubles the frames shown: 66 to 127 a second on scroll, 121 to 241 CEF frames in the two-second animation, and the app renders every one of them (`shown` is within two of `cef_frames` on every run).
2. The clock does not add frames on surya (212 shown with it, 221 without, the same 127 a second once normalised), because every CEF paint already gets a render on both timer paths; the timer never decided frames here the way it did in haktui.
3. The clock is better in what it controls: a 16 ms timer fires in 16.5 ms instead of 30.9 ms, the worst paint-to-draw on scroll drops from 9.5 ms to 5.9 ms at CEF 60 and from 8.8 ms to 7.4 ms at CEF 120, and idle CPU is 1.3% against 1.5%. Single runs, so the p2d max and idle deltas are indications, the timer delta is not in doubt.
4. So PR 2 ships with both defaults on (decision 27: a flag defaults on only where dtry measures better): CEF at the display rate because it doubles frames, the clock because its own measure halves and nothing it touches got worse. `SURYA_PUMP_TIMER=pool` and `SURYA_CEF_FPS=60` stay as the controls.

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
