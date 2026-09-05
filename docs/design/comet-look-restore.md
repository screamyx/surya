# Comet's look, restored

The app looks like zeron's comet again.
Every feature stays.

## The order

Owner, 2026-09-05 19:25, relayed by jag-0905-raven, verbatim:

> just revert back the gui to how zeron's comet look. can you do that?

then, one minute later:

> i mean only the theme, not functionality, features

then at 19:26:

> and the box/modal, remove them. the implementation is just atrocious and pure lazy

then at 19:32, which settled how far the revert goes:

> the new gui should look like zeron's comet, but with our feature built in

## The reference

Comet's own product screenshots are in this repo, at the subtree import commit
`fe35546`.
They are the reference, and they are citable by path:

| Path at `fe35546` | Size | What it shows |
| --- | --- | --- |
| `apps/landing/public/assets/app-screenshot.jpg` | 2560x1655 | The whole window: transcript, composer pill, diff pane |
| `apps/landing/public/assets/shots/sessions.png` | 907x627 | The session list and its selected row |
| `apps/landing/public/assets/shots/diff.png` | 907x627 | The changes pane |
| `apps/landing/public/assets/shots/history.png` | 907x627 | History |

Read one with `git show fe35546:apps/landing/public/assets/app-screenshot.jpg > /tmp/ref.jpg`.

All four are dark.
Comet shipped no light screenshot, so light is proved by token and by code, not
by a reference pixel.

## Before and after

Same binary tree, same rig, same seed.
The only difference between the two columns is the commit.
Left is `origin/main` at `20616ef`, right is `fix/comet-look`.
Shot with `app/scripts/critic-shots.sh` on the shared headless Xorg `:7`,
`panics=0` on all sixteen frames.

| | 1440x900 | 1100x700 |
| --- | --- | --- |
| Dark | [shell-dark-1440x900.png](comet-restore/shell-dark-1440x900.png) | [shell-dark-1100x700.png](comet-restore/shell-dark-1100x700.png) |
| Light | [shell-light-1440x900.png](comet-restore/shell-light-1440x900.png) | [shell-light-1100x700.png](comet-restore/shell-light-1100x700.png) |

![dark 1440x900](comet-restore/shell-dark-1440x900.png)

![light 1440x900](comet-restore/shell-light-1440x900.png)

## The question flow

This is the part the owner called atrocious.
Same pairs, with a question open.

| | 1440x900 | 1100x700 |
| --- | --- | --- |
| Dark | [question-dark-1440x900.png](comet-restore/question-dark-1440x900.png) | [question-dark-1100x700.png](comet-restore/question-dark-1100x700.png) |
| Light | [question-light-1440x900.png](comet-restore/question-light-1440x900.png) | [question-light-1100x700.png](comet-restore/question-light-1100x700.png) |

![question dark 1440x900](comet-restore/question-dark-1440x900.png)

Three things changed and one deliberately did not.

The two "Question ... answer below" boxes pinned above the feed are gone.
The needs-you rows are a plain list in the sidebar, not cards.
The window is flat.

The rounded glass question panel **stayed**.
surya-states checked and it is comet's own: `render_wizard` on this branch is
byte-identical to comet's at the import commit, 204 lines,
md5 `e14d577e3f5cf9102abdcda01c10bae7` on both sides.
Removing it would have been a new design decision, not a revert, and the owner's
19:32 message settles which way that goes.

### The needs-you row, before and after

surya-states, `crates/ui/src/inbox/chrome.rs`.

Before, at `ffa7cd5`:

```rust
pub fn row_card(theme: &Theme) -> Div {
    div()
        .flex().flex_col().gap(px(6.0)).p(px(10.0))
        .rounded(px(10.0))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface_card)
}
```

After, at `6e7c87d`:

```rust
pub fn row_line(theme: &Theme, first: bool) -> Div {
    div()
        .flex().flex_col().gap(px(6.0)).px(px(2.0)).py(px(10.0))
        .when(!first, |el| el.border_t_1().border_color(theme.border))
}
```

Gone: the corners, the outline, the fill.
`p(10)` became `px(2)/py(10)` because a line does not need side padding inside a
pane that already has 12.

## Measured, not eyeballed

Every number here was produced this session by reading the primary source.

### The theme is comet's, unmodified

