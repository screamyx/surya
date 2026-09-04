# Critique, round 1: after the token pass

Date 2026-09-05.
Target `mockup/shots`, all 24 re-shot after the change.
This covers the token pass only. Per-screen layout was held at the coordinator's instruction (03:07) until the research synthesis landed, so the composition findings from the baseline are still open by design.

## Verdict

**Rework, not reject.**

The theme is no longer stock. The type has a voice, the surfaces have an order, and one accent means one thing.
What is still wrong is exactly what was held: the screens are still shaped the same way.

## What changed, measured

Every number here came from the live app in headless Chrome, on all twelve routes. Neither `taste_audit.mjs` nor `slop_tells.mjs` ran: playwright is not installed in this repo and both scripts exit early without it.

| Check | Baseline | Now |
|---|---|---|
| Routes clearing the 2.5x heading-to-body bar | 1 of 12 (catalog, 2.57) | 9 of 12 |
| Distinct type sizes on Home | 6 (10, 12, 13, 14, 16, 24) | 5 (10, 12, 13, 15, 40) |
| Largest heading on Home | 24px over 14px body, 1.71x | 40px over 13px body, 3.08x |
| Text contrast failures, light, 12 routes | 0 measured at baseline (not run) | 0 |
| Text contrast failures, dark, 12 routes | not run | 0 |
| Horizontal overflow at 390 / 320 / 280 | not run | 0 at all three widths |
| Page background | `oklch(1 0 0)`, pure white | `oklch(0.966 0.009 100)`, warm parchment |
| Card surface vs page | identical, both `oklch(1 0 0)` | three steps: page 0.966, card 0.988, popover 0.998 |
| Radius values in use on Home | 8px on 40 elements, one language | 6 controls, 8 rows, 14 cards, 36 panels, pill for status |
| Accent | none; the only chroma was the destructive red | one terracotta, `oklch(0.558 0.135 39)` |
| Destructive vs accent hue separation | n/a | 18 vs 39, so a red badge no longer reads as an orange one |

## Findings still open

| # | Severity | Finding | Evidence | Owner |
|---|---|---|---|---|
| 1 | Critical | Home is still an inventory grouped by server, not a queue grouped by state | `home-desktop.png`: Needs you panel, then one card per workspace under a server heading | layout round, see `docs/research/synthesis.md` |
| 2 | Critical | Agents still shows three identical stat tiles above 600px of empty page | `agents-desktop.png` | layout round |
| 3 | Major | Review is four routes: feed, files, preview, result | `routes.tsx`, and four of the ten researchers named it independently | layout round |
| 4 | Major | Agent feed is 1.25x, preview 1.08x, files 1.00x heading-to-body | live measurement, all three are editor or transcript surfaces with no page title | layout round; these get a session header |
| 5 | Major | Messages leaves a 300px void between the thread header and the first message | `messages-desktop.png` | layout round |
| 6 | Minor | The inbox filter strip scrolls sideways at 390px with no visible cue that it does | `inbox-phone.png`, "Results" is cut at the right edge | add an edge fade |
| 7 | Minor | Home's row actions are full-width outline buttons on phone and read as inert | `home-phone.png`, three "Open" boxes | resolved by the Home rebuild |

## Fixed on the way past

Small things that were wrong before and are not now.

- The sidebar count badge on the active workspace row rendered ink on red at 3.16:1, because the sidebar's own active-row rule repainted it. Now carries the destructive pair through hover and active.
- Inactive tab labels used shadcn's stock `text-foreground/60`, which measured 4.04:1. Now the muted-foreground token, 5.6:1.
- `text-muted-foreground/70` on idle agent rows measured 2.22:1. The token without the opacity is 5.98:1.
- CodeMirror was pinned to `theme="light"`, so in dark mode the editor showed ivory text on a white page at 1.2:1. The editor now renders from the same tokens as everything else, with a five-colour syntax palette measured at 4.7:1 on the editor surface in both themes.
- The preview toolbar carried `shrink-0`, which pushed the page to 346px wide inside a 320px viewport. The agent model badge did the same at 280px.
- The Stop control on phone was a hollow lucide `Square` with the label hidden, which reads as an unchecked checkbox. It is a filled square now.
- The ship stepper drew finished steps in the accent and the step in flight in plain black, so the least important marker was the loudest. The current step is now the ringed one.

## Scope

This is judgement plus measurement, and they are separate.
The measurements above are real and reproducible with `chrome-devtools-axi`.
Whether the result is *good* is answered by looking at the shots, and by the layout round that has not happened yet.
