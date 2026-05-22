# Dracon Utilities — TODO

Audit date: 2026-05-22

---

## 🔴 Must Fix

- [x] **Settle GitHub Actions billing**
  - [BLOCKED: requires manual user action at https://github.com/settings/billing]
  - All CI jobs blocked: billing payment/spending limit issue
  - Not fixable from code — admin action required

- [x] **Bump `git2` in dracon-libs/dracon-git** (RUSTSEC-2026-0008)
  - Bumped from 0.18.3 to 0.21.0
  - Fixed API breaks: Result returns, Oid::ZERO_SHA1
  - Removed RUSTSEC-2026-0008 ignore from deny.toml
  - 40/40 tests pass in dracon-git, 456/456 tests pass in dracon-sync

- [x] **Investigate `wal-backup` daemon sync loop**
  - Root cause: ralph-loop AI agent actively modifying files in wal-backup repo
  - Sync daemon operating correctly — detecting and auto-committing
  - index.lock contention between ralph agent and sync daemon (2 incidents)
  - Stuck `find` process (PID 3733929, D state) — needs reboot to clear
  - Investigation report: audit-todo/wal-backup-investigation.md

## 🟡 Should Fix

- [x] **Monitor `proc-macro-error` removal** (yanked from crates.io)
  - NOT yanked as of 2026-05-22
  - Chain: `age 0.10.1 → i18n-embed-fl 0.7.0 → proc-macro-error 1.0.4`
  - age 0.11 drops i18n-embed-fl entirely
  - Research doc: audit-todo/proc-macro-error-investigation.md

- [x] **Add periodic incident ledger pruning**
  - Implemented: shared `enforce_retention()` function in report.rs
  - Runs every 1800 cycles (~30 min at 1s interval) in daemon main loop
  - All tests pass

- [x] **Review scribe prompt injection sanitization**
  - Replaced blocklist with structural separation
  - System message for instructions, user message for untrusted diff data
  - Added `ChatMessage::system()` constructor
  - Strengthened post-processing output validation

- [x] **Enable release profile optimizations**
  - Added `strip = true` and `lto = "thin"` to workspace release profile
  - Size reductions: dracon-sync 13M→10M (-23%), dracon-system 4.0M→3.2M (-20%), dracon-warden 6.2M→4.8M (-23%)

- [x] **Test `nix_auto_update` end-to-end**
  - Found and fixed parser bug: didn't handle merged-src `buildRustPackage (commonArgs // {` pattern
  - Added `});` as block-end marker
  - Tests added: `test_update_version_merged_src_style`, `test_update_version_merged_src_closing_detection`

## 🔵 Consider

- [x] **Update `EnvRestorer` docstring**
  - Updated from "334/334 pass" to "458/458 pass as of 2026-05-22"

- [x] **Document `Restart=always` behavior**
  - Added documentation in AGENTS.md with notes about stop vs restart behavior

- [x] **Run `cargo update` after billing resolved**
  - Ran cargo update: 11 packages updated
  - All 458 tests pass, cargo check clean
