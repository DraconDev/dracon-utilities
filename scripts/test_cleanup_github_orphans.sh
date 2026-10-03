#!/usr/bin/env bash
# Hermetic regression test for scripts/cleanup-github-orphans.sh.
# gh is stubbed (fixed repo list, recorded deletes); HOME is a fixture.
# ADDED 2026-10-03 (audit R4-M-09): --apply must require typed confirmation.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
work=$(mktemp -d "${TMPDIR:-/tmp}/dracon-cleanup-orphans-XXXXXX")
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/bin" "$work/home/Dev/localproj/.git"

cat > "$work/bin/gh" <<'EOF'
#!/usr/bin/env bash
# Stub: `repo list` prints the fixture names; `repo delete` records them.
if [[ "${1:-}" == "repo" && "${2:-}" == "list" ]]; then
    if [[ "${GH_FIXTURE_EMPTY:-0}" == "1" ]]; then
        printf 'keepme\nlocalproj\n'
    else
        printf 'foo-2\nbar-17\ntest-repo-alpha\nkeepme\nlocalproj\n'
    fi
    exit 0
fi
if [[ "${1:-}" == "repo" && "${2:-}" == "delete" ]]; then
    echo "${3:-}" >> "${FIXTURE_ROOT:?}/deleted"
    exit 0
fi
echo "unexpected gh invocation: $*" >&2
exit 2
EOF
chmod +x "$work/bin/gh"

run_cleanup() {
    FIXTURE_ROOT="$work" HOME="$work/home" PATH="$work/bin:$PATH" \
        timeout 60 "$SCRIPT_DIR/cleanup-github-orphans.sh" "$@"
}

# Case 1: dry-run is the default — lists only, deletes nothing.
run_cleanup >"$work/dry.out" 2>&1
grep -F 'Would delete: DraconDev/foo-2' "$work/dry.out" >/dev/null
grep -F 'Would delete: DraconDev/test-repo-alpha' "$work/dry.out" >/dev/null
test ! -e "$work/deleted"

# Case 2: --apply with the exact confirmation deletes orphans only.
printf 'DELETE 3 repos\n' | run_cleanup --apply >"$work/apply.out" 2>&1
grep -F 'About to PERMANENTLY delete 3 GitHub repos' "$work/apply.out" >/dev/null
test "$(wc -l < "$work/deleted")" = 3
grep -Fx 'DraconDev/foo-2' "$work/deleted" >/dev/null
grep -Fx 'DraconDev/bar-17' "$work/deleted" >/dev/null
grep -Fx 'DraconDev/test-repo-alpha' "$work/deleted" >/dev/null
if grep -F 'keepme' "$work/deleted" >/dev/null; then
    echo 'non-orphan repo was deleted' >&2
    exit 1
fi
rm "$work/deleted"

# Case 3: wrong confirmation aborts with nothing deleted.
if printf 'DELETE them all\n' | run_cleanup --apply >"$work/wrong.out" 2>&1; then
    echo '--apply accepted a wrong confirmation' >&2
    exit 1
fi
grep -F 'confirmation did not match' "$work/wrong.out" >/dev/null
test ! -e "$work/deleted"

# Case 4: closed stdin (piped/CI) aborts with nothing deleted.
if run_cleanup --apply < /dev/null >"$work/closed.out" 2>&1; then
    echo '--apply deleted with no confirmation read' >&2
    exit 1
fi
grep -F 'no confirmation read' "$work/closed.out" >/dev/null
test ! -e "$work/deleted"

# Case 5: zero targets exits quietly without prompting.
if ! GH_FIXTURE_EMPTY=1 FIXTURE_ROOT="$work" HOME="$work/home" PATH="$work/bin:$PATH" \
    timeout 60 "$SCRIPT_DIR/cleanup-github-orphans.sh" --apply < /dev/null >"$work/empty.out" 2>&1; then
    echo '--apply with zero targets should exit 0' >&2
    exit 1
fi
grep -F 'Nothing to delete.' "$work/empty.out" >/dev/null
test ! -e "$work/deleted"

echo 'orphan cleanup confirmation regression tests: ok'
