# rename-apply.sh: what it does, what it found, what it leaves

`scripts/rename-dry-run.sh` counts. `scripts/rename-apply.sh` edits.
It is the mechanical half of `docs/rename-surya-to-surya.md`, re-runnable on
any main, and it was tested by running it - not by reading it.

## Measured on a throwaway clone of main `e0855e8`

Historic. The current numbers, on main `77048b0` with the fixes below,
are in `docs/rename-runbook.md`.

```
surya_hits_before=2640  after=297  files_changed=274  paths_moved=6
```

Second run on the same tree: `files_changed=0 paths_moved=0`. It is idempotent.

The 297 that remain are all deliberate:

| Remaining | Where | Why |
| --- | --- | --- |
| 229 | `apps/ios`, `edge/`, `apps/landing`, `apps/www-redirect` | out of scope, rows 13-15 |
| 38 | `zeron.sh` | the domain stays until surya has one, row 10 |
| 22 | `zeronsh/comet` | the fork's provenance, rows 16-17 |
| 8 | `surya-dark`, `surya-light` and their family and display names | user state in `ui-settings.json`, row 19 |

`SURYA_*` is down to 2 hits, both inside `edge/` (out of scope) plus the
generated compat module that names the old prefix on purpose.

## Proof on the renamed tree

- `cargo check --workspace --all-targets`: **0 errors**. All twelve crates
  check under their new names (`surya-proto`, `surya-doc`, `surya-sync`,
  `surya-harness`, `surya-engine`, `surya-rpc`, `surya-update`, `surya-ui`,
  `surya-syntax`, `surya-theme`, `surya-a2ui`, `surya-mcp`).
- Headless tests, the CI set: **383 passed, 0 failed** on a clean run;
  `315 passed, 1 failed` on a loaded one, from a flake that fails on main too
  (below). Cargo stops the remaining test binaries after a failure, which is
  why the totals differ.

## The one failure: pre-existing on main, not the rename

`surya-engine`'s `empty_reasoning_deltas_are_heartbeats_not_journal_noise`
counts 0 non-empty reasoning deltas where it wants 1. It failed in two of
three loaded full runs on the renamed tree, which is too often to wave off as
noise.

The decisive check is the one that isolates the variable: **the same test,
the same loaded full-suite run, on unrenamed main**.

| Run | Result |
| --- | --- |
| renamed tree, loaded full suite | fails (2 of 3) |
| renamed tree, that test alone | passes |
| unrenamed main, that test alone | passes |
| **unrenamed main, loaded full suite** | **fails** |

It fails on main too. The rename does not cause it. The test polls
(`wait_for`) for a run to complete before counting journal entries, and on a
saturated box that poll loses. It belongs on the flake list, and it is not a
reason to hold the rename.

Recorded because an earlier version of this note called it a load flake on
weaker evidence - one isolated pass - and the isolated run was never the
comparison that mattered.

## A collision the plan did not foresee

Row 19 says "rename the Rust identifiers" for the themes. Doing that literally
does not compile:

```
error[E0428]: the name `surya_dark` is defined multiple times
```

`builtins.rs` already has `surya_light`/`surya_dark` - the new surya theme from
PR #2 - so renaming comet's `comet_light`/`comet_dark` onto those names
redefines them. The script renames comet's two to `comet_light`/`comet_dark`,
which is what they are, and keeps their ids, family and display names as user
state. Their display names matter as much as their ids: rewriting "Zeron Dark"
to "Surya Dark" would put two themes called "Surya Dark" in one picker.

This is the reason to run the script rather than review it.

## The compat the script does write

**The env alias, routed.** `app/crates/proto/src/env_compat.rs` reads
`SURYA_<name>`, falls back to `SURYA_<name>`, and warns once per variable per
process. It lives in proto because that is what everything already depends on,
and its signatures mirror `std::env` exactly - `var` returns the same
`Result`, `var_os` the same `Option` - so every call site keeps its `.ok()`,
`.is_err()` and `.unwrap_or_else(|_| …)` untouched. The script then routes the
16 user-set reads through it: `files_routed=6`,
`user_set_reads_still_direct=2`.

