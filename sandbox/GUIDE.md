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
| `font-sans`, `font-mono` | `.font_family(theme.font_sans.clone())`, `.font_family(theme.font_mono.clone())` |
| `text-syntax-keyword` | `.text_color(theme.syntax.keyword)` |
| `bg-terminal-ansi-1` | `theme.terminal.ansi[1]` |
| `leading-normal` | `.line_height(relative(1.5))` |
| `truncate` | `.truncate()` |
| `overflow-y-scroll` | `.id("stable-id").overflow_y_scroll()` |
| `hover:bg-element-hover` | `.hover(\|s\| s.bg(theme.element_hover))` |
| `active:bg-element-active` | `.active(\|s\| s.bg(theme.element_active))` |
| `focus:bg-element-hover` | `.track_focus(&handle).focus(\|s\| s.bg(theme.element_hover))` |
| `focus-visible:border-accent` | `.track_focus(&handle).focus_visible(\|s\| s.border_color(theme.accent))` |
| `focus-within:bg-surface` | `.track_focus(&handle).in_focus(\|s\| s.bg(theme.surface))` |
| `group` on the parent, `group-hover:text-accent` on the child | `.group("id")` and `.group_hover("id", \|s\| s.text_color(theme.accent))` |
| `group-active:bg-element-active` | `.group_active("id", \|s\| s.bg(theme.element_active))` |

Alpha modifiers such as `bg-warning/14` are admitted only on opaque tokens (and
the native `wash` helper). Tailwind multiplies existing alpha while GPUI
`opacity()` replaces it, so modifiers on already-translucent tokens are rejected
rather than silently changing the paint. Which tokens qualify is derived from
`src/theme.ts`, not listed by hand: a token that is opaque in both appearances
may carry a modifier. `text-dim` linted clean and painted nothing for months
after the Rust theme dropped the field, which is what a hand-written list buys.

Numeric spacing uses Tailwind's 0.25rem unit, not pixels. At the default 16px root,
`p-2` is 8px. Rust uses both fixed `px` and scalable `ui_rems`; preserve the original
unit on port-back when the design has not changed. Each allowed prefix has a finite
suffix set in the whitelist. Do not assume `w-whatever` or arbitrary decimals pass.
Text size utilities deliberately use `text-ui-*`: stock Tailwind text utilities
also set a different line height. Generated `text-ui-*` preserves GPUI default
`phi()` leading and its pixel rounding, checked against `style.rs` and `geometry.rs`.

## Banned constructs and unsupported semantics

No grid, responsive prefixes, arbitrary values or properties, negative or
important modifiers, selectors, pseudo-elements, peer/data/aria variants,
transforms, filters, backdrop blur, CSS variables in classes, gradients, CSS
shadows, sticky/fixed positioning, tables, or CSS outside the generated Tailwind
entry. No inline `style`, stylesheet injection, DOM mutation, JSX spreads,
dynamic class construction, or unreviewed imports. Literal classes or ternaries
of complete literal class lists are allowed. The lint fails closed on other
expressions.

Some of these exist in the fork (including grid). They are excluded by this
portable subset because a Tailwind spelling alone does not capture the native
layout or lifecycle contract. Do not silently widen the whitelist to make a
browser design pass. Add the verified method mapping, generator rule, rejection
tests and guide explanation together.

## Type and code colour

`font-mono` is `theme.font_mono`, which is Geist Mono. Use it for the diff
viewer, the terminal, transcript code blocks and the file editor, the same four
surfaces the native app puts it on.

`scripts/gen-theme.mjs` ships exactly the faces `ui/src/typography.rs` registers
with gpui: eight Geist and eight Geist Mono, in weights 400, 500, 600 and 700
with an italic of each. Before this, three sans faces shipped and the browser
synthesized `font-bold` and `italic` from the regular weight, which is not what
the native pane draws. The face list, the `@font-face` rules and the files under
`public/fonts` all come from that one Rust array.

`theme.font_sans_fixed` gets no class. It is a distinct `Theme` field but it
names the same family as `font_sans` in both appearances, which `theme.rs`
asserts at line 2571. The generator throws rather than guessing if that changes.

### Syntax

`Theme::syntax` is a `SyntaxPalette`, not an `Hsla`, so it never reached the
scalar token sweep and every code line in a ported screen painted flat at
`text-text/88`. That is the native colour of an unhighlighted line, so a
highlighted pane looked like a plain one.

