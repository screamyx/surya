# Design critique, round 5: RC2 verification of the round-4 inbox findings

2026-09-05 14:55, seat surya-tasks, persona `.claude/agents/design-critic.md`, judged against the round-4 record `docs/critique/round-4-rc1.md`.
Rendered from main `75ec9a3` ("inbox: one question, one place to answer it (round 4 I2, I3) (#72)"), which also carries #71 (title order, sheet) and #73 (browser follows the appearance, R1). `--features browser` build, 3 min 44 s.

## What was rendered

`app/scripts/critic-shots.sh` on `:7`, 60 s settle, two runs:

| Run | Knobs | Frames | colours | panics |
| --- | --- | --- | --- | --- |
| Inbox | `SURYA_MOCK_QUESTION=1`, 1440x900 and 1100x700, light and dark | `round-5-inbox-{light,dark}-{1440x900,1100x700}.png` | 11032-12316 | 0 |
| Browser | `SURYA_SHOT_PANES=browser` (the #62 launch knob, no click), `SURYA_BROWSER_URL` = a `data:` page with a `prefers-color-scheme: dark` rule | `round-5-browser-{light,dark}.png` | 15586 / 15284 | 0 |

Counters: `seeded=1 space=1 chat=1 run=1 tasks=4` per run, `shot=1 panics=0` for each of the 6 frames, 0 black.
Browser log (dark): "browser: switches [..., \"force-dark-mode\"] scheme=Dark", pane `visible=1 frames=1 size=518x806`, opened by the launch knob alone (B1 closed by #62).

## Round-4 findings, verdict

| # | Finding | Verdict | Evidence |
| --- | --- | --- | --- |
| I1 | Needs-you list rendered above the page title | **Accepted from code, order not observable in this rig** | `shell.rs:6305`: "Critique round 4, I4: while a question sheet is up the title hides"; `shell.rs:6378` renders the title only when `!sheet_up`. The mock question always raises the sheet for the one chat the rig seeds, so every inbox frame shows the list with the title correctly hidden, never list-under-title. A needs-you item from a second chat would show the order; the rig has one chat. |
| I2 | One question offered twice (card and sheet) | **Accepted** | Both cards are one line each: "Question  Which suites should gate the merge?  answer below", no option buttons; the sheet is the only place with options. All four frames. |
| I3 | "Question" badge plus bold "Question" label | **Accepted** | Cards carry the badge and the question text only. |
| I4 | Sheet drawn over the page title at 1100x700 | **Accepted** | `round-5-inbox-{light,dark}-1100x700.png`: no title under the sheet; the feed shows "Before I wire the reconciliation path..." and the "Question  Awaiting your answer..." row, then the sheet, no overlap. |
| R1 | Browser pane ignores the app's appearance | **Accepted** | Same `data:` page: light frame black-on-white, dark frame cream `#f5f0e8` on `#161412` as its dark rule says. Log `scheme=Dark`. |
| N2 | Chat title shown three or four times | **Improved** | The titlebar now shows only "surya @ devbox" (all six frames); the title remains in the page-title tier, the agents-tree row and the session row. |

## Still open (carried)

N1 page title in Geist sans (theme), N4 no action wears the accent (theme), L2 measure 736 px (owner call), N2 residual (tree row and session row both name the chat).

## What is actually good

- The inbox now reads as a queue inside the page: two quiet one-line rows, "answer below" pointing at the one sheet.
- A dark-aware page goes dark with the app. The browser pane is no longer a white slab in dark mode.

## RC2 verdict

The four inbox findings and R1 are closed on `75ec9a3`. Nothing new found in these six frames.