Those two are deliberate: `crates/mcp/src/tasks.rs` belongs to the tasks seat,
and `crates/rpc/tests/device_room.rs` is a test. Dev and test knobs
(`SURYA_MOCK_*`, `SURYA_DEMO_*`, `SURYA_ACP_*`) keep reading `std::env`
directly - nothing outside this repo sets them, so they need no alias.

**Three compat blocks**, each marker-guarded so a second run skips them
(`compat_blocks_added=3`):

| Block | Why |
| --- | --- |
| `links.rs` parses `surya://` as well as `surya://` | deep links live in chats people already have |
| `lib.rs` registers both schemes | the OS has to route the old one |
| `daemon.rs` disables and removes `surya.service` on install | two enabled units race for one IPC port |

**`cargo update -w`** runs when cargo is on PATH (`cargo_update=ok`) and is
skipped with a note when it is not, so the script still works without a
toolchain.

## What it still does not do

1. **Data dir.** Ruled: **copy** `~/.surya` to `~/.surya` on first start, never
   rename, because a copy keeps a rollback. That is a real migration at the
   point the data dir is resolved, with its own test - not a substitution, so
   it is not in this script.
2. **Bundle ids** (`sh.surya.app`, the notify id, the conversation URL type)
   wait on the owner picking a domain.

## How to run it

`docs/rename-runbook.md` is the operator's copy: the exact commands, the
counter to expect on every line, and what to do when one of them is wrong.
Read that on the day. This file stays the record of how the script was built
and what running it found.

## What the full dry run found, 2026-09-05, main 77048b0

The dry run was run on a throwaway clone, end to end, for the first time since
`deploy/` and the tasks-demo scripts landed. It found three breaks.

**`deploy/` was never in scope.** `install-engine.sh` builds
`cargo build --release -p surya` and installs `target/release/surya`;
`surya-engine.service` runs `ExecStart=%h/.local/bin/surya headless`;
`windows/build.ps1` builds `-p surya` and copies `release\surya.exe`. After
the rename none of that exists. The installer and the Windows packager both
fail. 17 lines across four files, now in scope.

**The selector missed all-caps.** `[Zz]eron` does not match `SURYA_`, so
`crates/proto/build.rs` and `crates/proto/src/build.rs` were never selected
and kept `SURYA_BUILD_SHA`. Both sides agreed, so nothing broke - but only by
luck. The selector now matches `SURYA_` too.

**Fixing that broke two files that must keep the old name.** With `SURYA_` in
the pattern, the GENERATED `env_compat.rs` went in scope on the SECOND run and
its fallback became `var_os("SURYA_{name}")` - the alias reading the new name
twice, the compat dead, the tree still compiling. And
`crates/engine/src/data_dir.rs` had its test fixtures rewritten to `.surya`,
so it adopted `.surya` into `.surya`: three tests red, and a module doc
reading "moves the data dir from `~/.surya` to `~/.surya`". Both files are now
excluded outright rather than masked line by line.

Only the second apply catches the middle one, which is why the runbook makes
that run mandatory.

**A guard, so the next directory does not sit outside the counters.** After
the substitution the script now reads the whole repo, subtracts the
out-of-scope trees and every hit the masks keep on purpose, and exits non-zero
on what is left. Proved with a positive, not an absence: a planted
`packaging/build.sh` containing `-p surya` printed `MISSED=1` and exited 1.

**Measured on the unpatched script**, so the comparison is honest:
`cargo check --workspace --all-targets` gives 0 errors and 21 warnings on the
renamed tree, and the *same* 21 warnings and 0 errors on unrenamed main. The
rename introduces nothing. The headless set was `177 passed, 3 failed` - the
three `data_dir` tests above, and no others.

**On the patched script** the same headless set is `723 passed, 0 failed, 10
ignored`, and all five `data_dir` tests are green.

## Not fixable here: comet's release workflow

`app/.github/workflows/release.yml` line 134 asserts
`ls dist/ | grep -q "surya-$ver-"`, while the renamed `package-linux.sh`
writes `surya-$ver-linux-$arch`. That workflow is comet's own and GitHub never
fires it from a subdirectory, so it is dead in this repo - but it is wrong the
moment anyone revives it. Owner call, not a script rule.
