# GPUI / React port contract

This is an agent's sketchpad for the native app. Read the current Rust screen and
its helpers first. Never use the deleted mockup or its history. Change one screen
at a time; keep the diff small enough to translate back by inspection.

## One-to-one layout and paint

`whitelist.json` is the executable contract, not all of Tailwind. Its generator
reads `Styled`, its proc-macro helpers, and the interaction traits at GPUI revision
`a07e9577ec788feb73c06fe7e307a3df8adaa895`. A method existing does not imply CSS has
identical defaults. The following pairs are exact within this sandbox:

| Tailwind | GPUI |
|---|---|
| `flex`, `flex-col`, `flex-row` | `.flex()`, `.flex_col()`, `.flex_row()` |
| `flex-1`, `flex-none`, `shrink-0` | `.flex_1()`, `.flex_none()`, `.flex_shrink_0()` |
| `items-center`, `items-baseline` | `.items_center()`, `.items_baseline()` |
| `justify-between`, `justify-center` | `.justify_between()`, `.justify_center()` |
| `w-full`, `h-full`, `size-full` | `.w_full()`, `.h_full()`, `.size_full()` |
| `min-w-0`, `min-h-0` | `.min_w_0()`, `.min_h_0()` |
| `px-12`, `gap-2`, `pt-6` | `.px(rems(3.0))`, `.gap(rems(0.5))`, `.pt(rems(1.5))` |
| `w-64`, `max-w-184` | `.w(rems(16.0))`, `.max_w(rems(46.0))` |
| `rounded-md`, `rounded-lg`, `rounded-full` | `.rounded(rems(0.375))`, `.rounded(rems(0.5))`, `.rounded(px(9999.0))` |
| `border`, `border-t`, `border-r` | `.border_1()`, `.border_t_1()`, `.border_r_1()` |
| `bg-surface`, `text-text-muted`, `border-border` | `.bg(theme.surface)`, `.text_color(theme.text_muted)`, `.border_color(theme.border)` |
| `bg-warning/14` | `.bg(theme.warning.opacity(0.14))` |
| `text-ui-13` | `.text_size(ui_rems(13.0))` |
| `font-medium`, `font-semibold` | `.font_weight(FontWeight::MEDIUM)`, `.font_weight(FontWeight::SEMIBOLD)` |
| `leading-normal` | `.line_height(relative(1.5))` |
| `truncate` | `.truncate()` |
| `overflow-y-scroll` | `.id("stable-id").overflow_y_scroll()` |
| `hover:bg-element-hover` | `.hover(\|s\| s.bg(theme.element_hover))` |
| `active:bg-element-active` | `.active(\|s\| s.bg(theme.element_active))` |
| `focus:bg-element-hover` | `.track_focus(&handle).focus(\|s\| s.bg(theme.element_hover))` |

Alpha modifiers such as `bg-warning/14` are admitted only on opaque tokens (and
the native `wash` helper). Tailwind multiplies existing alpha while GPUI
`opacity()` replaces it, so modifiers on already-translucent tokens are rejected
rather than silently changing the paint.

Numeric spacing uses Tailwind's 0.25rem unit, not pixels. At the default 16px root,
`p-2` is 8px. Rust uses both fixed `px` and scalable `ui_rems`; preserve the original
unit on port-back when the design has not changed. Each allowed prefix has a finite
suffix set in the whitelist. Do not assume `w-whatever` or arbitrary decimals pass.
Text size utilities deliberately use `text-ui-*`: stock Tailwind text utilities
also set a different line height. Generated `text-ui-*` preserves GPUI default
`phi()` leading and its pixel rounding, checked against `style.rs` and `geometry.rs`.

## Banned constructs and unsupported semantics

No grid, transitions, animation, responsive prefixes, arbitrary values/properties,
negative or important modifiers, selectors, pseudo-elements, group/peer/data/aria
variants, transforms, filters, backdrop blur, CSS variables in classes, gradients,
CSS shadows, sticky/fixed positioning, tables, or CSS outside the generated Tailwind
entry. No inline `style`, stylesheet injection, DOM mutation, JSX spreads, dynamic
class construction, or unreviewed imports. Literal classes or ternaries of complete
literal class lists are allowed. The lint fails closed on other expressions.

