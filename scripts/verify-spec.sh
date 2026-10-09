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
if ! cargo check --workspace --locked --quiet 2>&1; then
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
if output=$(cargo test --workspace --locked -- --test-threads=1 2>&1); then
  echo "PASS: Core unit tests pass"
else
  printf '%s\n' "$output"
  echo "FAIL: cargo test --workspace --locked failed"
  failures=$((failures + 1))
fi

# Invariant 5: sync convergence evidence tools preserve the selected-repository
# safety and forward-only invariants in deterministic local fixtures.
echo "--- Invariant 5: Sync convergence evidence tools ---"
if python3 -m unittest -v scripts.tests.test_sync_convergence; then
  echo "PASS: Sync convergence evidence tool tests pass"
else
  echo "FAIL: Sync convergence evidence tool tests failed"
  failures=$((failures + 1))
fi

# Invariant 6: audit gates reject failed tests and mismatched pinned metadata.
echo "--- Invariant 6: Audit gate regressions ---"
if python3 -m unittest -v scripts.tests.test_audit_regressions; then
  echo "PASS: Audit gate regressions pass"
else
  echo "FAIL: Audit gate regressions failed"
  failures=$((failures + 1))
fi

# Invariant 7: the installer service-gating suite (D4/F103 regression guard)
# passes and its fixture tracks install.sh's unit copy list. Added 2026-10-08
# after the suite was found 11/14 red with nothing running it.
echo "--- Invariant 7: Installer service gating ---"
if python3 -m unittest -v scripts.tests.test_install_service_gating; then
  echo "PASS: Installer service gating tests pass"
else
  echo "FAIL: Installer service gating tests failed"
  failures=$((failures + 1))
fi

# Invariant 8: the root release.sh dispatcher keeps its meta-only contract and
# delegates to the nested release scripts. FIXED 2026-10-09 (audit F128): this
# suite existed as the only coverage of that contract and was wired into
# neither CI nor this script.
echo "--- Invariant 8: Release dispatcher contract ---"
if bash scripts/test_release.sh; then
  echo "PASS: Release dispatcher tests pass"
else
  echo "FAIL: Release dispatcher tests failed"
  failures=$((failures + 1))
fi

# Invariant 9: the GitHub orphan cleanup confirmation regression still holds.
# FIXED 2026-10-09 (audit F128): same dead-suite class as F104.
echo "--- Invariant 9: GitHub orphan cleanup guard ---"
if bash scripts/test_cleanup_github_orphans.sh; then
  echo "PASS: GitHub orphan cleanup tests pass"
else
  echo "FAIL: GitHub orphan cleanup tests failed"
  failures=$((failures + 1))
fi

# Invariant 10: repin-nested-sources.py's fallback path (publish failure
# mid-repin) is covered. FIXED 2026-10-09 (audit F128).
echo "--- Invariant 10: Nested repin fallback ---"
if python3 -m unittest -v scripts.tests.test_repin_fallback; then
  echo "PASS: Nested repin fallback tests pass"
else
  echo "FAIL: Nested repin fallback tests failed"
  failures=$((failures + 1))
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
