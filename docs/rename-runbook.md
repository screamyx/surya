# Rename runbook: surya -> surya

Run this on frozen main, after the last feature PR merges.
It touches 329 files, so nothing rebases across it.

Dry-run source for every number below: main `9a358f40`, 2026-09-06 19:25, on
a throwaway clone. The counts grow as main grows - a small drift is normal, a
sudden drop is not. Re-run the dry run and refresh this table whenever main
has moved much; a runbook that disagrees with the script is worse than none.

## The commands

```
git fetch origin && git checkout -b rename/surya-to-surya origin/main

scripts/rename-apply.sh --check      # lists the files, writes nothing
scripts/rename-apply.sh              # applies, prints the counters
scripts/rename-apply.sh              # again: must print files_changed=0

cd app
cargo check --workspace --all-targets
cargo test --jobs 2 -p surya-proto -p surya-doc -p surya-rpc \
                    -p surya-engine -p surya-harness -p surya-mcp
cargo test --jobs 2 -p surya-a2ui
SURYA_SMOKE_CARGO=cargo scripts/smoke.sh
```

Those are CI's three jobs, in CI's order - the a2ui test and the smoke are
two steps of one job.
The smoke is a required check and links `-p surya`, so it is the slowest and
the one most likely to show a rename that only breaks at run time.

The second apply is not optional.
It is the only check that catches a rule which rewrites its own output, and
that class of bug has now been found three times in this script.

## Expected counters

| Line | Expect | Meaning |
| --- | --- | --- |
| `files in scope` | 329 | grows with main; a sudden drop means the scope broke |
| `files_changed` | 329 | first run |
| `paths_moved` | 6 | `apps/surya`, three `dist/` assets, the ui logo, the theme-import bin |
| `files_routed` | 8 | env reads sent through the compat alias |
| `user_set_reads_still_direct` | 1 | `crates/rpc/tests/device_room.rs`, a test |
| `compat_blocks_added` | 3 | links.rs, lib.rs, daemon.rs |
| `cargo_update` | `ok` | the lock regenerated |
| `surya_hits_before / after` | 3050 / 340 | every remaining hit is listed below |
| `missed` | 0 | the scope-drift guard found nothing |

Second and third run: `files_changed=0 paths_moved=0 compat_blocks_added=0`,
and `files in scope` drops to 25 (the files that keep the old name on purpose).

## If a step fails

| Symptom | What it means | Do |
| --- | --- | --- |
| `FATAL: expected hits=1 for the data dir adoption pair` | rustfmt reshaped the call, or it moved | fix the rule in `advance_data_dir_migration`, do not skip it |
| `MISSED=n - files with the old name that no rule covers` | a new directory arrived outside the scope | add it to `in_scope_files`, re-run from a clean tree |
| second run reports `files_changed` > 0 | a rule is rewriting its own output | find the file with `git diff`, mask or exclude it |
| `cargo_update=failed` or `=skipped`, then `STOPPING` | no cargo, or the lock is conflicted | the script prints every counter, then stops with exit 1 and leaves the tree renamed. Run `cd app && cargo update -w`, then run the script again - it passes, because the check reads that run's result |

The script edits in place.
If anything above fires, `git checkout -- .` and start again - it is
re-runnable on any clean main, and takes about three seconds.

## The 340 hits it leaves, all deliberate

Counted the way the script counts: `rg` over `app deploy docs .github`.
The eight rows sum to exactly 340 (`docs/` gained one as main moved).

| Hits | Where | Why |
| --- | --- | --- |
| 250 | `apps/ios`, `edge/`, `apps/landing`, `apps/www-redirect` | out of scope, rows 13-15 |
| 40 | `theme/src/{lib,builtins,vscode}.rs`, `ui/src/{theme,settings}.rs` | `surya-dark` / `surya-light` ids, family and display names are user state in `ui-settings.json` (row 19), down to the saved default and its test |
| 15 | `crates/engine/src/data_dir.rs` | the module that adopts the old dir, excluded from the rename |
| 15 | `docs/` | the rename plan, notes and this runbook, which have to name both words |
| 7 | `ui/src/{markdown/parser,transcript}.rs`, `harness/{src/adapter_install,tests/managed_install*}.rs` | `zeronsh/comet` provenance links (rows 16-17) |
| 6 | `app/docs/`, `app/README*.md` | same |
| 4 | `apps/surya/src/{main,update_cli}.rs` | `zeron.sh` and `edge.zeron.sh`, the domain until surya has one (row 10) |
| 3 | `ui/src/{links,lib}.rs`, `apps/surya/src/daemon.rs` | the compat blocks name the old scheme, registration and unit on purpose |

`crates/proto/src/env_compat.rs` shows zero here and still holds 11 hits: they
are all the all-caps `SURYA_` prefix, which `[Zz]eron` does not match. The same
gap kept `crates/proto/build.rs` out of the rename until 2026-09-05, and is why
the selector now matches `SURYA_` as well.

`AGENTS.md` is in scope but outside these four roots, so the script's counter
never sees its 3 remaining hits: lines 8 and 72 are `zeronsh/comet`
provenance, and line 55 is the masked bullet the operator deletes by hand.
`sandbox/` comes out clean.

`app/.github/` is not counted at all - `rg` skips dot directories, and the
script never had it in scope. It holds 17 more, across `deploy.yml`,
`release.yml`, `testflight.yml` and `FUNDING.yml`.

## Two things the script does not do

1. **Bundle ids.** `sh.surya.app`, the notify id and the conversation URL type
   wait on the owner picking a domain.
2. **`app/.github/workflows/release.yml` line 134.** It asserts
   `ls dist/ | grep -q "surya-$ver-"`, and the renamed `package-linux.sh`
   writes `surya-$ver-linux-$arch`. That workflow is comet's and never fires
   from a subdirectory, so it is dead here - but anyone who revives it gets a
   release job that fails on a name check. Owner call, not a script rule.

## After the merge

An existing Linux install keeps a stale `~/.local/bin/surya` binary.
`deploy/install-engine.sh` writes the new `~/.local/bin/surya` and rewrites the
unit to point at it, so nothing runs the old one - it is only disk.
The old `~/.surya` data dir is kept too, on purpose: the adoption copies, so a
user who goes back to the previous build still finds their sign-in.
