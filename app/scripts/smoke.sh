#!/usr/bin/env bash
# One headless engine, driven the way the app drives it, over the real
# websocket. Every feature merged tonight, in one run, on a fresh checkout.
#
#   a. remote auth   dial asked=2 accepted=1 rejected=1
#   b. files         writes asked=2 refused=1
#   c. tasks         created=1 seen=1
#   d. mail          sent=1 delivered=1 acked=1
#   e. cards         tool_uses=1 card_parts=1
#   f. states        needs_you asked=1 seen=1
#
# Bash and not a Rust integration test, on purpose: the point is to drive the
# ENGINE THE APP TALKS TO, from outside the workspace, through the binaries a
# user runs - `zeron headless`, `zeron mail`, `surya-mcp` - over a websocket
# nobody stubbed. A #[test] would link the engine into the test binary and
# prove a different thing. `crates/rpc/examples/rpc_probe.rs` already speaks
# the wire protocol from the shell, so no new Rust is needed.
#
# Two engines, on purpose. `surya-mcp` dials with `connect_ws` and carries no
# IPC token (crates/mcp/src/tasks.rs:130), so a token-enforcing engine locks
# the sidecar out. Step (a) gets its own token-enforcing engine and stops it;
# the rest run against the open loopback engine, which is how the app runs.
#
# Usage: app/scripts/smoke.sh
# Env:   SURYA_SMOKE_CARGO (default /store/surya-cargo here, `cargo` in CI)
#        SURYA_SMOKE_KEEP=1 to keep the temp dir and the logs

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
command -v cargo >/dev/null 2>&1 || PATH="$HOME/.cargo/bin:$PATH"
CARGO="${SURYA_SMOKE_CARGO:-$([ -x /store/surya-cargo ] && echo /store/surya-cargo || echo cargo)}"
WORK="$(mktemp -d /tmp/surya-smoke.XXXXXX)"
FAKE_CLAUDE="$ROOT/crates/harness/tests/fixtures/fake-claude.sh"
FAILURES=0
SKIPS=0
ENGINE_PID=""

# A free loopback port, asked of the kernel rather than guessed.
# `rpc_probe --stream` waits up to 30 s for an item, so a loop counted in
# iterations can sit for half an hour on a wedged stream. Every poll loop gets
# a wall-clock deadline instead.
POLL_SECONDS=45
deadline() { echo $(($(date +%s) + POLL_SECONDS)); }
before() { [ "$(date +%s)" -lt "$1" ]; }

