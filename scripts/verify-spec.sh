#!/usr/bin/env bash
# Reconcile script for respec
# Exit 0 when all invariants are satisfied.
# Exit non-zero with descriptive output when any check fails.

set -u
set -o pipefail

echo "=== Running spec verification ==="

failures=0

# Invariant 1: Project compiles
echo "--- Invariant 1: Project compiles ---"
if ! cargo check --quiet 2>&1; then
  echo "FAIL: cargo check failed"
  failures=$((failures + 1))
else
  echo "PASS: Project compiles"
fi

# Invariant 2: No blocking TODO comments
echo "--- Invariant 2: No blocking TODO comments ---"
if grep -r "FIXME:\|BLOCKING:" src/ --include="*.rs" 2>/dev/null; then
  echo "FAIL: Found FIXME: or BLOCKING: comments"
  failures=$((failures + 1))
else
  echo "PASS: No blocking TODO comments"
fi

# Invariant 3: Core unit tests pass (library tests only, no network integration tests)
echo "--- Invariant 3: Core unit tests pass ---"
output=$(cargo test --lib --quiet 2>&1)
if echo "$output" | grep -q "test result:.*FAILED"; then
  echo "FAIL: Some unit tests failed"
  failures=$((failures + 1))
else
  echo "PASS: Core unit tests pass"
fi

# --- Add more checks above this line ---

if [ "$failures" -eq 0 ]; then
  echo ""
  echo "=== All invariants satisfied ==="
else
  echo ""
  echo "=== $failures invariant(s) failing ==="
fi

exit "$failures"