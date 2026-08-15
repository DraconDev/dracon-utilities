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

# Invariant 3: the meta workspace has all nested utility repositories
echo "--- Invariant 3: Nested utility repositories ---"
missing=0
for utility in dracon-sync dracon-system dracon-warden; do
  if [[ -f "$utility/Cargo.toml" && -d "$utility/.git" ]]; then
    echo "PASS: $utility nested repository present"
  else
    echo "FAIL: $utility nested repository is missing"
    missing=$((missing + 1))
  fi
done
if [[ "$missing" -ne 0 ]]; then
  failures=$((failures + 1))
else
  echo "PASS: all nested utility repositories present"
fi

# Invariant 3b: CI, Nix, and workspace lock metadata agree on the nested
# standalone revisions and crate versions.  This intentionally does not
# require local HEADs to equal the pins while a nested utility is being
# prepared for release; CI uses --check-local after checking out the pins.
echo "--- Invariant 3b: Nested source pins ---"
if python3 scripts/check-nested-pins.py; then
  echo "PASS: CI/Nix/Cargo nested pins agree"
else
  echo "FAIL: nested source pins are inconsistent"
  failures=$((failures + 1))
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
