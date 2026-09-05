# Windows frame pacing experiments

The GPUI pin is `1a5638ebe6edd4e98ddf1b4278d2308330fe61ef`, combining the
[device latency and waitable presentation changes](https://github.com/screamyx/gpui-surya/pull/4).
All GPUI dependency declarations and lock sources use that revision.
Both experiment flags are off by default and operate independently.

| Flag | Enabled behavior | Default behavior |
| --- | --- | --- |
| `SURYA_FRAME_LATENCY=1` | Requests a device queue limit of one frame through DXGI and logs the readback on creation and recovery. Integer values 1 through 16 are accepted. | Unset does not call the setter; invalid values warn and leave the policy unchanged. |
| `SURYA_PRESENT_WAITABLE=1` | Creates a waitable swap chain and waits for readiness before requesting a GPUI frame. Message interruptions return to the message loop; bounded timeouts defer to the next display beat. | Unset or any value other than `1` preserves the existing swap-chain flags and frame callback behavior. |

The waitable gate retains a permit across unchanged views until a successful
Present. Creation, resize, and device recovery preserve the waitable flag and
handle ownership. The existing VSyncProvider and Present sync interval remain
unchanged. The waitable chain has its own queue policy, logged at startup;
it must not be confused with the separate device latency setter.

No CEF message-loop, frame-rate, timer, or zero-copy flag is enabled by this pin.
Neither new flag implicitly enables the other. Measure each alone against the
same default control before testing both together.

## Validation status

**Matched dtry numbers are pending.** No performance benefit is established.
The independent latency integration at `22caa8129706500380995c453eddc0d81a7ebeb3`
and present integration at `b0c43c0cd368039694a51d316fa6bdbbce3129f2` each passed
Windows release builds with `--locked` (`BUILD_EXIT=0`). The combined fork's
modules passed a Windows API compile probe using its `windows` 0.61.3 dependency
(`WINPROBE_EXIT=0`). The combined app revision still requires its build and
runtime checks. The lockfile also records the browser's existing Windows
dependency so the browser feature can build with `--locked`.

After the coordinator merges this pin with both flags off, obtain an explicit
desktop grant and compare default, latency alone, and waitable alone using the
same binary, workload, window size, and CEF settings. Record queue readback,
CEF paints, fresh browser surface submissions, browser preparation time per
submitted frame, paint-to-surface p95, and separate waitable-gate wait totals.
Check first paint, animation without input, scrolling, return from idle,
resize, and clean process shutdown; recovery remains a separate verification.

Browser preparation excludes renderer command execution and Present. The
paint-to-surface timer stops before physical scanout. A sampling-window gap
between CEF paints and surface submissions is not an exact discarded-frame
count, and successful Present calls are not physical display scans.
