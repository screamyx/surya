# Wave 3: wiring the panes into the shell

Owner seat: surya-files. Phase 2 starts on the coordinator's GO, after `feat/surya-look` (PR #2) and `feat/cef-pane` merge.
Sources read for this: `shell.rs` on `origin/main`, `origin/feat/surya-look`, `origin/feat/cef-pane`; `crates/ui/src/files/` (PR #11), `crates/ui/src/tasks/` (feat/tasks-ui), the inbox brief (`/tmp/surya-inbox-ui.md`, branch not pushed at 08:05); `docs/decisions.md` 20 and 22.

## What lands where

```
+------+----------------------------------+-----------------------------+
| rail |  main feed                       |  right pane (tabbed host)   |
| Agents| [InboxPane, when needs-you > 0] |  [Browser | Files | Tasks]  |
| Rail | transcript / cards               |  one RightSurface per tab   |
|      | composer pill                    |                             |
+------+----------------------------------+-----------------------------+
```

| Pane | Module (branch) | Constructor | Where it mounts |
|---|---|---|---|
| Browser | `browser_pane.rs` (feat/cef-pane) | `BrowserPane::new` | `RightSurface::Browser`, already wired by cef behind `feature = "browser"` |
| Files | `files/` (PR #11) | `FilesPane::new(engine: EngineHandle, space_id)` | new `RightSurface::Files` |
| Tasks | `tasks/` (feat/tasks-ui) | `TasksPane::new(client: Arc<RpcClient>, space, space_name)` | new `RightSurface::Tasks` |
| Inbox | `inbox/` (feat/inbox-ui) | `InboxPane::new(client, space)` | top of `render_main`, shown while needs-you items exist (decision 20) |
| Agents rail | `inbox/agents.rs` | `AgentsRail::new(client)` | inside `render_sidebar`, above the chat list |

Constructor mismatch to settle first: Files takes `EngineHandle`, Tasks and Inbox take `Arc<RpcClient>`. `RpcClient` is not `Clone`, so the shell can only hand out `EngineHandle` (from `AppState::engine()`). Phase 2 changes Tasks and Inbox to take `EngineHandle` (one-line constructor edits on their modules) rather than adding a second client type to the shell.

## The right pane is already a tabbed host

comet's shell keeps `right_tabs: HashMap<String, Vec<RightSurface>>` per panel key, with `set_right_active`, `toggle_right_pane`, `right_pane_open`, `resolved_right_active`, a surface picker (`render_surface_picker`, the `surface-card-*` rows) and a tab strip (`render_right_tab_strip`). cef's branch adds `RightSurface::Browser` by following that pattern exactly. Files and Tasks do the same. No new pane container.

## shell.rs edit points (function names, `origin/main` line numbers)

1. `pub enum RightSurface` (395): add `Files` and `Tasks` (unit variants, one per panel key like `Browser`).
2. `pub struct Shell` (1033 area): add `files_pane: Option<Entity<files::FilesPane>>`, `tasks_pane: Option<Entity<tasks::TasksPane>>`, `inbox_pane: Option<Entity<inbox::InboxPane>>`, `agents_rail: Option<Entity<inbox::AgentsRail>>`; init to `None` in `Shell::new` (1199).
3. Tab title match (1889 area, the `(*surface, title)` map): `Files => "Files"`, `Tasks => "Tasks"`.
4. `set_right_active` (1958) and the close-tab match (2183): Files and Tasks arms, no teardown needed (the entities stay cached; dropping the tab does not drop the pane).
5. `render_right_pane` (6116), the surface match at 6147: `Files => self.files_pane(cx).into_any_element()`, same for Tasks. Lazy getters mirror cef's `browser_pane(&mut self, cx)`: build once with the current space, cache on `self`.
6. `render_surface_picker` (6235): two more `row("surface-card-files", icons::FOLDER, "Files")` and `row("surface-card-tasks", icons::CHECK, "Tasks")` after the Terminal and Git rows, calling `add_files_surface` / `add_tasks_surface` (copies of cef's `add_browser_surface`).
7. Tab strip icon match (`render_right_tab_strip`, 6393): icon per surface.
8. `render_titlebar_cluster` (3565): no new buttons. The picker and keys open panes; the titlebar keeps cef's globe only.
9. `render_main` (5656): prepend `self.inbox_pane(cx)` when `AppState` reports needs-you items for the current space; hide it with `motion::COLLAPSE` when the count drops to zero.
10. `render_sidebar` (3831): `self.agents_rail(cx)` above the chat list.
11. `actions!(shell, [...])` (65) and `apply_keymap` (246): add `ToggleFiles`, `ToggleTasks`, `ToggleInbox`; bind `mod-shift-f`, `mod-shift-t`, `mod-shift-i` via `platform_combo`. cef owns the browser key.
12. Space change: `panel_key` (1770) already scopes tabs per chat/space. The cached Files and Tasks panes are keyed to a space, so the getters rebuild when `space_id` changes (compare against a stored id).

## Init calls (`lib.rs::run_app`, in this order)

```
composer::init(cx);
files::init(cx);      // FilesEditor key context, after composer
terminal::panel::init(cx);
tasks::init(cx);      // only if feat/tasks-ui ships one
inbox::init(cx);      // only if feat/inbox-ui ships one
app_menus::init(cx);
```

## Motion

Every open and close animates (owner, 05:1x: "everything should have one").

| Transition | Existing token | Note |
|---|---|---|
| right pane open / close | `right_tween` (WidthTween, `motion::RESIZE`, 200 ms ease-out) | already drives the pane width; Files and Tasks inherit it |
| tab switch inside the pane | `motion::TAB_SLIDE` (150 ms) | tab strip already uses it |
| inbox appears / disappears in the feed | `motion::COLLAPSE` (180 ms) height tween | new use, same helper as sidebar disclosures |
| agents rail rows | `motion::FADE_QUICK` (150 ms) | per row on state change |
| floating panel chrome | `surya::panel(theme, ELEVATION_PANEL)`, `PANEL_GAP`, `CANVAS_INSET` | from feat/surya-look; the right pane and main card already use them after that merge |

`motion::MotionMode` (feat/surya-look) gates all of it; reduced motion collapses every tween to its end state.

## Keys

| Key | Opens |
|---|---|
| mod-shift-f | Files tab (toggle) |
| mod-shift-t | Tasks tab (toggle) |
| mod-shift-i | Inbox (scroll the feed to it, or toggle if hidden by the user) |
| cef's binding | Browser tab |
| existing `ToggleChanges` | Git diff tab |

Inside the Files tab the tree owns up/down/left/right/enter/escape and the editor owns Ctrl/Cmd-S; both are already scoped to their focus handles, so they do not collide with the shell keymap.

## Proof plan (phase 2)

`cargo check -p zeron-ui`, `cargo test -p zeron-ui`, then a 5 s headless run per pane with `ZERON_OPEN_PANE=files|tasks|inbox|browser` (the env knob cef already uses as `ZERON_OPEN_BROWSER`), each printing `started=1 panics=0`.