All 24 fields are now tokens, admitted on `text-` only, because
`SyntaxPalette` is paint-only and every Rust call site reads it through
`.text_color()`.

| Tailwind | GPUI |
|---|---|
| `text-syntax-comment` | `.text_color(theme.syntax.comment)` |
| `text-syntax-keyword` | `.text_color(theme.syntax.keyword)` |
| `bg-terminal-ansi-1` | `theme.terminal.ansi[1]` |
| `text-syntax-string`, `-string-special`, `-escape` | `theme.syntax.string`, `.string_special`, `.escape` |
| `text-syntax-number`, `-boolean`, `-constant` | `theme.syntax.number`, `.boolean`, `.constant` |
| `text-syntax-type-name`, `-type-builtin`, `-constructor` | `theme.syntax.type_name`, `.type_builtin`, `.constructor` |
| `text-syntax-function`, `-function-builtin`, `-macro-name` | `theme.syntax.function`, `.function_builtin`, `.macro_name` |
| `text-syntax-property`, `-variable`, `-variable-special`, `-parameter` | the same fields |
| `text-syntax-operator`, `-punctuation`, `-tag`, `-attribute`, `-label` | the same fields |
| `text-syntax-invalid` | `.text_color(theme.syntax.invalid)` |

Each token is the colour `SyntaxPalette::color()` returns for the matching
`surya_syntax::HighlightKind`. There is no `text-syntax-embedded`: `Embedded`
is a `HighlightKind` that `color()` paints with `punctuation`, so it is not a
field and gets no token of its own.

The resolved values are not `SyntaxPalette::dark()`. `from_variant` overrides
every field from the theme variant's twelve seed colours through
`builtins.rs::syntax()`, and that is the chain the generator follows. It throws
if a field would fall back to the built-in `oklch()` expressions, which the
colour reader cannot evaluate.

### Terminal

`Theme::terminal` is a `TerminalColors`, and its `ansi` member is a
`[Hsla; 16]`, so nineteen more colours were invisible to the token sweep for the
same reason the syntax palette was. A ported terminal had the mono family and
none of its own colours.

| Tailwind | GPUI |
|---|---|
| `bg-terminal-background`, `text-terminal-background` | `theme.terminal.background`, what `resolve_color` returns for `CellColor::Background` |
| `bg-terminal-foreground`, `text-terminal-foreground` | `theme.terminal.foreground`, `CellColor::Foreground` |
| `bg-terminal-selection` | `theme.terminal.selection` |
| `bg-terminal-ansi-0` to `-15`, and the same on `text-` | `theme.terminal.ansi[N]`, `CellColor::Indexed(N)` |

Do not reach for `bg-bg` or `text-text` in a terminal. The terminal background
is its own near-black, not the page background, and `code-wash` and `code-text`
are the accent-family inline-code pair, not a terminal.

Three decisions worth stating, because each could have gone another way.

**Sixteen indexed names, not colour names.** The Rust is `pub ansi: [Hsla; 16]`
and `terminal::view.rs` reads it as `theme.terminal.ansi[ix as usize]`. Nothing
in the Rust calls slot 1 red or slot 5 magenta, and a theme is free to seed them
however it likes, so a name would be a claim the source does not make. The index
is the ANSI slot number and nothing more.

**Both `bg-` and `text-`, except the selection.** `resolve_color` takes a
`CellColor` and returns a colour, and a cell uses that same function for its
foreground and its background, so every one of these values is legitimately
either. The selection is different: `view.rs:503` only ever pushes it as a
`fill` quad behind the text, so it is a background alone. Nothing paints a
terminal border, so `border-terminal-*` is rejected.

**`terminal-` on the front of all of them.** `background`, `foreground` and
`selection` would otherwise shadow the `bg`, `text` and `selection` theme roles,
which are different colours for different surfaces. One prefix per Rust struct,
the same as `syntax-`.

The values resolve the same way every other token does, from the comet variants:
`variant()` builds the palette from the seed's `terminal_background`, the seed
text hardened to 4.5 against it, a border-tone selection wash, and one of the
`ANSI_*` constants. `Theme::from_variant` hardens the terminal foreground a
second time only when the variant is not a curated builtin, which these two are,
so that branch never runs. The generator throws if that guard disappears.

Slots 16 to 255 are not tokens. `terminal::view::extended_indexed_rgb` computes
them from the index, appearance-dependent, so they are not theme colours and
`bg-terminal-ansi-16` is rejected.

## Motion

