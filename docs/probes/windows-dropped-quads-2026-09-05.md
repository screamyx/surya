# Probe: filled quads drop on Windows (the "pill bug")

## Answer (round 5, seat surya-cef, 11:10 to 11:30)

The Windows shader reads the quad buffer with the wrong stride.
The fork added a `fade: EdgeFadeParams` field (8 floats, 32 bytes) to the GPU-facing `Quad` and `PolychromeSprite` structs and taught the Metal and wgpu shaders about it, but never touched the DirectX HLSL.
On Windows the instance buffer is built from the Rust struct (192 bytes per quad) while the shader still declares the old 160-byte layout, so every quad after the first in a batch is read from the wrong offset.
Which quads survive depends on where each batch starts, so the drops move whenever the scene is rebuilt (a scroll, a theme switch) and stay put between rebuilds, exactly what round 4 measured.

Sources, all read in the vendored fork checkout `~/.cargo/git/checkouts/zed-d032abea1bc23d84/e2ddcc6` (wingleeio/zed rev e2ddcc68, the rev `app/Cargo.toml:71` pins):

| Side | File and line | What it says |
| --- | --- | --- |
| Rust quad | `crates/gpui/src/scene.rs:577-588` | `pub struct Quad { ... pub border_widths: Edges<ScaledPixels>, pub fade: EdgeFadeParams, }` |
| Rust image | `crates/gpui/src/scene.rs:807-817` | `pub struct PolychromeSprite { ... pub corner_radii: Corners<ScaledPixels>, pub fade: EdgeFadeParams, pub tile: AtlasTile, }` |
| Rust fade | `crates/gpui/src/scene.rs:563-572` | `pub struct EdgeFadeParams { pub top_y: f32, pub bottom_y: f32, pub band_top: f32, pub band_bottom: f32, pub left_x: f32, pub right_x: f32, pub band_left: f32, pub band_right: f32, }` |
| HLSL quad | `crates/gpui_windows/src/shaders.hlsl:496-505` | `struct Quad { uint order; uint border_style; Bounds bounds; Bounds content_mask; Background background; Hsla border_color; Corners corner_radii; Edges border_widths; };` (no fade) |
| HLSL image | `crates/gpui_windows/src/shaders.hlsl:1204-1213` | `struct PolychromeSprite { ... Corners corner_radii; AtlasTile tile; };` (no fade) |
| wgpu quad | `crates/gpui_wgpu/src/shaders.wgsl:550-560` | `struct Quad { ... border_widths: Edges, fade: EdgeFadeParams, }` (has fade, this is why Linux paints) |
| Metal | `crates/gpui_macos/src/shaders.metal:37,111,220` | `float edge_fade_alpha(float2 position, EdgeFadeParams fade);` and `background_color.a *= edge_fade;` (has fade) |
| Buffer stride | `crates/gpui_windows/src/directx_renderer.rs:1027` and `:1056` | `create_buffer(device, std::mem::size_of::<T>(), buffer_size)`; `:1508-1523` sets `StructureByteStride: element_size` with `D3D11_RESOURCE_MISC_BUFFER_STRUCTURED` |
| Draw | `crates/gpui_windows/src/directx_renderer.rs:506-526` `draw_quads` and `:1119-1141` `draw_range` | one `DrawInstanced` per batch over a view of `first_instance..instance_count` |
| CPU side | `crates/gpui/src/window.rs:4054`, `:4372`, `:4533` | `fade: self.scaled_edge_fade()` on every quad and image; `:3623-3660` fills it, zeroed outside a fade scope |

Byte sizes from the field lists above: `Background` is 72 bytes (tag 4, color_space 4, solid 16, angle 4, two 20-byte stops, pad 4).
HLSL `Quad` is 4+4+16+16+72+16+16+16 = 160 bytes, the Rust one 192.
HLSL `PolychromeSprite` is 4+4+4+4+16+16+16+32 = 96 bytes, the Rust one 128, and there the extra field sits before `tile`, so even the first image of a batch reads its atlas tile from the fade bytes.

Fork history (`git log` in `~/.cargo/git/db/zed-d032abea1bc23d84`):
`shaders.hlsl` was last changed by upstream on 2026-07-09 (`f281770034`, "Store GPU-facing bools as PaddedBool32").
The fade landed in `ef2f35c3d9` "per-pixel EdgeFade for quads and images (metal + wgpu)" and `5d1f83d9f2` "horizontal per-pixel EdgeFade", both touching `scene.rs`, `window.rs` and `shaders.wgsl` only.
`Shadow`, `Underline`, `MonochromeSprite`, `Background`, `AtlasTile` and `TransformationMatrix` match on both sides, which is why text, icons, shadows and real underlines keep painting.
The "underlines" that dropped in the Add server dialog are 1px border-only quads (`window.rs` `paint_quad` splits them into strips, more quads per batch), not `Underline` primitives.

Round 4's ruled-out list stands: window background mode and theme colours are not involved, and neither is the D3D11 blend state (`directx_renderer.rs:1411-1427` is upstream's SrcAlpha/InvSrcAlpha, unchanged).

## Reproduction on winbox (DEBUG build)

