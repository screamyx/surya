# AGENTS.md

Guidance for Claude Code, Codex, and any other agent working in this repository.
`CLAUDE.md` is a one-line `@AGENTS.md` shim so every tool reads this same file.

## What surya is

surya is a fork of zeronsh/comet: a native GPUI desktop app in Rust whose engine daemon runs Claude Code, with haktui's browser (offscreen CEF), editor and tree.
`docs/decisions.md` is the source of truth for every product and engineering ruling.
Decision 22 describes the fork.
Read the decision that covers your task before you change direction.
`docs/handoff-2026-09-05.md` is the running handoff log.
Start from the newest `## STATE AT` block near its end.

## Layout

| Path | What it is |
|---|---|
| `app/` | The comet subtree and the cargo workspace. Every cargo command runs here. |
| `app/crates/browser/` | `surya-browser`, CEF 151 composited into the gpui window. Excluded from the workspace, see below. |
| `deploy/` | `install-engine.sh`, `surya-engine.service`, `windows/build.ps1`. End-user steps are in `docs/quickstart.md`. |
| `docs/` | Decisions, handoff, research, critique rounds, proof shots. |
| `tokens/ taste/ components/ accessibility/ frameworks/ workflows/ content/ design-systems/ scripts/` | The ux-ui-agent-skills design kit. Not part of the app build. Loaded on demand by the `ux-ui-expert` skill. |

## Build and test

Run from `app/`, in your own worktree.
CI runs these five commands.
`.github/workflows/ci.yml` holds the job split and adds `--jobs N` for the shared runner.

```
cargo test -p surya-proto -p surya-doc -p surya-rpc -p surya-engine -p surya-harness -p surya-mcp
cargo check --workspace --all-targets
cargo test -p surya-ui
cargo test -p surya-a2ui
SURYA_SMOKE_CARGO=cargo scripts/smoke.sh
```

Outside CI:

```
cargo build -p surya --features browser      # browser pane, off by default
cargo test -p surya-ui --features browser    # the browser-gated tests; the CI ui job cannot see them
cd crates/browser && cargo test               # the browser crate is its own workspace root
```

- `crates/browser` is a path dependency, not a workspace member. Its build script downloads the CEF binary, and `cargo check --workspace` must never pull it (`app/Cargo.toml`).
- Only the eight crates above run their tests in CI. Every other crate only gets `cargo check`. Run their tests locally when you touch them.
- CI has no `cargo fmt --check` because the upstream comet tree is not fmt-clean. Do not reformat files you did not otherwise change.

## UI checks on Linux

`scripts/tasks-pane-rig.sh` drives the Tasks pane with real clicks and real
keystrokes under Xvfb and prints what the engine saw:

```
caret_and_typing=1   clicked the quick-add box, typed a title, it landed
enter_created=1      pressed enter, the task reached the engine
key_nav=1            clicked a card, pressed n, the New task sheet opened
```

Frames land beside the counters, one timestamped folder per run, nothing
overwritten. `--hold` leaves the stack up and writes the display, port and pids
to `rig.env` so you can drive it by hand with `xdotool`.

This does not replace a dtry frame under decision 28. It is smoke: it catches a
broken journey before anyone spends a slot on the Windows box, and it works
when that box is busy.

Needs `Xvfb`, `xdotool` and `ffmpeg`. Three things cost a session to learn:

- Xvfb needs `+extension XTEST`. Without it `xdotool mousemove` is silently a
  no-op and `getmouselocation` keeps reporting screen centre.
- gpui logs `Found no xinput mouse pointers` on Xvfb. It is harmless; mouse and
  keyboard both work.
- The window does not paint until an event reaches it, so a screenshot taken
  straight after startup is black. One pointer move fixes it.

## Rules

