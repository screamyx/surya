# Public redaction plan, 2026-09-07

A proposal, not a redaction.
Nothing in `docs/` was edited to produce this file.
The owner rules on every row before anything is changed.

Scope: every tracked file under `docs/`, plus the code and workflow lines that carry the same details, plus all 185 images under `docs/`.

## Item 0: rotate the engine token, before anything else

Confirmed by two readers against the image itself.

`docs/quickstart.md:78` embeds `images/quickstart-servers-add.png`, which shows the Add server dialog with a fully legible 64-character engine token.
`docs/images/dtry-r4-servers-add-dark.png` shows the same value.

The token is not live.
remote2 compared it against the only two engines on the box, and it belongs to a pre-rename zeron engine.
No rotation is needed.

| Owner | Action | When |
|---|---|---|
| Whoever reshoots | Reshoot both images with the token blurred, or with an obviously fake value | Before any public flip |

No seat edits these two images on its own. The reshoot is ordered, not improvised.
The detail behind this item is in "Fix before public under either option" below.

## Two options for the owner

### Option A: redact in place

Rewrite the offending lines in the current files.
The handoff log keeps its history and its links, and the repository stays one piece.

The cost is that `docs/handoff-2026-09-05.md` carries 215 machine lines and 165 fleet lines out of 861.
Redacting it line by line is most of a rewrite, and the result reads worse than the original.

### Option B: move the operational logs to a private repository

Move `docs/handoff-2026-09-05.md` and `docs/acceptance/` to a private repository.
Keep `docs/decisions.md` after a light pass, because it is the product record and the thing a reader actually wants.
Keep the probe, perf and critique notes after the small fixes listed below.

This is the recommendation.
Almost every finding in the tables below sits in the two files Option B removes, and the ones that do not are a handful of lines plus a set of screenshots.

Option B does not cover the code fixtures or the images.
Those need fixing under either option.

## Fix before public under either option

Two findings are not "operational noise" and do not go away by moving a log file.

### 1. An engine token is legible in a screenshot the quickstart embeds

`docs/quickstart.md:78` embeds `![Add server dialog](images/quickstart-servers-add.png)`.
That image shows the Add server dialog with the Token field filled in.
Magnified, the field is fully legible: 64 hexadecimal characters wrapped over two lines, beginning `4b1e7c2d` and ending `4a3f2e`.
The value is deliberately not reproduced here, because writing it into a tracked file would put the credential into the repository text, where it currently is not.
The same value appears in `docs/images/dtry-r4-servers-add-dark.png`.
It appears nowhere in the repository text, so nothing in the tree says whether it is live or made up.

Three lines above the image, `docs/quickstart.md:34` tells the reader: "Keep that token private. Anyone who has it can run agents as you on that box."
`docs/quickstart.md:30` deliberately truncates the token in the sample output as `Token: 3f9c...  (64 characters)`.
The text follows the rule and the picture on the same page breaks it.

Checked after the fact: remote2 compared the pictured value against the only two engines on the box and it is a pre-rename zeron engine's token, so it is not live.
That does not change the fix.

| Action | Detail |
|---|---|
| redact | Reshoot both images with the token blurred, or with an obviously fake value. |

### 2. Another person's name, private repositories and private work titles are legible in screenshots

Five files under `docs/research/shots/` are captures of a signed-in Codex desktop app, not vendor marketing.

| File | What is legible |
|---|---|
| `codex-activity.png` | Account name "Ken Shishou" bottom left. The transcript reads "The repository is `screamyx/proton-pn1`". A Recents list of about twenty private chat titles. |
| `codex-home.png` | Same account name, same Recents list. |
| `codex-result.png` | Same account name, same Recents list. |
| `codex-session.png` | Same account name, same Recents list. |
| `codex-diff.png` | Same, plus the contents of a private skill file, `.agents/skills/agile-v-smoke-stack/SKILL.md`, and a tree of private skill names. |