Box: winbox, NVIDIA GeForce RTX 4080 (plus the AMD iGPU), Windows PowerShell 5.1, cargo 1.97.1 MSVC.
Tree: this repo at 3407d5f (main 4737c75 plus PR #52), shipped to `E:\surya-cef`, own cargo home `E:\surya-cef-cargo` (a copy of surya-remote's) and target `E:\surya-cef-target`.
Engine: the remote service on this box (`ws://devbox:27700`, saved Servers entry), the app on the last selected chat "Display All Leads Table".
Shots: `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)` of the surya window from a session-1 scheduled task (`E:\surya-cef-pshot.ps1`), because the window came up behind the owner's terminal and Chrome and `SetForegroundWindow` from a task does not raise it.
`SURYA_DEMO_CARDS` was set but the fixture chat never seeded within the 22 s before the shot (no "demo cards seeded" line), so the comparison uses a real chat instead of the a2ui cards.

| Build | What changed | exe time | Shot |
| --- | --- | --- | --- |
| A | `cargo build -p surya` (debug), nothing patched | 11:20:41 | shot removed, see the element table below |
| B | same tree, only the vendored `shaders.hlsl` in `E:\surya-cef-cargo\git\checkouts\zed-d032abea1bc23d84\e2ddcc6\crates\gpui_windows\src\` patched by `E:\surya-cef-hlsl-patch.ps1` (adds `struct EdgeFadeParams` at line 88, `EdgeFadeParams fade;` at 516 in `Quad` and at 1224 in `PolychromeSprite`), then `cargo clean -p gpui_windows` and `cargo build -p surya` | 11:23:40 | shot removed, see the element table below |

Same window size (1336x888), same chat, same scroll position. Elements checked: asked=5.

| Element | A (unpatched) | B (fade field added) |
| --- | --- | --- |
| User bubble "reply" | bare text, no bubble | filled rounded bubble |
| User bubble "In one short line..." | filled | filled |
| Transcript column container (rounded card behind the chat) | missing, text sits on the window background | drawn, with its border |
| "L" avatar circle next to "Local only" | missing | drawn |
| "Thought process" chevron chip | bare arrow | round chip |

Dropped in A: seen=4 of 5. Dropped in B: seen=0 of 5.
The only difference between the two binaries is the 32-byte field in the two HLSL structs, so the stride mismatch is the cause, not the blend state, the atlas or the window background path.

Cargo notes for whoever repeats this: a change inside a git checkout under `CARGO_HOME` does not trigger a rebuild (`rerun-if-changed` on `shaders.hlsl` is not enough for a git dependency), hence the `cargo clean -p gpui_windows`.
The first debug build from a warm cargo home and a cold target took 3m18s; the patched rebuild 36s.
Left on winbox: `E:\surya-cef`, `E:\surya-cef-cargo`, `E:\surya-cef-target`, `E:\surya-cef-*.ps1`, the two shots; the scheduled tasks are deleted and the app is stopped.

## Fixed in RC1 (12:35, release build)

PR #56 pins `screamyx/gpui-surya` f910653 (the three struct lines plus `edge_fade_alpha` ported into both fragments); it is in RC1 main e116422.
surya-remote's RC1 release exe (`E:\surya-remote-target\release\surya.exe`, built 12:18:57 from e116422) on the same chat and window as the A/B above, PrintWindow shot, since removed: the "reply" and "reply with the single word mango" bubbles, the transcript container card, the "L" avatar circle and the "Thought process" chip all paint.
Checked elements asked=5, dropped seen=0.

## Fix as shipped (PR #56) and the earlier candidate

Add the field to the two HLSL structs so the stride matches again.
Ignoring the field in the shader is enough to stop the drops; it just means no per-pixel edge fade on Windows until the fragment shaders learn it like `shaders.wgsl` did.

1. `crates/gpui_windows/src/shaders.hlsl:88`, before `struct TransformationMatrix`: add `struct EdgeFadeParams { float top_y; float bottom_y; float band_top; float band_bottom; float left_x; float right_x; float band_left; float band_right; };`
2. `crates/gpui_windows/src/shaders.hlsl:504`, after `Edges border_widths;` in `struct Quad`: add `EdgeFadeParams fade;`
3. `crates/gpui_windows/src/shaders.hlsl:1211`, between `Corners corner_radii;` and `AtlasTile tile;` in `struct PolychromeSprite`: add `EdgeFadeParams fade;`

Follow-up for the fork, not for the RC: port `edge_fade_alpha` from `shaders.wgsl` into `quad_fragment` and `polychrome_sprite_fragment` so scroll-edge fades work on Windows too, and add a `const_assert`-style size check between `scene.rs` and each shader so the next new field cannot silently skip a backend.
The fix belongs in the gpui fork (a new rev pinned in `app/Cargo.toml:71-72`), not in this repo.

---

# Round 4 (surya-remote): primary buttons paint as dim text on Windows

Date: 2026-09-05, 09:16 to 10:00 local. Seat surya-remote, round 4 on winbox.
Build: `main` at ca5abe9 (theme PR #2 merged as a139718), `cargo build --release -p surya` on winbox, cargo 1.97.1, exe 09:18:50.
Engine: the Linux service unit on the tailnet, dialed from the saved Servers entry (`ws://devbox:27700`, token).
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
| Add server dialog, shot removed | The dialog's "Add" button paints. In the same frame the Name and Port fields have their underline and the Host and Token fields do not. |
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
Reproduce: launch with `SURYA_DEMO_CARDS`, scroll the demo chat one notch at a time, capture each frame, diff.
Start in the Windows quad batching in the vendored gpui renderer (instance buffer boundaries, or the primitive sort by order/z when a scroll offset moves elements).
Checked and ruled out: window background mode, theme colours, macOS-only glass path.
