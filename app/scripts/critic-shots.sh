#!/usr/bin/env bash
# Design-critic shot rig: a throwaway engine with the mock harness, a real
# project (this clone), one chat streaming a paced mock run, then the headed
# shell on the shared headless Xorg (:7), light and dark, two window sizes.
#
#   scripts/critic-shots.sh <out_dir> [path/to/zeron] [project_dir]
# Writes <out_dir>/shell-{light,dark}-{1440x900,1100x700}.png and prints one
# `shot=1 bytes=N` line per frame.
set -euo pipefail
OUT=${1:?out dir}; ZERON=${2:-${CARGO_TARGET_DIR:-target}/debug/zeron}
PROJECT=${3:-$(cd "$(dirname "$0")/../.." && pwd)}
DISPLAY_NO=${SURYA_SHOT_DISPLAY:-:7}
[ -x "$ZERON" ] || { echo "no zeron binary at $ZERON" >&2; exit 2; }
mkdir -p "$OUT"
WORK=$(mktemp -d /tmp/critic-shots-XXXXXX)
PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
trap 'kill $ENGINE ${APP:-} 2>/dev/null || true; rm -rf "$WORK"' EXIT

# Mock harness, paced so the chat is still Running when the frame is taken;
# tables and code blocks so the transcript shows every row kind.
ZERON_DATA_DIR="$WORK/engine" ZERON_IPC_PORT=$PORT ZERON_HARNESS=mock \
ZERON_MOCK_DELAY_MS=600 ZERON_MOCK_REPEAT=6 ZERON_MOCK_TABLE=1 ZERON_MOCK_CODE=1 \
  env ${ZERON_MOCK_CARDS:+ZERON_MOCK_CARDS="$ZERON_MOCK_CARDS"} "$ZERON" headless > "$WORK/engine.log" 2>&1 &
ENGINE=$!
for _ in $(seq 1 60); do grep -q "IPC server listening" "$WORK/engine.log" 2>/dev/null && break; sleep 0.5; done

node - "$PORT" "$PROJECT" <<'JS'
const [port, project] = process.argv.slice(2);
const ws = new WebSocket(`ws://127.0.0.1:${port}`);
let id = 0; const pending = new Map();
const call = (method, params) => new Promise((ok, err) => {
  const n = ++id; pending.set(n, { ok, err }); ws.send(JSON.stringify({ id: n, method, params }));
});
ws.onmessage = (m) => { const f = JSON.parse(m.data); const p = pending.get(f.id); if (!p) return;
  if (f.err) p.err(new Error(f.err)); else if (f.ok !== undefined) p.ok(f.ok); pending.delete(f.id); };
ws.onopen = async () => {
  const dev = await call("LocalDevice", {});
  await call("Mutate", { op: "createSpace", spaceId: "space-surya", deviceId: dev.deviceId, path: project, name: "surya", gitDetected: true });
  await call("Mutate", { op: "createChat", chatId: "chat-critic", spaceId: "space-surya", branch: "main" });
  await call("Mutate", { op: "renameChat", chatId: "chat-critic", title: "Wire the Tasks pane into the shell" });
  for (const [tid, title, status, extra] of [
    ["t-1", "Wire the Tasks pane into the shell", "running", { owner: "surya-tasks" }],
    ["t-2", "Theme tokens for cards and chips", "running", { owner: "surya-theme" }],
    ["t-3", "Browser pane behind a URL bar", "queued", {}],
    ["t-4", "Rename zeron to surya", "blocked", { notes: "last change before the RC" }],
  ]) await call("Mutate", { op: "createTask", taskId: tid, spaceId: "space-surya", title, status, ...extra });
  await call("QueueCommand", { chatId: "chat-critic", command: { kind: "run", messageId: "m-1",
    request: { prompt: "Mount the Tasks pane in the shell rail and make the composer pill float over the feed.",
      cwd: project, sandbox: "workspace-write", model: null, reasoning: null, modelOptions: {}, autoApprove: true, resume: null } } });
  console.log("seeded=1 space=1 chat=1 run=1 tasks=4"); ws.close(); process.exit(0);
};
ws.onerror = (e) => { console.error("seed failed", e.message || e); process.exit(1); };
JS

exec 9>/store/surya-display7.lock
flock 9
for mode in light dark; do
  for geom in 1440x900 1100x700; do
    UI="$WORK/ui-$mode-$geom"; mkdir -p "$UI"
    printf '{"appearance":"%s","openTabs":["chat-critic"],"lastSpaceId":"space-surya"}\n' "$mode" > "$UI/ui-settings.json"
    ZERON_DATA_DIR="$UI" ZERON_IPC_PORT=$PORT ZERON_WINDOW_SIZE=$geom DISPLAY=$DISPLAY_NO \
      "$ZERON" > "$WORK/app-$mode-$geom.log" 2>&1 &
    APP=$!
    sleep 12
    DISPLAY=$DISPLAY_NO xrefresh; sleep 3
    ffmpeg -loglevel error -y -f x11grab -video_size 1600x1000 -i "$DISPLAY_NO" -frames:v 1 "$OUT/shell-$mode-$geom.png"
    kill $APP 2>/dev/null || true; wait $APP 2>/dev/null || true
    panics=$(grep -c "panicked at" "$WORK/app-$mode-$geom.log" || true)
    echo "shot=1 bytes=$(stat -c %s "$OUT/shell-$mode-$geom.png") panics=$panics out=$OUT/shell-$mode-$geom.png"
  done
done
flock -u 9
