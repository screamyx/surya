#!/usr/bin/env bash
# The address bar must follow a navigation NOBODY TYPED.
#
#   CARGO_TARGET_DIR=... CEF_PATH=... DISPLAY=:7 \
#     timeout 600 flock /store/surya-display7.lock \
#     app/scripts/browser-bar-follows-proof.sh
#
# Two tabs are opened from the environment and, after SURYA_SELFTEST_TABS
# seconds, the crate switches to the first one on its own. No keystroke is
# sent at any point. Before the fix the field latched on the first address it
# was given - the pane's own write came back as an `Edited` event after the
# guard flag had been cleared, so the pane read its own write as typing - and
# every later address change was ignored. The two frames here are the whole
# test: the bar must read the tab that is actually showing.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
TARGET="${CARGO_TARGET_DIR:-$(cd "$HERE/.." && pwd)/target}"
BIN="$TARGET/debug/surya"
OUT="${PROOF_OUT:-/store/agent-worktrees/surya-browser-ui/bar-follows}"
PY="${SURYA_XTEST_PYTHON:-python3}"
W=1440; H=900
SWITCH_AFTER=45
[ -x "$BIN" ] || { echo "no binary at $BIN"; exit 2; }
[ -n "${DISPLAY:-}" ] || { echo "no DISPLAY"; exit 2; }
rm -rf "$OUT"; mkdir -p "$OUT/data"
printf '{"appearance":"light"}\n' > "$OUT/data/ui-settings.json"
LOG="$OUT/surya.log"
PORT=$(( 20000 + RANDOM % 20000 ))

export SURYA_DATA_DIR="$OUT/data" SURYA_IPC_PORT=$PORT SURYA_CEF_CACHE="$OUT/data/cef" \
       SURYA_OPEN_PANE=browser RUST_LOG=info \
       SURYA_BROWSER_URL="https://example.com" \
       SURYA_BROWSER_URLS="https://example.net" \
       SURYA_SELFTEST_TABS="$SWITCH_AFTER"
"$BIN" > "$LOG" 2>&1 &
APP=$!
sleep 10
"$PY" "$HERE/x7-window.py" "$DISPLAY" 0 0 $W $H 240 || { echo "no window"; kill $APP; exit 3; }
for _ in $(seq 1 90); do
  grep -q "browser: created" "$LOG" && break
  sleep 2
done
# Both tabs loaded, before the crate switches on its own.
sleep 20; command -v xrefresh >/dev/null && xrefresh; sleep 3
ffmpeg -loglevel error -y -f x11grab -video_size "${W}x${H}" -i "$DISPLAY" -frames:v 1 "$OUT/before-switch.png"
# Past the switch.
sleep 35; command -v xrefresh >/dev/null && xrefresh; sleep 4
ffmpeg -loglevel error -y -f x11grab -video_size "${W}x${H}" -i "$DISPLAY" -frames:v 1 "$OUT/after-switch.png"
kill $APP 2>/dev/null; sleep 1; kill -9 $APP 2>/dev/null

echo "--- addresses the pane was given, in order:"
grep -E "browser: address" "$LOG" | tail -6
echo "--- the tab switch the crate did on its own:"
grep -iE "selftest|switch" "$LOG" | tail -4
echo "--- what the pane logged:"
grep -E "surya_browser_ui|browser-ui: " "$LOG" | tail -6
echo "proof: before=$OUT/before-switch.png ($(stat -c %s "$OUT/before-switch.png" 2>/dev/null || echo 0) bytes) after=$OUT/after-switch.png ($(stat -c %s "$OUT/after-switch.png" 2>/dev/null || echo 0) bytes)"
echo "read the two frames: the bar must name the tab that is showing, in both"
