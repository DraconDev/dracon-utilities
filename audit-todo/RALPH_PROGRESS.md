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

## Iteration 4 ✅
- **Item #4: Monitor `proc-macro-error` removal** ✅
  - **Status**: **NOT yanked** — confirmed via crates.io API on 2026-05-22
  - Dependency chain: `dracon-security → age 0.10.1 → i18n-embed-fl 0.7.0 → proc-macro-error 1.0.4`
  - Upgrade path identified: `age 0.10 → 0.11` + `secrecy 0.8 → 0.10` eliminates dep entirely
  - age 0.11 drops `i18n-embed-fl` (and thus `proc-macro-error`)
  - API break in age 0.11: `Decryptor` changed from enum to struct, `Encryptor::with_recipients` takes iterator
  - Wrote research doc: `audit-todo/proc-macro-error-investigation.md`
  - Cargo check passes clean
  - No code changes needed now — monitored and understood

## Iteration 5 ✅
- **Item #5: Add periodic incident ledger pruning** ✅
  - Extracted shared `enforce_retention()` function in `report.rs` (was inline in `append_incident_record` + duplicated in `enforce_retention_at_startup`)
  - Simplified `enforce_retention_at_startup` to delegate to shared function
  - Added periodic pruning every 1800 cycles (~30 min at 1s interval) in daemon main loop
  - All 456 tests pass, cargo check clean
  - Committed by sync daemon as `chore(sync): update ...`

## Iteration 6 ✅
- **Item #6: Review scribe prompt injection sanitization** ✅
  - Replaced blocklist `sanitize_for_prompt()` with **structural separation**: system message for authoritative instructions, user message for untrusted diff data
  - Added `ChatMessage::system()` constructor in `simple_ai.rs`
  - Split `build_commit_message_prompt()` → `build_system_instructions()` + `build_user_content()`
  - Removed fragile blocklist patterns (IGNORE, SYSTEM:, YOU ARE, etc.) — no longer needed
  - Strengthened post-processing output validation: rejects AI outputs starting with "I will", "I cannot", "I am", "You are"
  - All 456 + 65 tests pass across all crates, cargo check clean
  - Committed by sync daemon

## Iteration 7 ✅
- **Item #7: Enable release profile optimizations** ✅
  - Added `strip = true` and `lto = "thin"` to workspace `[profile.release]` in Cargo.toml
  - Size reductions: dracon-sync 13M→10M (-23%), dracon-system 4.0M→3.2M (-20%), dracon-warden 6.2M→4.8M (-23%)
  - Total savings: ~5.2M across all three binaries
  - cargo check passes clean
  - Committed as `chore(audit): enable release profile optimizations (strip=true, lto=thin)`
