# Windows device queue-limit experiment

The GPUI pin moves from `a07e9577ec` to `c2cf695f39`, which adds the
independent [DXGI queue-limit control](https://github.com/screamyx/gpui-surya/pull/2).
All four dependency declarations and the lockfile's GPUI source entries
use the same revision. The browser lock entry also records its existing
Windows dependency, needed when building with the browser feature.

`SURYA_FRAME_LATENCY=1` asks DXGI to limit the device queue to one frame.
Unset retains the previous policy. Values outside one through sixteen are
rejected. Startup and device recovery log the requested limit and actual
readback. This pin does not change CEF's frame cap, message loop, timer,
zero-copy default, or presentation sync interval.

The fork module passed an API compile check for `x86_64-pc-windows-msvc`
using `windows` 0.61.3, the fork's dependency. The full application build,
Windows readback, and matched runtime measurements for this pin are
pending. A compile check is not evidence of lower latency.

For the paired measurement, hold the app revision, window size, CEF path,
and workload fixed. Compare unset against `SURYA_FRAME_LATENCY=1` with
the other experiment flags unset. Record the actual queue readback,
CEF paints, fresh browser surface submissions, application preparation
time per submitted frame, and paint-to-surface p95. Preserve the raw logs
and prove that each process closes without forced termination.

The application preparation timer does not include the renderer's command
execution or `Present`. The difference between CEF paints and surface
submissions within a sampling window can include its boundary frames;
it must not be reported as an exact count of discarded display frames.

The control remains opt-in until measurements on the owner's Windows
machine justify changing the default.
