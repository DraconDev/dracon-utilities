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
if ! cargo check --workspace --quiet 2>&1; then
  echo "FAIL: cargo check --workspace failed"
  failures=$((failures + 1))
else
  echo "PASS: Project compiles"
fi

# Invariant 2: No blocking TODO comments
echo "--- Invariant 2: No blocking TODO comments ---"
if grep -r "FIXME:\|BLOCKING:" dracon-*/src/ --include="*.rs" 2>/dev/null; then
  echo "FAIL: Found FIXME: or BLOCKING: comments"
  failures=$((failures + 1))
else
  echo "PASS: No blocking TODO comments"
fi

# Invariant 3: GitHub feature-façade scaffold remains self-consistent
echo "--- Invariant 3: Feature façade scaffold self-test ---"
if ! python3 scripts/scaffold_feature_repos.py --self-test 2>&1; then
  echo "FAIL: scripts/scaffold_feature_repos.py --self-test failed"
  failures=$((failures + 1))
else
  echo "PASS: Feature façade scaffold self-test"
fi

# Invariant 4: Core unit tests pass
# (--workspace because these crates are binaries, not libraries, so --lib would fail)
echo "--- Invariant 4: Core unit tests pass ---"
output=$(cargo test --workspace -- --test-threads=1 2>&1)
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