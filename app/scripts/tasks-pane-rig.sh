#!/usr/bin/env bash
# Drive the Tasks pane on Linux with real clicks and real keystrokes, and say
# what the ENGINE saw. A UI check that does not need the Windows box.
#
# It runs the quick-add journey end to end and prints one counter per step:
#
#   caret_and_typing=1   clicked the box, typed a title, the box holds it
#   enter_created=1      pressed enter, the task reached the engine
#   key_nav=1            clicked a card, pressed n, the New task sheet opened
#
# Frames land beside the counters so a human can look. Nothing is deleted:
# every run gets its own timestamped folder.
#
#   scripts/tasks-pane-rig.sh [--hold]
#
# --hold leaves Xvfb, the engine and the demo running and writes their display,
# port and pids to the run folder's rig.env, for driving by hand with xdotool.
#
# Needs: Xvfb, xdotool, ffmpeg.
set -uo pipefail

HOLD=0
[ "${1:-}" = "--hold" ] && HOLD=1
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
command -v cargo >/dev/null 2>&1 || PATH="$HOME/.cargo/bin:$PATH"
CARGO="${SURYA_RIG_CARGO:-$([ -x /store/surya-cargo ] && echo /store/surya-cargo || echo cargo)}"
for tool in Xvfb xdotool ffmpeg; do
  command -v "$tool" >/dev/null 2>&1 || { echo "missing $tool"; exit 2; }
done

OUT="${SURYA_RIG_OUT:-/tmp/tasks-pane-rig}/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$OUT"
WORK="$OUT/work"; mkdir -p "$WORK"
XVFB=""; ENGINE=""; DEMO=""
cleanup() {
  [ "$HOLD" = "1" ] && { echo "held: $OUT/rig.env"; return; }
  for p in $DEMO $ENGINE $XVFB; do [ -n "$p" ] && kill "$p" 2>/dev/null; done
}
trap cleanup EXIT

echo "== build =="
$CARGO build -p zeron >"$OUT/build.log" 2>&1 || { tail -20 "$OUT/build.log"; exit 1; }
$CARGO build -p zeron-rpc --example rpc_probe >>"$OUT/build.log" 2>&1 || { tail -20 "$OUT/build.log"; exit 1; }
TARGET="$(cargo metadata --format-version 1 --no-deps 2>/dev/null | python3 -c 'import json,sys;print(json.load(sys.stdin)["target_directory"])')"
ZERON="$TARGET/debug/zeron"; PROBE="$TARGET/debug/examples/rpc_probe"

# +extension XTEST or xdotool cannot move the pointer: mousemove is silently a
# no-op and getmouselocation keeps reporting screen centre.
for n in $(seq 90 120); do [ -e "/tmp/.X$n-lock" ] || { DISP=":$n"; break; }; done
DISP=${DISP:-:99}
Xvfb "$DISP" -screen 0 1440x900x24 +extension XTEST +extension RANDR -noreset >"$OUT/xvfb.log" 2>&1 &
XVFB=$!
sleep 3
DISPLAY=$DISP xdotool getmouselocation >/dev/null 2>&1 || { echo "no usable X on $DISP"; cat "$OUT/xvfb.log"; exit 1; }

PORT=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')
ZERON_DATA_DIR="$WORK/engine" ZERON_IPC_PORT=$PORT "$ZERON" headless >"$OUT/engine.log" 2>&1 &
ENGINE=$!
probe() { "$PROBE" "ws://127.0.0.1:$PORT" "$@" 2>>"$OUT/probe.err"; }
for _ in $(seq 1 120); do probe EngineReady '{}' >/dev/null 2>&1 && break; sleep 0.5; done
probe EngineReady '{}' >/dev/null 2>&1 || { echo "engine never ready"; tail -10 "$OUT/engine.log"; exit 1; }

