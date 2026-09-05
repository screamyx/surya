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

# Every click and chord goes through python-xlib. When that import fails the
# scripts exit non-zero, and a rig that shrugged and shot anyway would hand
# back a green-looking shot of a screen nobody drove. So: any input step
# that fails ends the run, with the app killed and no screenshot written.
drive() {
  if ! "$PY" "$@"; then
    echo "proof: FAILED, input step did not run: $*" >&2
    echo "proof: no shot written. Set SURYA_XTEST_PYTHON to a python with python-xlib." >&2
    [ -n "${APP:-}" ] && { kill "$APP" 2>/dev/null; sleep 1; kill -9 "$APP" 2>/dev/null; }
    exit 4
  fi
}
W=1440; H=900
[ -x "$BIN" ] || { echo "no binary at $BIN (build with --features browser)"; exit 2; }
[ -f "$TARGET/debug/libcef.so" ] || { echo "no libcef.so next to the binary"; exit 2; }
[ -n "${DISPLAY:-}" ] || { echo "no DISPLAY; this rig needs one that paints (:7)"; exit 2; }

rm -rf "$OUT"; mkdir -p "$OUT"
DATA="$OUT/data"; mkdir -p "$DATA"
printf '{"appearance":"%s"}\n' "$APPEARANCE" > "$DATA/ui-settings.json"
LOG="$OUT/zeron.log"
SHOT="$OUT/browser-$APPEARANCE.png"
REFUSED_SHOT="$OUT/browser-$APPEARANCE-refused.png"

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
drive "$HERE/x7-window.py" "$DISPLAY" 0 0 $W $H 240
# The window maps long before the shell renders into it; a debug build under
# load takes another minute. Wait for the engine line, then the first frame.
for _ in $(seq 1 "$WAIT"); do
  grep -q "engine core assembled" "$LOG" && break
  sleep 2
done
# The engine is not the pane. Wait for the browser the pane opens and for
# the keymap the shell binds when it is built: without both, the first
# chord lands on a window that has not rendered and is simply lost.
for _ in $(seq 1 "$WAIT"); do
  grep -q "browser: created" "$LOG" && grep -q "browser-ui: keymap" "$LOG" && break
  sleep 2
done
sleep 15; command -v xrefresh >/dev/null && xrefresh; sleep 3

# The keyboard starts in the page, as it does after a person clicks a page.
drive "$HERE/x7-click.py" "$DISPLAY" $(( W - 200 )) $(( H / 2 ))
sleep 1

# A second tab, then three addresses typed into the bar and loaded. ctrl-a
# selects what the bar already shows, the way a fresh click in Chrome does.
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+t "example.org" enter
sleep 14
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+a "iana.org" enter
sleep 14
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+a "example.com" enter
sleep 16; command -v xrefresh >/dev/null && xrefresh; sleep 2

# Coordinates in the 1440x900 window, read off an earlier frame: the tab row
# sits at y=55, the first tab's close X at x=1013, the second tab at x=1090,
# and the address field at x=250 (its left end, clear of the buttons).
TAB1_X=1013; TAB2=1090; ROW_Y=55; BAR_X=250; BAR_Y=88
PAGE_X=$(( W - 200 )); PAGE_Y=$(( H / 2 ))

# ctrl-tab and shift-ctrl-tab. comet binds both context-less for its own
# session strip, so these are the chords that prove the pane takes them in
# the capture phase; if the keymap rule ever wins again they switch sessions
# and `browser-ui: switch tab` never appears.
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+tab
sleep 3
drive "$HERE/x7-keys.py" "$DISPLAY" shift+ctrl+tab
sleep 3

# A third tab, closed again by chord: ctrl-w must close a TAB, not the window.
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+t
sleep 3
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+w
sleep 3

# Middle click on a tab closes it, as it does in every strip.
drive "$HERE/x7-click.py" "$DISPLAY" "$TAB2" "$ROW_Y" 2
sleep 3
# The strip's own X on the LAST tab: a blank tab opens in its place rather
# than leaving the pane on an empty strip over a white page.
drive "$HERE/x7-click.py" "$DISPLAY" "$TAB1_X" "$ROW_Y"
sleep 3

# The blank tab's bar is empty and waiting; click it and select-all anyway,
# so a stray character cannot turn this into example.comexample.com.
drive "$HERE/x7-click.py" "$DISPLAY" "$BAR_X" "$BAR_Y"
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+a "example.com" enter
sleep 14
# A refused scheme has to say so on screen, not only on stdout.
drive "$HERE/x7-click.py" "$DISPLAY" "$BAR_X" "$BAR_Y"
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+a "file:///etc/passwd" enter
sleep 3; command -v xrefresh >/dev/null && xrefresh; sleep 2
ffmpeg -loglevel error -y -f x11grab -video_size "${W}x${H}" -i "$DISPLAY" -frames:v 1 "$REFUSED_SHOT"
# Escape puts the page's own address back and clears the refusal.
drive "$HERE/x7-keys.py" "$DISPLAY" escape
sleep 2

# Back to two tabs for the frame, then find and zoom.
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+t
sleep 2
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+a "example.com" enter
sleep 14
drive "$HERE/x7-click.py" "$DISPLAY" "$PAGE_X" "$PAGE_Y"
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+f "Domain"
sleep 5
drive "$HERE/x7-keys.py" "$DISPLAY" ctrl+equal ctrl+equal
sleep 4; command -v xrefresh >/dev/null && xrefresh; sleep 3

ffmpeg -loglevel error -y -f x11grab -video_size "${W}x${H}" -i "$DISPLAY" -frames:v 1 "$SHOT"
kill $APP 2>/dev/null; sleep 1; kill -9 $APP 2>/dev/null

echo "--- what the pane did, in its own words:"
grep -E "^browser-ui: " "$LOG" | grep -v "^browser-ui: key " | tail -12
echo "--- tabs, and what loaded:"
grep -E "browser: (created|address|load_end)" "$LOG" | tail -8
grep -E "^browser: t=" "$LOG" | tail -1 | sed -n 's/.*\(tabs=[0-9]* opened=[0-9]* active=[0-9]*\).*/\1/p'
echo "proof: appearance=$APPEARANCE shot=$SHOT ($(stat -c %s "$SHOT" 2>/dev/null || echo 0) bytes)"
echo "proof: refused-scheme frame=$REFUSED_SHOT ($(stat -c %s "$REFUSED_SHOT" 2>/dev/null || echo 0) bytes)"
