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
# How many two-second turns to wait for the engine line before giving up.
WAIT="${2:-90}"
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
# There is no window manager on the headless Xorg, so nothing places the
# window on screen and nothing gives it the keyboard: x7-window does both.
sleep 10
"$PY" "$HERE/x7-window.py" "$DISPLAY" 0 0 $W $H 240 || echo "window move failed"
# The window maps long before the shell renders into it; a debug build under
# load takes another minute. Wait for the engine line, then the first frame.
for _ in $(seq 1 "$WAIT"); do
  grep -q "engine core assembled" "$LOG" && break
  sleep 2
done
sleep 15; command -v xrefresh >/dev/null && xrefresh; sleep 3

# The keyboard starts in the page, as it does after a person clicks a page.
"$PY" "$HERE/x7-click.py" "$DISPLAY" $(( W - 200 )) $(( H / 2 )) || echo "click failed"
sleep 1

# A second tab, then three addresses typed into the bar and loaded. ctrl-a
# selects what the bar already shows, the way a fresh click in Chrome does.
"$PY" "$HERE/x7-keys.py" "$DISPLAY" ctrl+t "example.org" enter || echo "keys 1 failed"
sleep 14
"$PY" "$HERE/x7-keys.py" "$DISPLAY" ctrl+a "iana.org" enter || echo "keys 2 failed"
sleep 14
"$PY" "$HERE/x7-keys.py" "$DISPLAY" ctrl+a "example.com" enter || echo "keys 3 failed"
sleep 16; command -v xrefresh >/dev/null && xrefresh; sleep 2

# Find a word the page actually shows, then zoom two steps to 125%.
"$PY" "$HERE/x7-click.py" "$DISPLAY" $(( W - 200 )) $(( H / 2 )) || echo "click failed"
"$PY" "$HERE/x7-keys.py" "$DISPLAY" ctrl+f "Domain" || echo "keys 4 failed"
sleep 5
"$PY" "$HERE/x7-keys.py" "$DISPLAY" ctrl+equal ctrl+equal || echo "keys 5 failed"
sleep 4; command -v xrefresh >/dev/null && xrefresh; sleep 3

ffmpeg -loglevel error -y -f x11grab -video_size "${W}x${H}" -i "$DISPLAY" -frames:v 1 "$SHOT"
kill $APP 2>/dev/null; sleep 1; kill -9 $APP 2>/dev/null

echo "--- what the pane did, in its own words:"
grep -E "^browser-ui: " "$LOG" | grep -v "^browser-ui: key " | tail -12
echo "--- tabs, and what loaded:"
grep -E "browser: (created|address|load_end)" "$LOG" | tail -8
grep -E "^browser: t=" "$LOG" | tail -1 | sed -n 's/.*\(tabs=[0-9]* opened=[0-9]* active=[0-9]*\).*/\1/p'
echo "proof: appearance=$APPEARANCE shot=$SHOT ($(stat -c %s "$SHOT" 2>/dev/null || echo 0) bytes)"
