#!/usr/bin/env bash
# Reconcile script for respec
# Exit 0 when all invariants are satisfied.
# Exit non-zero with descriptive output when any check fails.

set -u
set -o pipefail

echo "=== Running spec verification ==="

failures=0

# Add your invariant checks below

# Example:
# echo "--- Invariant 1: Project compiles ---"
# if ! npm run build --quiet 2>/dev/null; then
#   echo "FAIL: npm run build failed"
#   failures=$((failures + 1))
# else
#   echo "PASS: Project compiles"
# fi

# --- Add more checks above this line ---

if [ "$failures" -eq 0 ]; then
  echo ""
  echo "=== All invariants satisfied ==="
else
  echo ""
  echo "=== $failures invariant(s) failing ==="
fi

exit "$failures"
