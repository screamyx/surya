# winbox measurements, 2026-09-06

Threaded CEF moved the measured browser preparation work off the GPUI main
thread in this single pass. Animation paint-to-surface p95 increased. Neither
DXGI flag demonstrated a consistent latency improvement on merged main.
Keep both DXGI flags off by default and repeat measurements before rollout.

## Builds and conditions

- Instrumented fixture: `b79b17380aa0a080b6ec4539ce087f8c8d6edda4`,
  `E:\surya-astra-bin-instrumented`, release build completed at 03:28:35 MY.
- Merged main: `a5c004b9ac7f98463de4abec8509f9b3a103ccba`,
  `E:\surya-astra-bin-combined`, locked release build completed at 04:17:41 MY.
- Both use fork `3640743b70a6e4647d90249fc6ccc5398322648a` from
  [fork PR 4](https://github.com/screamyx/gpui-surya/pull/4).
  [App PR 98](https://github.com/screamyx/surya/pull/98) is merged with flags off.
- Instrumented runs: 04:18:53 to 04:22:59 MY. Main runs: 04:24:40 to
  04:26:44 MY. No compilation ran during these two series.
- NVIDIA GeForce RTX 4080; main reports a 175 Hz display and selects CEF 120.
  The fixture retains CEF 60 and the pool pump; main uses the clock pump.
  These builds are different baselines. Compare options within each table.
- Each case used `SURYA_CEF_GPU=1`, ANIM delay 8 seconds, SCROLL delay 18
  seconds, 35 seconds before close, then a 6-second shutdown allowance.
  Other experiment flags were absent except the option in the row.
- Some fixture screenshots showed the desktop covering the app; occlusion
  was not controlled continuously. Main control scrolling and waitable
  animation were visually confirmed. These are single runs, not a repeated
  or randomized benchmark, and cannot establish a general performance win.

## Instrumented fixture

Each cell pair is ANIM / SCROLL. A surface is counted only when a fresh frame
is submitted. Preparation is CEF pump + handoff drain + surface preparation,
divided by fresh submissions. It excludes renderer execution and Present.
The p95 ends at surface submission, not physical display scanout.

| Option | CEF paints | Fresh surfaces | Preparation ms/surface | Paint-to-surface p95 ms |
|---|---:|---:|---:|---:|
| CPU, external pump; all flags absent | 122 / 111 | 122 / 104 | 0.742 / 0.755 | 7.245 / 6.738 |
| GPU, `SURYA_BROWSER_ZERO_COPY=1` | 120 / 113 | 120 / 110 | 0.887 / 1.793 | 6.480 / 7.175 |
| CPU, `SURYA_CEF_THREADED=1` | 121 / 111 | 117 / 104 | 0.092 / 0.080 | 11.698 / 6.670 |
| GPU + threaded, both above flags | 121 / 111 | 117 / 106 | 0.003 / 0.004 | 13.325 / 7.117 |
| CPU external, `SURYA_FRAME_LATENCY=1` | 120 / 114 | 116 / 109 | 0.603 / 0.676 | 6.364 / 6.766 |
| CPU external, `SURYA_PRESENT_WAITABLE=1` | 122 / 107 | 121 / 104 | 0.553 / 0.753 | 6.388 / 6.503 |

All six SCROLL rows sent 60 of 60 requested wheel events. All six runs report
CEF closed=1, UI asked/done=66/66, rejected=0, and runner close asked=1,
remaining=0. Threaded runs posted all 66 UI operations and measured zero CEF
pump microseconds on the main thread. Surface counts reconcile with sample
counts; preparation totals reconcile with the reported averages.

At the last 30-second snapshots, GPU external recorded 716 accelerated
paints and 716 copies; GPU threaded recorded 714 and 714. Both reported
failed=0, dead=0, and average copy time 1.27 ms. These are cumulative
snapshots, not the ANIM/SCROLL windows. The near-zero threaded preparation
number does not mean GPU copying became free: it runs on the CEF thread.

## Merged main

Main exposes the existing `shown` and `p2d` metrics, without the fixture's
main-thread preparation totals. All three cases use CPU surfaces and the
external clock pump. Each pair is ANIM / SCROLL.

| Option | CEF paints | Shown | Existing p2d p95 ms |
|---|---:|---:|---:|
| Both DXGI flags absent | 240 / 225 | 204 / 198 | 4.7 / 4.8 |
| `SURYA_FRAME_LATENCY=1` | 240 / 228 | 214 / 190 | 5.0 / 4.3 |
| `SURYA_PRESENT_WAITABLE=1` | 240 / 224 | 206 / 181 | 4.9 / 5.2 |

All three SCROLL rows sent 60 of 60 wheel events. All three report CEF
closed=1 and runner close asked=1, remaining=0.

Both builds logged device latency asked=1, device readback=1, applied=1
for that option. Both logged waitable scope=swap_chain and effective queue
latency=1 for the waitable option. This confirms configuration, not a latency
improvement.

Main's last waitable snapshot recorded asked=1412, ready=1200, input=161,
timeout=51, stale=0, failed=0, presents=1200, unused_frames=2282,
modal_frames=0, wait_total_us=752247, wait_max_us=6040. The waits happen
outside the fixture's browser preparation timer. They are cumulative at
that snapshot and must not be substituted for full-run or per-test totals.

## Timing and validation limits

ANIM requests a nominal 2000 ms window; actual elapsed window duration was
not recorded. SCROLL's printed interval covers sending the wheel events,
while its frame counters also include a subsequent 500 ms settle period.
Do not divide those frame counts by the printed interval and call it FPS.
CEF-minus-surface count differences also include window boundaries and
are not exact dropped or physically presented frame counts.

`loaded=1` records that navigation was requested; it does not independently
prove every test page was visibly loaded. The logs contain CEF browser-info
response timeouts during self-tests, plus
GPUI invalid-window-handle errors around shutdown. All runs still produced
both metric rows and confirmed clean CEF/runner shutdown. These errors are
not dismissed as harmless; their influence was not isolated in this slot.

The fixture waitable ANIM line joined a tracing timestamp directly after
`surface_p2d_p95_ms=6.388`. The parser now splits at a complete ISO timestamp
before reading metrics. Original and unwrapped logs remain available; no
numeric value was guessed or interpolated.

## Visible interaction and cleanup

A separate GPU-threaded fixture run, 04:26:57 to 04:28:08 MY, displayed
Example Domain. Double-clicking its title bar maximized the app and repainted
the CEF surface; telemetry changed from 518x786 to 518x1346. Clicking
"Learn more" loaded IANA's Example Domains page, confirmed in a screenshot.
Attempts to type into the address bar did not change its URL, so address
entry is not verified. This smoke run is excluded from the timed tables.

The smoke closed with CEF/UI asked/done/posted=30/30/30, rejected=0 and
runner close asked=1, remaining=0. Private screenshots include unrelated
desktop content and are retained outside the public evidence files.

At 04:28:07 MY an own-path process query returned zero app/helper processes.
Seven Astra scheduled tasks were deleted; the final runner completed at
04:28:08. GUI FREE was sent to surya-cef3 and jag-0906-osprey before 04:29.

## Reproduction and evidence

The fixture's runner and parser are in
[the instrumented branch](https://github.com/screamyx/surya/tree/b79b17380aa0a080b6ec4539ce087f8c8d6edda4/docs/perf/dtry-astra).
Obtain the coordinator's GUI grant before running a case. For example:

```sh
bash docs/perf/dtry-astra/run.sh sample 35 SURYA_ASTRA_BUILD=instrumented SURYA_SELFTEST_ANIM=8 SURYA_SELFTEST_SCROLL=18 SURYA_CEF_THREADED=1 SURYA_BROWSER_ZERO_COPY=1
```

Select `SURYA_ASTRA_BUILD=combined` for the exact merged-main snapshot.
Run one case at a time, wait for its close record, and vary only the table's
flags. The two flags `SURYA_FRAME_LATENCY` and `SURYA_PRESENT_WAITABLE`
were tested separately, never together. These are runner selector names;
the exact SHA in each launch log is authoritative.

Original output/error/launch/environment logs and private screenshots remain
in `/store/agent-worktrees/surya-browser-astra/runtime-results/`; remote
originals are under `E:\surya-astra-runs`. No private desktop screenshots
or engine settings are included in this PR. The rows below remove only
console wrapping and adjacent tracing records. Hashes identify the original
output files before that normalization.

Repeat randomized runs with foreground state controlled, actual window
durations recorded, and the CEF timeouts investigated before deciding
whether to enable threaded CEF or either DXGI flag by default.

## Raw self-test rows

### r1-cpu-external

```text
selftest: ANIM loaded=1 over 2000ms cef_frames=122 app_frames=123 pump_timer=pool pump_ms=8 p2d n=122 median=2.4ms p90=4.6ms max=12.3ms surface_shown=122 main_pump_us=80087 main_handoff_us=0 main_surface_us=10391 main_ms_per_shown=0.742 surface_p2d_n=122 surface_p2d_p95_ms=7.245
selftest: SCROLL loaded=1 wheel asked=60 sent=60 over 1705ms cef_frames=111 app_frames=105 pump_timer=pool pump_ms=8 p2d n=104 median=2.5ms p90=4.7ms max=5.7ms surface_shown=104 main_pump_us=69254 main_handoff_us=0 main_surface_us=9262 main_ms_per_shown=0.755 surface_p2d_n=104 surface_p2d_p95_ms=6.738
browser: shutdown closed=1 created=1 closed=1 popups_refused=0 visible=1 cef_threaded=false ui_asked=66 ui_done=66 ui_posted=0 ui_rejected=0
close_asked=1 close_remaining=0
```

### r1-gpu-external

```text
selftest: ANIM loaded=1 over 2000ms cef_frames=120 app_frames=121 pump_timer=pool pump_ms=8 p2d n=120 median=2.2ms p90=4.5ms max=4.8ms surface_shown=120 main_pump_us=106461 main_handoff_us=0 main_surface_us=30 main_ms_per_shown=0.887 surface_p2d_n=120 surface_p2d_p95_ms=6.480
selftest: SCROLL loaded=1 wheel asked=60 sent=60 over 1736ms cef_frames=113 app_frames=112 pump_timer=pool pump_ms=8 p2d n=110 median=1.0ms p90=3.7ms max=6.6ms surface_shown=110 main_pump_us=197218 main_handoff_us=0 main_surface_us=59 main_ms_per_shown=1.793 surface_p2d_n=110 surface_p2d_p95_ms=7.175
browser: shutdown closed=1 created=1 closed=1 popups_refused=0 visible=1 cef_threaded=false ui_asked=66 ui_done=66 ui_posted=0 ui_rejected=0
close_asked=1 close_remaining=0
```

### r1-cpu-threaded

```text
selftest: ANIM loaded=1 over 2000ms cef_frames=121 app_frames=119 pump_timer=pool pump_ms=8 p2d n=118 median=3.0ms p90=8.7ms max=12.1ms surface_shown=117 main_pump_us=0 main_handoff_us=157 main_surface_us=10590 main_ms_per_shown=0.092 surface_p2d_n=117 surface_p2d_p95_ms=11.698
selftest: SCROLL loaded=1 wheel asked=60 sent=60 over 1702ms cef_frames=111 app_frames=141 pump_timer=pool pump_ms=8 p2d n=105 median=2.3ms p90=5.0ms max=5.8ms surface_shown=104 main_pump_us=0 main_handoff_us=136 main_surface_us=8135 main_ms_per_shown=0.080 surface_p2d_n=104 surface_p2d_p95_ms=6.670
browser: shutdown closed=1 created=1 closed=1 popups_refused=0 visible=1 cef_threaded=true ui_asked=66 ui_done=66 ui_posted=66 ui_rejected=0
close_asked=1 close_remaining=0
```

### r1-gpu-threaded

```text
selftest: ANIM loaded=1 over 2000ms cef_frames=121 app_frames=120 pump_timer=pool pump_ms=8 p2d n=118 median=4.1ms p90=7.3ms max=9.3ms surface_shown=117 main_pump_us=0 main_handoff_us=343 main_surface_us=63 main_ms_per_shown=0.003 surface_p2d_n=117 surface_p2d_p95_ms=13.325
selftest: SCROLL loaded=1 wheel asked=60 sent=60 over 1706ms cef_frames=111 app_frames=140 pump_timer=pool pump_ms=8 p2d n=106 median=2.4ms p90=5.3ms max=6.2ms surface_shown=106 main_pump_us=0 main_handoff_us=368 main_surface_us=36 main_ms_per_shown=0.004 surface_p2d_n=106 surface_p2d_p95_ms=7.117
browser: shutdown closed=1 created=1 closed=1 popups_refused=0 visible=1 cef_threaded=true ui_asked=66 ui_done=66 ui_posted=66 ui_rejected=0
close_asked=1 close_remaining=0
```

### r1-device-latency

```text
selftest: ANIM loaded=1 over 2000ms cef_frames=120 app_frames=117 pump_timer=pool pump_ms=8 p2d n=116 median=2.1ms p90=4.6ms max=5.4ms surface_shown=116 main_pump_us=61227 main_handoff_us=0 main_surface_us=8774 main_ms_per_shown=0.603 surface_p2d_n=116 surface_p2d_p95_ms=6.364
selftest: SCROLL loaded=1 wheel asked=60 sent=60 over 1762ms cef_frames=114 app_frames=110 pump_timer=pool pump_ms=8 p2d n=109 median=2.4ms p90=4.9ms max=6.0ms surface_shown=109 main_pump_us=65164 main_handoff_us=0 main_surface_us=8563 main_ms_per_shown=0.676 surface_p2d_n=109 surface_p2d_p95_ms=6.766
browser: shutdown closed=1 created=1 closed=1 popups_refused=0 visible=1 cef_threaded=false ui_asked=66 ui_done=66 ui_posted=0 ui_rejected=0
close_asked=1 close_remaining=0
```

### r1-waitable

```text
selftest: ANIM loaded=1 over 2000ms cef_frames=122 app_frames=122 pump_timer=pool pump_ms=8 p2d n=121 median=2.4ms p90=4.7ms max=5.3ms surface_shown=121 main_pump_us=58296 main_handoff_us=0 main_surface_us=8630 main_ms_per_shown=0.553 surface_p2d_n=121 surface_p2d_p95_ms=6.388
selftest: SCROLL loaded=1 wheel asked=60 sent=60 over 1648ms cef_frames=107 app_frames=104 pump_timer=pool pump_ms=8 p2d n=104 median=2.3ms p90=4.7ms max=6.9ms surface_shown=104 main_pump_us=70602 main_handoff_us=0 main_surface_us=7674 main_ms_per_shown=0.753 surface_p2d_n=104 surface_p2d_p95_ms=6.503
browser: shutdown closed=1 created=1 closed=1 popups_refused=0 visible=1 cef_threaded=false ui_asked=66 ui_done=66 ui_posted=0 ui_rejected=0
close_asked=1 close_remaining=0
```

### main-control

```text
selftest: ANIM loaded=1 over 2000ms cef_frames=240 app_frames=204 shown=204 pump_timer=clock pump_ms=8 p2d n=204 median=1.9ms p90=4.3ms p95=4.7ms max=5.4ms
selftest: SCROLL loaded=1 wheel asked=60 sent=60 over 1751ms cef_frames=225 app_frames=200 shown=198 pump_timer=clock pump_ms=8 p2d n=198 median=2.5ms p90=4.1ms p95=4.8ms max=5.4ms
browser: shutdown closed=1 pumps=0 created=1 closed=1 popups_refused=0 visible=1
close_asked=1 close_remaining=0
```

### main-device-latency

```text
selftest: ANIM loaded=1 over 2000ms cef_frames=240 app_frames=215 shown=214 pump_timer=clock pump_ms=8 p2d n=214 median=1.8ms p90=4.4ms p95=5.0ms max=5.4ms
selftest: SCROLL loaded=1 wheel asked=60 sent=60 over 1771ms cef_frames=228 app_frames=192 shown=190 pump_timer=clock pump_ms=8 p2d n=190 median=1.8ms p90=3.9ms p95=4.3ms max=5.4ms
browser: shutdown closed=1 pumps=0 created=1 closed=1 popups_refused=0 visible=1
close_asked=1 close_remaining=0
```

### main-waitable

```text
selftest: ANIM loaded=1 over 2000ms cef_frames=240 app_frames=206 shown=206 pump_timer=clock pump_ms=8 p2d n=206 median=2.2ms p90=4.4ms p95=4.9ms max=8.5ms
selftest: SCROLL loaded=1 wheel asked=60 sent=60 over 1736ms cef_frames=224 app_frames=184 shown=181 pump_timer=clock pump_ms=8 p2d n=181 median=2.3ms p90=4.7ms p95=5.2ms max=12.3ms
browser: shutdown closed=1 pumps=0 created=1 closed=1 popups_refused=0 visible=1
close_asked=1 close_remaining=0
```

## Raw output SHA-256

| Run | SHA-256 of `.out.log` |
|---|---|
| r1-cpu-external | `d7ceb166e9aa6c6c5263ddd0bdacf97f2249a5870bbe32b1f4ad92cadf87a508` |
| r1-gpu-external | `dd97c9c8c965cee572fe95cb69c81b4ae74a476922d722ea5072e39c63c7ad85` |
| r1-cpu-threaded | `8e109b5c6a637398fd378ede437d921db757433b534656649814c35a0412fc04` |
| r1-gpu-threaded | `f42a441d2cba12e4ea24c0ed7b67188bdbeac19b9a3f6a83d3a244a3e90abf0f` |
| r1-device-latency | `216ab272ab1651a7b41d91c6db9127cbc489966c7a7606a4c5e9717772024909` |
| r1-waitable | `2c579f79775dce2f609bb11070e99a12cf7fb3848ebd67d4f317bc29ea759302` |
| main-control | `df4f35ecb0058eb0a1b3d024c5ef87679057d479359bc68c78740d298d4138a1` |
| main-device-latency | `45dd62020074485375581309200ceb29c01147aabe2c068630b0680d5722030e` |
| main-waitable | `5da43f4fdbce4cf636d63599ec053a1e4fc6fb3267e1b7e69841bb5b3786cc71` |

Validation: 18/18 extracted self-test rows; 9/9 runner clean-close records.
All six fixture runs reconciled preparation totals and fresh-surface sample
counts. The parser regression checks passed 3/3 after the timestamp fix,
which remains outside this docs-only PR.
