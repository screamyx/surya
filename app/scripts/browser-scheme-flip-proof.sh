#!/usr/bin/env bash
# Prove a mid-session theme change reaches the page: one run of the CEF
# proof pinned light, with `SURYA_PROOF_FLIP_SCHEME=<seconds>` flipping the
# app's appearance to dark part way through. The probe page
# (browser-scheme-page.html) is white under a light `prefers-color-scheme`
# and black under dark, so the frame dumps before the flip must be white
# and the last one black. Prints `proof: asked=2 matched=N`.
#
#   CARGO_TARGET_DIR=... CEF_PATH=... DISPLAY=:7 timeout 600 \
#     flock /store/surya-display7.lock app/scripts/browser-scheme-flip-proof.sh [seconds] [flip-at]
#
# Same rules as browser-scheme-proof.sh: the caller holds the display lock
# for the run only. The flip fires FLIP seconds after start; on a loaded box
# a cold launch needs about 150 s to load, so the default flips at 200 s of
# a 240 s run. Needs python3 (http.server + PIL).
set -u
WAIT="${1:-240}"
FLIP="${2:-200}"
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="${PROOF_OUT:-/tmp/surya-browser-scheme-flip-proof}"
rm -rf "$OUT"; mkdir -p "$OUT/www"
cp "$HERE/browser-scheme-page.html" "$OUT/www/index.html"
for _ in 1 2 3 4 5 6 7 8 9 10; do
  PORT=$(( 30000 + RANDOM % 20000 ))
  ss -Hltn 2>/dev/null | grep -q ":$PORT " || break
done
python3 -m http.server --bind 127.0.0.1 "$PORT" --directory "$OUT/www" > "$OUT/http.log" 2>&1 &
HTTP=$!
trap 'kill $HTTP 2>/dev/null' EXIT
sleep 1
URL="http://127.0.0.1:$PORT/index.html"
echo "serving $URL, flip at ${FLIP}s of ${WAIT}s"

PROOF_OUT="$OUT/run" SURYA_PROOF_APPEARANCE=light SURYA_PROOF_FLIP_SCHEME="$FLIP" \
  "$HERE/browser-xvfb-proof.sh" "$URL" "$WAIT" > "$OUT/run.log" 2>&1

# Mean luminance of the first frame dumped after the page loaded, and of
# the last one: white ~255 before the flip, black ~0 after.
python3 - "$OUT/run/frames" "$OUT/run/zeron.log" <<'PY' | tee "$OUT/measure.txt"
import sys, glob, os
from PIL import Image
frames = sorted(glob.glob(sys.argv[1] + "/frame-*.png"))
log = open(sys.argv[2], errors="replace").read()
def lum(p):
    im = Image.open(p).convert("L"); px = list(im.getdata()); return sum(px) / len(px)
loaded = "browser: load_end status=200" in log
flipped = "proof: scheme flipped" in log
if len(frames) < 2:
    print(f"frames={len(frames)} loaded={int(loaded)} flipped={int(flipped)}")
    print("proof: asked=2 matched=0"); sys.exit(1)
# The frame just before the flip is the light one: the most recent dump
# whose mtime is older than the flip line's arrival is not recorded, so use
# the brightest of the middle frames as "before" and the last as "after".
middle = frames[len(frames)//3: max(len(frames)//3 + 1, len(frames) - 1)] or frames[:-1]
before = max(lum(f) for f in middle)
after = lum(frames[-1])
matched = int(before > 192 and loaded) + int(after < 64 and flipped)
print(f"frames={len(frames)} loaded={int(loaded)} flipped={int(flipped)} before={before:.1f} (want >192) after={after:.1f} (want <64) last={os.path.basename(frames[-1])}")
print(f"proof: asked=2 matched={matched}")
sys.exit(0 if matched == 2 else 1)
PY
