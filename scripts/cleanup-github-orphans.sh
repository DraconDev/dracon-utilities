#!/usr/bin/env bash
# Clean up orphan GitHub repositories for DraconDev.
#
# Categories:
#   1. Suffix orphans: repos ending in -N (from the old suffix loop bug)
#   2. Test repos: test-repo-1 through test-repo-19
#   3. Other stale repos not present locally
#
# Usage:
#   ./cleanup-github-orphans.sh          # Dry run (list only)
#   ./cleanup-github-orphans.sh --apply  # Actually delete repos
#
# Requires: gh CLI with delete_repo scope
#   gh auth refresh -h github.com -s delete_repo

set -euo pipefail

APPLY=false
if [[ "${1:-}" == "--apply" ]]; then
    APPLY=true
fi

echo "🔍 Scanning DraconDev GitHub repos..."

# Get all repo names
ALL_REPOS=$(gh repo list DraconDev --limit 400 --json name --jq '.[].name')
TOTAL=$(echo "$ALL_REPOS" | wc -l)

# Category 1: Suffix orphans (name ends in -N)
SUFFIXED=$(echo "$ALL_REPOS" | grep -E -- '-[0-9]+$' || true)
SUFFIXED_COUNT=$(echo "$SUFFIXED" | grep -c . || true)

# Category 2: Test repos (exclude ones already counted as suffix orphans)
TEST_REPOS=$(echo "$ALL_REPOS" | grep -E '^test-repo' | grep -v -E -- '-[0-9]+$' || true)
TEST_COUNT=$(echo "$TEST_REPOS" | grep -c . || true)

# Get local repo names
LOCAL_REPOS=$(ls -d $HOME/Dev/*/.git 2>/dev/null | sed "s|$HOME/Dev/||;s|/.git||" | xargs -I{} basename {} || true)

# Category 3: Remote-only (not in local ~/Dev)
REMOTE_ONLY=""
while IFS= read -r name; do
    if ! echo "$LOCAL_REPOS" | grep -qxF "$name" 2>/dev/null; then
        REMOTE_ONLY="$REMOTE_ONLY\n$name"
    fi
done <<< "$ALL_REPOS"

# Exclude suffixed and test repos from remote-only (they're already counted)
REMOTE_ONLY_STALE=$(echo -e "$REMOTE_ONLY" | grep -v -E -- '-[0-9]+$' | grep -v -E '^test-repo' | grep -v '^$' || true)

echo ""
echo "📊 Summary:"
echo "   Total GitHub repos:    $TOTAL"
echo "   Local repos:           $(echo "$LOCAL_REPOS" | wc -l)"
echo "   Suffix orphans:       $SUFFIXED_COUNT"
echo "   Test repos:           $TEST_COUNT"
echo "   Other remote-only:    $(echo "$REMOTE_ONLY_STALE" | grep -c . || echo 0)"
echo ""

# Delete function
delete_repo() {
    local repo="$1"
    if $APPLY; then
        echo "🗑️  Deleting: DraconDev/$repo"
        if gh repo delete "DraconDev/$repo" --yes 2>&1; then
            echo "   ✅ Deleted"
        else
            echo "   ❌ Failed (need delete_repo scope? Run: gh auth refresh -h github.com -s delete_repo)"
        fi
    else
        echo "   Would delete: DraconDev/$repo"
    fi
}

echo "🏷️  Suffix orphans (from old suffix bug):"
if [[ -n "$SUFFIXED" ]]; then
    while IFS= read -r name; do
        delete_repo "$name"
    done <<< "$SUFFIXED"
else
    echo "   (none)"
fi

echo ""
echo "🧪 Test repos:"
if [[ -n "$TEST_REPOS" ]]; then
    while IFS= read -r name; do
        delete_repo "$name"
    done <<< "$TEST_REPOS"
else
    echo "   (none)"
fi

if $APPLY; then
    echo ""
    echo "✅ Cleanup complete. Run again to verify."
else
    echo ""
    echo "💡 Run with --apply to actually delete these repos."
fi

# NOTE: Category 3 (remote-only stale) is intentionally NOT auto-deleted.
# These repos may be legitimate remote-only projects. Review manually with:
#   gh repo list DraconDev --limit 400 --json name --jq '.[].name'
