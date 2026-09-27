#!/usr/bin/env bash
# Validate the parent release dispatcher without touching a real release.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# FIXED 2026-09-27 (audit rework round 3, F86): REPO_ROOT was computed and
# never read (shellcheck SC2034). Removed instead of allow-listed.
RELEASE_SH="$SCRIPT_DIR/release.sh"

pass=0
fail=0

assert() {
    local label="$1" actual="$2" expected="$3"
    if [[ "$actual" == "$expected" ]]; then
        printf '  ✓ %s\n' "$label"
        pass=$((pass + 1))
    else
        printf '  ✗ %s (expected %q, got %q)\n' "$label" "$expected" "$actual"
        fail=$((fail + 1))
    fi
}

assert_match() {
    local label="$1" actual="$2" pattern="$3"
    # SC2053 is a FALSE POSITIVE here and is disabled deliberately: every
    # caller passes a glob (`*meta-only*`, `*choose dr...*`), and quoting
    # the right-hand side would turn those into literal string compares
    # and silently stop matching.
    # shellcheck disable=SC2053
    if [[ "$actual" == $pattern ]]; then
        printf '  ✓ %s\n' "$label"
        pass=$((pass + 1))
    else
        printf '  ✗ %s (expected %q, got %q)\n' "$label" "$pattern" "$actual"
        fail=$((fail + 1))
    fi
}

echo '=== Parent release dispatcher ==='

if [[ -x "$RELEASE_SH" ]]; then
    echo '  ✓ release.sh is executable'
    pass=$((pass + 1))
else
    echo '  ✗ release.sh is not executable'
    fail=$((fail + 1))
fi

if bash -n "$RELEASE_SH"; then
    echo '  ✓ release.sh passes bash -n'
    pass=$((pass + 1))
else
    echo '  ✗ release.sh has syntax errors'
    fail=$((fail + 1))
fi

help="$($RELEASE_SH --help)"
assert_match 'help describes the meta-only dispatcher' "$help" '*meta-only*'

for utility in dracon-sync dracon-system dracon-warden; do
    nested_help="$($RELEASE_SH "$utility" --help)"
    assert_match "$utility dispatches to its nested release script" "$nested_help" '*release.sh*'
done

set +e
invalid_output="$($RELEASE_SH not-a-utility 1.2.3 2>&1)"
invalid_rc=$?
set -e
assert 'unknown utility exits 2' "$invalid_rc" '2'
assert_match 'unknown utility error is actionable' "$invalid_output" '*choose dracon-sync*'

if [[ "$fail" -eq 0 ]]; then
    echo "=== $pass checks passed ==="
    exit 0
fi

echo "=== $fail checks failed ($pass passed) ==="
exit 1
