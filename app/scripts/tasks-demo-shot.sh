#!/usr/bin/env bash
# Pixel proof for the Tasks pane on the shared headless Xorg (:7): throwaway
# engine, a seeded board (one space, six tasks), `zeron --tasks-demo`, one
# frame grabbed with ffmpeg. Prints `shot=1 bytes=N` and the PNG path.
#
#   scripts/tasks-demo-shot.sh [out.png] [path/to/zeron]
set -euo pipefail
OUT=${1:-/tmp/tasks-demo.png}
ZERON=${2:-${CARGO_TARGET_DIR:-target}/debug/zeron}
DISPLAY_NO=${SURYA_SHOT_DISPLAY:-:7}
[ -x "$ZERON" ] || { echo "no zeron binary at $ZERON" >&2; exit 2; }
WORK=$(mktemp -d /tmp/tasks-shot-XXXXXX)
PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
trap 'kill $ENGINE $DEMO 2>/dev/null || true; rm -rf "$WORK"' EXIT

ZERON_DATA_DIR="$WORK/engine" ZERON_IPC_PORT=$PORT "$ZERON" headless > "$WORK/engine.log" 2>&1 &
ENGINE=$!
for _ in $(seq 1 60); do grep -q "IPC server listening" "$WORK/engine.log" 2>/dev/null && break; sleep 0.5; done

# Seed over the RPC socket with node's built-in WebSocket (node >= 22).
node - "$PORT" <<'JS'
const port = process.argv[2];
const ws = new WebSocket(`ws://127.0.0.1:${port}`);
let id = 0; const pending = new Map();
const call = (method, params) => new Promise((ok, err) => {
  const n = ++id; pending.set(n, { ok, err });
  ws.send(JSON.stringify({ id: n, method, params }));
});
ws.onmessage = (m) => {
  const f = JSON.parse(m.data); const p = pending.get(f.id); if (!p) return;
  if (f.err) p.err(new Error(f.err)); else if (f.ok !== undefined) p.ok(f.ok);
  pending.delete(f.id);
};
ws.onopen = async () => {
  const dev = await call("LocalDevice", {});
  await call("Mutate", { op: "createSpace", spaceId: "space-demo", deviceId: dev.deviceId, path: "/tmp", name: "project-jag" });
  const t = (taskId, title, status, extra = {}) =>
    call("Mutate", { op: "createTask", taskId, spaceId: "space-demo", title, status, ...extra });
  await t("t-5", "Media tab scrolls sideways after Library upload (#553)", "queued", { links: ["https://github.com/screamyx/project-jag/issues/553"] });
  await t("t-1", "Add follow-up fields to leads", "running", { owner: "raven", notes: "migration + resource, PR open" });
  await t("t-2", "Reminder job the morning after a lead", "running", { owner: "kite" });
  await t("t-3", "Show next follow-up on the lead card in the PWA", "blocked", { notes: "waits on t-1 and t-2" });
  await t("t-4", "Tile jumps when a photo finishes uploading", "done", { owner: "heron", links: ["https://github.com/screamyx/project-jag/pull/591"] });
  await t("t-6", "Publish the 12 new reels to the catalog", "queued", { owner: "swift" });
  console.log("seeded=1 tasks=6"); ws.close(); process.exit(0);
};
ws.onerror = (e) => { console.error("seed failed", e.message || e); process.exit(1); };
JS

exec 9>/store/surya-display7.lock
flock 9
ZERON_DATA_DIR="$WORK/ui" ZERON_IPC_PORT=$PORT DISPLAY=$DISPLAY_NO \
  "$ZERON" --tasks-demo --tasks-space space-demo > "$WORK/demo.log" 2>&1 &
DEMO=$!
sleep 10
DISPLAY=$DISPLAY_NO xrefresh
sleep 3
ffmpeg -loglevel error -y -f x11grab -video_size 1600x1000 -i "$DISPLAY_NO" -frames:v 1 "$OUT"
kill $DEMO 2>/dev/null || true
flock -u 9
bytes=$(stat -c %s "$OUT")
panics=$(grep -c "panicked at" "$WORK/demo.log" || true)
echo "shot=1 bytes=$bytes panics=$panics out=$OUT"
grep "tasks-demo:" "$WORK/demo.log" || true