The Recents titles name outside work: "Investigate CarBase market guide", "Explain Meta catalog ad targeting", "Optimize Alphard record SEO", "Plan 3 AIA Facebook campaigns", "Explain ViewContent external ID", "Link WhatsApp data source", "Assess missing Advantage+ catalog", "Explain Lp CVR and DPA", "Explain marketing message failure".

Three more images leak private repositories.

| File | What is legible |
|---|---|
| `docs/images/browser-zero-copy-off.png` | `https://github.com/RizRiyz/luvus/pull/266`, session "luvus @ pc-ajim", branch `dock-row-tone`, and `http://100.83.77.3:8766/spin.html`. |
| `docs/images/browser-zero-copy-on.png` | The same. |
| `docs/shots/tasks-pane-2026-09-05.png` | A board titled "project-jag" with `screamyx/project-jag#553` and `screamyx/project-jag#591`, six task titles from that project, and agent names raven, kite, swift, heron. |

Decision 18 says it plainly: "No real email, hostname, or agent id in the repo."
A real display name and a private repository name are further over that line than a hostname.

Action: replace or remove. Reshoot, crop or drop these eight images. The owner rules on which.

A ninth, `docs/images/a2ui-dtry-r4-cards-1-2.png`, shows a session list entry reading "Downloads @ pc-ajim" over a chat titled "**Local Environment Configurati...".
The title is cut off, so nothing private is actually readable. Action: keep.

## The image pass

All 185 tracked images under `docs/` were looked at.
Method: four images at a time. The first twenty-one were opened at full size one by one, and the remaining 164 were opened as forty-one four-up contact sheets, with every flagged image then reopened at full resolution and magnified to read its text. Every finding below was read from the pixels, not inferred from the file name.

| Category | Count | Files |
|---|---|---|
| Credential legible | 2 | `images/quickstart-servers-add.png`, `images/dtry-r4-servers-add-dark.png` |
| Real person or private repository legible | 8 | the five `research/shots/codex-*.png`, `images/browser-zero-copy-{on,off}.png`, `shots/tasks-pane-2026-09-05.png` |
| Tailnet IP legible | 6 | `images/rc1-dtry-servers-{dark,light}.png`, `images/servers-command-line-{row,saved}.png`, `images/browser-zero-copy-{on,off}.png` |
| Owner path or kernel string legible | 6 | `images/quickstart-chat.png` (`/home/user2/Downloads`), `images/dtry-r5-{A,B,C}-*.png`, `images/version-guard-retest-reply.png`, `images/failed-gate-escape.png` |
| Third-party product screenshot | 50 | everything under `research/shots/` except the five codex files |
| Host name only, in window chrome | about 90 | most of `critique/shots/`, `design/`, `proof/`, `shots/` |
| Nothing outside the app window | 185 | every image. No desktop, no taskbar, no wallpaper, no second application, no notification. |

No image shows the owner's desktop.
Every capture is a window capture, which is what the shot rig is built to do.
The leaks are all inside the app window, in its own text.

A note on the third-party set: `docs/research/shots/` holds 50 screenshots of the Cline, Conductor, Cursor, Devin, Factory, GitHub Copilot, Kiro, OpenHands and Windsurf documentation and marketing sites, republished with no licence or attribution.
Four of them embed video thumbnails showing identifiable people.
That is a copyright and likeness question, not a privacy one.
Recommendation: keep, but add a line to `docs/research/` stating that these are vendor screenshots used for comparison, and replace them with links if the owner prefers.

## Code and configuration, outside docs/

These carry the same details and are unaffected by Option B.

