# Browser proof and author critique

Baseline: `485eba4` (2026-09-06). GPUI style contract:
`a07e9577ec788feb73c06fe7e307a3df8adaa895`. This is the author's visual inspection,
not the independent MERGE verdict. The coordinator owns that review.

## Passing checks

- Clean `npm ci`, `npm run lint`, `npm test` (5/5), `npm run build`.
- The lint's negative CLI test exits 1 for an arbitrary class and extra CSS.
- Four Chromium captures at 1440x900: light/dark, seeded/empty. Both seeded
  frames show two questions, `2 waiting`, one selected rail entry and its badge.
  Both empty frames show `Nothing needs you.`; none contains a composer.
- Mouse hover/press and keyboard Enter emit `OpenChat(wire-tasks)` visibly.
  Rows remain until their fixture props change. Both fixture switches and the
  sidebar toggle work. Sixteen frame actions emit a preview event.
- No browser runtime errors or horizontal overflow at 1440. The centered content
  measure is 736px, starting at client x=480. `measurements.json` records bounds.
- Native SVG viewboxes, Geist fonts, and the Claude mark's native brand color are
  preserved. Line heights use the fork's `phi()` and pixel rounding.

## Rendered critique

Inspected light/dark at 1440x900, plus 1280x900 and 390x900. The Needs you title
leads, the two plain rows share the native measure, and the empty sentence stays
under the heading. The screen retains the native density rather than adding
cards, a composer or a large display headline. The original Windows frames and
browser outputs are adjacent in `dark-comparison.png` and `light-comparison.png`.

The following are visible limits, not claims of exact fidelity:

- The Windows reference is 1448px wide including an 8px black strip; the browser
  captures the 1440px client. It is not scaled or cropped to conceal differences.
- Current registry selected-row fills are purple, versus gray in the reference.
  Font rasterization and the titlebar caption primitives differ by platform.
- The light reference is empty and carries a known overlapping skew banner.
  The browser omits the old bug. `light-seeded-1440x900.png` also proves the two
  question rows in light mode.
- Small native faint labels and badge tones have contrast limits. The live audit
  composites translucent backgrounds using Chromium's CSS Color 4 parsing:
  **180/232** measured text/icon states clear their threshold; 52 do not. Details
  are in `contrast.json`. This audit exits 1 and is not included in a green claim.
  Example: light question badges reach 3.26 to 3.72 against a 4.5 text threshold.
  A theme correction belongs in Rust first; this PR does not invent overrides.
- The desktop frame overflows below 404px: measured scroll width 404 at viewport
  widths 280, 320 and 390; no overflow at 414 or 1280. Mobile layout is not ported.
- Only Needs you is a real sandbox screen. Other rail/titlebar clicks report their
  callback intent. The fixture harness is not native routing or window management.

## Design-kit checks and their limits

The kit `accuracy_report.mjs` returned **6/37**, not green: it targets absent
`examples/` and template fixtures and cannot resolve its root Playwright dependency.
It does not assess this sandbox. Its Python gate touched a tracked bytecode cache;
that generated change was restored and is not part of the PR.

The kit's `verify_states.mjs` was also run against static exports of this screen
with the installed Chromium. It checked 63 states per theme, but treats translucent
white/black as opaque and parses `oklab()` channels as RGB. Its 28 light / 44 dark
failures therefore are not usable contrast measurements. The live composited audit
above replaces that measurement, without changing or suppressing the kit gate.

The rendered `taste_audit.mjs` flagged the native 20px/13px type ratio (1.5x) as
"timid" in both themes. The requested native visual reference wins over the kit's
2.5x heuristic. `slop_tells.mjs` reported no measurable tells in either theme.
These heuristics are not proof of taste or accessibility.

## Scope and reproduction

Everything in this change lives under `sandbox/`; nothing under `app/` changes.
The existing CI builds Rust only and excludes documentation pushes, so its workflow
is left unchanged. It never installs or builds this sandbox. No cargo commands or
Windows/dtry operations were used for this JS-only change.