Animation is admitted, and only as the native catalog. `app/crates/ui/src/motion.rs`
and `app/crates/proto/src/motion.rs` are the source; `scripts/motion.mjs` reads
every duration, delay, curve and endpoint out of them and emits both the
whitelist entries and the `@keyframes`, `@utility` and reduced-motion rules in
`src/tailwind.css`. Both files are recorded with their sha256 in
`whitelist.json`'s `sources` and in `src/theme.ts`'s `provenance`, so a curve
change in Rust turns `npm run lint` red.

There is no free duration, delay or easing utility. `duration-500`,
`ease-[cubic-bezier(0.16,1,0.3,1)]`, `transition-colors` and `animate-spin` are
all rejected. A designer picks a catalog entry or does not animate: a 320ms
tween that reads well in the browser is a value the native app cannot produce.

### Entrances and exits

Each of these is one `motion.rs` helper, wrapped in Rust as
`with_animation(id, SPEC.animation(), ...)`.

| Tailwind | GPUI | Spec |
|---|---|---|
| `motion-fade-in` | `motion::fade_in(id, element)` | 500ms `EASE_OUT_EXPO`, opacity 0 to 1, top 4px to 0 |
| `motion-fade-quick` | `motion::fade_quick(id, element)` | 150ms `EASE`, opacity 0 to 1 |
| `motion-menu-in` | `motion::menu_in(id, element)` | 140ms `EASE`, opacity 0.3 to 1, top -2px to 0 |
| `motion-menu-out` | `motion::menu_out(id, t, element)` | 100ms `EASE`, opacity 1 to 0, top 0 to -2px |
| `motion-dialog-in` | `motion::dialog_in(id, element)` | 180ms `EASE`, opacity 0 to 1, top 2px to 0 |
| `motion-splash-out` | `motion::splash_out(id, element)` | 500ms `EASE` after a 150ms hold, opacity 1 to 0, top 0 to -6px |
| `motion-chevron` | `changes.rs` fold chevron over `motion::CHEVRON` | 200ms `EASE`, opacity 0.25 to 1 |

`motion-fade-in`, `motion-menu-in`, `motion-menu-out`, `motion-dialog-in` and
`motion-tab-slide` move a relative inset, so they need `relative` or `absolute`
on the same element. The lint rejects them without one. This is not a style
rule: a GPUI `Style::default()` is `Position::Relative` and a CSS box is
`static`, so a `top` the native app paints does nothing in the browser.

### Transitions

These four are hand-rolled tweens in Rust rather than `with_animation`, so the
class carries the catalog timing and the properties its call sites blend.

| Tailwind | GPUI |
|---|---|
| `motion-hover-fade` | `motion::hover_blend(key, rest, hover)` with `.on_hover(motion::hover_listener(key))`, 150ms `EASE_TAILWIND` on color, background and border |
| `motion-resize` | `shell.rs` `WidthTween` over `motion::RESIZE`, 200ms `EASE_OUT` on width and height |
| `motion-collapse` | `changes.rs` fold body over `motion::COLLAPSE`, 180ms `EASE_OUT` on height |
| `motion-tab-slide` | `terminal/panel.rs` tab reorder over `motion::TAB_SLIDE`, 150ms `EASE_OUT` on left |

GPUI `.hover()` snaps by construction. `motion-hover-fade` is the browser
spelling of the manual blend the native app runs so a hover wash fades instead.
A hover that is meant to snap simply omits it.

### Loaders

Both loaders paint one repeating cell per grid position, offset by a phase the
pure math in `proto/src/motion.rs` computes. A negative CSS `animation-delay`
is that same offset, so each position gets its own class and there is no
arbitrary per-cell value to write.

| Tailwind | GPUI |
|---|---|
| `motion-surya-pulse-0` to `-4` | `loaders.rs::surya_loader` cell i, `motion::staggered_phase(delta, i, PULSE_STAGGER)`, 2400ms, opacity 0.08 to 1 and size 90% to 100% |
| `motion-gradient-spin-0` to `-3` | `loaders.rs::gradient_spinner` cell at distance d, `proto::gspin_cell_phase`, 750ms, opacity 1 to 0.1 and back |

The pulse cell breathes inside a fixed slot, exactly as in Rust: put the cell in
a sized parent and let the keyframes drive its percentage width and height. The
gradient spinner is a 3x3 grid whose cell distances are `[[3,2,3],[2,1,2],[1,0,1]]`.

### Reduced motion

