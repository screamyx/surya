# Not wired

Verbatim copies from `haktui/crates/haktui/src/browser/` on 2026-09-05, kept
for the seat that wires pins, phone emulation and CDP. Not declared as
modules, so they do not compile here; each still refers to haktui's `super::`
neighbours and its herdr pin sink.

| file | what | haktui decision |
| --- | --- | --- |
| `pin.rs` | alt+click a page element, compose a note, send it to the agent | D43 |
| `emulation.rs` | phone view mode through DevTools device metrics | D48 |
| `device.rs` | the phone's size and how it fits the pane | D48 |
