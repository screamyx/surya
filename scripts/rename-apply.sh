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
      -e 's/\x01THEMEFAM\x01/family("zeron", "Zeron"/g'
}

# Comet's two builtin themes keep their user-visible identity above, but their
# Rust identifiers cannot become `surya_dark`/`surya_light`: surya-theme
# already defines those (builtins.rs), and the rename would redefine them -
# E0428, caught by running this script rather than by reading it. Named for
# what they are, comet's originals.
rename_legacy_theme_fns() {
  sed -e 's/\bzeron_dark\b/comet_dark/g' -e 's/\bzeron_light\b/comet_light/g'
}

# Categories 1, 2, 3, 4, 17, 18, 19: every spelling of the name, and the env
# prefix. One pass, because splitting them leaves the tree uncompilable.
rewrite() {
  rename_legacy_theme_fns | mask | sed -e 's/ZERON_/SURYA_/g' \
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
ALIAS=app/crates/engine/src/env_compat.rs
if [ ! -f "$ALIAS" ]; then
  cat > "$ALIAS" <<'RUST'
//! `SURYA_*` with a one-release `ZERON_*` fallback.
//!
//! Generated by `scripts/rename-apply.sh`. A user's shell profile, systemd
//! `EnvironmentFile` or CI secret still says `ZERON_*`; reading it keeps them
//! working for one release, and the warning tells them what to change. Drop
//! this module and its call sites the release after.

use std::ffi::OsString;
use std::sync::OnceLock;

/// The variables a USER sets by hand, from `daemon.rs CAPTURED_ENV` and
/// `main.rs`. Test-only knobs (`SURYA_MOCK_*`, `SURYA_E2E_*`, `SURYA_DEMO_*`)
/// are deliberately absent: nothing outside this repo sets them, so they
/// rename without an alias.
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

/// `SURYA_<name>`, else `ZERON_<name>` with a warning logged once.
pub fn var_os(name: &str) -> Option<OsString> {
    if let Some(value) = std::env::var_os(format!("SURYA_{name}")) {
        return Some(value);
    }
    let old = std::env::var_os(format!("ZERON_{name}"))?;
    if let Ok(mut seen) = warned().lock()
        && seen.insert(name.to_owned())
    {
        tracing::warn!("renamed env var ZERON_{name}, use SURYA_{name}");
    }
    Some(old)
}

/// [`var_os`] as a `String`; a non-UTF-8 value reads as unset, matching
/// `std::env::var`'s behaviour for the callers that used it.
pub fn var(name: &str) -> Option<String> {
    var_os(name).and_then(|value| value.into_string().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every aliased name is bare: the helper adds the prefix, so a `SURYA_`
    /// left in this list would read `SURYA_SURYA_…`.
    #[test]
    fn the_aliased_names_carry_no_prefix() {
        for name in ALIASED {
            assert!(!name.starts_with("SURYA_"), "{name}");
            assert!(!name.starts_with("ZERON_"), "{name}");
        }
        assert_eq!(ALIASED.len(), 16, "the user-set set from the rename plan");
    }
}
RUST
  echo "wrote $ALIAS"
  if ! grep -q '^pub mod env_compat;' app/crates/engine/src/lib.rs; then
    printf '\npub mod env_compat;\n' >> app/crates/engine/src/lib.rs
    echo "declared env_compat in engine/src/lib.rs"
  fi
else
  echo "$ALIAS already present, left alone"
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

## NOT done by this script - real edits at named call sites

  1. Route every `std::env::var*("ZERON_…")` call through `env_compat::var`.
     The spelling is renamed above, so those calls now read SURYA_* ONLY;
     until they are routed a user's existing ZERON_* is ignored, which is the
     opposite of what the alias is for. Call sites: main.rs, daemon.rs,
     engine lib.rs, ipc.rs, profile.rs, repos.rs, sessions.rs, and ui
     sound/transcript/composer.
  2. Data dir: adopt `~/.zeron` on first start. UNRESOLVED - the coordinator's
     brief says COPY (keeps a rollback), docs/rename-zeron-to-surya.md row 5
     says RENAME (atomic, matches the 0.2.0 `.comet-native` migration). Ask
     before writing it.
  3. systemd: ship `surya.service` and remove the old unit on install, or two
     units race for one IPC port.
  4. URL scheme: register `surya://` AND keep `zeron://` for one release.
  5. Bundle ids (`sh.zeron.app`) wait on the owner picking a domain.
  6. `cargo update -w` regenerates Cargo.lock; not run here so the script
     works without a toolchain.
TODO
