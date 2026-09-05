# rename-apply.sh: what it does, what it found, what it leaves

`scripts/rename-dry-run.sh` counts. `scripts/rename-apply.sh` edits.
It is the mechanical half of `docs/rename-zeron-to-surya.md`, re-runnable on
any main, and it was tested by running it - not by reading it.

## Measured on a throwaway clone of main `e0855e8`

```
zeron_hits_before=2640  after=297  files_changed=274  paths_moved=6
```

Second run on the same tree: `files_changed=0 paths_moved=0`. It is idempotent.

The 297 that remain are all deliberate:

| Remaining | Where | Why |
| --- | --- | --- |
| 229 | `apps/ios`, `edge/`, `apps/landing`, `apps/www-redirect` | out of scope, rows 13-15 |
| 38 | `zeron.sh` | the domain stays until surya has one, row 10 |
| 22 | `zeronsh/comet` | the fork's provenance, rows 16-17 |
| 8 | `zeron-dark`, `zeron-light` and their family and display names | user state in `ui-settings.json`, row 19 |

`ZERON_*` is down to 2 hits, both inside `edge/` (out of scope) plus the
generated compat module that names the old prefix on purpose.

## Proof on the renamed tree

- `cargo check --workspace --all-targets`: **0 errors**. All twelve crates
  check under their new names (`surya-proto`, `surya-doc`, `surya-sync`,
  `surya-harness`, `surya-engine`, `surya-rpc`, `surya-update`, `surya-ui`,
  `surya-syntax`, `surya-theme`, `surya-a2ui`, `surya-mcp`).
- Headless tests, the CI set: **383 passed, 0 failed**.

## The one failure, and why it is not the rename

The first full run came back `316 passed, 1 failed`:
`surya-engine`'s `empty_reasoning_deltas_are_heartbeats_not_journal_noise`
counted 0 non-empty reasoning deltas where it wanted 1. (316, not 383, because
cargo stops the remaining test binaries once one fails.)

Three checks, in order:

| Check | Result |
| --- | --- |
| the same test on unrenamed main | passes |
| the same test alone on the renamed tree | passes |
| the whole suite again on the renamed tree | 383 passed, 0 failed |

It did not reproduce. The test polls (`wait_for`) for the run to complete
before counting journal entries, and nothing in it touches the name, so on a
box this loaded it reads as a timing flake rather than a rename fault. Worth
watching before the RC, but it is not a reason to hold the rename.

## A collision the plan did not foresee

Row 19 says "rename the Rust identifiers" for the themes. Doing that literally
does not compile:

```
error[E0428]: the name `surya_dark` is defined multiple times
```

`builtins.rs` already has `surya_light`/`surya_dark` - the new surya theme from
PR #2 - so renaming comet's `zeron_light`/`zeron_dark` onto those names
redefines them. The script renames comet's two to `comet_light`/`comet_dark`,
which is what they are, and keeps their ids, family and display names as user
state. Their display names matter as much as their ids: rewriting "Zeron Dark"
to "Surya Dark" would put two themes called "Surya Dark" in one picker.

This is the reason to run the script rather than review it.

## What it deliberately does not do

The behavioural compat work is real edits at named call sites, not
substitution. The script prints this list when it finishes:

1. **Route the env reads.** The script generates
   `app/crates/engine/src/env_compat.rs` (`SURYA_*`, then `ZERON_*` with a
   one-time warning, for the 16 user-set variables) and declares the module,
   but the call sites still read `SURYA_*` only. **Until they are routed
   through it, a user's existing `ZERON_*` is ignored** - the opposite of what
   the alias is for. Call sites: `main.rs`, `daemon.rs`, engine `lib.rs`,
   `ipc.rs`, `profile.rs`, `repos.rs`, `sessions.rs`, ui
   `sound`/`transcript`/`composer`.
2. **Data dir adoption.** Unresolved, and the two sources disagree: the
   coordinator's brief says **copy** `~/.zeron` (keeps a rollback), while
   `docs/rename-zeron-to-surya.md` row 5 says **rename** (atomic, matches the
   0.2.0 `.comet-native` migration). Needs a decision before anyone writes it.
3. **systemd**: ship `surya.service` and remove the old unit on install, or two
   units race for one IPC port.
4. **URL scheme**: register `surya://` and keep `zeron://` for one release.
5. **Bundle ids** (`sh.zeron.app`) wait on the owner picking a domain.
6. **`cargo update -w`** to regenerate `Cargo.lock`. Not run by the script, so
   it works without a toolchain.

## How to run it

```
scripts/rename-apply.sh --check    # list the files it would touch
scripts/rename-apply.sh            # apply, then print the counters
cd app && cargo update -w          # regenerate the lock
```

Apply it on the final main, after the last feature PR: it touches 274 files
and nothing rebases across it.
