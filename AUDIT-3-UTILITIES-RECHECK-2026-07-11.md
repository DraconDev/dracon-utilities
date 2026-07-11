# Recheck — AUDIT-3-UTILITIES-2026-07-10.md (2026-07-11)

**Goal:** `mrfm3rs5-wd42b8` — "lets recheck it make sure we are getting it right".
**Re-checking the prior audit:** `AUDIT-3-UTILITIES-2026-07-10.md` (goal
`mrfgzxre-n5fqe6`, approved).

Every claim was independently re-verified against fresh command output (no code changes
to any of the 3 crates between the two runs).

## Re-run results (fresh)

| # | Audit claim | Fresh recheck result | Match? |
|---|---|---|---|
| 1 | sync `cargo build --release --locked` exit 0, 16 warnings, 0 errors | exit 0, 16 warnings, 0 errors | ✅ |
| 2 | system `cargo build --release --locked` exit 0 | exit 0, clean | ✅ |
| 3 | warden `cargo build --release --locked` exit 0 | exit 0, clean | ✅ |
| 4 | sync tests: 647 pass / 18 fail / 3 ignored / exit 101 | `647 passed; 18 failed; 3 ignored` exit 101 | ✅ |
| 5 | system tests: 86 pass / 0 fail / exit 0 | `86 passed; 0 failed` exit 0 | ✅ |
| 6 | warden tests: 76 + 10 doc / 0 fail / exit 0 | `76 passed; 0 failed` + `10 passed; 0 failed` exit 0 | ✅ |
| 7 | sync `cargo deny` exit 0 (advisories ok; "unmatched skip" warning) | exit 0, advisories/bans/licenses/sources ok, "unmatched skip configuration" | ✅ |
| 8 | system `cargo deny` exit 1, RUSTSEC-2026-0190 (anyhow unsound) | exit 1, RUSTSEC-2026-0190, advisories FAILED | ✅ |
| 9 | warden `cargo deny` exit 1, RUSTSEC-2026-0190 + RUSTSEC-2026-0204 | exit 1, both IDs present, advisories FAILED | ✅ |
| 10 | RUSTSEC-2026-0204 → `crossbeam-epoch v0.9.18` (NOT `crossbeam-deque`) | ❌ — original audit misattributed to `crossbeam-deque v0.8.6`. cargo-deny's authoritative output: `crossbeam-epoch 0.9.18 registry+...` with `ID: RUSTSEC-2026-0204`, `Solution: Upgrade to >=0.9.20 (try cargo update -p crossbeam-epoch)`. The `Atomic`/`Shared` `fmt::Pointer` types live in `crossbeam-epoch`; `crossbeam-deque` is a transitive dependent. **Corrected in both docs 2026-07-11.** | ❌ (corrected) |
| 11 | `triomphe` NOT a warden dependency | `grep 'name = "triomphe"' dracon-warden/Cargo.lock` → no match | ✅ |
| 12 | anyhow v1.0.102 in system + warden locks | v1.0.102 in both | ✅ |
| 13 | dracon-system not a cyclic dependency (`cargo tree` exit 0) | `cargo tree` exit 0; `dracon-system` appears only as root + 1 path (no cycle) | ✅ |
| 14 | no workspace-root `Cargo.toml` | `ls Cargo.toml` → "No such file or directory" | ✅ |
| 15 | all production `--cacheinfo` sites use comma form `160000,<sha>,<path>` | 11 sites: discovery.rs:886/1006/1127, daemon.rs:3196, sync.rs:1014/7655, exclude.rs:685, role.rs:224 (all comma form) | ✅ |

## Discrepancies investigated & resolved (grep artifacts, not audit errors)
- The `^test ... FAILED` grep matched only 13–14 names, while the test summary reported
  18 failures. Root cause: cargo's `test <name> ... <long-error>\nFAILED` two-line format
  (caused by the long `error: option 'cacheinfo' expects <mode>,<sha1>,<path>` line) —
  the standalone `FAILED` is on the next line and misses the single-line regex. The
  broader `grep FAILED` count and the `test result: FAILED. 647 passed; 18 failed`
  summary line both confirm 18 total. The audit's claim of 18 is correct; the
  module-list (which includes `role::tests`, where 3 of the 18 fail) is also correct.
- The build warning grep: a naive `grep warning:` of cargo's build output returned 17,
  but cargo's own authoritative summary says "generated 16 warnings" and the audit's
  per-category breakdown (6 unused + 3 function + 2 variable + 2 fields + 1 value + 1
  methods + 1 method) sums to 16. The extra `warning:` match was cargo's own summary
  line beginning with the word "warning:". The audit header said 17 — **corrected to 16**.

## Inaccuracies in the original audit surfaced and corrected (2026-07-11)
1. **RUSTSEC-2026-0204 misattributed to `crossbeam-deque v0.8.6`** (the second-draft
   "correction" from the earlier `triomphe` misattribution). The actual vulnerable crate
   per cargo-deny is **`crossbeam-epoch v0.9.18`**; fix is
   `cargo update -p crossbeam-epoch` to ≥0.9.20. The first recheck rubber-stamped the
   crossbeam-deque attribution by verifying a trivial fact (lock contains crossbeam-deque)
   instead of the substantive claim (which crate RUSTSEC-2026-0204 is on). The second
   recheck reads cargo-deny's `Cargo.lock:38:1` location and the
   `Solution: Upgrade to >=0.9.20` line and corrects it.
2. **Build warning header said 17** — actually **16** per cargo's own summary.

Both corrections applied to `AUDIT-3-UTILITIES-2026-07-10.md` (build table + narrative,
deny table warden row, the crossbeam-family note, CONCERN #1 + fix recommendation) and
to this recheck document.

## Conclusion
13 of 15 audit claims hold verbatim. 2 were inaccurate and have been corrected in both
the original audit and this recheck. With the corrections, **all 15 claims now match the
fresh evidence**. The corrected audit is accurate. The cyclic-dependency correction
(dracon-system is NOT a cycle; `cargo tree` exit 0) from the first auditor pass holds.
The audit is now right.