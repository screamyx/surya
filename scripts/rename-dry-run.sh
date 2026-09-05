#!/usr/bin/env bash
# zeron -> surya rename: inventory from the live tree. Prints counts only,
# never edits. Pairs are files=N hits=M (files that match, total matches).
#   scripts/rename-dry-run.sh            # from the repo root
set -euo pipefail
cd "$(dirname "$0")/.."
RG=(rg --no-messages -g '!target' -g '!node_modules' -g '!.git' -g '!Cargo.lock')

pair() { # pair <label> <pattern> [paths...]
  local label=$1 pattern=$2; shift 2
  local files hits
  files=$({ "${RG[@]}" -l "$pattern" "$@" 2>/dev/null || true; } | wc -l | tr -d ' ')
  hits=$({ "${RG[@]}" -o "$pattern" "$@" 2>/dev/null || true; } | wc -l | tr -d ' ')
  printf '%-44s files=%-4s hits=%s\n' "$label" "$files" "$hits"
}

echo "# rename dry run, $(date +%F' '%T), $(git rev-parse --short HEAD)"
echo
echo "## totals (app/ tree, any case)"
pair "zeron (lowercase)"        'zeron'  app
pair "Zeron (capitalised)"      'Zeron'  app
pair "ZERON_* (env vars)"       'ZERON_[A-Z0-9_]+' app
echo "Cargo.lock lines naming a zeron crate: $(grep -c 'zeron' app/Cargo.lock)"
echo
echo "## crates and Cargo manifests (rename together, one commit)"
pair "Cargo.toml package/dep names"  'zeron' app/Cargo.toml app/crates/*/Cargo.toml app/apps/*/Cargo.toml
pair "use zeron_* / zeron_*:: paths" 'zeron_(proto|doc|sync|harness|engine|rpc|update|ui|syntax|theme)\b' app/crates app/apps
pair "CI -p zeron-* flags"           'zeron-(proto|doc|rpc|engine|harness|sync|ui|update|syntax|theme)' .github
echo "package names:"; grep -h '^name = ' app/crates/*/Cargo.toml app/apps/*/Cargo.toml | sort | tr '\n' ' '; echo
echo
echo "## binaries"
pair "[[bin]] name / target/debug/zeron"  '(name = "zeron"|debug/zeron\b|release/zeron\b)' app/apps app/scripts app/dist .github
pair "zeron-theme-import bin"             'zeron-theme-import' app
pair "surya-mcp (already fine)"           'surya-mcp' app
echo
echo "## env vars (alias ZERON_* -> SURYA_* for one release)"
"${RG[@]}" -o 'ZERON_[A-Z0-9_]+' app | sed 's/.*://' | sort | uniq -c | sort -rn | awk '{printf "  %-36s hits=%s\n", $2, $1}'
echo "user-set (documented in daemon.rs CAPTURED_ENV + main.rs): DATA_DIR EDGE_URL EDGE_TOKEN ORG_ID WORKOS_CLIENT_ID WORKOS_API_BASE IPC_PORT BIND IPC_TOKEN CALLBACK_PORT HARNESS DEVICE_NAME ENGINE ENGINE_TOKEN USER_ID WORKTREES_DIR"
echo
echo "## data dir, files, service names"
pair "~/.zeron data dir"              '\.zeron\b' app/apps app/crates app/scripts app/dist
pair "ui-settings.json"               'ui-settings\.json' app/crates
pair "ipc-token / session.json / local-profile.json" '(ipc-token|session\.json|local-profile\.json)' app/crates app/apps
pair "zeron.service (systemd)"        'zeron\.service' app
pair "sh.zeron.app (launchd, bundle)" 'sh\.zeron' app
pair "url scheme zeron:// + app_id"   '(register_url_scheme\("zeron"\)|app_id: Some\("zeron|zeron://)' app/crates app/apps
pair "zeron.sh domain / edge URL"     'zeron\.sh' app/apps app/crates app/scripts app/dist
echo
echo "## packaging"
pair "dist/ (desktop, plist, icons, README)"  'zeron' app/dist
pair "scripts/package-*.sh, dev-demo, e2e"    'zeron' app/scripts
echo "files named *zeron*:"; find app -iname '*zeron*' -not -path '*/target/*' | sed 's/^/  /'
echo
echo "## OUT of scope (leave untouched)"
pair "apps/ios"          '[Zz]eron' app/apps/ios
pair "edge/ (Cloudflare)" '[Zz]eron' app/edge
pair "apps/landing + www-redirect" '[Zz]eron' app/apps/landing app/apps/www-redirect
echo
echo "## docs"
pair "app/*.md (ARCHITECTURE, CONTEXT, README)" '[Zz]eron' app/ARCHITECTURE.md app/CONTEXT.md app/README.md app/README.zh-CN.md app/THIRD_PARTY_NOTICES.md
pair "app/docs/ (PARITY.md etc)"               '[Zz]eron' app/docs
pair "repo docs/"                              '[Zz]eron' docs
echo
echo "## tests and fixtures"
pair "test fn names with zeron"     'fn [a-z_]*zeron[a-z_]*\(' app/crates app/apps
pair "zeronsh/comet upstream links" 'zeronsh' app
