#!/usr/bin/env bash
# Move surya's operational logs into a private repository, right before the
# public flip. Dry run by default; pass --apply to make changes.
#
# What moves: docs/handoff-*.md and everything under docs/acceptance/.
# These carry a tailnet host and IPs, owner machine paths, sleep and quota
# lines, and seat ids. docs/research/public-redaction-plan-2026-09-07.md
# explains why, and lists what stays.
#
# Deleting a file does not remove it from git history. This script does not
# rewrite history, and none is needed: the public-readiness research found no
# secret, key or credential ever committed, and the one token visible in a
# screenshot was checked and is not live.
#
# Idempotent. Run it twice and the second run reports nothing to do.

set -euo pipefail

PRIVATE_REPO="${SURYA_PRIVATE_REPO:-screamyx/surya-private}"
WORK="${SURYA_PRIVATE_CLONE:-/tmp/surya-private-clone}"
POINTER_FILE="docs/decisions.md"
APPLY=0

for arg in "$@"; do
  case "$arg" in
    --apply) APPLY=1 ;;
    -h|--help) sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

say() { printf '%s\n' "$*"; }
run() {
  if [ "$APPLY" -eq 1 ]; then
    "$@"
  else
    printf '  would run: %s\n' "$*"
  fi
}

cd "$(git rev-parse --show-toplevel)"

if [ -n "$(git status --porcelain)" ]; then
  echo "working tree is dirty; commit or stash first" >&2
  exit 1
fi

# 1. Collect the files, from the index, so untracked strays are never moved.
mapfile -t LOGS < <(git ls-files 'docs/handoff-*.md' 'docs/acceptance/*' | sort)

if [ "${#LOGS[@]}" -eq 0 ]; then
  say "Nothing to move: no handoff or acceptance files are tracked."
  say "Already done, or the paths changed."
  exit 0
fi

say "Private repository: $PRIVATE_REPO"
say "Files to move: ${#LOGS[@]}"
for f in "${LOGS[@]}"; do say "  $f"; done
say ""

if [ "$APPLY" -eq 0 ]; then
  say "DRY RUN. Nothing was changed. Pass --apply to do it."
  say ""
fi

# 2. Create the private repository if it is not there yet.
if gh-axi api "/repos/$PRIVATE_REPO" >/dev/null 2>&1; then
  say "Step 1: $PRIVATE_REPO already exists, reusing it."
else
  say "Step 1: create $PRIVATE_REPO, private."
  run gh-axi api POST /user/repos \
    --field "name=${PRIVATE_REPO#*/}" \
    --field private=true \
    --field "description=Operational logs for surya. Private on purpose."
fi

# 3. Clone it, or reuse an existing clone.
say "Step 2: clone into $WORK."
if [ -d "$WORK/.git" ]; then
  run git -C "$WORK" fetch origin
  run git -C "$WORK" checkout main
  run git -C "$WORK" pull --rebase
else
  run git clone "https://github.com/$PRIVATE_REPO.git" "$WORK"
fi

# 4. Copy under logs/, preserving the path below docs/.
say "Step 3: copy the files under logs/ in the private clone."
for f in "${LOGS[@]}"; do
  dest="$WORK/logs/${f#docs/}"
  run mkdir -p "$(dirname "$dest")"
  run cp "$f" "$dest"
done

say "Step 4: commit and push in the private clone."
run git -C "$WORK" add logs
if [ "$APPLY" -eq 1 ]; then
  if git -C "$WORK" diff --cached --quiet; then
    say "  nothing new to commit in $PRIVATE_REPO"
  else
    git -C "$WORK" -c user.name="$(git config user.name)" \
        -c user.email="$(git config user.email)" \
        commit -q -m "logs: import surya operational logs"
    git -C "$WORK" push -q origin HEAD
  fi
else
  say "  would run: git -C $WORK commit -m 'logs: import surya operational logs'"
  say "  would run: git -C $WORK push origin HEAD"
fi

# 5. Verify every file landed before deleting anything here.
say "Step 5: verify each file is present in the private clone before deleting."
missing=0
for f in "${LOGS[@]}"; do
  dest="$WORK/logs/${f#docs/}"
  if [ "$APPLY" -eq 1 ]; then
    if [ ! -f "$dest" ] || ! cmp -s "$f" "$dest"; then
      echo "  MISSING or DIFFERENT in the private clone: $dest" >&2
      missing=1
    fi
  else
    say "  would compare: $f against $dest"
  fi
done
if [ "$missing" -eq 1 ]; then
  echo "refusing to delete anything: the copy is incomplete" >&2
  exit 1
fi

# 6. Delete from surya, in one commit, with the pointer.
say "Step 6: delete them from surya and leave a pointer."
run git rm -q -- "${LOGS[@]}"

POINTER="Operational logs (the handoff log and the acceptance records) live in the private repository $PRIVATE_REPO, under logs/."
if ! grep -qF "$POINTER" "$POINTER_FILE" 2>/dev/null; then
  if [ "$APPLY" -eq 1 ]; then
    printf '\n%s\n' "$POINTER" >> "$POINTER_FILE"
    git add "$POINTER_FILE"
  else
    say "  would append the pointer line to $POINTER_FILE"
  fi
else
  say "  pointer already in $POINTER_FILE"
fi

if [ "$APPLY" -eq 1 ]; then
  git commit -q -m "docs: move the operational logs to $PRIVATE_REPO"
  say ""
  say "Done. One commit here, one in $PRIVATE_REPO."
  say "The files stay in this repository's history. That is expected and no rewrite is planned."
else
  say "  would run: git commit -m 'docs: move the operational logs to $PRIVATE_REPO'"
  say ""
  say "DRY RUN finished. Nothing changed."
fi
