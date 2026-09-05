#!/usr/bin/env bash
# Prove an agent drives the browser pane over the real route: a headless
# engine with the mock harness (`ZERON_MOCK_BROWSER=<url>`) runs the real
# surya-mcp binary, whose browser_* tools go engine -> app pane -> CEF. The
# app runs on the display with the Browser tab open, attached to that
# engine, so the tools act on the page the owner would be looking at.
#
#   CARGO_TARGET_DIR=... CEF_PATH=... DISPLAY=:7 app/scripts/browser-agent-proof.sh [seconds]
#
# Takes the display lock itself around the app's window time (launch, wait,
# xrefresh, grab), never around a build; do not wrap it in another flock.
# Needs: zeron + zeron-browser-helper built with --features browser, and
# surya-mcp, in the same target dir; node (the seed); python3 (the page
# server); ffmpeg (the grab).
#
# Prints, at the end:
#   proof: tools_called=4 ok=4 card=1 attached=1 screenshot=<png> frames=<n>
set -u
WAIT="${1:-180}"
HERE="$(cd "$(dirname "$0")" && pwd)"
TARGET="${CARGO_TARGET_DIR:-$(cd "$HERE/.." && pwd)/target}"
BIN="$TARGET/debug/zeron"
MCP="$TARGET/debug/surya-mcp"
OUT="${PROOF_OUT:-/tmp/surya-browser-agent-proof}"
rm -rf "$OUT"; mkdir -p "$OUT/www"
[ -x "$BIN" ] || { echo "no binary at $BIN (build -p zeron --features browser)"; exit 2; }
[ -x "$MCP" ] || { echo "no surya-mcp at $MCP (build -p surya-mcp)"; exit 2; }
[ -f "$TARGET/debug/libcef.so" ] || { echo "no libcef.so next to the binary"; exit 2; }

cp "$HERE/browser-agent-page.html" "$OUT/www/index.html"
cp "$HERE/browser-agent-page-two.html" "$OUT/www/two.html"
for _ in 1 2 3 4 5 6 7 8 9 10; do
  HTTP_PORT=$(( 30000 + RANDOM % 20000 ))
  ss -Hltn 2>/dev/null | grep -q ":$HTTP_PORT " || break
done
python3 -m http.server --bind 127.0.0.1 "$HTTP_PORT" --directory "$OUT/www" > "$OUT/http.log" 2>&1 &
HTTP=$!
for _ in 1 2 3 4 5 6 7 8 9 10; do
  PORT=$(( 20000 + RANDOM % 10000 ))
  ss -Hltn 2>/dev/null | grep -q ":$PORT " || break
done
URL="http://127.0.0.1:$HTTP_PORT/index.html"
echo "page $URL engine port $PORT"

# 1. The engine, headless, with the mock harness driving surya-mcp.
ZERON_DATA_DIR="$OUT/engine" ZERON_IPC_PORT=$PORT ZERON_HARNESS=mock \
  ZERON_MOCK_BROWSER="$URL" SURYA_MCP_EXECUTABLE="$MCP" RUST_LOG=info \
  "$BIN" headless > "$OUT/engine.log" 2>&1 &
ENGINE=$!
cleanup() {
  kill "$APP" 2>/dev/null; kill "$ENGINE" 2>/dev/null; kill "$HTTP" 2>/dev/null
  sleep 1; kill -9 "$APP" "$ENGINE" 2>/dev/null
}
APP=""
trap cleanup EXIT
for _ in $(seq 1 180); do grep -q "IPC server listening" "$OUT/engine.log" 2>/dev/null && break; sleep 0.5; done
grep -q "IPC server listening" "$OUT/engine.log" || { echo "engine did not listen in 90 s"; tail -20 "$OUT/engine.log"; exit 4; }

# 2. A space and a chat, so the app has a transcript to show.
node - "$PORT" "$OUT/www" seed <<'JS'
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
  await call("Mutate", { op: "createSpace", spaceId: "space-probe", deviceId: dev.deviceId, path: project, name: "probe", gitDetected: false });
  await call("Mutate", { op: "createChat", chatId: "chat-probe", spaceId: "space-probe", branch: "main" });
  await call("Mutate", { op: "renameChat", chatId: "chat-probe", title: "Read the probe page and follow its link" });
  console.log("seeded=1 space=1 chat=1"); ws.close(); process.exit(0);
};
ws.onerror = (e) => { console.error("seed failed", e.message || e); process.exit(1); };
JS

