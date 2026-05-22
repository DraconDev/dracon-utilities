# Progress

## Iteration 1 ✅
- **Item #1: Settle GitHub Actions billing** → [BLOCKED: requires manual user action at https://github.com/settings/billing]
- Created OPEN_QUESTIONS.md with P0/P1/P2 items
- Billing is an external admin issue (payment method / spending limit), not fixable from code
- Marked as BLOCKED, moving to next item

## Iteration 2 ✅
- **Item #2: Bump git2 0.18.3→0.21.0 in dracon-libs/dracon-git** ✅
  - Updated Cargo.toml: `git2 = "0.18"` → `git2 = "0.21"`
  - Fixed API breaks: `shorthand()` returns `Result`, `summary()` returns `Result<Option<_>>`, `url()` returns `Result`, `path()` returns `Result`, `Oid::zero()` → `Oid::ZERO_SHA1`
  - Removed `RUSTSEC-2026-0008` ignore from `deny.toml`
  - 40/40 tests pass in dracon-git, 456/456 tests pass in dracon-sync
  - Committed as `fix(audit): bump git2 0.18.3→0.21.0 in dracon-libs/dracon-git, remove deny.toml ignore for RUSTSEC-2026-0008`