| Location | What it carries | Action |
|---|---|---|
| `app/crates/engine/src/ipc.rs:187-188` | Test fixture uses the real tailnet IP `100.64.0.9` | redact, use a documentation-range address |
| `app/crates/ui/src/settings/servers/active.rs:214-215` | Test fixture uses `ws://100.83.77.3:27700` | redact |
| `app/crates/ui/src/settings/servers/active.rs:129` | Test fixture uses `ws://PC-Ajim:27700` | redact |
| `app/crates/ui/src/settings/servers/add.rs:116` | Placeholder text reads "Host, e.g. 100.64.0.9 or build-box", so the real IP ships in the user interface | redact |
| `app/crates/ui/src/settings/servers/add.rs:201,214,216,219` | Test fixtures use `pc-ajim` and `100.64.0.9` | redact |
| `docs/quickstart.md:28,31,88,89` | Sample output and command lines use `100.64.0.9` | redact |
| `.github/workflows/ci.yml:11-12` | Runner paths naming `user2` | keep, it is the runner's real account and CI needs it. Confirm with the owner. |
| `scripts/__pycache__/*.pyc` | Two compiled Python files are tracked | remove from the index and add `__pycache__/` to `.gitignore`. Hygiene, not privacy. |

`sandbox/annotate/server.mjs:74` and `server.test.mjs:49,60,62` also match a tailnet pattern.
They use the generic suffix `.ts.net` and the made-up host `tail123`, so: keep.

## docs/, file by file

Legend: `move` = goes private under Option B, `redact` = rewrite the line, `keep` = fine as it is.

### The two operational logs

| File | Lines | What they carry | Action |
|---|---|---|---|
| `docs/handoff-2026-09-05.md` | 861 total | 215 lines naming `pc-ajim`, `dtry`, `E:\`, `/home/user2` or `user2`; 165 lines naming fleet tooling, quota or sleep; 10 lines with the tailnet host or IP (14, 16, 158, 189, 468, 502, 508, 517, 530, 605); 2 lines describing unfixed security findings (573, 580) | move |
| `docs/acceptance/e2e-2026-09-06.md` | 3, 4, 5, 9, 33, 37, 45, 95, 102, 104, 109, 125, 155, 163, 166, 193, 195, 226, 227 | Line 5 is the gallery URL `http://pc-ajim.tail82fec1.ts.net:8613/e2e/`. Line 148 names a saved engine `100.83.77.3:27710`. The rest name dtry, E:\ paths and seat ids. | move |

On line 573 of the handoff: it records the two browser findings in plain text, including "Browser.Call/Watch unauthenticated on the open loopback socket" and "browser_eval = unrestricted JS by design".
Both are now covered honestly in `SECURITY.md`, so publishing them is no longer a disclosure problem.
It is the surrounding operational detail that argues for moving the file, not those two lines.

### decisions.md

| Lines | What they carry | Action |
|---|---|---|
| 307, 313 | Decision 24's title and body name `pc-ajim`, the runner `pc-ajim-surya` and the service `actions.runner.screamyx-surya.pc-a...` | redact the host, keep the decision |
| 325, 327, 366, 367, 376, 377, 385, 389, 393, 394 | Name `dtry` as the Windows machine, and `E:\` | keep. "dtry" is a nickname with no network meaning, and decisions 28 to 31 are unreadable without it. |
| 20 fleet lines | Name agb, luvus, seat ids and quota | redact. Decision 18 already rules that fleet tooling is "not mentioned in product code or README", and these are product rulings. |

decisions.md already says "the owner" throughout with no name, which is what decision 18 asks for.

### Probe, perf and critique notes

| File | Lines | Action |
|---|---|---|
| `docs/probes/windows-dropped-quads-2026-09-05.md` | 37, 39, 40, 41, 42, 47, 48, 65, 70, 89, 90, 91, 104, 105, 106, 108, 109, 110, 115, 127 | keep, redact the two `pc-ajim` lines |
| `docs/probes/rc1-dtry-2026-09-05.md` | 1, 9, 18, 19, 20, 21, 22, 23, 24, 42 | keep, redact line 42 which carries `ws://100.83.77.3:27700` |
| `docs/perf/browser-scroll-2026-09-05.md` | 5, 27, 41, 43, 58, 60, 61, 83, 88, 90, 91, 119, 127 | keep, dtry and E:\ only |
| `docs/perf/windows-frame-pacing-measured-2026-09-06.md` | 1, 11, 13, 120, 125, 129, 140 | keep |
| `docs/perf/windows-frame-pacing.md` | 30 | keep |
| `docs/rc-2026-09-06.md` | 3, 5, 7, 15, 36, 38, 41, 61 | keep, redact the four `pc-ajim` lines |
| `docs/probes/external-texture/README.md` | 16, 17, 18, 27 | keep |
| `docs/probes/windows-zero-copy-2026-09-05.md` | 4, 43 | keep |
| `docs/probes/agui-disconnect-2026-09-05.md` | 40 | keep, redact the `user2` path |
| `docs/probes/headless-controls-2026-09-05.md` | 5 | keep |
| `docs/critique/round-3-2026-09-05.md` | 34 | keep |
| `docs/critique/round-5-rc2.md` | 27 | keep |
| `docs/design/comet-look-restore.md` | 8, 192, 308, 321 | keep, redact the seat ids |
| `docs/research/synthesis.md` | 16 | keep |
| `docs/research/orca.md` | 4 | keep |
| `docs/research/claude-code-desktop.md` | 4 | keep |

