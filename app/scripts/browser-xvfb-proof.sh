#!/usr/bin/env bash
# Prove the CEF pane paints on a box with no display: launch zeron under
# Xvfb with the Browser surface open, wait, screenshot, and grep the log
# for the browser's own counters. Prints asked/paint pairs, never a bare 0.
#
#   CARGO_TARGET_DIR=... CEF_PATH=... app/scripts/browser-xvfb-proof.sh [url] [seconds]
#
# Needs: xvfb-run, ffmpeg (x11grab), a `zeron` built with --features browser.
set -u
URL="${1:-https://example.com}"
WAIT="${2:-25}"
TARGET="${CARGO_TARGET_DIR:-$(cd "$(dirname "$0")/.." && pwd)/target}"
BIN="$TARGET/debug/zeron"
OUT="${PROOF_OUT:-/tmp/surya-browser-proof}"
mkdir -p "$OUT"
[ -x "$BIN" ] || { echo "no binary at $BIN (build with --features browser)"; exit 2; }
[ -x "$TARGET/debug/zeron-browser-helper" ] || echo "warning: no zeron-browser-helper next to zeron; CEF will re-exec zeron"
[ -f "$TARGET/debug/libcef.so" ] || { echo "no libcef.so next to the binary"; exit 2; }

DATA="$OUT/data"
rm -rf "$DATA"; mkdir -p "$DATA"
LOG="$OUT/zeron.log"
SHOT="$OUT/window.png"
W=1600; H=1000

RUN='
  set -u
  export ZERON_DATA_DIR="'"$DATA"'" ZERON_OPEN_BROWSER=1 SURYA_BROWSER_URL="'"$URL"'" \
         SURYA_CEF_CACHE="'"$DATA"'/cef" SURYA_BROWSER_DUMP="'"$OUT"'/frames" RUST_LOG=info
  "'"$BIN"'" > "'"$LOG"'" 2>&1 &
  APP=$!
  # gpui'"'"'s X11 backend on a headless server draws one frame and then waits
  # for an Expose (measured 2026-09-05: renders stayed at 1 for 40s, went to
  # 13 within 6s of one xrefresh). Kick it once the window is up and once
  # more before the grab.
  sleep 10; command -v xrefresh >/dev/null && xrefresh
  sleep $(( '"$WAIT"' > 13 ? '"$WAIT"' - 13 : 1 )); command -v xrefresh >/dev/null && xrefresh; sleep 3
  ffmpeg -loglevel error -y -f x11grab -video_size '"${W}x${H}"' -i "$DISPLAY" -frames:v 1 "'"$SHOT"'"
  kill $APP 2>/dev/null; sleep 1; kill -9 $APP 2>/dev/null
'
if [ -n "${DISPLAY:-}" ]; then
  # An existing X server (a headless Xorg on the GPU, or a real desktop).
  echo "using DISPLAY=$DISPLAY"
  bash -c "$RUN"
else
  xvfb-run -a -s "-screen 0 ${W}x${H}x24" bash -c "$RUN"
fi
echo "--- log: $LOG"
grep -E "browser: (create|created|view_rect #1|on_paint #1|on_paint #[0-9]+:|address|load_end|LOAD ERROR|cef initialize|switches)" "$LOG" | head -20
grep -E "browser: view ->" "$LOG" | head -2
LAST=$(grep -E "^browser: t=" "$LOG" | tail -1)
echo "--- last heartbeat: ${LAST:-none}"
ASKED=$(grep -c "browser: create asked=1" "$LOG")
FRAMES=$(grep -E "^browser: t=" "$LOG" | tail -1 | sed -n 's/.* frames=\([0-9]*\).*/\1/p')
echo "proof: asked=$ASKED frames=${FRAMES:-0} screenshot=$SHOT ($(stat -c %s "$SHOT" 2>/dev/null || echo 0) bytes) dumps=$(ls "$OUT"/frames 2>/dev/null | wc -l)"