Run `npm run proof` with the dev server on port 5177. The capture script accepts
`CHROMIUM_PATH` and `SANDBOX_URL`; it falls back to the committed reference images
when `/store/surya-gallery` is unavailable. `npm run audit:states` repeats the live
contrast measurement. See `GUIDE.md` for the native port-back checks and Windows
proof required when a screen diff is applied to the product.

## Skill round validation

The three sandbox skills now include one wrong/right pair each, linked to real
example files. The same lint accepts all three right specimens and rejects all
three wrong specimens; all **8/8** tests pass, including import/execution of the
right JSX fragments, source-checked native patches, and exact skill excerpt checks.
`npm run lint`, `npm run build`, and the four-frame browser proof pass.

The instruction gates report `SKILL.md: 304/320 lines, 11 always-on rules checked,
7 rule file(s), 7 routed.` and `Scanned 222 file(s).` with no emoji found. All three
new skill frontmatters validate; their local links and heading anchors resolve.

No live screen design changed in this round, so `CHANGES.md` starts empty of design
rounds. The proof rerun used the newer installed Chromium (1243 cache instead of
1228); heading bounds were 101px wide versus 100.640625px in the original capture,
with x/y, height and row bounds unchanged. The original comparison artifacts remain
as the baseline; fresh four-state comparisons are at `/tmp/surya-sandbox-round2-proof`.

## Round three: annotation proxy and final skill nits

The bespoke lavish-live fork fronts Vite directly, stamps host elements with
`sandbox/src/...:line:column`, and uses native semantic colors for its shadow-root
review chrome. No session stack, registry, backend rewriting, Vue lookup,
service-worker handling or agent messaging command was carried over.

Author gates: `npm run lint`, all 26 tests, and `npm run build` pass. Annotation
browser proof passes with real Vite and Chromium: light/dark question-row pins,
390px portrait touch on the rail, draft persistence across reload, CLI polling,
reply then done, pin reopening, refresh, theme/fixture isolation, and a clean
unproxied Vite page. A real temporary source-line shift updates through HMR on
the proxy's own origin; the original source is restored. The screen still has
its existing desktop layout on a phone (405px layout viewport at a 390px device).

Capture review found and fixed the copied touch handler's synthetic click
retargeting to the newly opened backdrop. The regression is exercised by the
phone proof. Pin controls use 44px targets and open on tap; no action needs hover.
The screenshots are [dark](annotate-dark.png), [light](annotate-light.png), and
[portrait touch](annotate-dark-touch.png). Exact received JSON lines are in
[annotate-pins.jsonl](annotate-pins.jsonl); both question-row pins resolve to
`sandbox/src/screens/needs_you/rows.tsx:7:5`. Proof comments are test data, so they
do not become accepted design edits in CHANGES.md.

The whitelist exemption covers annotation tooling only. Screen imports still
cannot reach it; all tooling remains subject to the 500-line and no-emoji gates.
Tailwind explicitly scans only screen source to keep tooling out of the bundle.
Both retained proxy tests and new lifecycle, invalid-input and tooling-boundary
tests pass. All three skills now use the repo's `invocation: model` frontmatter;
the port-back example quotes its tested native before fragment above the pair.

Repository gates report `SKILL.md: 304/320 lines, 11 always-on rules checked,
7 rule file(s), 7 routed.` and `Scanned 222 file(s). OK: no emoji in UI output or
taste files.` The 304/320 figure is a line budget, not failing checks. No extra
router rows were added in round three.

Tailnet status has no private HTTPS mapping for the local annotator at :5178.
The tool prints `tailscale serve --bg --https=8515 http://127.0.0.1:5178`; this
command was not run. Remote reachability therefore remains unverified. Loopback
HTTP and the forwarded HMR path are proven; this is browser tooling proof, not
Windows RC proof. Native screen and contrast gaps above remain recorded.

The original four-frame screen proof was rerun successfully after this change,
including all 16 frame actions. Fresh baseline captures and measurements are at
`/tmp/surya-sandbox-round3-proof`; the existing comparison images remain unchanged
because no screen render code changed. Start, stop and status were also exercised
against the real local annotator.
