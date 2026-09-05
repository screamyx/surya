# Rename plan: zeron -> surya

The RC ships as surya. The fork still says zeron in crate names, the binary, env vars, the data dir, service names and docs.
This is the inventory and the order of work. Counts come from `scripts/rename-dry-run.sh` (run it again before starting, the tree moves).
Snapshot at `a1c87cb`, 2026-09-05 08:16: `zeron` lowercase files=260 hits=2140, `Zeron` files=81 hits=285, `ZERON_*` files=54 hits=268.

The rename is the last code change before the RC: it touches most files, so it lands after every feature PR and nothing rebases across it.

## Inventory by category

| # | Category | Count | Action | Risk |
| --- | --- | --- | --- | --- |
| 1 | Cargo package and dep names (`zeron-*`, 10 crates + `zeron` bin; `surya-a2ui`, `surya-mcp` already done) | files=14 hits=52 | rename to `surya-*` | Cargo.lock regenerates (38 lines); one commit with #2 |
| 2 | `use zeron_*` paths and `zeron_*::` calls | files=155 hits=994 | mechanical sed, same commit as #1 | the whole workspace stops compiling between #1 and #2, never split |
| 3 | Binary `zeron` (`[[bin]]`, `target/debug/zeron` in scripts, dist, CI) and `zeron-theme-import` | files=8 hits=13; files=1 hits=2 | rename to `surya`, `surya-theme-import` | `scripts/*.sh`, `dist/zeron.desktop`, `.github/workflows/ci.yml` name the path |
| 4 | Env vars `ZERON_*` (66 distinct) | files=54 hits=268 | read `SURYA_*` first, fall back to `ZERON_*` for one release, warn once | a user's shell or systemd env file keeps working |
| 5 | Data dir `~/.zeron`, `~/.zeron-dev` | files=24 hits=54 | new default `~/.surya`; adopt `~/.zeron` by rename on first start (same pattern as the 0.2.0 `.comet-native` migration in `main.rs`) | a live `zeron headless` still writing to `~/.zeron` while the new binary renames it; stop the service first |
| 6 | Files inside the data dir: `ui-settings.json`, `ipc-token`, `session.json`, `local-profile.json`, `composer-defaults.json`, `logs/` | files=17 hits=37 | leave; names carry no brand | none; `data-engine` and `mail.sqlite3` do not exist in this tree today |
| 7 | systemd unit `zeron.service`, `EnvironmentFile=%h/.zeron/env`, install path `%h/.zeron/app/current/zeron` | files=3 hits=3 (+ #5) | new unit `surya.service`; uninstall the old unit on `surya daemon install` | two units racing for one IPC port if the old one is left enabled |
| 8 | launchd label and bundle id `sh.zeron.app`, notify bundle id, `sh.zeron.app.conversation` URL type | files=7 hits=11 | new ids under the surya domain once the owner picks one | macOS notification permission is per bundle id, users re-approve |
| 9 | URL scheme `zeron://`, `register_url_scheme("zeron")`, `app_id: "zeron"` | files=4 hits=8 | register `surya://` and keep `zeron://` for one release | deep links in old chats |
| 10 | Domain `zeron.sh`, `edge.zeron.sh`, `install.sh` URLs | files=8 hits=18 | leave until surya has a domain; edge is out of scope | none for the RC (local-first) |
| 11 | Packaging: `dist/zeron.desktop`, `zeron.png`, `zeron.svg`, `macos/Info.plist`, `package-linux.sh`, `package-macos.sh` (`zeron-notarize.zip`) | files=8 hits=83 | rename files and strings; zip and tarball become `surya-<ver>-<os>-<arch>` | none in CI (packaging is not run there) |
| 12 | Windows launcher and zip | none in the tree today | name it `surya-windows-x64.zip` when the cef seat adds it | avoid a second rename |
| 13 | iOS app (`apps/ios/Zeron*`) | files=29 hits=139 | leave, out of scope | none |
| 14 | Cloudflare edge (`edge/`) | files=12 hits=68 | leave, out of scope; env names inside the Worker stay `ZERON_*` | none |
| 15 | landing + www-redirect | files=4 hits=43 | leave | none |
| 16 | Docs: `app/ARCHITECTURE.md`, `CONTEXT.md`, `README*.md`, `THIRD_PARTY_NOTICES.md`, `app/docs/` incl `PARITY.md`, repo `docs/` | files=25 hits=184 | separate docs commit; keep "forked from zeronsh/comet v0.2.34" lines | none |
| 17 | Test names (`fn *zeron*`, 16) and fixtures; `zeronsh/comet` upstream links (17) | files=9 hits=16; files=11 hits=17 | rename test fns with #2; keep upstream links, they are provenance | none |
| 18 | GitHub: repo already `screamyx/surya`; `ci.yml` lists `-p zeron-*` | files=1 hits=5 | same commit as #1 | red CI on the rename PR if missed |
| 19 | Theme ids `zeron-dark`, `zeron-light`, loader names `zeron_loader`, `zeron-pulse` motion | in #2 totals | rename the Rust identifiers; keep the theme **ids** as aliases, they live in users' `ui-settings.json` | a saved theme selection falls back to default if the id changes |

## Env var alias

One helper, one place: `fn env(name) -> Option<String>` that reads `SURYA_<name>`, then `ZERON_<name>`, and logs `renamed env var ZERON_<name>, use SURYA_<name>` once per process.
Every `std::env::var("ZERON_…")` call (main.rs, daemon.rs, engine lib.rs, ipc.rs, profile.rs, repos.rs, sessions.rs, ui sound/transcript/composer) goes through it.
`daemon.rs CAPTURED_ENV` writes both names into the unit for one release.
Test-only knobs (`ZERON_MOCK_*`, `ZERON_E2E_*`, `ZERON_DEMO_*`) rename without an alias.

## Order of commits

1. **Code, one commit.** Cargo names (#1), `use` paths (#2), binary and bin names (#3), CI flags (#18), test fn names (#17), Rust identifiers (#19). Tool: `rg -l` the patterns, `sed` the four spellings (`zeron-`, `zeron_`, `zeron`, `Zeron`) inside `app/crates`, `app/apps/zeron`, `app/Cargo.toml`, `.github`; then `cargo update -w` for the lock. Never touch `apps/ios`, `edge`, `apps/landing`, `apps/www-redirect`.
2. **Compat, one commit.** Env alias helper (#4), data dir adopt-by-rename (#5), `surya.service` + old unit removal (#7), `surya://` + `zeron://` (#9), theme id aliases (#19).
3. **Packaging, one commit.** dist files, package scripts, bundle ids (#8, #11).
4. **Docs, one commit.** (#16) plus this file's status line.

## Proof after the rename

- `cargo check --workspace --all-targets` clean; `cargo test -p surya-proto -p surya-doc -p surya-rpc -p surya-engine -p surya-harness -p surya-mcp` (the CI "headless tests" set) green.
- `app/scripts/e2e-smoke.sh` passes with the new binary name.
- Alias proof: start `zeron headless` on a temp `ZERON_DATA_DIR`, create a space, stop it; start `surya headless` with the same `ZERON_DATA_DIR` and no `SURYA_*` set; `WatchSpaces` returns the space (`asked=1 seen=1`) and the log shows the rename warning once (`warned=1`).
- Data dir proof: with `~/.zeron` present and `~/.surya` absent, a fresh `surya` start renames the dir and reads `ui-settings.json` from it (`migrated=1`).
- `scripts/rename-dry-run.sh` after: every in-scope row reads `files=0 hits=0` except #10 (domain), #13, #14, #15, and the provenance lines in #16/#17.

## Not decided

- The surya bundle id and URL scheme domain (owner call; `sh.zeron.app` stays until then).
- Whether `~/.zeron` is renamed or copied. Rename is atomic and cheap; copy keeps a rollback. This plan says rename, matching the existing 0.2.0 migration.
