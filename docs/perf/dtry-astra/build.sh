#!/usr/bin/env bash
# Keep the shared Linux cargo slot held until the Windows build has exited.
set -eu
shim=/store/agent-worktrees/surya-browser-astra/dtry-cargo-shim
mkdir -p "$shim"
cat > "$shim/cargo" <<'SHIM'
#!/usr/bin/env bash
exec ssh -o BatchMode=yes -o ServerAliveInterval=15 -o ServerAliveCountMax=20 dtry 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File E:\surya-astra\docs\perf\dtry-astra\build.ps1 -Seed'
SHIM
chmod +x "$shim/cargo"
export PATH="$shim:$PATH"
export CARGO_BUILD_JOBS=3
exec /store/surya-cargo build --release -p zeron --features browser
