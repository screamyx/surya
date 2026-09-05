#!/usr/bin/env bash
# Linux packaging: build the release binary and produce
#   target/package/zeron-<version>-linux-<arch>.tar.gz
# containing the binary, the .desktop entry, and the icon, plus an install.sh
# that drops them into ~/.local (XDG) paths.
#
# Usage: scripts/package-linux.sh [--browser]
#   --browser  build with the CEF browser pane (cargo feature `browser`) and
#              ship Chromium's runtime files and zeron-browser-helper beside
#              the binary. Mirrors deploy/windows/build.ps1 -Browser: every
#              name below is required, and a missing one stops the build
#              rather than producing a tarball whose Chromium dies at
#              start-up. Needs CEF_PATH set to the directory the cef crate
#              downloads CEF into (app/crates/browser/README.md).
# Env:   PROFILE=debug for a fast unoptimized package (CI smoke); default release.
#        CARGO=<path>  the cargo to call (this box throttles through
#                      /store/surya-cargo).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="$(cd "$ROOT/.." && pwd)"
command -v cargo >/dev/null 2>&1 || PATH="$HOME/.cargo/bin:$PATH"
CARGO="${CARGO:-cargo}"
PROFILE="${PROFILE:-release}"
ARCH="$(uname -m)"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
OUT_DIR="$ROOT/target/package"
STAGE="$OUT_DIR/zeron-$VERSION-linux-$ARCH"
TARBALL="$STAGE.tar.gz"

BROWSER=0
for arg in "$@"; do
  case "$arg" in
    --browser) BROWSER=1 ;;
    *) echo "unknown argument: $arg (usage: package-linux.sh [--browser])" >&2; exit 2 ;;
  esac
done

if [[ "$BROWSER" == 1 && -z "${CEF_PATH:-}" ]]; then
  echo "--browser needs CEF_PATH (a directory the cef crate downloads CEF into, once);" >&2
  echo "unset, it would re-download 1.4 GB into the build dir" >&2
  exit 1
fi

cd "$ROOT"
FEATURES=()
[[ "$BROWSER" == 1 ]] && FEATURES=(--features browser)
if [[ "$PROFILE" == "release" ]]; then
  "$CARGO" build --release -p zeron "${FEATURES[@]}"
  BUILT="$ROOT/target/release"
else
  "$CARGO" build -p zeron "${FEATURES[@]}"
  BUILT="$ROOT/target/debug"
fi
# CARGO_TARGET_DIR moves the whole thing; the CEF runtime is copied there by
# the cef crate's build script, so both must be read from the same place.
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  BUILT="$CARGO_TARGET_DIR/$([[ "$PROFILE" == "release" ]] && echo release || echo debug)"
fi
BIN="$BUILT/zeron"
[[ -x "$BIN" ]] || { echo "no binary at $BIN" >&2; exit 1; }

rm -rf "$STAGE" "$TARBALL"
mkdir -p "$STAGE"
install -m 755 "$BIN" "$STAGE/zeron"
install -m 644 "$ROOT/dist/zeron.desktop" "$STAGE/zeron.desktop"
install -m 644 "$ROOT/dist/zeron.png" "$STAGE/zeron.png"
mkdir -p "$STAGE/licenses/fonts"
cp "$ROOT/crates/ui/assets/fonts/licenses/"* "$STAGE/licenses/fonts/"

