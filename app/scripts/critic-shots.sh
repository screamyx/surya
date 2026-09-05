#!/usr/bin/env bash
# Design-critic shot rig: a throwaway engine with the mock harness, a real
# project (this clone), one chat streaming a paced mock run, then the headed
# shell on the shared headless Xorg (:7), light and dark, two window sizes.
#
#   scripts/critic-shots.sh <out_dir> [path/to/zeron] [project_dir]
# Writes <out_dir>/shell-{light,dark}-{1440x900,1100x700}.png and prints one
# `shot=1 bytes=N` line per frame.
#
# SURYA_XTEST_PYTHON: the python that drives XTEST clicks, for the frames that
# need one (SHOT_CLICK). It must have python-xlib, which the system python on
# this box does NOT: without it every click dies with ModuleNotFoundError. A
# working venv lives on /store, so it survives a /tmp clear:
#
#   SURYA_XTEST_PYTHON=/store/agent-worktrees/surya-browser-ui/.xtest-venv/bin/python
#
# A click that fails now ends the run with exit 5 and writes no png, rather
# than returning a green-looking frame of a screen nobody drove. A grab that
# comes back black ends it with exit 6, on the same reasoning. Both need the
# thing they check to be checkable: without Pillow the black test cannot run
# and the frame is reported UNVERIFIED.
set -euo pipefail
OUT=${1:?out dir}; ZERON=${2:-${CARGO_TARGET_DIR:-target}/debug/zeron}
PROJECT=${3:-$(cd "$(dirname "$0")/../.." && pwd)}
DISPLAY_NO=${SURYA_SHOT_DISPLAY:-:7}
[ -x "$ZERON" ] || { echo "no zeron binary at $ZERON" >&2; exit 2; }
mkdir -p "$OUT"
WORK=$(mktemp -d /tmp/critic-shots-XXXXXX)
PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
trap 'kill $ENGINE ${APP:-} 2>/dev/null || true; cp "$WORK"/app-*.log "$OUT"/ 2>/dev/null; rm -rf "$WORK"' EXIT

# Mock harness, paced so the chat is still Running when the frame is taken;
# tables and code blocks so the transcript shows every row kind.
ZERON_DATA_DIR="$WORK/engine" ZERON_IPC_PORT=$PORT ZERON_HARNESS=mock \
ZERON_MOCK_DELAY_MS=600 ZERON_MOCK_REPEAT=6 ZERON_MOCK_TABLE=1 ZERON_MOCK_CODE=1 \
  env ${ZERON_MOCK_CARDS:+ZERON_MOCK_CARDS="$ZERON_MOCK_CARDS"} "$ZERON" headless > "$WORK/engine.log" 2>&1 &
ENGINE=$!
# Assembly takes ~13 s on a loaded box (mail ingress + stores); give it 90 s
# and fail loudly with the engine log instead of letting the seed step guess.
for _ in $(seq 1 180); do grep -q "IPC server listening" "$WORK/engine.log" 2>/dev/null && break; sleep 0.5; done
if ! grep -q "IPC server listening" "$WORK/engine.log" 2>/dev/null; then
  echo "engine did not listen within 90 s (frames=0); engine.log tail:"; tail -20 "$WORK/engine.log"; exit 4
fi

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

