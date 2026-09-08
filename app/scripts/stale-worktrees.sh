#!/usr/bin/env bash
# Which worktrees can go. Squash merge rewrites every branch commit into a
# new commit on main, so `git branch --merged` and `git branch -r --contains`
# both say "not merged" about a branch whose PR landed hours ago. The only
# record that survives a squash is GitHub's own: the PR state, and the
# `refs/pull/N/head` ref that still points at the branch's last commit.
#
# This script asks that record for every worktree and prints one verdict per
# row. It never removes anything unless you pass --remove, and then it only
# removes rows marked REMOVE that this uid owns and that hold no local edits.
#
#   REMOVE     head is the last commit of a merged PR, tree is clean
#   ON-MAIN    detached at a commit main already has, tree is clean
#   EDITS      would be REMOVE or ON-MAIN but has modified or untracked files
#   OPEN-PR    head is the last commit of an open PR
#   CLOSED-PR  head is the last commit of a PR that closed without merging
#   PUSHED     head is on a remote branch that has no PR
#   UNPUSHED   head is on no remote ref and no PR; the work exists only here
#   OTHER-UID  owned by another user; state printed, tree not inspected
#
# `refs/pull/*/head` is force-fetched on every run. A cached pr/N ref can sit
# one commit behind the real PR head and call a merged worktree unmerged.
#
# Usage: app/scripts/stale-worktrees.sh [--remove] [--repo owner/name]
# Needs: git, gh-axi. Run from any checkout of the repo.

set -euo pipefail

remove=0
repo=""
while [ $# -gt 0 ]; do
    case "$1" in
        --remove) remove=1 ;;
        --repo) repo="$2"; shift ;;
        -h|--help) sed -n '2,26p' "$0"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

common=$(git rev-parse --path-format=absolute --git-common-dir)
main_wt=$(dirname "$common")
me=$(id -un)

if [ -z "$repo" ]; then
    url=$(git -C "$main_wt" remote get-url origin)
    url=${url%.git}; url=${url%/}
    repo=$(echo "$url" | tr : / | awk -F/ '{print $(NF-1)"/"$NF}')
fi

git -C "$main_wt" fetch -q origin main '+refs/pull/*/head:refs/remotes/pr/*'

# PR number -> merged | closed | open, from GitHub, not from git. A row is
# `  N,title,state,author,draft,review`; the title may hold commas, so the
# state is read from the end.
declare -A pr_state
while IFS= read -r line; do
    n=${line%%,*}
    st=$(echo "$line" | awk -F, '{print $(NF-3)}')
    pr_state[$n]=$st
done < <(gh-axi pr list --repo "$repo" --state all --limit 1000 \
         | grep -E '^ +[0-9]+,' | sed -E 's/^ +//')
if [ "${#pr_state[@]}" = 0 ]; then
    echo "no pull requests read from $repo; check gh-axi and --repo" >&2
    exit 1
fi

# One worktree per block in the porcelain listing.
rows=()
path=""; head=""; branch=""
flush() {
    [ -z "$path" ] && return
    [ "$path" = "$main_wt" ] && { path=""; return; }
    rows+=("$path|$head|$branch")
    path=""; head=""; branch=""
}
while IFS= read -r line; do
    case "$line" in
        "worktree "*) flush; path=${line#worktree } ;;
        "HEAD "*) head=${line#HEAD } ;;
        "branch "*) branch=${line#branch refs/heads/} ;;
        "detached") branch="(detached)" ;;
        "") ;;
    esac
done < <(git -C "$main_wt" worktree list --porcelain; echo)
flush

# Which PR refs and which remote branches contain a commit. main is checked
# first: a commit main already has is an ancestor of every PR branch cut
# after it, so the PR list would name every one of them.
prs_containing() {
    git -C "$main_wt" for-each-ref --format='%(refname:short)' \
        --contains "$1" refs/remotes/pr 2>/dev/null | sed 's#^pr/##' | sort -n
}
on_main() {
    git -C "$main_wt" merge-base --is-ancestor "$1" origin/main 2>/dev/null
}
remote_branches() {
    git -C "$main_wt" for-each-ref --format='%(refname:short)' \
        --contains "$1" refs/remotes/origin 2>/dev/null \
        | grep -vE '^origin/(main|HEAD)$' | paste -sd ' ' -
}
tree_edits() {
    # "clean", "modified", "untracked", or "unreadable" for another uid.
    local out
    if ! out=$(git -C "$1" status --porcelain 2>/dev/null); then
        echo unreadable; return
    fi
    if [ -z "$out" ]; then echo clean
    elif echo "$out" | grep -qv '^??'; then echo modified
    else echo untracked; fi
}

printf '%-48s %-40s %-10s %s\n' PATH BRANCH VERDICT WHY
to_remove=()
for row in "${rows[@]}"; do
    IFS='|' read -r path head branch <<<"$row"
    short=${head:0:8}
    owner=$(stat -c %U "$path" 2>/dev/null || echo '?')
    prs=$(prs_containing "$head")
    merged=""; open=""; closed=""
    for n in $prs; do
        case "${pr_state[$n]:-}" in
            merged) merged="$merged #$n" ;;
            open) open="$open #$n" ;;
            closed) closed="$closed #$n" ;;
        esac
    done
    if [ "$owner" != "$me" ]; then
        if on_main "$head"; then why="on main"
        elif [ -n "$merged" ]; then why="merged$merged"
        elif [ -n "$open" ]; then why="open$open"
        elif [ -n "$(remote_branches "$head")" ]; then why="pushed, no PR"
        else why="unpushed $short"; fi
        printf '%-48s %-40s %-10s %s\n' "$path" "$branch" OTHER-UID "$owner, $why"
        continue
    fi
    edits=$(tree_edits "$path")
    if on_main "$head"; then
        if [ "$edits" = clean ]; then verdict=ON-MAIN; why="$short is on main"; to_remove+=("$path")
        else verdict=EDITS; why="$short is on main, $edits files"; fi
    elif [ -n "$merged" ]; then
        if [ "$edits" = clean ]; then
            verdict=REMOVE; why="merged$merged"; to_remove+=("$path")
        else verdict=EDITS; why="merged$merged, $edits files"; fi
    elif [ -n "$open" ]; then
        verdict=OPEN-PR; why="open$open, $edits"
    elif [ -n "$closed" ]; then
        verdict=CLOSED-PR; why="closed without merge$closed, $edits"
    elif remote=$(remote_branches "$head") && [ -n "$remote" ]; then
        verdict=PUSHED; why="on $remote, no PR, $edits"
    else
        ahead=$(git -C "$main_wt" rev-list --count "origin/main..$head")
        verdict=UNPUSHED; why="$ahead commits on no remote ref, $edits"
    fi
    printf '%-48s %-40s %-10s %s\n' "$path" "$branch" "$verdict" "$why"
done

[ "$remove" = 1 ] || exit 0
echo
for path in "${to_remove[@]}"; do
    if git -C "$main_wt" worktree remove --force "$path" 2>/dev/null; then
        echo "removed $path"
    elif [ -d "$path" ] && [ -z "$(ls -A "$path")" ]; then
        # The tree is gone but the top folder sits in a directory this uid
        # cannot write, as /store/agent-worktrees does. Say so, do not sudo.
        echo "emptied $path, top folder needs: sudo rmdir $path"
    else
        echo "could not remove $path" >&2
    fi
done
git -C "$main_wt" worktree prune