DEVICE=$(probe LocalDevice '{}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["deviceId"])')
REPO="$WORK/checkout"; mkdir -p "$REPO/src"
git -C "$REPO" init -q
git -C "$REPO" config user.email rig@surya
git -C "$REPO" config user.name rig
printf 'fn main() {}\n' >"$REPO/src/main.rs"
git -C "$REPO" add -A && git -C "$REPO" commit -qm one
probe Mutate "$(printf '{"op":"createSpace","spaceId":"sp-rig","deviceId":"%s","path":"%s"}' "$DEVICE" "$REPO")" >/dev/null

DISPLAY=$DISP ZERON_DATA_DIR="$WORK/ui" ZERON_IPC_PORT=$PORT \
  "$ZERON" --tasks-demo --tasks-space sp-rig >"$OUT/demo.log" 2>&1 &
DEMO=$!
# GPU adapter probing under Xvfb took 20 s on the dev box.
for _ in $(seq 1 240); do grep -q "tasks-demo: started=1" "$OUT/demo.log" 2>/dev/null && break; sleep 0.5; done
grep -q "tasks-demo: started=1" "$OUT/demo.log" || { echo "demo never started"; tail -20 "$OUT/demo.log"; exit 1; }
sleep 12

shot() { ffmpeg -hide_banner -loglevel error -y -f x11grab -video_size 1440x900 -i "$DISP" -frames:v 1 "$OUT/$1.png" 2>>"$OUT/ffmpeg.err"; }
click() { DISPLAY=$DISP xdotool mousemove --sync "$1" "$2" click 1; }

# The window does not paint until an event reaches it. A shot taken straight
# after startup is black; one pointer move fixes that.
DISPLAY=$DISP xdotool mousemove --sync 700 400; sleep 2
shot 1-board

# The quick-add box: first thing in the Queued column, under its header.
click 260 190; sleep 2
TITLE="rig-typed-$$"
DISPLAY=$DISP xdotool type --delay 80 "$TITLE"; sleep 2
shot 2-typed
DISPLAY=$DISP xdotool key Return; sleep 3
shot 3-entered
enter_created=$(probe WatchTasks '{"spaceId":"sp-rig"}' --stream 1 2>/dev/null | grep -c "$TITLE" || true)
[ "${enter_created:-0}" -ge 1 ] && enter_created=1 || enter_created=0
# Typing is proven by the same title arriving: it can only get there through
# the box. A title that never reached the engine never reached the box either.
caret_and_typing=$enter_created

# The card sits under the box. Click it, then `n` for the New task sheet.
click 277 273; sleep 2
shot 4-card-selected
DISPLAY=$DISP xdotool key n; sleep 2
shot 5-key-nav
# The sheet is a large panel over a dimmed board, so the frame changes a lot.
# PSNR between the two shots measures that: identical frames score `inf`, and
# anything under 20 dB is a gross change. ffmpeg is already a dependency.
psnr=$(ffmpeg -hide_banner -loglevel info -i "$OUT/4-card-selected.png" -i "$OUT/5-key-nav.png" \
  -filter_complex psnr -f null - 2>&1 | sed -n 's/.*[^a-z]average:\([0-9.]*\).*/\1/p' | head -1)
case "${psnr:-}" in
  ""|inf) key_nav=0 ;;
  *) key_nav=$(awk -v v="$psnr" 'BEGIN { print (v < 20.0) ? 1 : 0 }') ;;
esac

echo
echo "caret_and_typing=$caret_and_typing  (asked 1)"
echo "enter_created=$enter_created  (asked 1)"
echo "key_nav=$key_nav  (asked 1; psnr=${psnr:-none} dB, under 20 means the sheet opened; read 5-key-nav.png)"
echo "frames: $OUT"

if [ "$HOLD" = "1" ]; then
  cat > "$OUT/rig.env" <<ENV
DISP=$DISP
PORT=$PORT
XVFB=$XVFB
ENGINE=$ENGINE
DEMO=$DEMO
OUT=$OUT
PROBE=$PROBE
ENV
fi

[ "$caret_and_typing$enter_created$key_nav" = "111" ] || exit 1