CEF_VERSION=""
if [[ "$BROWSER" == 1 ]]; then
  # The cef crate's build script copied Chromium's runtime files next to the
  # binary; CEF loads them from the binary's own folder (the zeron binary
  # carries an $ORIGIN rpath under this feature, apps/zeron/build.rs), so the
  # tarball carries the same set. Every name is required: a missing one is a
  # Chromium that fails at start-up, not a smaller tarball. CREDITS.html is
  # Chromium's third-party notices; archive.json names the exact CEF and
  # Chromium build that was downloaded.
  CEF_REQUIRED=(
    zeron-browser-helper
    libcef.so
    libEGL.so libGLESv2.so
    libvk_swiftshader.so vk_swiftshader_icd.json libvulkan.so.1
    chrome-sandbox
    chrome_100_percent.pak chrome_200_percent.pak resources.pak
    icudtl.dat v8_context_snapshot.bin
    CREDITS.html archive.json
  )
  MISSING=()
  for name in "${CEF_REQUIRED[@]}"; do
    [[ -e "$BUILT/$name" ]] || MISSING+=("$name")
  done
  if (( ${#MISSING[@]} > 0 )); then
    echo "browser runtime incomplete in $BUILT: missing ${MISSING[*]}" >&2
    echo "(build with --browser and CEF_PATH set, not from a plain build)" >&2
    exit 1
  fi
  LOCALE_COUNT=$(find "$BUILT/locales" -maxdepth 1 -name '*.pak' -type f 2>/dev/null | wc -l)
  if (( LOCALE_COUNT < 1 )); then
    echo "browser runtime incomplete: no *.pak in $BUILT/locales" >&2
    exit 1
  fi
  for name in "${CEF_REQUIRED[@]}"; do
    cp -a "$BUILT/$name" "$STAGE/$name"
  done
  # chrome-sandbox is Chromium's SUID helper. It ships mode 0755, and
  # install.sh makes it root-owned 4755. That step is what turns the page
  # sandbox on: without it the app runs each page unsandboxed and says so at
  # start-up (app/crates/browser/src/sandbox.rs).
  chmod 755 "$STAGE/chrome-sandbox"
  mkdir -p "$STAGE/locales"
  cp -a "$BUILT/locales/." "$STAGE/locales/"
  cp "$REPO/deploy/CEF-LICENSE.txt" "$STAGE/CEF-LICENSE.txt"
  CEF_VERSION=$(sed -n 's/.*cef_binary_\([^+]*\)+g[0-9a-f]*+chromium-\([0-9.]*\).*/CEF \1, Chromium \2/p' "$BUILT/archive.json" | head -1)
  if [[ -z "$CEF_VERSION" ]]; then
    echo "archive.json beside the binary does not name a cef_binary_<cef>+g<hash>+chromium-<version> archive" >&2
    exit 1
  fi
  echo "== browser: $CEF_VERSION; shipped ${#CEF_REQUIRED[@]} required files, $LOCALE_COUNT locales, CEF-LICENSE.txt"
fi

{
  echo "surya linux app"
  echo "version: $VERSION"
  echo "arch: $ARCH"
  echo "built: $(date -Iseconds) on $(hostname)"
  echo "run: ./zeron (or ./install.sh, then zeron)"
  if [[ "$BROWSER" == 1 ]]; then
    echo "browser pane: yes, $CEF_VERSION, BSD-3-Clause (CEF-LICENSE.txt, CREDITS.html)"
    echo "page sandbox: needs the root-owned chrome-sandbox that install.sh sets up"
  else
    echo "browser pane: no"
  fi
} > "$STAGE/VERSION.txt"

cat >"$STAGE/install.sh" <<'INSTALL'
#!/usr/bin/env bash
# Install Zeron into ~/.local (no root needed).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
install -Dm755 "$HERE/zeron" "$HOME/.local/bin/zeron"
install -Dm644 "$HERE/zeron.desktop" "$HOME/.local/share/applications/zeron.desktop"
install -Dm644 "$HERE/zeron.png" "$HOME/.local/share/icons/hicolor/1024x1024/apps/zeron.png"
command -v update-desktop-database >/dev/null 2>&1 \
  && update-desktop-database "$HOME/.local/share/applications" || true

# The browser pane runs each web page in a separate, locked-down process, so
# that a bad page cannot reach the rest of your machine. Chromium needs this
# small helper to be owned by root before it can do that, and setting the
# owner is the one step here that asks for your password.
#
# You can skip it. The app still runs, and it prints at start-up that pages
# are not sandboxed. To do it later, run this script again.
if [ -f "$HERE/chrome-sandbox" ] && [ "${ZERON_SKIP_SANDBOX_SETUP:-}" != "1" ]; then
  if command -v sudo >/dev/null 2>&1; then
    echo "Setting up the browser page sandbox (asks for your password)."
    if sudo chown root:root "$HERE/chrome-sandbox" && sudo chmod 4755 "$HERE/chrome-sandbox"; then
      echo "Page sandbox ready."
    else
      echo "Skipped. Web pages will run unsandboxed; the app will say so when it starts."
    fi
  else
    echo "No sudo here, so the page sandbox is not set up. Web pages will run unsandboxed."
  fi
fi
echo "Installed. Make sure ~/.local/bin is on your PATH."
INSTALL
chmod 755 "$STAGE/install.sh"

tar -czf "$TARBALL" -C "$OUT_DIR" "$(basename "$STAGE")"
rm -rf "$STAGE"
echo "packaged: $TARBALL"
echo "entries: $(tar -tzf "$TARBALL" | wc -l)"
# `sed -n 1,40p` rather than `head -40`: head closes the pipe, tar dies of
# SIGPIPE, and under `pipefail` that makes this script exit 141 after having
# succeeded. sed reads the stream to the end.
tar -tzf "$TARBALL" | sed -n '1,40p'
