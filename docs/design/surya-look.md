# The surya look

Floating rounded panels on a soft canvas.
Light first, dark as good.
Decision 22, from the owner's three sketches: "inspiration: raycast, craft, cleanshot".

![light](surya-spec-light.png)

![dark](surya-spec-dark.png)

Those two images are the palette and the geometry rendered from the real token
values, not a screenshot of the app.
The app could not be photographed on this machine.
See "What is not proven" at the bottom.

## The layout

```
+------------------------------------------------------------------+
|  16px canvas margin                                              |
|  +----------+ 8 +---------------------------+ 8 +-------------+  |
|  |          |   |                           |   | url bar     |  |
|  |  rail    |   |   conversation            |   |-------------|  |
|  |  card    |   |                           |   | +---------+ |  |
|  |          |   |                           |   | | files   | |  |
|  |          |   |   +-------------------+   |   | | card    | |  |
|  |          |   |   |  composer  pill   |   |   | +---------+ |  |
|  +----------+   +---------------------------+   +-------------+  |
+------------------------------------------------------------------+
```

Three panels, each its own card.
The canvas shows between them.
The file browser is a card floating over the right pane, CleanShot style.

## The numbers

| Token | Value | Where |
|---|---|---|
| Canvas margin | 16px | window edge to the outermost panel |
| Panel seam | 8px | between two panels |
| Panel radius | 14px | the three panels |
| Float radius | 12px | a card resting on a panel |
| Control radius | 6px | chips, inputs, buttons |
| Composer radius | 26px | pinned, so a growing pill stays a pill |

The margin is twice the seam on purpose.
At 10 against 8 the two read as the same measurement and the panels looked scattered.

## The type

| Step | Size | Weight | Used for |
|---|---|---|---|
| Display | 28px | 600 | session title, the one heading on an empty window |
| Title | 19px | 600 | panel and page headers |
| Body | 14px | 400 | prose and rows |
| Label | 13px | 500 | buttons and chips |
| Caption | 11px | 500 | timestamps, counts |
| Mono | 12.5px | 400 | paths, ids, branches |

Display over body is **2.00**.
Body over caption is **1.27**.
Counts and PR numbers use tabular figures, so a number that changes in place does not jitter.

## The colour

One accent, terracotta, and warm neutrals.
Never pure black on pure white.

| | Light | Dark |
|---|---|---|
| Canvas | `#eae5dc` | `#0b0a08` |
| Panel | `#fdfcfa` | `#232019` |
| Card on a panel | `#fdfcfa` | `#2e2922` |
| Text | `#26221e` | `#ece6dc` |
| Accent | `#b4552d` | `#e08a5a` |

Light and dark are not each other inverted.
In light the panel is the brightest thing and the card on top of it stays level, separated by a hairline and a shadow.
In dark every layer climbs: canvas, then panel, then card.
A dark card painted in the panel's own tone disappears into it, which is exactly what the first render showed.

## Depth without glass

Every panel gets its depth from four things that work on any build of gpui:
the canvas tone under it, a hairline border, one soft shadow, and the gap.

Not from backdrop blur.
The browser pane may swap comet's gpui fork for Zed's own, which has no blur and no transparent window.
Glass still works where the fork offers it, but it is never the mechanism.

Two elevation levels and no more: a panel on the canvas, and a card on a panel.
One shadow each, never a stack - layered shadows sum into a grey rim on a light field.

## Motion

The panels already slide open and closed on comet's own width tween.
What was missing was the off switch.

Settings now carries a **Motion** row with three states.

| Choice | Effect |
|---|---|
| System | follows this machine's reduce-motion setting |
| Full | full travel |
| Reduced | panels change state without travel, feedback stays |

`System` reads the macOS accessibility flag and GNOME's `enable-animations`.
Windows has no reader yet and falls back to full motion.
This closes the gap `docs/PARITY.md` 1.12 records.

## What is not proven

No screenshot of the running app.
gpui paints to a Vulkan swapchain that never presents into an Xvfb drawable on this machine.
Forcing the software renderer with `VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.json` selects llvmpipe and still grabs black,
so the GPU was never the cause.

What the images above do prove: the palette and the geometry, drawn to the same numbers the Rust code reads.
What they do not prove: that the shell code lays out the way this mock does.
That needs a real frame from a real window.