# 3. The app on the display, attached to the engine, Browser tab open.
# The display lock only around the window time (launch, wait, grab).
W=1600; H=1000
exec 9>/store/surya-display7.lock
flock 9
ZERON_DATA_DIR="$OUT/ui" ZERON_IPC_PORT=$PORT ZERON_OPEN_PANE=browser \
  SURYA_BROWSER_URL="about:blank" SURYA_CEF_CACHE="$OUT/ui/cef" \
  SURYA_BROWSER_DUMP="$OUT/frames" RUST_LOG=info ZERON_WINDOW_SIZE=${W}x${H} \
  "$BIN" > "$OUT/zeron.log" 2>&1 &
APP=$!
sleep 10; command -v xrefresh >/dev/null && xrefresh
for _ in $(seq 1 "$WAIT"); do grep -q "browser-agent: attached" "$OUT/zeron.log" 2>/dev/null && break; sleep 1; done
ATTACHED=$(grep -c "browser-agent: attached" "$OUT/zeron.log"); ATTACHED=${ATTACHED:-0}
echo "attached=$ATTACHED after the wait"
# Let CEF finish its first (blank) load before the agent's turn.
for _ in $(seq 1 60); do grep -q "browser: load_end" "$OUT/zeron.log" 2>/dev/null && break; sleep 1; done

# 4. The agent's turn: one run on the chat; the mock calls the four tools.
node - "$PORT" "$OUT/www" run <<'JS'
const [port, project] = process.argv.slice(2);
const ws = new WebSocket(`ws://127.0.0.1:${port}`);
let id = 0; const pending = new Map();
const call = (method, params) => new Promise((ok, err) => {
  const n = ++id; pending.set(n, { ok, err }); ws.send(JSON.stringify({ id: n, method, params }));
});
ws.onmessage = (m) => { const f = JSON.parse(m.data); const p = pending.get(f.id); if (!p) return;
  if (f.err) p.err(new Error(f.err)); else if (f.ok !== undefined) p.ok(f.ok); pending.delete(f.id); };
ws.onopen = async () => {
  await call("QueueCommand", { chatId: "chat-probe", command: { kind: "run", messageId: "m-1",
    request: { prompt: "Open the probe page, read it, follow its link, and show me what you see.",
      cwd: project, sandbox: "workspace-write", model: null, reasoning: null, modelOptions: {}, autoApprove: true, resume: null } } });
  console.log("run=1"); ws.close(); process.exit(0);
};
ws.onerror = (e) => { console.error("run failed", e.message || e); process.exit(1); };
JS
for _ in $(seq 1 120); do grep -q "^browser-mock: tools_called=" "$OUT/engine.log" 2>/dev/null && break; sleep 1; done
sleep 4; command -v xrefresh >/dev/null && xrefresh; sleep 3
SHOT="$OUT/window.png"
ffmpeg -loglevel error -y -f x11grab -video_size ${W}x${H} -i "${DISPLAY:-:7}" -frames:v 1 "$SHOT"
kill "$APP" 2>/dev/null; sleep 1; kill -9 "$APP" 2>/dev/null; APP=""
flock -u 9

echo "--- engine: mock and broker"
grep -E "^browser-mock:" "$OUT/engine.log" | tail -6
echo "--- app: agent ops"
grep -E "^browser-agent:|^devtools:|^browser: (created|load_end|address)" "$OUT/zeron.log" | tail -12
LINE=$(grep -E "^browser-mock: tools_called=" "$OUT/engine.log" | tail -1)
CALLED=$(sed -n 's/.*tools_called=\([0-9]*\).*/\1/p' <<<"$LINE"); CALLED=${CALLED:-0}
OKS=$(sed -n 's/.*tools_called=[0-9]* ok=\([0-9]*\).*/\1/p' <<<"$LINE"); OKS=${OKS:-0}
CARD=$(grep -c "screenshot shown in the transcript as card" "$OUT/engine.log"); CARD=${CARD:-0}
FRAMES=$(ls "$OUT/frames" 2>/dev/null | wc -l)
echo "proof: tools_called=$CALLED ok=$OKS card=$CARD attached=$ATTACHED screenshot=$SHOT ($(stat -c %s "$SHOT" 2>/dev/null || echo 0) bytes) frames=$FRAMES"
[ "$CALLED" -eq 4 ] && [ "$OKS" -eq 4 ] && [ "$ATTACHED" -ge 1 ]