free_port() { python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()'; }

# Every counter prints as a pair, and a mismatch is the exit code.
check() { # check <label> <expected> <actual>
  if [ "$2" = "$3" ]; then printf 'ok   %s %s\n' "$1" "$3"
  else printf 'FAIL %s expected=%s got=%s\n' "$1" "$2" "$3"; FAILURES=$((FAILURES + 1)); fi
}
# A skipped step is not a passed step. Counted, and the run says PARTIAL with
# its own exit code, so CI can refuse it while a human still sees what ran.
skip() { printf 'SKIP %s: %s\n' "$1" "$2"; SKIPS=$((SKIPS + 1)); }

# grep exits 1 when it matches nothing, and `set -o pipefail` turns that into a
# dead script rather than a counter of zero. Every count goes through here.
count() { printf '%s' "$2" | grep -c -- "$1" 2>/dev/null || true; }
# ...and a count clamped to one, for "did this happen at all".
once() { local n; n="$(count "$1" "$2")"; [ "${n:-0}" -ge 1 ] && echo 1 || echo 0; }

cleanup() {
  [ -n "$ENGINE_PID" ] && kill "$ENGINE_PID" 2>/dev/null || true
  if [ "${SURYA_SMOKE_KEEP:-0}" = "1" ]; then printf '\nkept: %s\n' "$WORK"
  else [ -n "$WORK" ] && rm -rf "$WORK"; fi
}
trap cleanup EXIT

probe() { # probe <method> <params-json> [--stream n]
  "$RPC_PROBE" "ws://127.0.0.1:$PORT" "$@" 2>>"$WORK/probe.err"
}

start_engine() { # start_engine <data-dir> [bind]
  local dir="$1" bind="${2:-}"
  PORT="$(free_port)"
  local args=()
  [ -n "$bind" ] && args+=(--bind "$bind")
  ZERON_DATA_DIR="$dir" ZERON_IPC_PORT="$PORT" \
    CLAUDE_CODE_EXECUTABLE="$FAKE_CLAUDE" \
    ZERON_WORKOS_CLIENT_ID="" \
    "$ZERON" headless "${args[@]}" >"$WORK/engine.log" 2>&1 &
  ENGINE_PID=$!
  for _ in $(seq 1 100); do
    if "$RPC_PROBE" "ws://127.0.0.1:$PORT" EngineReady '{}' >/dev/null 2>&1; then return 0; fi
    kill -0 "$ENGINE_PID" 2>/dev/null || { echo "engine died:"; tail -20 "$WORK/engine.log"; exit 1; }
    sleep 0.2
  done
  echo "engine never became ready:"; tail -20 "$WORK/engine.log"; exit 1
}

stop_engine() { [ -n "$ENGINE_PID" ] && kill "$ENGINE_PID" 2>/dev/null; wait "$ENGINE_PID" 2>/dev/null || true; ENGINE_PID=""; }

echo "== build =="
chmod +x "$FAKE_CLAUDE"
$CARGO build -p zeron -p surya-mcp >"$WORK/build.log" 2>&1 || { tail -30 "$WORK/build.log"; exit 1; }
$CARGO build -p zeron-rpc --example rpc_probe >>"$WORK/build.log" 2>&1 || { tail -30 "$WORK/build.log"; exit 1; }
TARGET="$(cargo metadata --format-version 1 --no-deps 2>/dev/null | python3 -c 'import json,sys;print(json.load(sys.stdin)["target_directory"])')"
ZERON="$TARGET/debug/zeron"
MCP="$TARGET/debug/surya-mcp"
RPC_PROBE="$TARGET/debug/examples/rpc_probe"
for bin in "$ZERON" "$MCP" "$RPC_PROBE"; do [ -x "$bin" ] || { echo "missing $bin"; exit 1; }; done
echo "built: $(basename "$ZERON") $(basename "$MCP") $(basename "$RPC_PROBE")"

# ---------------------------------------------------------------- a. auth
echo
echo "== a. remote auth (PR #3) =="
# A token-enforcing engine: an explicit token makes even a loopback socket
# enforce it (crates/engine/src/ipc.rs, enforces_token).
TOKEN="smoke-$(date +%s)"
PORT="$(free_port)"
ZERON_DATA_DIR="$WORK/auth" ZERON_IPC_PORT="$PORT" ZERON_IPC_TOKEN="$TOKEN" \
  ZERON_WORKOS_CLIENT_ID="" "$ZERON" headless >"$WORK/auth-engine.log" 2>&1 &
ENGINE_PID=$!
accepted=0; rejected=0
for _ in $(seq 1 100); do
  if ZERON_DATA_DIR="$WORK/auth" ZERON_IPC_PORT="$PORT" ZERON_IPC_TOKEN="$TOKEN" \
     "$ZERON" mail drain >/dev/null 2>&1; then accepted=1; break; fi
  kill -0 "$ENGINE_PID" 2>/dev/null || { echo "auth engine died:"; tail -20 "$WORK/auth-engine.log"; exit 1; }
  sleep 0.2
done
# The same socket, no token: refused at the handshake.
if "$RPC_PROBE" "ws://127.0.0.1:$PORT" LocalDevice '{}' >/dev/null 2>&1; then rejected=0; else rejected=1; fi
check "dial asked=2 accepted=" 1 "$accepted"
check "dial asked=2 rejected=" 1 "$rejected"
stop_engine

# ------------------------------------------------------- the shared engine
echo
echo "== engine for b-f (open loopback, as the app runs) =="
REPO="$WORK/checkout"
mkdir -p "$REPO/src"
git -C "$REPO" init -q
git -C "$REPO" config user.email smoke@surya
git -C "$REPO" config user.name smoke
printf 'fn main() {}\n' >"$REPO/src/main.rs"
git -C "$REPO" add -A
git -C "$REPO" commit -qm one
start_engine "$WORK/engine"
DEVICE="$(probe LocalDevice '{}' | python3 -c 'import json,sys;print(json.load(sys.stdin)["deviceId"])')"
echo "device: $DEVICE  port: $PORT"

# --------------------------------------------------------------- b. files
echo
echo "== b. files (PR #5) =="
probe Mutate "$(printf '{"op":"createSpace","spaceId":"sp-smoke","deviceId":"%s","path":"%s"}' "$DEVICE" "$REPO")" >/dev/null
TREE="$(probe FilesTree '{"spaceId":"sp-smoke","path":"","depth":2}')"
tree_files="$(printf '%s' "$TREE" | python3 -c 'import json,sys;print(json.dumps(json.load(sys.stdin)).count("main.rs"))')"
check "tree asked=1 found_main_rs=" 1 "$tree_files"
READ="$(probe FilesRead '{"spaceId":"sp-smoke","path":"src/main.rs"}')"
HASH="$(printf '%s' "$READ" | python3 -c 'import json,sys;print(json.load(sys.stdin)["hash"])')"
saved=0; refused=0
probe FilesWrite "$(printf '{"spaceId":"sp-smoke","path":"src/main.rs","content":"fn main() { /* smoke */ }\\n","expectedHash":"%s"}' "$HASH")" >/dev/null && saved=1
OUT="$(probe FilesWrite '{"spaceId":"sp-smoke","path":"src/main.rs","content":"stale\n","expectedHash":"deadbeef"}' || true)"
# FileWrite is a tagged enum: a stale hash comes back as kind=refused, with
# the bytes on disk, rather than as an RPC error.
refused="$(once '"kind":"refused"' "$OUT")"
check "writes asked=2 saved=" 1 "$saved"
check "writes asked=2 refused=" 1 "$refused"

# --------------------------------------------------------------- c. tasks
echo
echo "== c. tasks over the MCP tool (PR #6) =="
mcp_call() { # mcp_call <tool> <args-json>
  printf '%s\n%s\n' \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}' \
    "$(printf '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"%s","arguments":%s}}' "$1" "$2")" |
    SURYA_ENGINE_URL="ws://127.0.0.1:$PORT" SURYA_WORKSPACE=sp-smoke "$MCP" 2>>"$WORK/mcp.err"
}
# The tool's `workspace` argument IS the space id (crates/mcp/src/tasks.rs).
CREATED="$(mcp_call create_task '{"title":"smoke task","workspace":"sp-smoke"}' | tail -1)"
printf '%s\n' "$CREATED" >"$WORK/create_task.json"
created="$(once 'smoke task' "$CREATED")"
SEEN="$(probe WatchTasks '{"spaceId":"sp-smoke"}' --stream 1 || true)"
printf '%s\n' "$SEEN" >"$WORK/watch_tasks.json"
seen="$(once 'smoke task' "$SEEN")"
check "tasks created=" 1 "$created"
check "tasks seen=" 1 "$seen"