## Repository settings, and the one hazard in the flip

Applied on 2026-09-07 with `gh-axi`, on branch `chore/public-readiness`.

| Setting | Before | After |
|---|---|---|
| Description | "Web harness for Claude Code: code, live preview with pins, task board, persistent agents" | "surya: a native desktop app for Claude Code with a browser, editor and task board (Windows first)" |
| Topics | none | claude-code, desktop, gpui, rust, windows |
| Dependabot vulnerability alerts | disabled, `GET vulnerability-alerts` returned 404 | enabled, returns 204 |
| Actions `allowed_actions` | `all` | `selected`, with GitHub-owned and verified creators allowed plus the pattern `actions/*` |

The workflows use only `actions/checkout@v4` and `actions/upload-artifact@v4`, so the narrowed list cannot break CI.

### Branch protection: there is none

`GET /repos/screamyx/surya/branches/main/protection` returns FORBIDDEN with this token, and so does `GET /rulesets`.
`GET /repos/screamyx/surya/branches/main` is readable and returns `protected: false`.

So `main` has no protection at all today.
The owner should set, before the flip: require a pull request, require the `ci` check to pass, and block force pushes.

### The hazard: the fork approval gate cannot be set while the repository is private

`.github/workflows/ci.yml:30-31` triggers on `pull_request`, and line 64 is `runs-on: [self-hosted, surya-ci]`.
Both runners live on pc-ajim.
On a public repository that means a stranger's fork pull request can run their code on that machine once a run is approved.

The setting that guards this is `PUT /repos/{owner}/{repo}/actions/permissions/fork-pr-contributor-approval`.
Reading it today returns: "Fork PR approval is not allowed for private repositories."
It cannot be set until after the flip.

That makes the flip an ordered operation, not a single click:

1. Before the flip, take the `pull_request` trigger off the self-hosted runners, or move pull request runs to GitHub-hosted runners.
2. Flip the repository to public.
3. Immediately set fork pull request approval to require approval for all outside collaborators, in Settings, Actions, General, or through the endpoint above.
4. Set branch protection on `main`.
5. Turn on private vulnerability reporting, which is what `SECURITY.md` points at. `PUT /repos/screamyx/surya/private-vulnerability-reporting` returns 404 today, because the feature does not exist on a private repository.
6. Only then put the `pull_request` trigger back on the self-hosted runners, if that is still wanted.

`GET /repos/screamyx/surya/actions/permissions/fork-pr-workflows-private-repos` currently reads `run_workflows_from_fork_pull_requests: false`, and `GET /actions/permissions/access` reads `access_level: none`, so nothing is exposed while the repository stays private.

## What was checked and found clean

- No email address appears in any tracked file's contents. The only addresses are upstream copyright lines in `app/THIRD_PARTY_NOTICES.md` and made-up ones in Rust tests.
- No API key, password or private key was found in any tracked file.
- The sample data is made up throughout: Ahmad F., Mei Ling, Rajesh K., Zul, Farah, and a Toyota Alphard listing. Decision 18 allows exactly this.
- No image shows anything outside the application window.
