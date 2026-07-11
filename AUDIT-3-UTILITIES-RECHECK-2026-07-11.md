# Recheck — AUDIT-3-UTILITIES-2026-07-10.md (2026-07-11)

**Goal:** `mrfm3rs5-wd42b8` — "lets recheck it make sure we are getting it right".
**Re-checking the prior audit:** `AUDIT-3-UTILITIES-2026-07-10.md` (goal
`mrfgzxre-n5fqe6`, approved).

Every claim was independently re-verified against fresh command output (no code changes
to any of the 3 crates between the two runs, so results must match).

## Re-run results (fresh)

| # | Audit claim | Fresh recheck result | Match? |
|---|---|---|---|
| 1 | sync `cargo build --release --locked` exit 0 | exit 0, 17 warnings, 0 errors | ✅ |
| 2 | system `cargo build --release --locked` exit 0 | exit 0, clean | ✅ |
| 3 | warden `cargo build --release --locked` exit 0 | exit 0, clean | ✅ |
| 4 | sync tests: 647 pass / 18 fail / 3 ignored / exit 101 | `647 passed; 18 failed; 3 ignored` exit 101 | ✅ |
| 5 | system tests: 86 pass / 0 fail / exit 0 | `86 passed; 0 failed` exit 0 | ✅ |
| 6 | warden tests: 76 + 10 doc / 0 fail / exit 0 | `76 passed; 0 failed` + `10 passed; 0 failed` exit 0 | ✅ |
| 7 | sync `cargo deny` exit 0 (advisories ok; "unmatched skip" warning) | exit 0, advisories/bans/licenses/sources ok, "unmatched skip configuration" | ✅ |
| 8 | system `cargo deny` exit 1, RUSTSEC-2026-0190 (anyhow unsound) | exit 1, RUSTSEC-2026-0190, advisories FAILED | ✅ |
| 9 | warden `cargo deny` exit 1, RUSTSEC-2026-0190 + RUSTSEC-2026-0204 | exit 1, both IDs present, advisories FAILED | ✅ |
| 10 | RUSTSEC-2026-0204 → `crossbeam-deque v0.8.6` | warden lock: `crossbeam-deque v0.8.6` present | ✅ |
| 11 | `triomphe` NOT a warden dependency | `grep 'name = "triomphe"' dracon-warden/Cargo.lock` → no match | ✅ |
| 12 | anyhow v1.0.102 in system + warden locks | v1.0.102 in both | ✅ |
| 13 | dracon-system not a cyclic dependency (`cargo tree` exit 0) | `cargo tree` exit 0; `dracon-system` appears only as root + 1 path (no cycle) | ✅ |
| 14 | no workspace-root `Cargo.toml` | `ls Cargo.toml` → "No such file or directory" | ✅ |
| 15 | all production `--cacheinfo` sites use comma form `160000,<sha>,<path>` | 11 sites: discovery.rs:886/1006/1127, daemon.rs:3196, sync.rs:1014/7655, exclude.rs:685, role.rs:224 (all comma form) | ✅ |

## Discrepancies investigated & resolved
- The `^test ... FAILED` grep matched only 13–14 names, while the test summary reported
  18 failures. Root cause: cargo's `test <name> ... <long-error>\nFAILED` two-line format
  (caused by the long `error: option 'cacheinfo' expects <mode>,<sha1>,<path>` line) —
  the standalone `FAILED` is on the next line and misses the single-line regex. The
  broader `grep FAILED` count and the `test result: FAILED. 647 passed; 18 failed`
  summary line both confirm 18 total. The audit's claim of 18 is correct; the
  module-list (which includes `role::tests`, where 3 of the 18 fail) is also correct.

## Conclusion
**All 15 audit claims independently re-verified.** The audit
`AUDIT-3-UTILITIES-2026-07-10.md` is accurate. The two attributions corrected during the
first auditor pass (RUSTSEC-2026-0204 → `crossbeam-deque v0.8.6`, not `triomphe`; no
cyclic dependency in dracon-system) both hold under fresh verification.