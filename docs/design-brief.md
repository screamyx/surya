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
- Any second accent. One accent, spent only where an action matters.

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
| Newsreader Variable (serif) | Page titles only | Home greeting, Settings, Result title, New ask, section anchors |
| Geist Variable (sans) | Everything functional | Rows, labels, buttons, body |
| Geist Mono Variable | Anything a machine wrote | Paths, model ids, counts, diffs, timestamps |

The scale has to clear 2.5x heading-to-body, which the baseline failed on ten of twelve screens.
Body drops to 13px for dense rows and 15px for reading, the page title lands at 40px on desktop and 30px on phone.
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
