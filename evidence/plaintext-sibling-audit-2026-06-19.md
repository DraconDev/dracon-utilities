# .plaintext Sibling File Audit (2026-06-19)

## Context

The warden's pre-push hook scans for plaintext secrets
in push diffs. Files with a `<path>.plaintext` sibling
are silently exempted from the scan. This audit
evaluates all 20 .plaintext sibling files: (1) the
parent file, (2) the category of exemption needed,
(3) the systemic fix that would eliminate the need for
the exemption.

## Inventory (20 files)

### Category A: Test fixtures with intentional secret patterns (14 files)

These are test files that contain intentional secret
patterns (e.g., `let secret = "dev-secret"`) to test
the scanner's detection logic. The scanner correctly
flags them as "secrets", but they're test fixtures,
not real secrets.

| Parent file | Repo | Reason |
|-------------|------|--------|
| `dracon-warden/src/main.rs` | dracon-utilities | Test fixture |
| `dracon-warden/src/tests.rs` | dracon-utilities | Test fixture |
| `dracon-warden/src/security/src/lib.rs` | dracon-utilities | Test fixture |
| `dracon-warden/src/security/src/modules/scanner.rs` | dracon-utilities | Test fixture |
| `dracon-warden/src/security/tests/atomic_write_test.rs` | dracon-utilities | Test fixture |
| `dracon-warden/src/security/tests/scanner_edge_cases_test.rs` | dracon-utilities | Test fixture |
| `dracon-warden/src/security/tests/plaintext_sibling_test.rs` | dracon-utilities | Test fixture (meta-test for the .plaintext mechanism) |
| `dracon-warden/src/security/tests/leak_prevention_test.rs` | dracon-utilities | Test fixture |
| `dracon-warden/src/security/tests/comprehensive_test.rs` | dracon-utilities | Test fixture |
| `dracon-warden/src/security/tests/scanner_stress.rs` | dracon-utilities | Test fixture |
| `dracon-warden/src/security/tests/redos_stress_test.rs` | dracon-utilities | Test fixture |
| `dracon-sync/src/sync.rs` | dracon-utilities | Test fixture |
| `dracon-sync/src/git/mod.rs` | dracon-utilities | Test fixture |
| `dracon-sync/src/report.rs` | dracon-utilities | Test fixture |
| `dracon-sync/src/bump.rs` | dracon-utilities | Test fixture |
| `dracon-sync/src/release.rs` | dracon-utilities | Test fixture |
| `dracon-sync/src/daemon.rs` | dracon-utilities | Test fixture |
| `dracon-code/crates/dracon-core/src/policy/secret_scan.rs` | dracon-code | Test fixture |
| `dracon-platform/apis/services/billing-api/src/lib.rs` | dracon-platform | Test fixture |

**Systemic fix needed**: The scanner should detect
test fixture contexts (e.g., `#[cfg(test)]` modules,
`mod tests {}` blocks, `// SAFETY:` or `// TEST FIXTURE:`
comments) and skip the scan within those contexts.
This would eliminate 19 of the 20 .plaintext files.

### Category B: Documentation files with pattern descriptions (1 file)

| Parent file | Repo | Reason |
|-------------|------|--------|
| `docs/design/warden-hook-pi-goals-skip-2026-06-18.md` | dracon-utilities | Design doc describing the pattern (documentation, not a real secret) |

**Systemic fix needed**: The pre-push hook already
skips `.pi/goals/*` files. Extending the skip to
`docs/design/*` would eliminate this .plaintext file.
However, design docs may legitimately describe patterns
that look like secrets. A more targeted fix: skip
`.md` files in `docs/design/` (the pre-push hook already
scans only added lines, so the risk is low).

## Pattern Analysis

The .plaintext sibling mechanism was introduced as a
quick fix for test files that contain intentional secret
patterns. The mechanism works but creates 20 manual
exemptions that must be maintained.

## Systemic Improvement

The scanner should be enhanced to:

1. **Detect test contexts**: Skip lines within
   `#[cfg(test)]` modules, `mod tests {}` blocks, or
   lines marked with `// TEST FIXTURE:` comments.

2. **Detect mock contexts**: Skip lines within
   `mock_*` or `*_mock` functions, or lines that
   contain `mock_secret` or `test_secret` in the
   variable name.

3. **Detect documentation contexts**: Skip lines in
   `.md` files under `docs/design/` (the pre-push
   hook already only scans added lines).

These improvements would eliminate the need for
20 .plaintext sibling files. However, scanner source
code changes are out of scope for this audit.

## Recommendation

**KEEP all 20 .plaintext files** for now. They
represent the correct workaround for the current
scanner limitations. The systemic fix (scanner
context detection) is a daemon source code change
that requires:
1. Modifying `dracon-warden/src/main.rs` (out of
   scope per the prior goal's constraint)
2. Testing the new context detection
3. Removing the 20 .plaintext files after the new
   scanner is deployed

The 20 .plaintext files are documented in the
`docs/design/warden-plaintext-sibling.md` design doc
(as referenced in the pre-push hook comments).