- Every file 500 lines max (decision 13). Split before you cross it.
- Never run cargo in a checkout another agent shares. Build and test in your own git worktree. The shared checkout is docs-only.
- Windows is the product, Mac is next, Linux is a test bench (decision 28). A Linux run is smoke, never RC proof.
- The look is comet's own and the features are ours (decision 26). New panes use comet's tokens.
- No emoji anywhere: UI, code, docs, commit messages. `python3 scripts/check_no_emoji.py` checks this file, the skills, and the kit's specs.
- After moving items out of a module, check every child test module for names it was inheriting through `use super::*`. A test module that reaches a trait or type through its parent's private imports compiles until the item moves, then fails. Four branches broke this way on 2026-09-08.
- Quote a cargo run only from a log that shows the crate was built in your worktree: `Compiling <crate> vX (<your worktree>/app/crates/<dir>)` for build and test, `Checking <crate> ...` for check. A shared target dir can replay a green from cached artifacts in under a second with nothing compiled, and neither a pure move nor an added test will show it in the count. For positive proof, take the binary path from the `Running unittests` line and grep it for a symbol only your branch has.

## Pull requests

- One PR per change. Do not stack PRs.
- A PR merges only with all three: the author's own green run, an independent review verdict of MERGE, and green CI including smoke.
- Check for conflicts against main before asking for the verdict, with the modern form: `git merge-tree --write-tree --name-only HEAD origin/main`.
  It exits 1 and prints `CONFLICT` lines when the branch conflicts.
  The legacy three-argument form always exits 0 and prints a diff either way, so its result cannot be read as clean or conflicted.
- Squash-merge, then delete the branch.
- Squash leaves no trace in git that a branch merged, so `git branch --merged` and `git branch -r --contains` both call a landed branch unmerged.
  `app/scripts/stale-worktrees.sh` asks GitHub instead: it force-fetches `refs/pull/*/head`, reads each PR's state through gh-axi, and prints one verdict per worktree (REMOVE, EDITS, OPEN-PR, CLOSED-PR, PUSHED, UNPUSHED, OTHER-UID).
  It is a dry run by default. `--remove` deletes only the REMOVE and ON-MAIN rows this uid owns; a top folder under root-owned `/store/agent-worktrees` is left empty for `sudo rmdir`.
- After a merge batch, run `app/scripts/stale-worktrees.sh` once and remove what it marks REMOVE. It is a manual step, not a hook: squash merges happen on GitHub, so no local git event fires, and the EDITS rows need a person to decide.
- A UI change ships with a Windows winbox screenshot in the PR or the gallery (decision 28). A Linux shot is smoke, not proof.

## CI and git

- CI is two jobs (`.github/workflows/ci.yml`, decision 33). `linux` runs on GitHub-hosted `ubuntu-latest` and is guarded by the repository being public, so it costs nothing while surya is private. `windows` runs on the owner's PC winbox, label `[self-hosted, surya-win]`, at idle priority.
- Only actions matching `actions/*` are allowed to run here. The repository's Actions policy is `allowed_actions: selected`, so a third-party action makes the whole workflow a `startup_failure` with no jobs.
- `browser-nightly.yml` still runs on `[self-hosted, surya-ci]` on devbox. Never `sudo rm -rf` or `apt-get` in that one: it is the owner's shared machine. Those two steps are fine in `ci.yml`'s `linux` job, which is a throwaway VM.
- Pushes that only touch `docs/**` or `*.md` skip CI.
- `app/.github/workflows/` is comet's own release pipeline. GitHub never runs it from here. Leave it alone.
- Commit subjects are `area: summary` or `type(scope): summary`, as in `browser:`, `inbox:`, `fix(ui):`, `docs(handoff): HH:MM summary`. PRs are squash-merged and keep the `(#N)` suffix.
- Remotes: `origin` is screamyx/surya, `comet` is zeronsh/comet for subtree pulls.

## Design work

The design kit is for look-and-feel judgment, not for the Rust build.
Invoke the `ux-ui-expert` skill when a task is about visual design or tokens.
It carries the persona, the verification gates, and the router to the `design-*` skills.
Its HTML gates do not apply to GPUI code.
`/critique` renders screenshots of a build and argues for rejection. Run it after a UI change, never instead of the tests.
