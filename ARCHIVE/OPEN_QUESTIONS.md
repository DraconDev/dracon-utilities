# OPEN_QUESTIONS.md

## P0 — Blocking Completion

### 1. GitHub Actions billing (🔴)
- **Severity**: P0 — blocks all CI
- **Issue**: All CI jobs silently fail with *"Recent account payments have failed or your spending limit needs to be increased."*
- **Required action**: Visit https://github.com/settings/billing, resolve the payment issue, and verify CI runs again
- **Status**: [BLOCKED: requires manual user action on GitHub.com]
- **HYPOTHESIS**: Account may have an expired payment method or spending limit that needs manual reset

### 2. Bump `git2` in dracon-libs/dracon-git (🔴) ✅ DONE
- **Severity**: P0 — was open RUSTSEC advisory, now resolved
- **Issue**: RUSTSEC-2026-0008: Unsoundness in `Buf` struct dereferencing
- **Status**: ✅ **Done in iteration 2**
- **Changes**: `git2` bumped 0.18.3→0.21.0, API breaks fixed (shorthand/summary/url/path Result changes), `Oid::zero()`→`Oid::ZERO_SHA1`, `deny.toml` ignore entry removed
- **Verification**: 40/40 dracon-git tests pass, 456/456 dracon-sync tests pass, `cargo deny check advisories` clean
- **Commits**: `a6e3ee47` (dracon-utilities), `b38d73e`+`c3cafe7` (dracon-libs)

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
