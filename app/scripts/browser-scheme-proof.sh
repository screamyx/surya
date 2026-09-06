#!/usr/bin/env bash
# Prove the browser pane passes the app's appearance to the page: run the
# CEF proof twice, once with the app pinned light and once dark, against a
# local page that is all white under `prefers-color-scheme: light` and all
# black under dark (browser-scheme-page.html), then measure the last CEF
# frame dump of each run. Prints `proof: asked=2 matched=N`.
#
#   CARGO_TARGET_DIR=... CEF_PATH=... DISPLAY=:7 timeout 600 \
#     flock /store/surya-display7.lock app/scripts/browser-scheme-proof.sh [seconds]
#
# Same display-lock rules as browser-xvfb-proof.sh: the caller holds the lock
# for the run only. On a loaded box the page sometimes never loads: the app
# log shows Chromium's "Network service crashed or was terminated, restarting
# service" about a second after "engine core assembled" and no load_end
# (2026-09-05: 5 of 6 runs without SURYA_CEF_LOG, 0 of 2 with it, in and out
# of heavy.slice alike). Cause not found; rerun, and set SURYA_CEF_LOG=<file>
# to keep Chromium's own log for the next look. Needs python3 (http.server + PIL for the measurement).
# `SURYA_SCHEME_MODES="dark"` runs one mode (a rerun after a starved launch);
# the default is both. A cold debug launch on a loaded box needs 180 s or
# more before the pane opens (measured 2026-09-05: 150 s to load_end).
set -u
WAIT="${1:-180}"
MODES="${SURYA_SCHEME_MODES:-light dark}"
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="${PROOF_OUT:-/tmp/surya-browser-scheme-proof}"
mkdir -p "$OUT"
WWW="$OUT/www"; mkdir -p "$WWW"
cp "$HERE/browser-scheme-page.html" "$WWW/index.html"

# A loopback web server on a free port; the pane allows 127.0.0.1 addresses.
for _ in 1 2 3 4 5 6 7 8 9 10; do
  PORT=$(( 30000 + RANDOM % 20000 ))
  ss -Hltn 2>/dev/null | grep -q ":$PORT " || break
done
python3 -m http.server --bind 127.0.0.1 "$PORT" --directory "$WWW" > "$OUT/http.log" 2>&1 &
HTTP=$!
trap 'kill $HTTP 2>/dev/null' EXIT
sleep 1
URL="http://127.0.0.1:$PORT/index.html"
echo "serving $URL"

# Mean luminance of the last CEF frame dump of a run: white page ~255, black ~0.
luminance() { # luminance <frames dir>
  python3 - "$1" <<'PY'
import sys, glob
from PIL import Image
files = sorted(glob.glob(sys.argv[1] + "/frame-*.png"))
if not files:
    print("none"); sys.exit(0)
im = Image.open(files[-1]).convert("L")
w, h = im.size
px = list(im.getdata())
print(f"{sum(px) / len(px):.1f} {w}x{h} {files[-1].rsplit('/', 1)[-1]}")
PY
}

ASKED=0; MATCHED=0
for MODE in $MODES; do
  ASKED=$((ASKED + 1))
  RUN="$OUT/$MODE"
  rm -rf "$RUN"
  PROOF_OUT="$RUN" SURYA_PROOF_APPEARANCE="$MODE" "$HERE/browser-xvfb-proof.sh" "$URL" "$WAIT" > "$OUT/run-$MODE.log" 2>&1
  SWITCHES=$(grep -m1 "browser: switches" "$RUN/surya.log" 2>/dev/null || echo "(no switches line)")
  LUM=$(luminance "$RUN/frames")
  MEAN=${LUM%% *}
  case "$MODE" in
    light) EXPECT="mean > 192" ; OK=$(awk -v m="$MEAN" 'BEGIN{print (m+0 > 192) ? 1 : 0}') ;;
    dark)  EXPECT="mean < 64"  ; OK=$(awk -v m="$MEAN" 'BEGIN{print (m+0 < 64) ? 1 : 0}') ;;
  esac
  [ "$MEAN" = "none" ] && OK=0
  # A white frame is also what CEF paints before any page (its opaque
  # background), so a match needs the page to have actually loaded.
  LOADED=$(grep -c "browser: load_end status=200" "$RUN/surya.log" 2>/dev/null); LOADED=${LOADED:-0}
  [ "$LOADED" -ge 1 ] || OK=0
  MATCHED=$((MATCHED + OK))
  echo "$MODE: $SWITCHES"
  echo "$MODE: load_end=$LOADED last frame luminance $LUM (expected $EXPECT) matched=$OK window=$RUN/window.png"
done
echo "proof: asked=$ASKED matched=$MATCHED"
[ "$MATCHED" -eq "$ASKED" ]
