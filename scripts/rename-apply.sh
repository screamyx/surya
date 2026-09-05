#!/usr/bin/env bash
# zeron -> surya rename, applied. The executable half of
# docs/rename-zeron-to-surya.md; scripts/rename-dry-run.sh counts, this edits.
#
#   scripts/rename-apply.sh            # apply, then print the counters
#   scripts/rename-apply.sh --check    # list the files it would touch
#
# Re-runnable: every transform matches only the OLD spelling and every
# generated file is written once, so a second run is a no-op.
#
# It does the mechanical categories. It does NOT write the behavioural compat
# code (data dir adoption, systemd unit, URL scheme); those are real edits at
# named call sites and are listed at the end as work for a human. The script
# says so rather than pretending.
set -euo pipefail
cd "$(dirname "$0")/.."

CHECK=0
[ "${1:-}" = "--check" ] && CHECK=1

RG=(rg --no-messages -g '!target' -g '!node_modules' -g '!.git' -g '!Cargo.lock')

# Everything in scope. apps/ios, edge/, apps/landing and apps/www-redirect are
# out of scope per the plan (rows 13, 14, 15) and never appear here.
in_scope_files() {
  "${RG[@]}" -l -e '[Zz]eron' \
    app/crates app/apps/zeron app/apps/surya app/Cargo.toml app/scripts \
    app/dist app/ARCHITECTURE.md app/CONTEXT.md app/README.md \
    app/README.zh-CN.md app/THIRD_PARTY_NOTICES.md app/docs \
    .github docs 2>/dev/null | sort -u
}

# --------------------------------------------------------------------------
# What must SURVIVE the rename. Masked before the substitution, restored
# after, because a plain s/zeron/surya/ turns `zeronsh/comet` into
# `suryash/comet` and takes the upstream provenance with it.
#
#   zeronsh        the fork's provenance (rows 16, 17)
#   zeron.sh       the domain, until surya has one (row 10)
#   zeron-dark     comet's two builtin themes: their ids, family and display
#   zeron-light    names are user state, saved in ui-settings.json (row 19)
# --------------------------------------------------------------------------
mask() {
  sed -e 's/zeronsh/\x01PROVENANCE\x01/g' \
      -e 's/zeron\.sh/\x01DOMAIN\x01/g' \
      -e 's/"zeron-dark"/\x01THEMEDARK\x01/g' \
      -e 's/"zeron-light"/\x01THEMELIGHT\x01/g' \
      -e 's/"Zeron Dark"/\x01THEMENAMED\x01/g' \
      -e 's/"Zeron Light"/\x01THEMENAMEL\x01/g' \
      -e 's/family_id: "zeron"/\x01THEMEFAMID\x01/g' \
      -e 's/family("zeron", "Zeron"/\x01THEMEFAM\x01/g'
}
unmask() {
  sed -e 's/\x01PROVENANCE\x01/zeronsh/g' \
      -e 's/\x01DOMAIN\x01/zeron.sh/g' \
      -e 's/\x01THEMEDARK\x01/"zeron-dark"/g' \
      -e 's/\x01THEMELIGHT\x01/"zeron-light"/g' \
      -e 's/\x01THEMENAMED\x01/"Zeron Dark"/g' \
      -e 's/\x01THEMENAMEL\x01/"Zeron Light"/g' \
      -e 's/\x01THEMEFAMID\x01/family_id: "zeron"/g' \
      -e 's/\x01THEMEFAM\x01/family("zeron", "Zeron"/g' \
      -e 's/\x01DATADIR\x01/.surya/g' \
      -e 's/\x01DATADIRPREV\x01/.zeron/g'
}

# Comet's two builtin themes keep their user-visible identity above, but their
# Rust identifiers cannot become `surya_dark`/`surya_light`: surya-theme
# already defines those (builtins.rs), and the rename would redefine them -
# E0428, caught by running this script rather than by reading it. Named for
# what they are, comet's originals.
# The data dir migration moves along by one: the call that adopts
# `.comet-native` into `.zeron` becomes the one that adopts `.zeron` into
# `.surya`. The global substitution alone gets this WRONG - it rewrites the
# first argument and leaves the second, producing `(".surya", ".comet-native")`
# and silently dropping the whole zeron -> surya adoption, which is the one
# this release needs. Masked here so both names land right.
advance_data_dir_migration() {
  sed -e 's/adopt_and_report(&home, "\.zeron", "\.comet-native")/adopt_and_report(\&home, "\x01DATADIR\x01", "\x01DATADIRPREV\x01")/g'
}

