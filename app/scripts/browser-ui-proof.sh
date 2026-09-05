#!/usr/bin/env bash
# Prove the browser pane's chrome: two tabs, the find bar with a count, and
# a page at 125%, shot at 1440x900 in light and dark, with the pane's own
# counters read back off the log.
#
#   CARGO_TARGET_DIR=... CEF_PATH=... DISPLAY=:7 \
#     timeout 600 flock /store/surya-display7.lock app/scripts/browser-ui-proof.sh light
#
# The keys are real X keystrokes through XTEST (scripts/x7-keys.py), not a
# test harness: the pane sees what a person's keyboard sends. Set
# SURYA_XTEST_PYTHON to a python with python-xlib.
#
# Needs: ffmpeg (x11grab), a `zeron` built with --features browser, and an X
# display that actually paints (the headless Xorg on the GPU, :7).
set -u
APPEARANCE="${1:-light}"
WAIT="${2:-150}"
HERE="$(cd "$(dirname "$0")" && pwd)"
TARGET="${CARGO_TARGET_DIR:-$(cd "$HERE/.." && pwd)/target}"
BIN="$TARGET/debug/zeron"
OUT="${PROOF_OUT:-/tmp/surya-browser-ui-proof}/$APPEARANCE"
PY="${SURYA_XTEST_PYTHON:-python3}"
W=1440; H=900
[ -x "$BIN" ] || { echo "no binary at $BIN (build with --features browser)"; exit 2; }
[ -f "$TARGET/debug/libcef.so" ] || { echo "no libcef.so next to the binary"; exit 2; }
[ -n "${DISPLAY:-}" ] || { echo "no DISPLAY; this rig needs one that paints (:7)"; exit 2; }

rm -rf "$OUT"; mkdir -p "$OUT"
DATA="$OUT/data"; mkdir -p "$DATA"
printf '{"appearance":"%s"}\n' "$APPEARANCE" > "$DATA/ui-settings.json"
LOG="$OUT/zeron.log"
SHOT="$OUT/browser-$APPEARANCE.png"

if [ -z "${ZERON_IPC_PORT:-}" ]; then
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    ZERON_IPC_PORT=$(( 20000 + RANDOM % 20000 ))
    ss -Hltn 2>/dev/null | grep -q ":$ZERON_IPC_PORT " || break
  done
fi
echo "proof: appearance=$APPEARANCE port=$ZERON_IPC_PORT display=$DISPLAY"

export ZERON_DATA_DIR="$DATA" ZERON_IPC_PORT SURYA_CEF_CACHE="$DATA/cef" \
       ZERON_OPEN_PANE=browser SURYA_BROWSER_URL="https://example.com" \
       SURYA_CEF_LOG="$OUT/cef.log" RUST_LOG=info
"$BIN" > "$LOG" 2>&1 &
APP=$!
# gpui's X11 backend on a headless server paints one frame and then waits
# for an Expose; xrefresh is the kick.
sleep 20; command -v xrefresh >/dev/null && xrefresh
REMAIN=$(( WAIT > 20 ? WAIT - 20 : 1 ))
sleep "$REMAIN"; command -v xrefresh >/dev/null && xrefresh; sleep 3

# The keyboard has to be in the page for the pane's chords to be the ones
# under test, so click the page first, then drive the chrome.
"$PY" "$HERE/x7-click.py" "$DISPLAY" $(( W - 200 )) $(( H / 2 )) || echo "click failed"
sleep 1

# 1. a second tab, typed into and loaded; 2. back to the first; 3. the find
# bar with a query that matches; 4. zoom to 125% (two ladder steps).
"$PY" "$HERE/x7-keys.py" "$DISPLAY" \
  ctrl+t "example.com" enter \
  || echo "keys step 1 failed"
sleep 12; command -v xrefresh >/dev/null && xrefresh; sleep 2
"$PY" "$HERE/x7-keys.py" "$DISPLAY" \
  ctrl+f "Domain" \
  || echo "keys step 2 failed"
sleep 4
"$PY" "$HERE/x7-keys.py" "$DISPLAY" ctrl+equal ctrl+equal || echo "keys step 3 failed"
sleep 3; command -v xrefresh >/dev/null && xrefresh; sleep 2

ffmpeg -loglevel error -y -f x11grab -video_size "${W}x${H}" -i "$DISPLAY" -frames:v 1 "$SHOT"
kill $APP 2>/dev/null; sleep 1; kill -9 $APP 2>/dev/null

echo "--- the pane's own counters, last line:"
grep -E "^browser-ui: " "$LOG" | tail -1
echo "--- every line the pane printed:"
grep -cE "^browser-ui: " "$LOG" | sed 's/^/lines=/'
grep -E "browser: (created|address|load_end)" "$LOG" | tail -4
echo "proof: appearance=$APPEARANCE shot=$SHOT ($(stat -c %s "$SHOT" 2>/dev/null || echo 0) bytes)"