# SURYA_SHOT_PANES="files tasks" (default): also open each right-pane surface
# through the shell's `ZERON_OPEN_PANE` knob and take shell-<pane>-<mode>.png at
# 1440x900. Add `browser` (needs a `--features browser` build): the Browser
# surface on https://example.com; CEF needs ~25 s and two Expose kicks before
# its first paint. SURYA_SHOT_PANES="" skips the pane frames.
# `ZERON_OPEN_BROWSER` is gone since PR #30: only `ZERON_OPEN_PANE` opens a pane.
GEOMS="${SURYA_SHOT_GEOMS-1440x900 1100x700}" # "" skips the plain shell frames
shoot() { # shoot <name> <mode> <geom> <extra env...>
  local name=$1 mode=$2 geom=$3; shift 3
  local UI="$WORK/ui-$name"; mkdir -p "$UI"
  printf '{"appearance":"%s","openTabs":["chat-critic"],"lastSpaceId":"space-surya"}\n' "$mode" > "$UI/ui-settings.json"
  env ZERON_DATA_DIR="$UI" ZERON_IPC_PORT=$PORT ZERON_WINDOW_SIZE=$geom DISPLAY=$DISPLAY_NO "$@" \
    "$ZERON" > "$WORK/app-$name.log" 2>&1 &
  APP=$!
  sleep "${SHOT_WAIT:-24}"
  DISPLAY=$DISPLAY_NO xrefresh; sleep 3
  if [ "${SHOT_WAIT:-24}" -gt 12 ]; then DISPLAY=$DISPLAY_NO xrefresh; sleep 3; fi
  # SHOT_CLICK="x,y": one XTEST click at that root position after the settle
  # (scripts/x7-click.py, run with the SURYA_XTEST_PYTHON from the header),
  # then SHOT_CLICK_WAIT s more before the grab. The 1440x900 window sits at
  # +80+50 on the 1600x1000 display, so the titlebar globe is at 129,69.
  if [ -n "${SHOT_CLICK:-}" ]; then
    # A click that did not happen must not produce a screenshot. x7-click.py
    # exits non-zero when python-xlib is missing, and a rig that shrugged and
    # shot anyway handed back `shot=1 panics=0` for a screen nobody touched
    # (23:42, 2026-09-05). So this ends the run instead.
    if ! "${SURYA_XTEST_PYTHON:-python3}" "$(dirname "$0")/x7-click.py" \
        "$DISPLAY_NO" "${SHOT_CLICK%,*}" "${SHOT_CLICK#*,}"; then
      echo "shot=0 frames=0 FAILED: the click at $SHOT_CLICK did not run, so $name would be a frame of an undriven screen" >&2
      echo "set SURYA_XTEST_PYTHON to a python with python-xlib (see the header)" >&2
      kill $APP 2>/dev/null || true
      exit 5
    fi
    sleep "${SHOT_CLICK_WAIT:-30}"; DISPLAY=$DISPLAY_NO xrefresh; sleep 3
  fi
  ffmpeg -loglevel error -y -f x11grab -video_size 1600x1000 -i "$DISPLAY_NO" -frames:v 1 "$OUT/$name.png"
  kill $APP 2>/dev/null || true; wait $APP 2>/dev/null || true
  local panics; panics=$(grep -c "panicked at" "$WORK/app-$name.log" || true)
  # A grab with <= 2 colours is a black display, not a frame (10:44 incident).
  # Counting them needs Pillow. With it, a black grab is fatal and no png
  # survives; without it, say so loudly rather than imply the frame was
  # checked, because "colours=?" next to "shot=1" reads as a pass.
  local colours; colours=$(python3 -c "from PIL import Image; im=Image.open('$OUT/$name.png').convert('RGB'); print(len(im.getcolors(1<<20) or [0]*3000))" 2>/dev/null || echo "?")
  if [ "$colours" = "?" ]; then
    echo "WARN: no Pillow here, so a black display cannot be told from a frame; $name is UNVERIFIED" >&2
  elif [ "$colours" -le 2 ]; then
    rm -f "$OUT/$name.png"
    echo "shot=0 frames=0 FAILED: $name is a black display, not a frame (colours=$colours)" >&2
    echo "the X server mapped the window but never presented; see the display notes in AGENTS.md" >&2
    exit 6
  fi
  echo "shot=1 bytes=$(stat -c %s "$OUT/$name.png") colours=$colours panics=$panics out=$OUT/$name.png"
  case "$name" in *browser*) grep -E "browser: (on_paint #1|load_end|LOAD ERROR)" "$WORK/app-$name.log" | head -3 || true;; esac
  grep -E "ZERON_OPEN_PANE" "$WORK/app-$name.log" | head -1 || true
}

# The display lock wraps only launch + xrefresh + grab (rule 10:12). Wait a
# bounded time and say who holds it, so a queued rig never looks hung.
exec 9>/store/surya-display7.lock
if ! flock -n 9; then
  echo "waiting for /store/surya-display7.lock (held by: $(ps -eo user,args | grep '[f]lock /store/surya-display7.lock' | head -1 | cut -c1-100))"
  flock -w "${SHOT_LOCK_WAIT:-300}" 9 || { echo "display lock still held after ${SHOT_LOCK_WAIT:-300}s, giving up (frames=0)"; exit 3; }
fi
for mode in light dark; do
  for geom in $GEOMS; do
    shoot "shell-$mode-$geom" "$mode" "$geom"
  done
done
for pane in ${SURYA_SHOT_PANES-files tasks}; do
  for mode in light dark; do
    case "$pane" in
      browser)
        # Builds before #62 ignore the launch knob (round 3, B1): set
        # SURYA_SHOT_BROWSER_CLICK=129,69 to open the pane from the globe instead.
        SHOT_WAIT="${SHOT_WAIT:-28}" SHOT_CLICK="${SURYA_SHOT_BROWSER_CLICK:-}" \
          shoot "shell-browser-$mode" "$mode" 1440x900 \
          ZERON_OPEN_PANE=browser SURYA_BROWSER_URL="${SURYA_BROWSER_URL:-https://example.com}" \
          SURYA_CEF_CACHE="$WORK/cef-$mode" RUST_LOG=info ;;
      *) shoot "shell-$pane-$mode" "$mode" 1440x900 ZERON_OPEN_PANE="$pane" ;;
    esac
  done
done
flock -u 9