rename_legacy_theme_fns() {
  sed -e 's/\bzeron_dark\b/comet_dark/g' -e 's/\bzeron_light\b/comet_light/g'
}

# Categories 1, 2, 3, 4, 17, 18, 19: every spelling of the name, and the env
# prefix. One pass, because splitting them leaves the tree uncompilable.
rewrite() {
  advance_data_dir_migration | rename_legacy_theme_fns | mask | sed -e 's/ZERON_/SURYA_/g' \
                                       -e 's/zeron_/surya_/g' \
                                       -e 's/zeron-/surya-/g' \
                                       -e 's/Zeron/Surya/g' \
                                       -e 's/zeron/surya/g' | unmask
}

count() { # count <label> <pattern>
  local files hits
  files=$({ "${RG[@]}" -l "$2" app docs .github 2>/dev/null || true; } | wc -l | tr -d ' ')
  hits=$({ "${RG[@]}" -o "$2" app docs .github 2>/dev/null || true; } | wc -l | tr -d ' ')
  printf '%-34s files=%-5s hits=%s\n' "$1" "$files" "$hits"
}
hits_of() { { "${RG[@]}" -o "$1" app docs .github 2>/dev/null || true; } | wc -l | tr -d ' '; }

echo "# rename-apply, $(date +%F' '%T), $(git rev-parse --short HEAD)"
echo
echo "## before"
count "zeron (any case)" '[Zz]eron'
count "  of which zeronsh (keep)" 'zeronsh'
count "  of which zeron.sh (keep)" 'zeron\.sh'
count "ZERON_* env vars" 'ZERON_[A-Z0-9_]+'
BEFORE=$(hits_of '[Zz]eron')

mapfile -t FILES < <(in_scope_files)
echo
echo "## files in scope: ${#FILES[@]}"

if [ "$CHECK" = 1 ]; then
  printf '%s\n' "${FILES[@]}" | sed 's/^/  /'
  echo
  echo "--check: nothing written."
  exit 0
fi

# --------------------------------------------------------------------------
# 1. Contents.
# --------------------------------------------------------------------------
CHANGED=0
for file in "${FILES[@]}"; do
  [ -f "$file" ] || continue
  before_sum=$(cksum < "$file")
  rewrite < "$file" > "$file.rename-tmp"
  if [ "$(cksum < "$file.rename-tmp")" != "$before_sum" ]; then
    # Written back through the original inode so the mode is kept: a script
    # that loses its executable bit breaks CI in a way nobody reads as this.
    cat "$file.rename-tmp" > "$file"
    CHANGED=$((CHANGED + 1))
  fi
  rm -f "$file.rename-tmp"
done
echo "files_changed=$CHANGED"

# --------------------------------------------------------------------------
# 2. Paths. `-depth` so a directory moves after the files inside it.
# --------------------------------------------------------------------------
MOVED=0
move_one() {
  local from=$1 to=$2
  [ -e "$from" ] || return 0
  [ -e "$to" ] && return 0
  git mv "$from" "$to" 2>/dev/null || mv "$from" "$to"
  MOVED=$((MOVED + 1))
  echo "  moved $from -> $to"
}
move_one app/apps/zeron app/apps/surya
while IFS= read -r path; do
  move_one "$path" "$(dirname "$path")/$(basename "$path" | sed 's/zeron/surya/; s/Zeron/Surya/')"
done < <(find app -depth -iname '*zeron*' \
  -not -path '*/target/*' -not -path '*/.git/*' \
  -not -path 'app/apps/ios/*' -not -path 'app/edge/*' \
  -not -path 'app/apps/landing/*' -not -path 'app/apps/www-redirect/*' 2>/dev/null)
echo "paths_moved=$MOVED"

# --------------------------------------------------------------------------
# 3. The env alias, as real code. `SURYA_*` first, `ZERON_*` for one release
# with a warning logged once. Its own module, so the script never has to patch
# around an existing function body.
# --------------------------------------------------------------------------
ALIAS=app/crates/proto/src/env_compat.rs
if [ ! -f "$ALIAS" ]; then
  cat > "$ALIAS" <<'RUST'