| Check | Result |
| --- | --- |
| `zeron_dark()` vs `fe35546` | byte-identical, 33 lines, md5 `ba0cd47865ab4552c49db58f70bbe4c8` |
| `zeron_light()` vs `fe35546` | byte-identical, 33 lines, md5 `165e100a5201a1bb66a44c9fe627d238` |
| `fn variant()` vs `fe35546` | byte-identical, 73 lines |
| `Theme::from_variant` vs `fe35546` | 101 lines both sides, **one** added line, additive: `theme.warning_wash = warning_wash_for(...)` |
| `glass_hover`, `input_glass_bg`, `wash`, `is_frost` | identical |
| `BUBBLE_RADIUS` 16, `PANEL_RADIUS` 10, `CONTROL_RADIUS` 6 | unchanged since the import |

`ThemeSelection::default()` is `zeron-light` / `zeron-dark` again, and so is the
registry fallback.
System following is untouched.

### The floating layout is gone, in pixels

One scanline, `y=500`, dark, 1440x900, run-length encoded.

Before, `origin/main`:

```
x=   0  (11,10,8)     16px   <- canvas margin
x=  17  (51,46,38)   254px   <- the rail, a card
x= 272  (11,10,8)      5px   <- the gap between two cards
x= 281  (51,46,38)  1142px   <- the conversation, a card
x=1427  (11,10,8)     13px   <- canvas margin
```

After, `fix/comet-look`:

```
x=   0  (24,24,24)   255px   <- the sidebar column, flush to the window edge
x= 255  (48,48,48)     1px   <- its right hairline
x= 256  (13,13,13)   ...     <- the main area, to the window edge
```

The 16px canvas inset is 0.
The 8px inter-panel gap is 0.
Two planes meet at a 1px hairline, which is what comet draws.

### Against comet's own frame

Comet's window is frosted over a purple desktop, so its absolute RGB carries the
wallpaper and will never match a headless grab composited over black.
The relation is what transfers, and it does:
in `app-screenshot.jpg` at `y=1100` the left column reads `(29,19,43)` against a
main area of `(13,9,24)`.
Ours reads `(24,24,24)` against `(13,13,13)`.
Lighter sidebar, darker main, hairline between.

## What was kept on purpose

`crates/ui/src/surya.rs` still exists and still holds the geometry and the type
scale.
`surya-light` and `surya-dark` are still built in and still pickable in
Appearance.
Nothing was deleted, so bringing the look back is a decision, not a rebuild.

The reduced-motion switch stays.
`ZERON_WINDOW_SIZE`, which the shot rig needs, stays.

## Deviations, written down rather than left silent

**Decision 20 is superseded.**
It said the needs-you queue sits at the top of the feed, at full weight.
Replacement, written by surya-states, who wrote the original:

> The needs-you queue is the spine of the app, and it lives on its own page in
> the sidebar. It is a plain list, not cards, and it never sits over the
> transcript: the feed is the conversation, and a queue drawn on top of it hides
> the composer the user needs to answer with. Superseded the original wording
> (2026-09-05, owner via raven 19:26 and 19:32) - the same queue, moved off the
> feed and stripped of its boxes.

**Session-row sub-lines went back to `text_muted.opacity(0.5)`.**
That is comet's own value and it measures about 2.3:1 on the panel, under AA.
Round 2 of the design critique had moved it to `text_faint` for that reason.
The reading still stands; the order is comet's look, so it went back with the
rest, and this is the one place where fidelity and legibility disagree.

**Two tasks-pane cards moved off a literal `12.0` onto `Theme::PANEL_RADIUS`.**
Comet has no task board, so there is no reference screen.
Comet's card radius is 10 and has been since the import, so our board is drawn in
comet's radius language.

**The nav rail keeps its accent-tinted selected wash.**
This was queried and cleared.
The rail paints `theme.element_active`, a comet token, and comet derives that
token from the accent: `builtins.rs:125`, `active: accent.primary.with_alpha(if
dark { 0.18 } else { 0.10 })`, in a function that is byte-identical to the
import.
Comet has two selected treatments and always did, accent-tinted for
`element_active` and neutral `glass_selected_bg` for the session list, and we use
each where comet uses it.

## Still open

Permissions.
Comet renders none at the import commit and `zeron_doc::MessagePart` has no
`Permission` variant, so making them inline would be doc and engine work, not a
look change.
They render as they did.
Flagged to raven by surya-states.

## Green

`cargo build -p zeron` exit 0.
`zeron-theme` 26 passed, 0 failed.
`zeron-ui` 652 passed, 0 failed.
Sixteen frames, `panics=0` on every one.
