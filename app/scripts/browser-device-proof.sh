#!/usr/bin/env bash
# Prove the device picker changes the page's layout viewport: one run of the
# CEF proof with `SURYA_PROOF_DEVICE=<seconds>`, which reads the page's
# innerWidth over DevTools, picks the iPhone 15 preset, waits for the
# reload, and reads it again. The app prints
# `proof: device asked=2 matched=N before=<w> after=393`.
#
#   CARGO_TARGET_DIR=... CEF_PATH=... DISPLAY=:7 timeout 600 \
#     flock /store/surya-display7.lock app/scripts/browser-device-proof.sh [seconds] [switch-at]
#
# Same rules as the other proofs: the caller holds the display lock for the
# run only. Default switch at 200 s of a 240 s run (cold launch ~150 s).
set -u
WAIT="${1:-240}"
AT="${2:-200}"
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="${PROOF_OUT:-/tmp/surya-browser-device-proof}"
rm -rf "$OUT"; mkdir -p "$OUT/www"
cp "$HERE/browser-agent-page.html" "$OUT/www/index.html"
cp "$HERE/browser-agent-page-two.html" "$OUT/www/two.html"
for _ in 1 2 3 4 5 6 7 8 9 10; do
  PORT=$(( 30000 + RANDOM % 20000 ))
  ss -Hltn 2>/dev/null | grep -q ":$PORT " || break
done
python3 -m http.server --bind 127.0.0.1 "$PORT" --directory "$OUT/www" > "$OUT/http.log" 2>&1 &
HTTP=$!
trap 'kill $HTTP 2>/dev/null' EXIT
sleep 1
URL="http://127.0.0.1:$PORT/index.html"
echo "serving $URL, device switch at ${AT}s of ${WAIT}s"
PROOF_OUT="$OUT/run" SURYA_PROOF_DEVICE="$AT" "$HERE/browser-xvfb-proof.sh" "$URL" "$WAIT" > "$OUT/run.log" 2>&1
grep -E "^emulation:|^devtools:|^proof: device" "$OUT/run/surya.log" | tail -8
LINE=$(grep -E "^proof: device" "$OUT/run/surya.log" | tail -1)
echo "${LINE:-proof: device asked=2 matched=0 (no proof line; see $OUT/run/surya.log)} window=$OUT/run/window.png"
grep -q "matched=2" <<<"$LINE"
