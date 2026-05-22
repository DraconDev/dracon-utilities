# OPEN_QUESTIONS.md

## P0 — Blocking Completion

### 1. GitHub Actions billing (🔴)
- **Severity**: P0 — blocks all CI
- **Issue**: All CI jobs silently fail with *"Recent account payments have failed or your spending limit needs to be increased."*
- **Required action**: Visit https://github.com/settings/billing, resolve the payment issue, and verify CI runs again
- **Status**: [BLOCKED: requires manual user action on GitHub.com]
- **HYPOTHESIS**: Account may have an expired payment method or spending limit that needs manual reset

### 2. Bump `git2` in dracon-libs/dracon-git (🔴)
- **Severity**: P0 — open RUSTSEC advisory, potential UB
- **Issue**: RUSTSEC-2026-0008: Unsoundness in `Buf` struct dereferencing
- **Status**: Not yet started — deferred while billing is unresolved
- **Dependency**: Needs dracon-libs sibling repo access

### 3. Investigate `wal-backup` daemon sync loop (🔴)
- **Severity**: P0 — CPU/ledger waste from rapid cycling
- **Issue**: 12+ rapid triage entries, stale index.lock detected
- **Status**: Not yet started

## P1 — Important but Not Blocking

### 4. Monitor `proc-macro-error` removal (🟡)
- Chain: `age 0.10.1 → i18n-embed-fl 0.7.0 → proc-macro-error 1.0.4`
- Will break `cargo update` if transitive dep falls out of resolution

### 5. Add periodic incident ledger pruning (🟡)
- Currently only prunes at daemon startup

### 6. Review scribe prompt injection sanitization (🟡)
- Blocklist approach is fragile

### 7. Enable release profile optimizations (🟡)
- Reduce binary sizes

### 8. Test `nix_auto_update` end-to-end (🟡)
- Untested feature

## P2 — Nice to Have

### 9. Update `EnvRestorer` docstring (🔵)
### 10. Document `Restart=always` behavior (🔵)
### 11. Run `cargo update` after billing resolved (🔵)
