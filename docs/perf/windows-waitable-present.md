# Windows waitable presentation experiment

The GPUI pin moves independently from `a07e9577ec` to `15443a0b1e`, the
[waitable presentation experiment](https://github.com/screamyx/gpui-surya/pull/3).
All four declarations and the GPUI lock sources use that revision. The
browser lock entry records its existing Windows dependency as well.

`SURYA_PRESENT_WAITABLE=1` creates a waitable swap chain and waits for its
readiness before GPUI builds the next frame. The existing display provider
still requests redraws. Pending messages return control to the message
loop; a bounded timeout defers the frame until a later display beat.
The default creation flags and scheduling remain unchanged when unset.

The gate retains a permit across unchanged views until an actual Present.
Creation, resize, and device recovery preserve the waitable flag and handle
ownership. The fork logs its queue-latency readback and scheduling counters.

This branch does not include the independent device queue-limit patch.
Waitable chains have their own queue policy; this experiment therefore
measures the complete waitable-chain behavior, not a pure timing change
with an otherwise identical device queue. Compare both against the same
unchanged control before considering a combination.

The new fork module passed the Windows API probe using `windows` 0.61.3.
The full application build and matched Windows measurements are pending.
First paint, a page animating without input, scrolling, returning from
idle, resize, and device recovery still need runtime verification.

Report CEF paints and fresh surface submissions alongside the browser
preparation and paint-to-surface timers. The gate runs before those timers;
its wait duration must be reported separately. Successful Present calls
are not physical display scans. No performance benefit is established yet.
