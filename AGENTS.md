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
cargo test -p zeron-proto -p zeron-doc -p zeron-rpc -p zeron-engine -p zeron-harness -p surya-mcp
cargo check --workspace --all-targets
cargo test -p zeron-ui
cargo test -p surya-a2ui
SURYA_SMOKE_CARGO=cargo scripts/smoke.sh
```

Outside CI:

```
cargo build -p zeron --features browser      # browser pane, off by default
cd crates/browser && cargo test               # the browser crate is its own workspace root
```

- `crates/browser` is a path dependency, not a workspace member. Its build script downloads the CEF binary, and `cargo check --workspace` must never pull it (`app/Cargo.toml`).
- Only the eight crates above run their tests in CI. Every other crate only gets `cargo check`. Run their tests locally when you touch them.
- CI has no `cargo fmt --check` because the upstream comet tree is not fmt-clean. Do not reformat files you did not otherwise change.

## Rules

- Every file 500 lines max (decision 13). Split before you cross it.
- Never run cargo in a checkout another agent shares. Build and test in your own git worktree. The shared checkout is docs-only.
- Windows is the product, Mac is next, Linux is a test bench (decision 28). A Linux run is smoke, never RC proof.
- The look is comet's own and the features are ours (decision 26). New panes use comet's tokens.
- Crate names are mixed on purpose: upstream crates stay `zeron-*`, new ones are `surya-*`. The rename lands last, on frozen main, from `docs/rename-runbook.md`. Do not rename piecemeal.
- No emoji anywhere: UI, code, docs, commit messages. `python3 scripts/check_no_emoji.py` checks this file, the skills, and the kit's specs.

## Pull requests

- One PR per change. Do not stack PRs.
- A PR merges only with all three: the author's own green run, an independent review verdict of MERGE, and green CI including smoke.
- Check `git merge-tree` against main before asking for the verdict.
- Squash-merge, then delete the branch.
- A UI change ships with a Windows dtry screenshot in the PR or the gallery (decision 28). A Linux shot is smoke, not proof.

## CI and git

- CI runs on self-hosted runners tagged `[self-hosted, surya-ci]`, all on one shared machine. Never `sudo rm -rf` or `apt-get` from a workflow (`.github/workflows/ci.yml`).
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
