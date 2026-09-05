# Design brief: surya 1.0 RC

Written 2026-09-05, before any code changed.
Required by `taste/design-taste.md` ("Brief Inference"), which forbids styling anything until domain, audience, mood, motion depth and reference anchor are decided.

## Domain

A web harness for Claude Code.
Agents run on machines you own, grouped by workspace, and you steer them.
The screens are dense and technical: file trees, diffs, tool calls, task boards, agent mail.

## Audience

One person, checking on work that is already running.
Half the sessions are a phone at 390px, held one-handed, often at night.
The other half is a 1440px desktop with the rail open.
This person is not browsing. They arrive with a question: what needs me, and what happened while I was gone.

## Mood adjective

**Warm workshop.**

Not cold, not futuristic, not another blue-grey dashboard.
The tool should feel like a well-lit desk with paper on it, not a cockpit.
Warmth is the differentiator because every other agent UI in this category is cold.

Three things the mood rules out, in advance:
- Any cool grey. Every neutral in this system carries a yellow-brown cast.
- Any gradient. The depth comes from surfaces sitting on each other.
- Any second accent. One accent, and it is a spice.

### What the accent is for

"Spent only where an action matters" turned out to be the wrong half of the rule, so here is the whole one.

The accent marks **agent state and text identity**, not buttons:

| Wears the accent | Does not |
|---|---|
| Running and needs-you state: the chip, the dot, the rail badge | Ordered-list numbers and bullets |
| The caret and the text selection | Inline code and code blocks |
| The activity glyph | File paths, tab underlines, blockquote rails |

**Primary actions use the solid plate, not the accent.** Near-black on the light panel, cream on the dark one. That plate is the highest contrast the palette can produce, so it reads as primary more strongly than terracotta ever could - terracotta on parchment is *lower* contrast than near-black on parchment. Linear, Craft and Raycast all do this.

The one exception, at most once per screen: when the action **is** the attention item, it may carry the accent. A permission card's Approve is the needs-you item and the action in the same control, so it may wear it. A second accent-bearing button on the same screen means one of them is not really primary.

This is why a screen can end up with no accent-bearing button at all and still be right: in an app whose spine is an attention queue, the accent's most valuable job is saying "this one needs you", not decorating a button the user is already looking at.

## Reference anchor

`design-systems/library/claude/DESIGN.md`, the Anthropic system.
Chosen from the seven candidates because it is the only light-native one, and the brief calls for off-white and off-black.
It is also the honest choice: surya is a harness for Claude Code, so its surface reading as part of that family is product truth, not decoration.

What is taken, quoted from that spec:

| Element | The spec's words | Value |
|---|---|---|
| Page | "Warm parchment canvas ... evoking premium paper, not screens" | `#f5f4ed` |
| Card | "Ivory ... the lightest surface - used for cards and elevated containers" | `#faf9f5` |
| Ink | "not pure black but a warm, almost olive-tinted dark" | `#141413` |
| Accent | "Terracotta Brand ... deliberately earthy and un-tech" | `#c96442` |
| Neutrals | "every gray has a yellow-brown undertone" | `#5e5d59`, `#87867f` |
| Border | "Border Cream ... the gentlest possible containment" | `#f0eee6`, `#e8e6dc` |
| Depth | "Ring-based shadow system (`0px 0px 0px 1px`) creating border-like depth" | ring, not drop shadow |
| Gradients | "Claude's design is **gradient-free** in the traditional sense" | none |

What is deliberately not taken: the marketing pacing.
That spec describes a product page with 64px serif heroes and magazine section spacing.
surya is a working tool. The serif appears at the page-title tier only, and the density stays.

## Type

Three families, each with one job.

| Family | Job | Where |
|---|---|---|
| Geist Variable (sans), semibold | Page titles | Home greeting, Settings, Result title, New ask, section anchors |
| Geist Variable (sans) | Everything functional | Rows, labels, buttons, body |
| Geist Mono Variable | Anything a machine wrote | Paths, model ids, counts, diffs, timestamps |

The scale has to clear 2x heading-to-body, which the baseline failed on ten of twelve screens.
Body drops to 13px for dense rows, and the page title lands at 28px.

Three numbers here changed when the native shell was built, and the reasons are worth keeping.

**The page title is sans, not Newsreader.** No serif face is bundled in `app/crates/ui/assets/fonts`, and adding one before the RC would mean shipping a font file, its licence and a new gpui registration for a single text style. The serif is a real improvement and stays on the list for after the RC; until then the page title is Geist semibold, which the type scale already distinguishes by size and weight alone.

**28px, not 40.** 40px was drawn against a web mockup with a page-wide header. In the native shell the title sits inside a floating panel next to a rail, and at 40 it crowds the panel it lives in. The scale is in `app/crates/ui/src/surya.rs`.

**2x, not 2.5x.** Display 28 over body 14. 2.5x from a 14px body means a 35px title, which runs into the same crowding; the ratio follows the title, not the other way round. `surya::tests::the_scale_has_real_contrast` prints the measured figure so it cannot drift silently.
Every count is `tabular-nums`, so numbers in a column line up.

## Motion depth

**Level 2 of 4: functional.**

Content arrives, it does not perform.
Rises of 8px and fades over 160-220ms, staggered 30ms down a list.
Springs stay on the things a finger touches: sheets, dialogs, the composer.
Nothing loops, nothing parallaxes, nothing moves on scroll.
The reduced-motion path shows the same content, instantly.

## Layout family per screen

The Variance Mandate: adjacent surfaces must not share a layout family, or the product reads as one template with different words in it.

| Screen | Family | Focal point |
|---|---|---|
| Home | Editorial header, then an asymmetric two-column brief | The greeting and the single count of what needs you |
| New ask | Centred single-field composer, everything else demoted | The prompt field, full width, large type |
| Needs you | Stacked correspondence, first card at full weight, rest condensed | The first question, opened |
| Agents | One lead row, then an indented tree | The agent that is waiting on you |
| Agent feed | Transcript, full bleed, sticky composer | The conversation itself |
| Preview | Device frame against a flat field | The rendered app |
| Result | Two columns, wide left, narrow decision panel right | Ship it |
| Cards | Gallery, uneven grid | The first card, at double width |
| Files | Three panes, tree / editor / meta | The open file |
| Tasks | Board, columns weighted by count | Running |
| Messages | List and thread, split | The unread thread |
| Settings | Left-aligned sections against a wide margin, no cards | Account |

## Non-negotiables inherited

- Stock shadcn components only, base-ui `render` prop, no new component library.
- No hardcoded colour, radius or spacing in `.tsx`. Tokens only.
- One CSS file: `mockup/src/index.css`.
- Every file under 500 lines.
- Data files are read-only. The copy stays exactly as written.
- No emoji, anywhere. lucide icons or plain words.
- Works at 390px first, 1440px second.
