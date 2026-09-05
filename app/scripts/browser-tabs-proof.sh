#!/usr/bin/env bash
# Prove one browser per tab: open two tabs at start (a red page, then a blue
# one, so the blue tab is active), grab the pane, then run again with the
# self-test that switches back to the first tab from the pump, and grab
# again. The pane's centre pixel says which tab is on screen. Prints
# `proof: asked=2 matched=N`.
#
#   CARGO_TARGET_DIR=... CEF_PATH=... DISPLAY=:7 timeout 900 \
#     flock /store/surya-display7.lock app/scripts/browser-tabs-proof.sh [seconds]
#
# Same display-lock rules as browser-xvfb-proof.sh: the caller holds the lock
# for the run only. Needs python3 (http.server + PIL).
set -u
WAIT="${1:-150}"
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="${PROOF_OUT:-/tmp/surya-browser-tabs-proof}"
mkdir -p "$OUT"
WWW="$OUT/www"; mkdir -p "$WWW"
printf '<!doctype html><html><head><meta charset="utf-8"><title>tab A red</title><style>html,body{margin:0;height:100%%;background:#c00}</style></head><body></body></html>' > "$WWW/a.html"
printf '<!doctype html><html><head><meta charset="utf-8"><title>tab B blue</title><style>html,body{margin:0;height:100%%;background:#00c}</style></head><body></body></html>' > "$WWW/b.html"

for _ in 1 2 3 4 5 6 7 8 9 10; do
  PORT=$(( 30000 + RANDOM % 20000 ))
  ss -Hltn 2>/dev/null | grep -q ":$PORT " || break
done
python3 -m http.server --bind 127.0.0.1 "$PORT" --directory "$WWW" > "$OUT/http.log" 2>&1 &
HTTP=$!
trap 'kill $HTTP 2>/dev/null' EXIT
sleep 1
A="http://127.0.0.1:$PORT/a.html"
B="http://127.0.0.1:$PORT/b.html"
echo "serving $A and $B"

# The colour at the middle of the pane in the window grab (1600x1000, the
# right pane spans about x 926..1446, y 140..925; the middle is well inside
# it at any pane width the shell allows): "red", "blue" or what it found.
# The grab, not a frame dump: dumps happen on paint 1..3 and every 30th,
# and a static page painted after a tab switch may never reach one.
dominant() { # dominant <window.png>
  python3 - "$1" <<'PY'
import sys, os
from PIL import Image
path = sys.argv[1]
if not os.path.exists(path):
    print("none"); sys.exit(0)
im = Image.open(path).convert("RGB")
w, h = im.size
x, y = int(w * 0.74), int(h * 0.55)
r, g, b = im.getpixel((x, y))
name = "red" if r > 150 and b < 80 else "blue" if b > 150 and r < 80 else f"rgb({r},{g},{b})"
print(f"{name} at {x},{y} of {w}x{h}")
PY
}

run() { # run <name> <expected colour> <extra env...>
  local name=$1 expect=$2; shift 2
  local dir="$OUT/$name"
  rm -rf "$dir"
  env PROOF_OUT="$dir" SURYA_BROWSER_URLS="$B" "$@" "$HERE/browser-xvfb-proof.sh" "$A" "$WAIT" > "$OUT/run-$name.log" 2>&1
  local opened created switched heartbeat
  opened=$(grep -c "browser: tab_open" "$dir/zeron.log" 2>/dev/null); opened=${opened:-0}
  created=$(grep -c "browser: created id=" "$dir/zeron.log" 2>/dev/null); created=${created:-0}
  switched=$(grep -c "browser: selftest tabs: activated" "$dir/zeron.log" 2>/dev/null); switched=${switched:-0}
  heartbeat=$(grep -E "^browser: t=" "$dir/zeron.log" | tail -1 | grep -oE "tabs=[0-9]+ opened=[0-9]+ active=[0-9]+|bg_paints=[0-9]+ kept_frames=[0-9]+" | tr '\n' ' ')
  local colour; colour=$(dominant "$dir/window.png")
  local ok=0
  [ "${colour%% *}" = "$expect" ] && [ "$opened" -ge 2 ] && [ "$created" -ge 2 ] && ok=1
  echo "$name: tab_open=$opened created=$created switched=$switched $heartbeat"
  echo "$name: pane shows $colour (expected $expect) matched=$ok window=$dir/window.png"
  MATCHED=$((MATCHED + ok))
}

MATCHED=0
run two-tabs blue
run switch-back red SURYA_SELFTEST_TABS=$(( WAIT > 40 ? WAIT - 30 : 10 ))
echo "proof: asked=2 matched=$MATCHED"
[ "$MATCHED" -eq 2 ]
