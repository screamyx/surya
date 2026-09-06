# Contributing

surya is a native GPUI desktop app in Rust whose engine daemon runs Claude Code.
`AGENTS.md` is the full working guide and this file is the short version of it.
`docs/decisions.md` is the source of truth for every product and engineering ruling.
Read the decision that covers your change before you write code.

## Build and test

Run these from `app/`.
CI runs the same five commands.

```
cargo test -p surya-proto -p surya-doc -p surya-rpc -p surya-engine -p surya-harness -p surya-mcp
cargo check --workspace --all-targets
cargo test -p surya-ui
cargo test -p surya-a2ui
SURYA_SMOKE_CARGO=cargo scripts/smoke.sh
```

The browser pane is off by default and builds separately.

```
cargo build -p surya --features browser
cd crates/browser && cargo test
```

`crates/browser` is a path dependency, not a workspace member.
Its build script downloads the CEF binary, and `cargo check --workspace` must never pull it.

Only the eight crates named above run their tests in CI.
Every other crate gets `cargo check` only, so run its tests locally when you touch it.

There is no `cargo fmt --check` in CI, because the upstream comet tree is not formatting clean.
Do not reformat files you did not otherwise change.

## Rules

- Every file is 500 lines maximum. Split before you cross it.
- No emoji anywhere: user interface, code, docs, commit messages. `python3 scripts/check_no_emoji.py` checks this.
- No em dash. Use a plain dash instead.
- New panes use comet's tokens. The look is comet's own and the features are ours.
- `app/.github/workflows/` is comet's own release pipeline and never runs from here. Leave it alone.

## Pull requests

One pull request per change. Do not stack them.

A pull request merges only with all three: your own green run, an independent review, and green CI including smoke.
Check `git merge-tree` against `main` before you ask for the review.
Merges are squash merges, and the branch is deleted after.

Commit subjects are `area: summary` or `type(scope): summary`.
`browser:`, `inbox:` and `fix(ui):` are all examples.

A change to the user interface ships with a Windows screenshot in the pull request.
Windows is the product, Mac is next, and Linux is a test bench, so a Linux screenshot is smoke and not proof.

## Security

Do not open a public issue for a security problem.
`SECURITY.md` has the private reporting route and the threat model.
