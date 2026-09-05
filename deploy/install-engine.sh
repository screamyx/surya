#!/usr/bin/env bash
# Install the surya engine on this Linux box as a user service and print what
# the Windows app needs: the dial string and the token.
#
#   deploy/install-engine.sh                 # bind to this box's tailnet IPv4
#   deploy/install-engine.sh --bind 0.0.0.0  # every interface (LAN / VPN)
#   deploy/install-engine.sh --no-build      # reuse target/release/zeron
#
# Re-running is safe: it rebuilds, reinstalls the binary, rewrites the env
# file and the unit, restarts the service, and keeps the existing token.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="$ROOT/app"
BIND=""
PORT="${SURYA_PORT:-27700}"
DATA_DIR="${SURYA_DATA_DIR:-$HOME/.local/share/surya-engine}"
CARGO="${SURYA_CARGO:-cargo}"
BUILD=1

usage() { sed -n '2,12p' "$0"; exit "${1:-0}"; }
while [ $# -gt 0 ]; do
  case "$1" in
    --bind) BIND="$2"; shift 2 ;;
    --port) PORT="$2"; shift 2 ;;
    --data-dir) DATA_DIR="$2"; shift 2 ;;
    --cargo) CARGO="$2"; shift 2 ;;
    --no-build) BUILD=0; shift ;;
    -h|--help) usage ;;
    *) echo "unknown option: $1" >&2; usage 1 ;;
  esac
done

say() { printf '\n== %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

# 1. Bind address: the tailnet IPv4 unless told otherwise.
if [ -z "$BIND" ]; then
  if command -v tailscale >/dev/null 2>&1; then
    BIND="$(tailscale ip -4 2>/dev/null | head -n1 || true)"
  fi
  [ -n "$BIND" ] || die "no tailnet address found; pass --bind <ip> (0.0.0.0 for every interface)"
fi
case "$BIND" in
  127.*|::1) die "--bind $BIND is loopback; the Windows app cannot reach it. Use the tailnet IP or 0.0.0.0" ;;
esac

# 2. Build the release binary.
BIN_DIR="$HOME/.local/bin"
TARGET_DIR="${CARGO_TARGET_DIR:-$APP/target}"
if [ "$BUILD" = 1 ]; then
  command -v "${CARGO%% *}" >/dev/null 2>&1 || die "cargo not found; install Rust from https://rustup.rs then re-run"
  say "building the engine (release, this takes a while the first time)"
  (cd "$APP" && $CARGO build --release -p zeron)
fi
[ -x "$TARGET_DIR/release/zeron" ] || die "no binary at $TARGET_DIR/release/zeron (run without --no-build)"

# 3. Install the binary and the helper.
say "installing to $BIN_DIR"
install -d "$BIN_DIR"
install -m755 "$TARGET_DIR/release/zeron" "$BIN_DIR/zeron.new"
mv -f "$BIN_DIR/zeron.new" "$BIN_DIR/zeron"
cat > "$BIN_DIR/surya-engine" <<'HELPER'
#!/usr/bin/env bash
# surya-engine status|token|logs|restart|stop|start - manage the engine service.
set -euo pipefail
ENV_FILE="$HOME/.config/surya/engine.env"
[ -f "$ENV_FILE" ] || { echo "not installed: run deploy/install-engine.sh" >&2; exit 1; }
set -a; . "$ENV_FILE"; set +a
case "${1:-status}" in
  status) systemctl --user --no-pager status surya-engine || true; "$HOME/.local/bin/zeron" status || true ;;
  token) cat "$ZERON_DATA_DIR/ipc-token" 2>/dev/null || { echo "no token yet: is the service running?" >&2; exit 1; } ;;
  dial) echo "ws://$ZERON_BIND:$ZERON_IPC_PORT" ;;
  logs) journalctl --user -u surya-engine -n "${2:-50}" --no-pager ;;
  restart|stop|start) systemctl --user "$1" surya-engine ;;
  *) echo "usage: surya-engine status|token|dial|logs [n]|restart|stop|start" >&2; exit 1 ;;
esac
HELPER
chmod 755 "$BIN_DIR/surya-engine"

# 4. Settings the service reads. PATH must include the agent CLIs (claude).
CONF_DIR="$HOME/.config/surya"
install -d -m700 "$CONF_DIR" "$DATA_DIR"
ENV_FILE="$CONF_DIR/engine.env"
umask 077
cat > "$ENV_FILE" <<ENVF
# surya engine settings, read by ~/.config/systemd/user/surya-engine.service
ZERON_BIND=$BIND
ZERON_IPC_PORT=$PORT
ZERON_DATA_DIR=$DATA_DIR
PATH=$HOME/.local/bin:$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin
RUST_LOG=info
ENVF
umask 022
say "wrote $ENV_FILE"

# 5. The port must be free unless our own service holds it.
if command -v ss >/dev/null 2>&1; then
  holder="$(ss -Hltnp "sport = :$PORT" 2>/dev/null | grep -o 'pid=[0-9]*' | head -n1 | cut -d= -f2 || true)"
  if [ -n "$holder" ]; then
    unit_pid="$(systemctl --user show -p MainPID --value surya-engine 2>/dev/null || echo 0)"
    if [ "$holder" != "$unit_pid" ]; then
      die "port $PORT is held by pid $holder (not the surya-engine service); stop it or pass --port <other>"
    fi
  fi
fi

# 6. The user service.
UNIT_DIR="$HOME/.config/systemd/user"
install -d "$UNIT_DIR"
install -m644 "$ROOT/deploy/surya-engine.service" "$UNIT_DIR/surya-engine.service"
systemctl --user daemon-reload
systemctl --user enable surya-engine >/dev/null 2>&1 || true
say "starting the service"
systemctl --user restart surya-engine
if ! loginctl show-user "$USER" -p Linger 2>/dev/null | grep -q 'Linger=yes'; then
  loginctl enable-linger "$USER" 2>/dev/null \
    || echo "note: run 'sudo loginctl enable-linger $USER' so the engine keeps running after you log out"
fi

# 7. Wait for the socket, then print what the Windows app needs.
for _ in $(seq 1 30); do
  if [ -s "$DATA_DIR/ipc-token" ] && (exec 3<>"/dev/tcp/$BIND/$PORT") 2>/dev/null; then break; fi
  sleep 1
done
if ! (exec 3<>"/dev/tcp/$BIND/$PORT") 2>/dev/null; then
  systemctl --user --no-pager status surya-engine || true
  die "the engine is not listening on $BIND:$PORT; see: surya-engine logs"
fi
TOKEN="$(cat "$DATA_DIR/ipc-token")"
say "the engine is running"
cat <<DONE
  Server (host):  $BIND
  Port:           $PORT
  Token:          $TOKEN
  Dial string:    ws://$BIND:$PORT

In the Windows app: Settings -> Servers -> Add server, paste the three values, Connect.
Later:  surya-engine status | token | logs | restart
DONE
