# Design critique, round 4: RC1 acceptance record

2026-09-05 12:50, seat surya-tasks, persona `.claude/agents/design-critic.md`, judged against decision 22, `docs/design-brief.md` and the round-3 record `docs/critique/round-3-2026-09-05.md`.
Rendered from RC1 main `e116422` ("Merge PR #56: pin gpui to screamyx/gpui-surya"), `--features browser` build, 8 min 14 s.
Not in RC1 and therefore still open here: #62 (browser launch knob) and #63 (tasks columns scroll), both merged to main after the cut and due in RC2.

## What was rendered

`app/scripts/critic-shots.sh` on the headless Xorg `:7`, 60 s settle per frame, three rig runs:

| Run | Knobs | Frames | colours | panics |
| --- | --- | --- | --- | --- |
| Shell + panes | default GEOMS, `SURYA_SHOT_PANES="files tasks browser"`, `SURYA_SHOT_BROWSER_CLICK=129,69` | `round-4-shell-{light,dark}-{1440x900,1100x700}.png`, `round-4-shell-{files,tasks,browser}-{light,dark}.png` | 10019-17046 | 0 |
| Cards | `ZERON_MOCK_CARDS=crates/a2ui/fixtures`, 1440x900 | `round-4-cards-{light,dark}-1440x900.png` | 15441 / 15546 | 0 |
| Inbox | `ZERON_MOCK_QUESTION=1`, both sizes | `round-4-inbox-{light,dark}-{1440x900,1100x700}.png` | 12647-14885 | 0 |

Counters: `seeded=1 space=1 chat=1 run=1 tasks=4` per run, `shot=1 panics=0` for each of the 18 frames, 0 black.

New in the rig this round: the box has no pointer tool, so `scripts/x7-click.py` sends one XTEST click (python-xlib in a uv venv), and `SHOT_CLICK=x,y` in `critic-shots.sh` fires it after the settle.
That is how the Browser surface opened on RC1, which still has the round-3 B1 knob bug: the log `round-4-shell-browser-light.log` goes from "visible=0 frames=0 size=0x0" to `visible=1 frames=2 size=518x806` after "click: asked=1 sent=1 at 129,69".
The inbox needed no knob: `shell.rs` `inbox_visible()` shows the needs-you list whenever `inbox_count(cx) > 0`, and the mock harness's question mode leaves two questions waiting.

Still not judged: drag, tabs, the edit sheet, answering a question (one click per frame is all the rig does), phone width (window floor 900x600), motion.

## Verdict per screen

| Screen | Verdict | Notes |
| --- | --- | --- |
| Shell, light and dark, 1440x900 | **accept** | Round-3 state plus #50: the agents tree ("Done" group, one row) sits between the entries and "All projects". Canvas `#0b0a08`, panels `#332e26` in dark as in round 3. |
| Shell, light and dark, 1100x700 | **accept** | Holds; nothing clips. |
| Files pane | **accept** | Unchanged from round 3. |
| Tasks pane | **fix in RC2** | Third column still cut at "Don" (round-3 N3); the fix is #63, merged after the cut. |
| Browser pane, light | **accept** | Tab "Example Do..." with the globe, back / forward greyed, reload, URL pill "https://example.com/", the page painted. The one chrome the pane needs, nothing more. |
| Browser pane, dark | **accept, one nit (R1)** | Same chrome in dark; the page itself is a white slab because example.com has no dark styling. |
| Cards, light and dark | **accept** | Identical to round 3: neutral paths and code, muted bars with one `text` bar, `text` tab underline. N4 (no action wears the accent) stands as a theme question. |
| Inbox, 1440x900 | **fix (I1, I2, I3)** | Rail: "Needs you" badge "2" in the accent, "Waiting for you" group with the chat, session chip "Input". Feed: two Question cards stacked above the page title, the question sheet below. It works, but the page title has lost the top of the panel and the same question is offered twice. |
| Inbox, 1100x700 | **fix (I4)** | The question sheet is drawn over the page title: "Wire the Tasks pane into the shell" shows through the sheet's top edge under "QUESTION 1/2" in both appearances. |

## Round-3 findings, rechecked on RC1

| # | Finding | RC1 |
| --- | --- | --- |
| N1 | Page title in Geist sans, brief wants the serif | still open, no serif face bundled |
| N2 | Chat title shown three times | still open, and the agents tree makes it four in the plain shell (titlebar, page title, tree row, session row) |
| N3 | Tasks third column clipped | still open on RC1, fixed on main by #63 |
| N4 | No action wears the accent | still open (theme question) |
| B1 | `ZERON_OPEN_PANE=browser` dead at launch | still open on RC1, fixed on main by #62; the rig's click is the RC1 workaround |
| L2 | Transcript measure 736 px | still open, owner call |

## New findings, by owner

Shell and inbox (surya-files, #50 was theirs):

| # | Severity | Finding | Evidence | Fix |
| --- | --- | --- | --- | --- |
| I1 | Major | The needs-you list renders above the page title, so the title tier no longer heads the panel | `round-4-inbox-light-1440x900.png`: cards at y 120-380, title at y 410. `shell.rs:6284`: "the page title below drops its titlebar clearance when the list above has already cleared it" is deliberate; `docs/wave3-shell-wiring.md:97` planned the opposite: "a title row in the feed panel ... at the top of `render_main`, above the inbox slot" | Title row first, inbox list under it; the list is a queue inside the page, not a banner over it |
| I2 | Major | One question, two answer surfaces in one view: the inbox card for the current chat shows the same options as the question sheet below it | `round-4-inbox-light-1440x900.png`: "Which sync strategy should the rewrite use?" with three option buttons at y 360, and again in the sheet at y 650 | Collapse the current chat's own item to a one-line row (or hide it) while its sheet is open; keep full cards for other chats |
| I3 | Minor | "Question" twice per card: the badge and then a bold "Question" label | inbox cards, both frames; `inbox/model.rs:51` badge text, `inbox/mod.rs:146` item title | Drop the bold label when it equals the badge |
| I4 | Major | At 1100x700 the question sheet overlaps the page title; the title shows through the sheet's translucent top | `round-4-inbox-{light,dark}-1100x700.png` around y 510 | The sheet should push the title (flex column) or the title should hide while a sheet is up; a translucent sheet must not sit on text |

Browser pane (surya-cef):

| # | Severity | Finding | Evidence | Fix |
| --- | --- | --- | --- | --- |
| R1 | Minor | The pane does not pass the app's appearance to the page: in dark mode a site with dark styling would still render light | `round-4-shell-browser-dark.png` (example.com has no dark styling, so this frame only shows the chrome / page contrast); `crates/browser/src/lib.rs:54` mentions a dark-canvas text bug but no `prefers-color-scheme` | Set CEF's preferred colour scheme from the theme appearance (Chromium `--force-dark-mode` is the blunt knob; the per-browser `SetPreferredColorScheme` is the right one) |

Rig (mine, in this PR): `x7-click.py`, `SHOT_CLICK`, `SURYA_SHOT_BROWSER_CLICK`.

## What is actually good

- The inbox self-shows: no click, no knob, the badge and the rail group and the session chip all agree, and the cards' option buttons are real controls.
- The browser pane's chrome is the quietest in the app: a tab, three greyed controls, one pill.
- Nothing regressed from round 3: every accepted item there is still accepted here.

## RC1 verdict

Ship RC1 as a candidate.
Blocking for RC2: I1, I2, I4 (the inbox and the page title fight for the top of the panel), plus the two fixes already on main (#62, #63).
Carry: N1, N2, N4, L2, I3, R1.
