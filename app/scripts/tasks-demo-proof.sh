#!/usr/bin/env bash
# Headless proof for `zeron --tasks-demo` (surya task board pane): start a
# throwaway local engine, open only the Tasks pane under Xvfb for N seconds,
# count starts and panics. Prints `started=1 panics=0` on success.
#
#   scripts/tasks-demo-proof.sh [seconds] [path/to/zeron]
set -euo pipefail
SECS=${1:-5}
ZERON=${2:-${CARGO_TARGET_DIR:-target}/debug/zeron}
[ -x "$ZERON" ] || { echo "no zeron binary at $ZERON" >&2; exit 2; }
WORK=$(mktemp -d /tmp/tasks-demo-XXXXXX)
PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
trap 'kill $ENGINE 2>/dev/null || true; rm -rf "$WORK"' EXIT

ZERON_DATA_DIR="$WORK/engine" ZERON_IPC_PORT=$PORT "$ZERON" headless > "$WORK/engine.log" 2>&1 &
ENGINE=$!
for _ in $(seq 1 60); do
  grep -q "IPC server listening" "$WORK/engine.log" 2>/dev/null && break
  sleep 0.5
done

set +e
ZERON_DATA_DIR="$WORK/ui" ZERON_IPC_PORT=$PORT SURYA_DEMO_EXIT_SECS=$SECS \
  xvfb-run -a -s "-screen 0 1440x900x24" "$ZERON" --tasks-demo > "$WORK/demo.log" 2>&1
DEMO_EXIT=$?
set -e

started=$(grep -c "tasks-demo: started=1 window=1" "$WORK/demo.log" || true)
panics=$(grep -c "panicked at" "$WORK/demo.log" || true)
echo "demo_exit=$DEMO_EXIT started=$started panics=$panics"
grep "tasks-demo:" "$WORK/demo.log" || true
if [ "$started" != "1" ] || [ "$panics" != "0" ]; then
  echo "--- demo.log (tail)"; tail -40 "$WORK/demo.log"
  echo "--- engine.log (tail)"; tail -20 "$WORK/engine.log"
  exit 1
fi