//! `SURYA_*` with a one-release `ZERON_*` fallback.
//!
//! Generated by `scripts/rename-apply.sh`. A user's shell profile, systemd
//! `EnvironmentFile` or CI secret still says `ZERON_*`; reading it keeps them
//! working for one release, and the warning tells them what to change. Drop
//! this module and its call sites the release after.
//!
//! It lives in proto because that is the crate everything else already
//! depends on, and the signatures mirror `std::env` exactly - `var` returns
//! the same `Result`, `var_os` the same `Option` - so a call site keeps its
//! `.ok()`, `.is_err()` or `.unwrap_or_else(|_| …)` unchanged.

use std::env::VarError;
use std::ffi::OsString;
use std::sync::OnceLock;

/// The variables a USER sets by hand, from `daemon.rs CAPTURED_ENV` and
/// `main.rs`. Dev and test knobs (`SURYA_MOCK_*`, `SURYA_DEMO_*`,
/// `SURYA_ACP_*`, and the rest) are deliberately absent: nothing outside this
/// repo sets them, so they rename without an alias.
pub const ALIASED: &[&str] = &[
    "DATA_DIR",
    "EDGE_URL",
    "EDGE_TOKEN",
    "ORG_ID",
    "WORKOS_CLIENT_ID",
    "WORKOS_API_BASE",
    "IPC_PORT",
    "BIND",
    "IPC_TOKEN",
    "CALLBACK_PORT",
    "HARNESS",
    "DEVICE_NAME",
    "ENGINE",
    "ENGINE_TOKEN",
    "USER_ID",
    "WORKTREES_DIR",
];

fn warned() -> &'static std::sync::Mutex<std::collections::HashSet<String>> {
    static WARNED: OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> = OnceLock::new();
    WARNED.get_or_init(Default::default)
}

fn warn_once(name: &str) {
    if let Ok(mut seen) = warned().lock()
        && seen.insert(name.to_owned())
    {
        tracing::warn!("renamed env var ZERON_{name}, use SURYA_{name}");
    }
}

/// `SURYA_<name>`, else `ZERON_<name>` with a warning logged once per name.
/// Same `Option` as [`std::env::var_os`].
pub fn var_os(name: &str) -> Option<OsString> {
    if let Some(value) = std::env::var_os(format!("SURYA_{name}")) {
        return Some(value);
    }
    let old = std::env::var_os(format!("ZERON_{name}"))?;
    warn_once(name);
    Some(old)
}

