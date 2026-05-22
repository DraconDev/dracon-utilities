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

## Iteration 3 ✅
- **Item #3: Investigate `wal-backup` daemon sync loop** ✅
  - **Root cause identified**: Ralph-loop AI agent (`audit-fixes` state, `.ralph/audit-loop/RALPH.md`) actively modifying files in wal-backup repo
  - Sync daemon is operating correctly — it detects each agent modification and auto-commits
  - index.lock contention between ralph agent and sync daemon (2 confirmed incidents)
  - Stuck `find` process (PID 3733929, D state since May 21) — cannot be killed, needs reboot
  - Wrote investigation report: `audit-todo/wal-backup-investigation.md`
  - Cargo check passes clean
  - No code changes needed in dracon-sync — sync daemon behavior is correct
