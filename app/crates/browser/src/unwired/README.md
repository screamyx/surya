# Not wired

Verbatim copies from `haktui/crates/haktui/src/browser/` on 2026-09-05, kept
for the seat that wires pins. Not declared as modules, so they do not compile
here; each still refers to haktui's `super::` neighbours and its herdr pin
sink.

| file | what | haktui decision |
| --- | --- | --- |
| `pin.rs` | alt+click a page element, compose a note, send it to the agent | D43 |

Wired on 2026-09-05 (feat/browser-cdp), so no longer here: `emulation.rs` and
`device.rs` became `src/emulation.rs` (four presets, user agent override, no
touch circle); their DevTools route is `src/devtools.rs`.