/// `SURYA_<name>`, else `ZERON_<name>` with a warning logged once per name.
/// Same `Result` as [`std::env::var`], so `.ok()` and `.is_err()` still work.
pub fn var(name: &str) -> Result<String, VarError> {
    match std::env::var(format!("SURYA_{name}")) {
        Ok(value) => Ok(value),
        Err(VarError::NotPresent) => {
            let old = std::env::var(format!("ZERON_{name}"));
            if old.is_ok() {
                warn_once(name);
            }
            old
        }
        // A SURYA_* that is set but not UTF-8 is the user's own mistake and is
        // reported as such, not silently replaced by the old name.
        Err(other) => Err(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The helper adds the prefix, so a `SURYA_` left in this list would read
    /// `SURYA_SURYA_…`.
    #[test]
    fn the_aliased_names_carry_no_prefix() {
        for name in ALIASED {
            assert!(!name.starts_with("SURYA_"), "{name}");
            assert!(!name.starts_with("ZERON_"), "{name}");
        }
        assert_eq!(ALIASED.len(), 16, "the user-set set from the rename plan");
    }

    /// New name wins, old name still works, neither set reads as unset.
    /// One process, one variable, so the warning set is not a shared fixture.
    #[test]
    fn the_new_name_wins_and_the_old_one_still_works() {
        let name = "RENAME_PROBE";
        // SAFETY: single-threaded test, and this variable is used nowhere else.
        unsafe {
            std::env::remove_var("SURYA_RENAME_PROBE");
            std::env::remove_var("ZERON_RENAME_PROBE");
        }
        assert!(var(name).is_err(), "unset");
        assert_eq!(var_os(name), None);

        unsafe { std::env::set_var("ZERON_RENAME_PROBE", "old") };
        assert_eq!(var(name).as_deref(), Ok("old"), "the old name is honoured");

        unsafe { std::env::set_var("SURYA_RENAME_PROBE", "new") };
        assert_eq!(var(name).as_deref(), Ok("new"), "the new name wins");

        unsafe {
            std::env::remove_var("SURYA_RENAME_PROBE");
            std::env::remove_var("ZERON_RENAME_PROBE");
        }
    }
}
RUST
  echo "wrote $ALIAS"
fi
grep -q '^pub mod env_compat;' app/crates/proto/src/lib.rs \
  || printf '\npub mod env_compat;\n' >> app/crates/proto/src/lib.rs
grep -q '^tracing' app/crates/proto/Cargo.toml \
  || sed -i 's/^chrono.workspace = true$/chrono.workspace = true\ntracing.workspace = true/' app/crates/proto/Cargo.toml
grep -q 'surya-proto' app/apps/surya/Cargo.toml \
  || sed -i 's/^surya-engine.workspace = true$/surya-proto.workspace = true\nsurya-engine.workspace = true/' app/apps/surya/Cargo.toml

# --------------------------------------------------------------------------
# 3b. Route the user-set reads through it. Only the 16 names, only outside
# tests, and never crates/mcp/src/tasks.rs - that file belongs to the tasks
# seat. The dev and test knobs keep reading std::env directly: nothing outside
# this repo sets them, so they need no alias.
# --------------------------------------------------------------------------
ROUTED=0
USER_SET='DATA_DIR|EDGE_URL|EDGE_TOKEN|ORG_ID|WORKOS_CLIENT_ID|WORKOS_API_BASE|IPC_PORT|BIND|IPC_TOKEN|CALLBACK_PORT|HARNESS|DEVICE_NAME|ENGINE|ENGINE_TOKEN|USER_ID|WORKTREES_DIR'
while IFS= read -r file; do
  case "$file" in */tests/*|*/crates/mcp/*) continue;; esac
  before_sum=$(cksum < "$file")
  perl -pi -e "s/(?:std::)?env::var_os\(\"SURYA_($USER_SET)\"\)/surya_proto::env_compat::var_os(\"\1\")/g;
               s/(?:std::)?env::var\(\"SURYA_($USER_SET)\"\)/surya_proto::env_compat::var(\"\1\")/g" "$file"
  [ "$(cksum < "$file")" != "$before_sum" ] && ROUTED=$((ROUTED + 1))
done < <("${RG[@]}" -l "env::var(_os)?\(\"SURYA_($USER_SET)\"\)" app/crates app/apps/surya 2>/dev/null || true)
echo "files_routed=$ROUTED"
UNROUTED=$({ "${RG[@]}" -o "env::var(_os)?\(\"SURYA_($USER_SET)\"\)" app/crates app/apps/surya 2>/dev/null || true; } | wc -l | tr -d ' ')
echo "user_set_reads_still_direct=$UNROUTED"

# --------------------------------------------------------------------------
# 3c. Compat that is behaviour, not spelling. Each block is guarded by a
# marker so a second run leaves it alone.
# --------------------------------------------------------------------------
COMPAT=0

# The old scheme keeps working for one release: deep links live in chats
# people already have.
LINKS=app/crates/ui/src/links.rs
if [ -f "$LINKS" ] && ! grep -q 'LEGACY_SCHEME' "$LINKS"; then
  perl -0pi -e 's{(pub fn parse_surya_conversation_link\(url: &str\) -> Result<ConversationDeepLink, &.static str> \{
)    let rest = url
        \.strip_prefix\("surya://open/chat/"\)
        \.ok_or\("not a Surya conversation link"\)\?;}{$1    // One release of both: a link minted before the rename is in chats
    // people already have.
    const LEGACY_SCHEME: &str = "zeron://open/chat/";
    let rest = url
        .strip_prefix("surya://open/chat/")
        .or_else(|| url.strip_prefix(LEGACY_SCHEME))
        .ok_or("not a Surya conversation link")?;}s' "$LINKS"
  grep -q 'LEGACY_SCHEME' "$LINKS" && { COMPAT=$((COMPAT + 1)); echo "  links.rs accepts zeron:// too"; }
fi

UI=app/crates/ui/src/lib.rs
if [ -f "$UI" ] && ! grep -q 'register_url_scheme("zeron")' "$UI"; then
  perl -pi -e 's{^(\s*)cx\.register_url_scheme\("surya"\)\.detach\(\);}{$1cx.register_url_scheme("surya").detach();\n$1// One release: the OS still routes surya:// links minted before the rename.\n$1cx.register_url_scheme("zeron").detach();}' "$UI"
  grep -q 'register_url_scheme("zeron")' "$UI" && { COMPAT=$((COMPAT + 1)); echo "  lib.rs registers zeron:// too"; }
fi

# Two units enabled means two daemons racing for one IPC port.
DAEMON=app/apps/surya/src/daemon.rs
if [ -f "$DAEMON" ] && ! grep -q 'retire_legacy_unit' "$DAEMON"; then
  perl -pi -e 's{^(\s*)run\("systemctl", &\["--user", "enable", "--now", SYSTEMD_UNIT\]\)\?;}{$1retire_legacy_unit();\n$1run("systemctl", &["--user", "enable", "--now", SYSTEMD_UNIT])?;}' "$DAEMON"
  cat >> "$DAEMON" <<'RUST'

/// The pre-rename unit, disabled and removed on install.
///
/// Leaving it enabled means two daemons start on boot and race for one IPC
/// port. Best-effort on purpose: a machine that never had the old unit must
/// still install cleanly, so every step here ignores its own failure.
fn retire_legacy_unit() {
    const LEGACY_UNIT: &str = "zeron.service";
    let _ = run_quiet("systemctl", &["--user", "disable", "--now", LEGACY_UNIT]);
    // Same resolution as `systemd_unit_path`, with the old unit name.
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().ok().map(|home| home.join(".config")));
    if let Some(config) = config {
        let legacy = config.join("systemd/user").join(LEGACY_UNIT);
        if legacy.exists() {
            let _ = std::fs::remove_file(&legacy);
            println!("Removed the pre-rename unit ({}).", legacy.display());
        }
    }
    let _ = run_quiet("systemctl", &["--user", "daemon-reload"]);
}
RUST
  grep -q 'retire_legacy_unit' "$DAEMON" && { COMPAT=$((COMPAT + 1)); echo "  daemon.rs retires zeron.service"; }
fi
echo "compat_blocks_added=$COMPAT"

# --------------------------------------------------------------------------
# 3d. The lock. Package names changed, so it is stale by definition.
# --------------------------------------------------------------------------
if command -v cargo >/dev/null 2>&1; then
  (cd app && cargo update -w >/dev/null 2>&1) && echo "cargo_update=ok" || echo "cargo_update=failed"
else
  echo "cargo_update=skipped (no cargo on PATH)"
fi

# --------------------------------------------------------------------------
# 4. Counters.
# --------------------------------------------------------------------------
echo
echo "## after"
count "zeron (any case)" '[Zz]eron'
count "  of which zeronsh (kept)" 'zeronsh'
count "  of which zeron.sh (kept)" 'zeron\.sh'
count "ZERON_* env vars" 'ZERON_[A-Z0-9_]+'
echo
echo "zeron_hits_before=$BEFORE after=$(hits_of '[Zz]eron') files_changed=$CHANGED paths_moved=$MOVED"

cat <<'TODO'

## NOT done by this script - decisions or owner input

  1. Data dir: copy `~/.zeron` to `~/.surya` on first start (ruled: COPY, not
     rename - it keeps a rollback). Needs a real migration at the point the
     data dir is resolved, with its own test; not a substitution.
  2. Bundle ids (`sh.zeron.app`, the notify id, the conversation URL type)
     wait on the owner picking a domain.
  3. `crates/mcp/src/tasks.rs` reads SURYA_IPC_PORT directly and is NOT routed
     here: that file belongs to the tasks seat.
  4. Dev and test knobs (`SURYA_MOCK_*`, `SURYA_DEMO_*`, `SURYA_ACP_*`, and
     the rest) rename without an alias on purpose - nothing outside this repo
     sets them.
TODO
