# Critique, baseline: the surya mockup before the restyle

Date 2026-09-05.
Target `mockup/shots` (24 PNGs, twelve surfaces at 1440x900 and 390x844) plus the live app at `http://127.0.0.1:4790`.
Stance and format from `.claude/agents/design-critic.md`.

## How this was measured, honestly

`scripts/taste_audit.mjs` and `scripts/slop_tells.mjs` did not run.
Playwright is not installed in this repo, and both scripts exit early when it is missing (`taste_audit: playwright not installed - SKIPPED`).
Instead every number below came from the live app in headless Chrome through `chrome-devtools-axi eval`, running the same checks those scripts run: largest heading over dominant body size, distinct radii, distinct shadows, distinct border and text colours, and sibling groups of three or more with identical class and near-identical box.
The screenshots were opened and looked at, one by one.

## Verdict

**Reject.**
This is a correct application of a stock theme, not a designed product.
Nothing here is broken, and nothing here is chosen.

### Three reasons a senior designer sends this back

1. **The type scale does not exist.**
   Across the twelve surfaces the largest heading is 1.08x to 2.57x the body size, and ten of the twelve are below 1.8x.
   Files is 14px over 13px (1.08x). Preview is 14px over 12px (1.17x). Agent feed is 16px over 12px (1.33x).
   The kit's own bar is 2.5x, and `.claude/rules/components.md` names the failure exactly: "24px over 16px is bold body text."
   That is what every heading in this mockup is.

2. **Every surface is the same surface.**
   White card, one hairline, one 8px radius, no elevation, stacked in one centred column.
   On Home the "Needs you" panel, the studio card and the laptop card are the same object at the same weight.
   On Agents, three stat tiles of identical size sit in a row: 2 Working, 1 Needs you, 2 Done, none more important than the others.
   On Tasks, four equal columns of equal cards.
   The eye lands nowhere on any of them.

3. **The page is pure white and the ink is one grey.**
   `--background` is `oklch(1 0 0)`, which is `#ffffff` exactly.
   `--card` is the same value, so a card on the page is invisible except for its border.
   48 of the 100 text nodes on Home are one muted grey (`oklch(0.556 0 0)`) and 35 are one near-black (`oklch(0.145 0 0)`): two colours doing all the work, with no third level for a label or a number.

## Findings

| # | Severity | Finding | Evidence | Fix |
|---|---|---|---|---|
| 1 | Critical | Type scale is flat on every screen | ratio 1.08 (files), 1.17 (preview), 1.33 (agent feed, tasks), 1.43 (inbox), 1.5 (result), 1.71 (home, new ask, agents), 2.14 (settings), 2.57 (catalog). Ten of twelve below the 2.5 bar | Real display sizes on Home, Result and Settings headers. Drop body to a 13/15 pairing so the jump is visible |
| 2 | Critical | No focal point on Home, Agents, Tasks, Cards, Needs you | `agents-desktop.png`: three stat tiles, identical box, identical weight. `home-desktop.png`: three cards of one weight in one column. `tasks-desktop.png`: four equal columns | One lead element per screen with more size, weight or its own row. Demote the rest |
| 3 | Critical | Card surface equals page surface | `--card: oklch(1 0 0)` and `--background: oklch(1 0 0)` in `mockup/src/index.css:56,58`. A card is a 1px line on white | Off-white page, white card, or an inset surface. Give the elevation scale something to sit on |
| 4 | Major | One radius everywhere | 8px on 40 elements on Home, the single most common value on ten of twelve screens | A radius language: tight on inputs and chips, wider on panels, pill only on status |
| 5 | Major | No elevation scale | Home carries three distinct shadow values and only five elements have any shadow at all. Messages carries one | Two real levels: resting panels flat with a border, floating things (dialog, sheet, popover) lifted |
| 6 | Major | One hairline colour | `oklch(0.922 0 0)` on every bordered element on Home (10 of 10). Nothing reads as stronger or weaker | Two border weights, and a stronger one for the element that leads |
| 7 | Major | Pure white page | `oklch(1 0 0)`, the stock nova default, untouched | Off-white. `taste/design-taste.md` bans the pure pair |
| 8 | Major | No accent, so nothing is emphasised | The only chromatic value on Home is `oklch(0.577 0.245 27.325)`, the destructive red, used 11 times as a status dot and a badge. Every affirmative button is near-black | One accent that means "this is the thing to do", separate from the red that means "something is wrong" |
| 9 | Major | Screens end without ending | `agents-desktop.png` has 240px of content and 660px of nothing under it. `tasks-desktop.png` has four empty column wells below one card each | Fill the space or close the page. `.claude/rules/components.md`: "A page ends on purpose" |
| 10 | Minor | Counts are not tabular | Stat tiles on Agents, badge counts in the rail, `+22 / -9` on Result all use proportional figures | `font-variant-numeric: tabular-nums` on every count and diff stat |
| 11 | Minor | Sidebar is the same white as the page | `--sidebar: oklch(0.985 0 0)` against `--background: oklch(1 0 0)`, a difference of 1.5% lightness. The rail does not read as a rail | Separate the chrome from the canvas |

## What is good, and stays

- The copy. No filler, no em dashes, no "Lorem", no "No data" empty states. "Answer these and your agents carry on" is a real sentence.
- The data. Real agent names, real file paths, real diffs, believable timestamps.
- The agent feed layout. The tool-call rows, the diff card and the composer are the one place with a genuine information hierarchy.
- No horizontal overflow at 1440 or 390 on any of the twelve routes (`scrollW` equals `innerW` on all).
- No emoji anywhere.

## Scope

This is judgement, not measurement. The numbers above are real and reproducible.
Whether the result is *good* after the fix is a separate question, answered by looking again.