`App::reduce_motion` is honored by every `with_animation` element in gpui:
oneshots snap to their end state, repeating ones to phase zero, and no frames
are scheduled. Media queries are banned in classes, so the browser equivalent is
generated once into `src/tailwind.css` as an unlayered
`@media (prefers-reduced-motion: reduce)` block that pins each `motion-*` class
to that same rest or end state. It sits outside Tailwind's layers, so it wins
without `!important`. No screen spells the preference, the same way no Rust
caller does.

### What is not admitted

- **Scale and rotate.** GPUI divs have no scale or rotate transform at the
  pinned revision, only `svg` transformations. `motion.rs` says so itself, and
  `menu_in`/`dialog_in` approximate their CSS scale component with fade plus
  translate. `scale-95` and `rotate-90` are rejected, and the browser gets the
  same approximation the native app paints.
- **`disabled:`.** GPUI has `hover`, `active`, `focus`, `focus_visible`,
  `in_focus`, `group_hover` and `group_active` and no disabled style at all.
  A disabled control is a different literal class list, not a variant.
- **`motion-safe:` and `motion-reduce:`.** Media-query variants. The generated
  block above covers the one preference that has a native pair.
- **`SCROLL_GLIDE`.** `rail.rs` drives 500ms `EASE_IN_OUT` over `scrollTop`
  itself. CSS `scroll-behavior: smooth` picks its own duration, so there is no
  honest class.
- **The animated surya mark.** `loaders.rs::surya_mark_loader` staggers 34
  cells along a flight axis; that is 34 arbitrary delays, not a finite set.
- **The sidebar chevron rotation.** `shell/spaces.rs` rotates an svg through
  `Transformation::rotate` over `COLLAPSE`. `motion-chevron` is the diff pane's
  crossfade, which is a different element.

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
- `whitelist.json` still records GPUI revision `a07e9577ec788feb73c06fe7e307a3df8adaa895`
  while `app/Cargo.toml` now pins `3640743b70a6e4647d90249fc6ccc5398322648a`. The
  motion work did not rebase the whitelist onto the newer revision; nothing here
  was read from it. Regenerating against the new pin is its own change.
- The gradient spinner's per-row tints are raw hex in
  `proto::motion::GSPIN_ROW_TINTS`, not theme tokens, so the sandbox has no
  admitted color for them. `motion-gradient-spin-*` carries the timing only and
  a screen picks an admitted token for the cell fill.
- Reduced motion is generated into the stylesheet, so a screen cannot opt out of
  it or preview it by writing a class. Preview it with the browser preference.
- `motion-collapse` and `motion-tab-slide` are CSS transitions between two class
  lists, while Rust drives them as `with_animation` elements from measured
  pixel values. The timing matches; the trigger does not, and a port back has to
  restore the tween and its epoch key.
- The syntax tokens resolve the two comet variants, the same as every other
  token. A user-supplied theme that seeds fewer than the twelve syntax colours
  would fall back to `SyntaxPalette::dark()`, whose `oklch()` and
  `git_graph_tone()` expressions `scripts/rust-colors.mjs` cannot evaluate. The
  generator throws in that case rather than emitting a guessed colour.
- Highlighting itself is not ported. A screen spells the token per span; the
  parser, the capture precedence and `HighlightKind` stay in Rust.
- `theme.glyph` (`GlyphPalette`: light, mid, deep) is the last colour-carrying
  `Theme` field the token sweep still misses. It is what `mini_glyph_spinner`
  tints its rows with, and `scripts/rust-colors.mjs` already skips it explicitly
  in `accentRoles()`. Three colours, same shape as the two fixed here.
- Terminal slots 16 to 255 are computed from the index by
  `extended_indexed_rgb`, not held by the theme, so a ported terminal can spell
  the sixteen theme slots and nothing above them.
- The terminal is painted with `canvas` fill quads in Rust, not with divs, so a
  browser terminal built from `bg-terminal-*` elements matches the colours but
  not the run-merging the native renderer does per row.

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

## Annotation tooling

[annotate/](annotate/README.md) fronts Vite for the live design loop. Its shadow-root
review controls and CSS are tooling, exempt from the screen whitelist. The file
size and no-emoji checks still apply; screen imports cannot cross into this folder.
Tailwind scans only `src/`, so review chrome never enters the screen bundle.
A pin supplies `sandbox/src/...:line:column`; retain its file and line in CHANGES.md
when recording the accepted edit. The source line is a pointer to recheck, not a
command to execute.