Some of these exist in the fork (including grid and animation). They are excluded
by this portable subset because a Tailwind spelling alone does not capture the
native layout or lifecycle contract. Browser-only APIs such as sticky positioning,
pseudo-elements, media queries and DOM effects have no direct admitted GPUI pair.
Do not silently widen the whitelist to make a browser design pass. Add the verified
method mapping, generator rule, rejection tests and guide explanation together.

## State and events

Props in, callbacks out. Use typed plain records and stable string ids corresponding
to the Rust model. Derived counts stay derived. A component receives `rows`, selected
ids, busy ids and failures; callbacks name intentions such as `onOpenChat(chatId)`
and `onAnswer(rowId, labels)`. On port-back these become `cx.emit(...)` or existing
`cx.listener(...)` calls. Keep rendering helpers pure.

Only `useState` for local UI state. No context, effects, refs, memo hooks, reducers,
fetching, subscriptions, timers, persistence, routing or backend. The harness owns
fixture selection and an event log. It may replace fixture props to demonstrate a
result. An answer must not delete a native row optimistically: the engine watch
owns removal. Keep transport, authorization and business logic in Rust.

## Names and file boundaries

One React component per GPUI struct, with the same exported name. `NeedsYou.tsx`
exports `NeedsYouPane`; `shell.tsx` exports `Shell`. A Rust `impl` helper is a plain
render function in React, not a new stateful component. `needs_you/rows.tsx` mirrors
`inbox/needs_you/rows.rs`; `chrome.tsx` mirrors `inbox/chrome.rs`. Shell helper files
are an explicit split of the oversized upstream `shell.rs` to respect the 500-line
limit. Each file names its Rust source. `main.tsx` is the browser harness exception.

## Tokens and generation

Run `npm run generate` after Rust theme changes. The generator parses `Theme` in
`app/crates/ui/src/theme.rs`, follows its `from_variant` assignments into
`app/crates/theme/src/builtins.rs`, and resolves the two comet variants. This matters:
`Theme::dark()` alone is not the final active palette. It emits `src/theme.ts` and
the only CSS file, `src/tailwind.css`, with Tailwind v4 `@theme` entries and light/dark
variable scopes. The immutable provider mark color is parsed from
`icons.rs::claude_brand()`, not approximated with a danger/accent token. Never edit generated output or invent colors in a component.
The generator rejects unknown source expressions and missing scalar color fields.

`npm run generate:whitelist -- /path/to/gpui-surya` verifies the exact revision and
reads method bodies and macro prefixes. Commit regenerated files with their sources.
Fonts and SVG geometry are copied from the native assets; icons use `currentColor`.
The toolkit's independent JSON palette is not the app palette.

## Port in, per screen, from Rust to React

1. In your own worktree, choose the next authorized native screen and record its
   Rust revision, theme, viewport and Windows reference frames. Include seeded,
   empty and relevant selected, busy or failure states. Do not use the old mockup.
2. Read the GPUI struct, its `Render` implementation, split render helpers and
   surrounding shell. Trace the model fields, derived values, element ids, focus
   and scroll ownership, emitted events and listener callbacks before writing JSX.
3. Create one React component with the same struct name and mirror the Rust file
   boundaries. Name each source file in a header comment; keep `impl` render helpers
   as pure functions. Reuse the existing Shell frame and stay under 500 lines.
4. Translate the element tree and style calls through `whitelist.json`, preserving
   nesting, ordering, flex constraints, token roles and interaction states. Record
   fixed `px` versus scalable `ui_rems` units for the return trip. If a style has no
   admitted pair, document the gap instead of adding arbitrary CSS or classes.
5. Express native model inputs as typed fixture props with stable ids. Keep counts
   derived and turn events/listeners into intent callbacks. Leave subscriptions,
   transport and routing in Rust; use only local `useState` where needed. Let the
   harness supply fixtures and show callback results without a backend.
6. Run `npm run generate` to refresh native tokens and assets, then `npm run lint`,
   `npm test` and `npm run build`. Extend the browser proof for the new screen's
   fixtures and meaningful interactions; verify mouse and keyboard callbacks.