# ---------------------------------------------------------------- d. mail
echo
echo "== d. mail (PR #1) =="
for chat in smoke-a smoke-b; do
  probe Mutate "$(printf '{"op":"createChat","chatId":"%s","spaceId":"sp-smoke","deviceId":"%s"}' "$chat" "$DEVICE")" >/dev/null
  probe Mutate "$(printf '{"op":"renameChat","chatId":"%s","title":"%s"}' "$chat" "$chat")" >/dev/null
done
# One real turn each, so both agents have a run configuration a mail turn can
# borrow. `scenario:happy` is the fake-claude fixture's completed transcript.
run_turn() { # run_turn <chat> <prompt>
  probe QueueCommand "$(python3 -c 'import json,sys;print(json.dumps({"chatId":sys.argv[1],"command":{"kind":"run","request":{"prompt":sys.argv[2],"harness":"claude-code","cwd":sys.argv[3],"sandbox":"workspace-write","autoApprove":True,"attachments":[]},"messageId":sys.argv[4]}}))' "$1" "$2" "$REPO" "msg-$1-$RANDOM")" >/dev/null
}
run_turn smoke-a "scenario:happy warm up A"
run_turn smoke-b "scenario:happy warm up B"
# Wait on the engine's own status rather than a fixed sleep: a loaded runner
# was the flakiest thing in this script.
wait_idle() { # wait_idle <chat>...
  local dl; dl="$(deadline)"
  while before "$dl"; do
    local snap idle=1
    snap="$(probe WatchSessions '{}' --stream 1 || true)"
    for chat in "$@"; do
      printf '%s' "$snap" | python3 -c '
import json,sys
chat=sys.argv[1]
try: rows=json.load(sys.stdin)
except Exception: sys.exit(1)
rows=rows if isinstance(rows,list) else rows.get("sessions",[])
sys.exit(0 if any(r.get("chatId")==chat and r.get("status")=="idle" for r in rows) else 1)
' "$chat" || idle=0
    done
    [ "$idle" = "1" ] && return 0
    sleep 0.3
  done
  printf 'note: gave up waiting for %s to go idle after %ss\n' "$*" "$POLL_SECONDS"
}
wait_idle smoke-a smoke-b
# The BODY carries the scenario, so the mail turn itself completes.
SEND="$(ZERON_DATA_DIR="$WORK/engine" ZERON_IPC_PORT="$PORT" "$ZERON" mail send smoke-b "scenario:happy please check the diff" --from smoke-a)"
sent="$(printf '%s' "$SEND" | sed -n 's/^sent=\([0-9]*\).*/\1/p')"
sent="${sent:-0}"
delivered=0; acked=0; carried=0
DL="$(deadline)"
while before "$DL"; do
  ROWS="$(probe 'Mail.List' '{"agent":"smoke-b"}')"
  delivered="$(printf '%s' "$ROWS" | python3 -c 'import json,sys;print(sum(1 for m in json.load(sys.stdin)["messages"] if m["deliveredAt"]))')"
  acked="$(printf '%s' "$ROWS" | python3 -c 'import json,sys;print(sum(1 for m in json.load(sys.stdin)["messages"] if m["ackedAt"]))')"
  [ "$acked" = "1" ] && break
  sleep 0.5
