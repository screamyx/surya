# Probe: primary buttons paint as dim text on Windows (the "pill bug")

Date: 2026-09-05, 09:16 to 10:00 local. Seat surya-remote, round 4 on dtry.
Build: `main` at ca5abe9 (theme PR #2 merged as a139718), `cargo build --release -p zeron` on dtry, cargo 1.97.1, exe 09:18:50.
Engine: the Linux service unit on the tailnet, dialed from the saved Servers entry (`ws://pc-ajim:27700`, token).
Screenshots are native Windows captures cropped to the app window (1336x888), stored under `docs/images/`.

## Answer

The pill is not a theme or window-background problem.
Whole filled rectangles go missing from individual frames on the Windows renderer, and which ones go missing changes with input.
The same button paints in one frame and not in the next.

Evidence, all from this round:

| Shot | What it shows |
| --- | --- |
| `a2ui-dtry-r4-cards-3-5.png` | "Save follow-up" (form card) paints as a white pill. "Approve" (approval card) and "Open PR" (diff card) are dim text, no pill. |
| `a2ui-dtry-r4-cards-1-2.png` | Same chat after one wheel scroll: "Save follow-up" is now dim text, no pill. "Set follow-ups" (table card) dim too. |
| `a2ui-dtry-r4-cards-5-6.png` | "Approve" and "Open PR" dim. The secondary "Change it first" and the metric card tabs paint fine. |
| `quickstart-servers-add.png` | The dialog's "Add" button paints. In the same frame the Name and Port fields have their underline and the Host and Token fields do not. |
| `quickstart-servers.png` (light) and `dtry-r4-servers-dark.png` | Page-level "Add server" is dim text, no pill, in both themes, same as before the theme PR. |
| `dtry-r4-appearance-light.png` | Right after switching to Light: the System and Light preview cards are only partly drawn, "Add theme" is dim text, the accent swatches and the Glass control are half painted. |
| `dtry-r4-chat-dark.png`, `dtry-r4-chat-light.png` | The transcript in both themes. |

Two captures of the same view with no input in between were byte-identical (`cmp` equal, 0 of 1186368 pixels differ).
So the drop is stable within a frame and changes only when the scene is rebuilt.

The checkbox in the form card shows the same thing: a partial arc in one frame (`a2ui-dtry-r4-cards-3-5.png`) and a proper checked box in the next (`a2ui-dtry-r4-cards-1-2.png`), with no click on it.

On Linux the same card paints its primary pill: `docs/images/a2ui-x7-04-approval.png` (surya-a2ui, DISPLAY=:7) shows "Approve" as a dark filled pill in the light theme.

## Other things seen this round (not the pill bug)

Counters are `asked=N seen=M`.

- Send a message in the demo-cards chat over the remote engine: `asked=1 delivered=0`. The app shows "Not delivered, click to retry" and the sidebar row says "Failed". The engine spawned the harness (`claude ... --resume=66d5e2d4...`, child of the unit pid) and it sat idle: 2 s of CPU in 185 s, MCP servers up, waiting on stdin. The engine journal at `RUST_LOG=info` has no warning or error for it. The seeded demo cards disappeared from the transcript after the send.
- New chat from the sidebar "+" button and from `ctrl+n`: `asked=2 opened=0`.
- Switch to the other session by clicking its sidebar row: `asked=2 switched=0`. The click only revealed the row's hover "Archive" affordance.
- Cancel on the Add server dialog keeps the draft: reopening shows the previous values.
- The Type tool of the windows-dtry MCP pastes long strings through the clipboard and once pasted stale clipboard text instead; that frame was discarded.

## Why the Opaque experiment was not run

The queued experiment was "force `WindowBackgroundAppearance::Opaque` in `open_main_window`".
The code already resolves to Opaque on Windows, before and after the theme PR:

- `app/crates/ui/src/theme.rs:757`: `GLASS_ALPHA` is `1.0` on every platform but macOS.
- `app/crates/ui/src/theme.rs:854-856`: `is_glass()` is `glass().a < 1.0`, so it is false on Windows.
- `app/crates/ui/src/theme.rs:945-950`: `window_background_appearance()` returns `Blurred` only when `is_glass()`, else `Opaque`.
- `app/crates/ui/src/lib.rs:296` passes that into `WindowOptions.window_background`; `appearance.rs:331` re-applies it on theme swaps.
- The pre-#2 tree (`a139718^`) had the same three lines (`lib.rs:274`, `theme.rs:947-949`).

Forcing Opaque would change nothing, so the build was not spent.

## The colour path is fine

- a2ui buttons: `app/crates/a2ui/src/render.rs:362` paints `Primary` with `el.bg(theme.solid)`; `app/crates/ui/src/cards.rs:68` maps it from the ui theme; `theme.rs:984` (dark, `neutral(0.922)`) and `theme.rs:1076` (light, `neutral(0.205)`).
- Settings buttons: `app/crates/ui/src/popover.rs:1025` `btn_primary` paints `.bg(theme.text)`.
- Both helpers paint correctly in some frames (dialog "Add", form "Save follow-up"), so the values are right.

## Where to look next

The missing primitives are all filled quads with rounded corners or 1px underlines, drawn inside the scrolled transcript or the settings page, while the same element type inside the modal usually paints.
Reproduce: launch with `ZERON_DEMO_CARDS`, scroll the demo chat one notch at a time, capture each frame, diff.
Start in the Windows quad batching in the vendored gpui renderer (instance buffer boundaries, or the primitive sort by order/z when a scroll offset moves elements).
Checked and ruled out: window background mode, theme colours, macOS-only glass path.