7. Capture the browser screen at the reference viewport in both themes. Inspect
   it beside the Windows frames, including empty and interactive states. Correct
   translation errors and record remaining differences under known gaps. Review
   this baseline before iterating its design; use the procedure below to port the
   resulting screen diff back into GPUI.

## Port back, per screen, from the diff

1. Record the baseline Rust revision, fixture, theme, viewport and screenshots.
2. Run `npm run lint`, `npm test`, `npm run build`, then the browser proof script.
   Inspect seeded and empty states in light and dark; click and keyboard-test controls.
3. Read `git diff -- sandbox/src/screens sandbox/src/shell.tsx sandbox/src/shell`.
   Match each component/helper to the Rust source named in its header.
4. Translate changed layout classes using `whitelist.json`; preserve existing
   element ids, scroll handles, focus handles, subscriptions and event ownership.
5. Translate props into native fields/derived values and callbacks into native
   events/listeners. Do not port fixture controls, URL switches or event-log copy.
6. If a token changes, change the Rust source first, regenerate, and review both
   themes. Never copy a browser-computed hex into a native view.
7. Run required Rust checks in your own worktree. Capture the real Windows app and
   compare at the same size. Browser screenshots prove the sketchpad only. Submit
   the native change separately; coordinator review and green CI still gate merge.

## Known gaps

- Reference captures include an 8px black strip outside the client. The sandbox
  renders a 1440x900 client viewport; comparisons retain the original reference.
- The supplied light reference is empty and has the old skew banner over its heading.
  The sandbox keeps the heading clear and omits that known bug; its light seeded
  capture uses the same two questions as dark.
- Seeded questions are the native collapsed-row fixture (`answer below`). Opening
  them emits a callback shown by the harness. No chat screen or composer is ported.
- The current registry uses purple selected-row fills; the reference uses gray.
  The current Rust value is retained.
- The dark Windows reference shows an accent dot and accent-colored `Input`
  status on the rail chat row. `src/shell/rail.tsx` currently renders `Input` as
  plain faint age text without the dot; this status treatment is not yet ported.
- Browser font rasterization and native Windows caption controls differ. Titlebar
  actions and unported rail destinations report fixture events, not OS commands.
- Shell/rail colors follow the current resolved theme. Reference build and stored
  settings may differ; no sampled color overrides are introduced to conceal drift.
- Small faint labels and some badge states inherit native contrast limits.
  `proof/contrast.json` records the measured failures; palette changes belong in
  the native theme first, not in sandbox-only overrides.
- This desktop frame keeps the native fixed sidebar and gutters. Narrow mobile
  screens are not a new design target; no responsive rules are added.

## Documentation

Setup follows [Tailwind's Vite integration](https://tailwindcss.com/docs/installation/using-vite)
and [theme variables](https://tailwindcss.com/docs/theme). Local state follows
[React useState](https://react.dev/reference/react/useState). The pinned GPUI source,
not similarity to Tailwind's API names, decides what can port back.

## Reproduce proof

After `npm ci`, run `npm run build`, `npm test`, and start `npm run dev -- --port 5177`.
In another terminal run `npm run proof`. Install Chromium with
`npx playwright install chromium` if needed, or set `CHROMIUM_PATH` to an existing
Chromium executable. `SANDBOX_URL` overrides the local server URL. Original Windows
reference files must be available at the paths listed in `scripts/proof.mjs`; the
script copies them without modification and captures side-by-side comparisons.

## Worked examples and live design rounds

The three `sandbox-*` skills point to this guide and open with the tested pairs in
`examples/`. `npm run lint` checks that all three right versions pass and all three
wrong versions fail; `npm test` imports the right specimens and checks that skill
excerpts match the files. To inspect one rejection directly, run
`node scripts/lint-examples.mjs examples/design/wrong/rows.tsx` from `sandbox/`.

The port-in example adds an explicit component/helper boundary contract to the
same JSX lint. The port-back modules export native before/after fragments and a
unit record; their lint verifies the original source and a single mapped padding
edit. This is not a Rust compiler or an automatic checker for every native render.
The design example uses the ordinary class/inline-style lint. These teaching edits
are not applied to the live screen or recorded as accepted design changes.

For live owner-led rounds, use `sandbox-design` -> "Live design loop". Its running
brief is `CHANGES.md`; `sandbox-port-back` consumes that brief with the source diff.