done
MSGS="$(probe WatchDocMessages '{"chatId":"smoke-b"}' --stream 1 || true)"
carried="$(once '\[MAIL ' "$MSGS")"
check "mail sent=" 1 "$sent"
check "mail delivered=" 1 "$delivered"
check "mail acked=" 1 "$acked"
check "mail transcript_carries_envelope=" 1 "$carried"

# --------------------------------------------------------------- e. cards
echo
echo "== e. cards (PR #7) =="
probe Mutate "$(printf '{"op":"createChat","chatId":"smoke-card","spaceId":"sp-smoke","deviceId":"%s"}' "$DEVICE")" >/dev/null
probe Mutate '{"op":"renameChat","chatId":"smoke-card","title":"smoke-card"}' >/dev/null
# `card-raw` and not `card`: the short-form `card` scenario's input holds no
# A2UI, so its Card can only be lifted from the store surya-mcp writes, and
# this run never calls show_card through the sidecar. `card-raw` carries its
# A2UI in the tool input and is drawable without a store - which is what a
# transcript-level assertion can honestly prove.
run_turn smoke-card "scenario:card-raw draw me one"
card_parts=0; from_input=0
DL="$(deadline)"
while before "$DL"; do
  CARD="$(probe WatchDocMessages '{"chatId":"smoke-card"}' --stream 1 || true)"
  # A drawn card REPLACES its tool chip, so counting a leftover show_card
  # tool_use would be counting the failure case. What proves the storeless
  # path is the card carrying the surface id from the tool input.
  card_parts="$(once '"kind":"card"' "$CARD")"
  from_input="$(once '"surfaceId":"s1"' "$CARD")"
  if [ "$card_parts" = "1" ]; then break; fi
  sleep 0.5
done
printf '%s\n' "$CARD" >"$WORK/cards.json"
check "cards drawn=" 1 "$card_parts"
check "cards from_tool_input=" 1 "$from_input"

# -------------------------------------------------------------- f. states
echo
echo "== f. states (PR #9) =="
if ! grep -q 'scenario:permission' "$FAKE_CLAUDE"; then
  skip "needs_you" "this fake-claude has no scenario:permission (PR #42)"
else
  probe Mutate "$(printf '{"op":"createChat","chatId":"smoke-perm","spaceId":"sp-smoke","deviceId":"%s"}' "$DEVICE")" >/dev/null
  probe Mutate '{"op":"renameChat","chatId":"smoke-perm","title":"smoke-perm"}' >/dev/null
  # The scenario blocks on a can_use_tool request until the host answers, so
  # the queue must fill and then drain - one asked, one seen, one answered.
  run_turn smoke-perm "scenario:permission run the migration"
  asked=1; seen=0; answered=0; request_id=""
  DL="$(deadline)"
  while before "$DL"; do
    NEEDS="$(probe WatchNeedsYou '{}' --stream 1 || true)"
    request_id="$(printf '%s' "$NEEDS" | python3 -c '
import json,sys
try: rows=json.load(sys.stdin)
except Exception: sys.exit(0)
rows=rows if isinstance(rows,list) else rows.get("items",[])
for r in rows:
    if r.get("chatId")=="smoke-perm":
        print(r.get("id","")); break
' || true)"
    [ -n "$request_id" ] && { seen=1; break; }
    sleep 0.3
  done
  if [ "$seen" = "1" ]; then
    probe RespondPermission "$(printf '{"requestId":"%s","decision":"allow"}' "$request_id")" >/dev/null
    # Answered means the queue DRAINS: the card the user was asked to act on
    # is gone. The harness result string is not a transcript part, so looking
    # for it there would assert on something the doc never holds.
    DL="$(deadline)"
    while before "$DL"; do
      LEFT="$(probe WatchNeedsYou '{}' --stream 1 || true)"
      if [ "$(once 'smoke-perm' "$LEFT")" = "0" ]; then answered=1; break; fi
      sleep 0.3
    done
  fi
  check "needs_you asked=" 1 "$asked"
  check "needs_you seen=" 1 "$seen"
  check "needs_you answered=" 1 "$answered"
fi

echo
if [ "$FAILURES" -gt 0 ]; then
  echo "SMOKE FAILED: $FAILURES mismatched, $SKIPS skipped"
  exit 1
fi
if [ "$SKIPS" -gt 0 ]; then
  echo "SMOKE PARTIAL: every counter matched, but $SKIPS step(s) never ran"
  exit 2
fi
echo "SMOKE OK: every counter matched, 0 skipped"
exit 0
