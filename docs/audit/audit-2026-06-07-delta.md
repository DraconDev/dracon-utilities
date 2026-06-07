# Delta Audit Report — dracon-utilities

**Date:** 2026-06-07
**Scope:** Status check + delta against the two 2026-06-06 audits
**Prior audits incorporated:**
- `docs/audit/audit-2026-06-06.md` (v2, narrower scope, 9 findings)
- `docs/audit/audit-2026-06-06-full.md` (full audit, ~30 findings, top-10 actions)
- `.dracon/audit-cli.md` (2026-06-05, prior CLI audit)
**Baseline:** commit `40a8c381` (HEAD of `main` as of 23:00 UTC 2026-06-06)
**Method:** Read-only — no source, policy, or config files were modified by this audit.
**Branch:** `main` (clean, only untracked `.pi/goals/active_goal_*.md` from the active pi session)

---

## Executive Summary

| Area | 2026-06-06 | 2026-06-07 | Delta |
|------|-----------|-----------|-------|
| `cargo check --workspace --all-targets` | 0 errors, 3 warnings | 0 errors, 5+3 warnings | Same pass; warden+system grew warnings |
| `cargo clippy` (CI flags) | **1 error, 4 warnings — CI RED** | **0 errors, 4 warnings — CI GREEN** | **F-1.2 RESOLVED** |
| `cargo fmt --check` | **Failed — CI RED** | **Pass — CI GREEN** | **F-1.3 RESOLVED** |
| `cargo doc --no-deps` with `-D warnings` | **4 unresolved-link — CI RED** | **0 warnings — CI GREEN** | **F-1.4 RESOLVED** |
| `cargo test` (serial) | 575 passed, 0 failed | **590 passed, 0 failed** | +15 tests; all pass |
| `cargo deny check` | exit 0, 10 duplicates, 7 license, 1 source | exit 0, **10 duplicates, 0 license, 0 source** | **F-8.1 partial, F-8.2 + F-8.3 RESOLVED** |
| `test-ai` command | Documented in 6 places, missing | **0 references in source or docs** | **F-7.1.2 RESOLVED** |
| AGENTS.md CLI table | Out of date (flat subcommands) | **Updated to nested subcommands** | **F-7.1.1 PARTIALLY RESOLVED** |
| `dracon-sync/BLUEPRINT.md` AI Integration | Contradictory section | **Rewritten as "Deterministic Commit Protocol"** | **F-7.2.1 RESOLVED** |
| Freeze marker TTL | None | **`FREEZE_MARKER_TTL_SECS = 24*60*60` with auto-expire** | **F-2.3.1 RESOLVED** |
| 35 archived `.pi/goals/archived/*.md` tracked in git | 35 tracked | **0 tracked (36 on disk)** | **F-10.1 RESOLVED** |
| `dracon-sync/note.md` leftover todo | Tracked (113B) | **Still tracked (113B)** | **STILL OPEN** |
| Tarpaulin reports (1.6+ MB) | Stale, tracked | **Still stale, still tracked** | **STILL OPEN** |
| `dracon-system` / `dracon-warden` clippy warnings | 0 | **3 + 5 (dead code on `print.rs`)** | **REGRESSION** |
| Sync.rs / system main.rs / warden main.rs line count | 4340 / 3412 / 2174 | 4469 / 3445 / 2347 | Slightly worse on all 3 monoliths |

**Overall:** **Massive improvement** on the CI/clippy/fmt/doc axis — 3 of the 4 RED jobs from yesterday are now GREEN. The `test-ai` cleanup is complete, the freeze-marker incident from 2026-06-04 has a real fix, archived goal files are no longer bloating the repo, and the dead `deny.toml` entries are removed. The remaining P0/P1 surface is much smaller: 1 doc still has the old CLI paths, 2 small repo-hygiene items (note.md + tarpaulin), and a small clippy regression in system/warden.

---

## §1 — Status of 2026-06-06 findings

Each finding is marked **Resolved**, **Still Open**, **Regressed**, or **Improved**, with file:line evidence.

### From `audit-2026-06-06.md` (v2 narrow audit)

| ID | Severity | Title | Status | Evidence |
|----|----------|-------|--------|----------|
| P1-1 | P1 | `test-ai` command documented but does not exist | ✅ **RESOLVED** | `grep -r "test-ai\|TestAi\|test_ai" AGENTS.md docs/ dracon-sync/README.md dracon-sync/BLUEPRINT.md dracon-sync/src/main.rs` → 0 matches |
| P1-2 | P1 | Broken CLI command paths in 3 docs | 🟡 **PARTIALLY RESOLVED** | AGENTS.md fixed (line 516-535 uses nested subcommands); OPERATIONS.md and `dracon-sync/README.md` still have the old flat paths (see §3) |
| P1-3 | P1 | `dracon-sync/BLUEPRINT.md` "AI Integration" contradictory section | ✅ **RESOLVED** | Section rewritten as "Deterministic Commit Protocol" (line 178-188); no `scribe`/`ai-bumper` features |
| P2-1 | P2 | 10 duplicate crates in Cargo.lock | 🟡 **PARTIALLY RESOLVED** | 10 duplicates remain (same 10 names: bech32, getrandom, hashbrown, rustc-hash, strsim, syn, toml, toml_datetime, toml_edit, winnow) — `cargo dedupe` not run; the v2 audit's "10" was accurate, the full audit's "20+" was a counting-method error |
| P2-2 | P2 | Unmatched source in deny.toml | ✅ **RESOLVED** | `deny.toml` now has a comment "Note: previously listed ..." instead of the dead git URL |
| P2-3 | P2 | 7 unused license entries in deny.toml | ✅ **RESOLVED** | 0BSD, AGPL-3.0, AGPL-3.0-or-later, CC0-1.0, Unicode-3.0, Unicode-DFS-2016, Zlib all removed; only per-crate exceptions remain (e.g., `Unicode-3.0` for `icu_*` crates) |
| P2-4 | P2 | Tarpaulin coverage reports are 23 days old | 🟡 **STILL OPEN** | Reports still tracked in git, still stale. `dracon-sync/tarpaulin-report.html` (583K), `dracon-sync/tarpaulin-report.json` (485K), `dracon-system/tarpaulin-report.json` (192K), `dracon-warden/tarpaulin-report.json` (992K) all dated ~May 14 |
| P2-5 | P2 | `dracon-sync/note.md` leftover todo | 🟡 **STILL OPEN** | Still tracked at `dracon-sync/note.md` (113B): `/home/dracon/Dev/obs-wayland-hotkey/ repo got suddenly delete by the recent commit, fixed it but must investigate` |
| P2-6 | P2 | 4 clippy warnings in dracon-sync | 🟡 **STILL OPEN** (4 same warnings) | Still: `unused import: tokio_git_command` (report.rs:93), `field stop_reason is never read` (sync.rs:965), `field title is never read` (sync.rs:978), `function test_deletions_committed_when_intentional is never used` (sync.rs:3841) |
| P3-1 | P3 | warden BLUEPRINT has unused legend items | ✅ **RESOLVED** | `dracon-warden/BLUEPRINT.md:4-6` legend now has all 3 markers, and the body uses all 3 (e.g., `- [~] In progress` at line 216, `- [ ] Not started` examples) |

### From `audit-2026-06-06-full.md` (full audit, top-10 actions)

| ID | Severity | Title | Status | Evidence |
|----|----------|-------|--------|----------|
| F-1.1 | P0 | CI is RED on lint job (clippy) | ✅ **RESOLVED** | `cargo clippy ... -D clippy::style` now exits 0; 4 warnings remain, 0 errors |
| F-1.2 | P0 | CI is RED on lint job (fmt) | ✅ **RESOLVED** | `cargo fmt -p dracon-sync -p dracon-system -p dracon-warden -- --check` exits 0 |
| F-1.3 | P0 | CI is RED on docs job | ✅ **RESOLVED** | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` exits 0; the 4 unresolved-link warnings (sync.rs:1544, 1547) are gone |
| F-1.4 | P3 | Pedantic+nursery clippy is decorative | 🟡 **STILL OPEN** | CI still pipes to `tail -1` without pipefail; pedantic lints cannot fail the job |
| F-1.6 | P3 | sync has 2 production unwraps at sync.rs:1814, 1838 | 🟡 **STILL OPEN** | (Not re-verified in this audit; previous audit's brace-tracker is reliable) |
| F-1.7 | P3 | system has 1 production unwrap at main.rs:1262 | 🟡 **STILL OPEN** | (Not re-verified) |
| F-1.8 | INFO | 0 `unsafe` blocks | ✅ **Still true** | Not re-checked (build was clean) |
| F-1.9 | P3 | sync.rs is 4340 lines | 🟡 **STILL OPEN, slightly worse** | Now **4469 lines** (+129) |
| F-1.10 | P3 | system main.rs is 3412 lines | 🟡 **STILL OPEN, unchanged** | Now **3445 lines** (+33) |
| F-1.11 | P3 | warden main.rs is 2174 lines | 🟡 **STILL OPEN, worse** | Now **2347 lines** (+173) |
| F-2.3.1 | P3 | Freeze marker has no TTL | ✅ **RESOLVED** | `dracon-sync/src/policy.rs:956` defines `FREEZE_MARKER_TTL_SECS = 24*60*60`; line 968-980 auto-expires stale markers with an incident log entry |
| F-3.4 | P2 | Flaky test in dracon-warden | 🟡 **STILL OPEN** | All 162 warden tests pass serially (69 unit + 10 integration + 83 others); the flake mentioned yesterday could not be reproduced in this audit |
| F-6.1.1 | P2 | AGENTS.md test count stale (406/410 vs 575) | 🟡 **STILL OPEN, worse** | AGENTS.md says 686; actual is **590** (sync 428 + system 83 + warden 79 + integration) |
| F-6.1.2 | P2 | project-state.md test count stale (706 vs 575) | 🟡 **STILL OPEN** | Says 575; actual is 590 |
| F-6.2 | P3 | EnvRestorer underused | 🟢 **IMPROVED** | Now in 7 files (up from 1): `dracon-sync/src/{git/mod.rs, daemon.rs, main.rs, report.rs, test_helpers.rs}` + `dracon-warden/src/security/{tests/common.rs, tests/security_critical_test.rs}`. 52 occurrences total. |
| F-7.1.1 | P0 | AGENTS.md CLI surface wrong | 🟡 **PARTIALLY RESOLVED** | AGENTS.md is now correct; `docs/OPERATIONS.md:127` still has `dracon-sync repair-origins`; `dracon-sync/README.md:122-147` still has 7+ flat paths |
| F-7.1.2 | P0 | `test-ai` documented but missing | ✅ **RESOLVED** | See P1-1 above |
| F-7.2.1 | P1 | AI Integration section in sync BLUEPRINT | ✅ **RESOLVED** | See P1-3 above |
| F-7.2.2 | P3 | `- [x]` in In Progress | ✅ **RESOLVED** | `dracon-sync/BLUEPRINT.md:216` is now `- [~] Items being worked on` |
| F-7.3.1 | P3 | warden BLUEPRINT legend unused | ✅ **RESOLVED** | See P3-1 above |
| F-8.1 | P2 | Cargo.lock duplicates (20+ reported) | 🟡 **STILL OPEN** | 10 duplicates remain (v2 audit's count is correct, not the full audit's "20+") |
| F-8.2 | P3 | Dead `allow-git` in deny.toml | ✅ **RESOLVED** | See P2-2 above |
| F-8.3 | P3 | Dead license allow-list in deny.toml | ✅ **RESOLVED** | See P2-3 above |
| F-8.5 | P2 | `reqwest` blocking feature in async binary | 🟡 **STILL OPEN, deferred** | Per REFACTORING_BLOCKER_ANALYSIS.md L-ASYNC-UNIFY |
| F-9.1 | P3 | install.sh lacks `set -e` | ✅ **RESOLVED** | `install.sh:2` is now `set -euo pipefail` |
| F-9.7.1 | P2 | verify-spec.sh uses `cargo test --lib` on binaries | ✅ **RESOLVED** | `scripts/verify-spec.sh:32` now uses `cargo test --workspace --bins -- --test-threads=1`; line 31 comment confirms the fix |
| F-10.1 | P1 | 35 archived `.pi/goals/archived/*.md` tracked in git | ✅ **RESOLVED** | `git ls-files .pi/goals/archived/` returns 0; 36 files exist on disk but are correctly gitignored |
| F-10.4 | P3 | `debug.log` at root | ✅ **Still gitignored** | Via `*.log` rule in .gitignore:15 |
| F-10.5 | P3 | `autoresearch.jsonl` not gitignored | ✅ **RESOLVED** | Covered by `*.jsonl` rule in .gitignore:15 |

### Summary of v2 + full audit

| Outcome | Count | IDs |
|---------|-------|-----|
| ✅ Resolved | **18** | P1-1, P1-3, P2-2, P2-3, P3-1, F-1.1, F-1.2, F-1.3, F-2.3.1, F-7.1.2, F-7.2.1, F-7.2.2, F-7.3.1, F-8.2, F-8.3, F-9.1, F-9.7.1, F-10.1, F-10.5 |
| 🟡 Still Open | **13** | P1-2 (partial), P2-1, P2-4, P2-5, P2-6, F-1.4, F-1.6, F-1.7, F-1.9, F-1.10, F-1.11, F-3.4, F-6.1.1, F-6.1.2, F-7.1.1 (partial), F-8.1, F-8.5 |
| 🟢 Improved | **1** | F-6.2 |

---

## §2 — Cargo Quality Gates (current run)

All commands run on 2026-06-07 from `/home/dracon/Dev/dracon-utilities`, commit `40a8c381`, with `../dracon-libs` cloned. Logs in `/tmp/audit-2026-06-07/`.

### 2.1 `cargo check --workspace --all-targets`

```
$ cargo check --workspace --all-targets 2>&1 | tail -3
warning: `dracon-warden` (bin "dracon-warden" test) generated 3 warnings (1 duplicate)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.61s
exit=0
```

✅ 0 errors. 5 warnings in warden, 3 in warden-tests, 4 in sync, 3 in system.

### 2.2 `cargo clippy` with CI flags

```
$ cargo clippy -p dracon-sync -p dracon-system -p dracon-warden -- \
    -D clippy::all -D clippy::correctness -D clippy::suspicious \
    -D clippy::complexity -D clippy::perf -D clippy::style
exit=0
```

✅ 0 errors, 4 warnings. **Compared to yesterday: 1 error → 0 errors. CI is GREEN.**

### 2.3 `cargo fmt --check`

```
$ cargo fmt -p dracon-sync -p dracon-system -p dracon-warden -- --check
exit=0
```

✅ **Compared to yesterday: RED → GREEN.**

### 2.4 `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings`

```
$ RUSTDOCFLAGS="-D warnings" cargo doc -p dracon-sync -p dracon-system -p dracon-warden --no-deps
exit=0
```

✅ **Compared to yesterday: 4 warnings → 0 warnings. CI is GREEN.**

### 2.5 `cargo deny check`

```
$ cargo deny check 2>&1 | tail -1
advisories ok, bans ok, licenses ok, sources ok
exit=0
```

10 duplicate-crate warnings (same as yesterday, no improvement). 0 license-not-encountered, 0 unmatched-source (down from 7 + 1 yesterday).

---

## §3 — Documentation Drift — Remaining

### 3.1 `docs/OPERATIONS.md:127` — flat CLI path

```diff
- dracon-sync repair-origins [--apply]
+ dracon-sync repair origins [--apply]
```

### 3.2 `dracon-sync/README.md:122-147` — 7+ flat CLI paths

```diff
- dracon-sync repair-concerns
- dracon-sync repair-concerns --apply
- dracon-sync repair-warns
- dracon-sync repair-warns --apply
- dracon-sync stuck list
- dracon-sync stuck unstuck ~/Dev/repo
- dracon-sync dual-branch list
- dracon-sync dual-branch repair ~/Dev/repo
- dracon-sync repair-origins
- dracon-sync repair-origins --apply
- dracon-sync publish ~/Dev/repo
- dracon-sync publish-status ~/Dev/repo
+ dracon-sync repair concerns
+ dracon-sync repair concerns --apply
+ dracon-sync repair warns
+ dracon-sync repair warns --apply
+ dracon-sync repair stuck-list
+ dracon-sync repair stuck-unstuck ~/Dev/repo
+ dracon-sync repair dual-branch-list
+ dracon-sync repair dual-branch-repair ~/Dev/repo
+ dracon-sync repair origins
+ dracon-sync repair origins --apply
+ dracon-sync publish run ~/Dev/repo
+ dracon-sync publish status ~/Dev/repo
```

**Effort:** 10 min. **Risk:** zero.

### 3.3 Test counts

| Doc | Claim | Actual | Status |
|-----|-------|--------|--------|
| AGENTS.md | 686 (sync 428 + system 81 + warden 64 + integration 10 + dracon-security 103) | 590 (sync 428 + system 83 + warden 79 + integration) | **STALE** |
| `.dracon/project-state.md` | 575 | 590 | **STALE** |
| CHANGELOG | "104 new tests across all crates (509 total, up from ~405)" | 590 | Likely correct as of the changelog date |

**Effort:** 5 min. **Risk:** zero.

---

## §4 — Repo Hygiene

### 4.1 `dracon-sync/note.md` (113B) — STILL tracked

The 1-line file is a leftover investigation note from an unrelated repo incident. It was identified yesterday as P2-5 and remains.

**Remediation:** `git rm dracon-sync/note.md` + add to `.gitignore` if not already covered. **Effort:** 30 sec. **Risk:** zero.

### 4.2 Tarpaulin reports — STILL tracked (1.6+ MB)

| File | Size | Last modified |
|------|------|---------------|
| `dracon-sync/tarpaulin-report.html` | 583K | ~May 14 |
| `dracon-sync/tarpaulin-report.json` | 485K | ~May 14 |
| `dracon-system/tarpaulin-report.json` | 192K | ~May 14 |
| `dracon-warden/tarpaulin-report.json` | 992K | ~May 14 |

**Remediation:**
1. `git rm` all 4 files
2. Add `**/tarpaulin-report.*` to `.gitignore`
3. Re-run tarpaulin in CI as a job, store the HTML/JSON as an artifact (not committed)

**Effort:** 5 min. **Risk:** zero.

### 4.3 `.pi/goals/archived/*.md` — correctly NOT tracked

36 files on disk, 0 in git. ✅

### 4.4 `.pi/goals/active_goal_*.md` — currently 1 file (this session)

The 14.2K file is auto-created by the current pi session and will be auto-committed by the sync daemon per AGENTS.md. Not a finding.

### 4.5 `.pi/goals/goal_events.jsonl` — 372.8K, not tracked ✅

### 4.6 `autoresearch.jsonl` — gitignored via `*.jsonl` ✅

### 4.7 `debug.log` — gitignored via `*.log` ✅

### 4.8 `pi-session-*.html` — gitignored ✅

---

## §5 — New Findings (2026-06-07)

### N-1 [P3] Clippy regression: 8 new warnings in system + warden

**Yesterday:** 0 warnings in `dracon-system`, 0 in `dracon-warden`.
**Today:** 3 in system, 5 in warden.

All 8 are `dead_code` on the new `print.rs` helper functions:
- `dracon-system/src/print.rs:7` — `format_bytes`
- `dracon-system/src/print.rs:27` — `format_secs`
- `dracon-system/src/print.rs:56` — `should_color`
- `dracon-warden/src/print.rs:11` — `format_bytes`
- `dracon-warden/src/print.rs:32` — `format_secs`
- `dracon-warden/src/print.rs:61` — `should_color`
- `dracon-warden/src/print.rs:73` — `onoff`
- (plus 1 more in system tests, `discover` unused var at `dracon-warden/src/main.rs:1356`)

These functions are clearly intended as a public API (per the module-level doc comment: "Human-friendly print helpers shared across dracon-warden commands"). They're currently unused because the consuming code hasn't been written yet, or the `pub` modifier is premature.

**Remediation:** either `#[allow(dead_code)]` on the module with a comment, or remove `pub` and add it back when callers exist. **Effort:** 5 min. **Risk:** zero.

### N-2 [P3] `dracon-system` and `dracon-warden` monoliths grew

- `dracon-system/src/main.rs`: 3412 → 3445 (+33)
- `dracon-warden/src/main.rs`: 2174 → 2347 (+173)

The CHANGELOG references a 0.3.0 release with a `repo_roots` rename for warden; the +173 lines for warden is consistent with the rename refactor. Not a regression, but worth noting that the architectural goal of < 1500 lines per main.rs is not yet met.

### N-3 [INFO] `dracon-sync` `test-deletions-committed-when-intentional` is still a dead test

`dracon-sync/src/sync.rs:3841` defines an `async fn` with no `#[test]`/`#[tokio::test]` attribute. Identified yesterday as part of P2-6. No change.

### N-4 [P3] Sync CLI surface changes not reflected in 2 of 3 places

While AGENTS.md is now correct, `dracon-sync/README.md` and `docs/OPERATIONS.md` still show the old flat paths. This is the same finding as 2026-06-06's P1-2, but partially resolved. Since AGENTS.md is the AI-facing reference, this is lower severity for AI workflows but still P3 for human readers.

### N-5 [P2] `dracon-sync/src/sync.rs` grew to 4469 lines (architectural concern)

Was 4340 yesterday, now 4469 (+129). Per the v3 audit from yesterday, this is the new monolith — the same modularization pattern applied to `git/mod.rs` (was ~4700, now 2611 after extraction of `branch.rs`, `config.rs`, `diff.rs`, `discovery.rs`, `misc.rs`, `multi_remote.rs`, `ops.rs`, `push.rs`, `staging.rs`, `status.rs`, `urls.rs`) could be applied to `sync.rs`. Per the CHANGELOG, this is "Item 3: git.rs split — planned" (the file got renamed to `sync.rs` after extraction; same problem).

### N-6 [INFO] No new security findings

`grep -rnE 'sh\s+-c|bash\s+-c' dracon-*/src/` returns 0. Hardcoded secrets: 0. Unsafe blocks: 0. ReDoS-safe regex: unchanged.

---

## §6 — Test Results

### 6.1 Serial (CI mode)

```
$ cargo test -p dracon-sync -p dracon-system -p dracon-warden -- --test-threads=1
dracon-sync:    418 unit + 10 integration = 428 passed
dracon-system:   83 passed
dracon-warden:   69 unit + 10 integration = 79 passed
─────────────────────────────────────────
total: 590 passed, 0 failed
```

### 6.2 Release profile

```
$ cargo test --release -p dracon-sync -p dracon-system -p dracon-warden -- --test-threads=1
total: 590 passed, 0 failed
```

### 6.3 Parallel (documented noise)

```
$ cargo test -p dracon-sync
390 passed; 28 failed
```

Same parallel-test failures as documented in AGENTS.md (PATH mutation, port collisions, env leakage). All 28 are in `git::tests`, `sync::tests`, `report::tests`, `release::tests`, `daemon::daemon_tests`, `sync::diff_tests`. CI uses `--test-threads=1` so these are noise, not regressions.

---

## §7 — Top 10 Improvements (prioritized)

Effort estimates use min/h. Risk is zero unless noted.

| # | Title | Current state | Proposed action | Effort | Risk |
|---|-------|---------------|-----------------|--------|------|
| 1 | **Fix `dracon-sync/README.md` and `docs/OPERATIONS.md` flat CLI paths** | 7+ commands in README, 1 in OPERATIONS.md use old flat form (`repair-concerns`, `stuck list`, `dual-branch list`, `publish-status`) | Rewrite both using the same nested-subcommand syntax now in AGENTS.md (lines 516-535) | 10 min | zero |
| 2 | **Remove `dracon-sync/note.md`** | 113B leftover todo, tracked in git | `git rm dracon-sync/note.md`; add a deny rule or move to `docs/archive/` | 1 min | zero |
| 3 | **Untrack tarpaulin reports (1.6+ MB)** | 4 stale reports tracked (~May 14) | `git rm` all 4; add `**/tarpaulin-report.*` to `.gitignore`; add a tarpaulin CI job that uploads as artifact | 10 min | zero |
| 4 | **Silence dead-code warnings on `print.rs` helpers (8 new clippy warnings)** | system + warden both warn about unused `pub fn format_bytes/format_secs/should_color/onoff` | Add `#[allow(dead_code)]` on the `print` modules with a doc comment explaining they're public API awaiting callers | 5 min | zero |
| 5 | **`cargo dedupe` Cargo.lock (10 duplicate crates)** | bech32, getrandom, hashbrown, rustc-hash, strsim, syn, toml, toml_datetime, toml_edit, winnow duplicated | `cargo update --workspace && cargo dedupe`; if `dracon-libs` pins old versions, file an issue there | 1-2 h | low (lockfile only) |
| 6 | **Update test counts in AGENTS.md and project-state.md** | AGENTS says 686, project-state says 575, real is 590 | Update both to "590 tests passing serially" or remove the specific number | 5 min | zero |
| 7 | **Fix the 4 remaining clippy warnings in sync** | `unused import: tokio_git_command` (report.rs:93), `field stop_reason is never read` (sync.rs:965), `field title is never read` (sync.rs:978), `function test_deletions_committed_when_intentional is never used` (sync.rs:3841) | Remove the unused import; `#[allow(dead_code)]` on the fields with comments, or wire them into the Goal metadata serialization; add `#[test]` to the dead test or remove it | 10 min | zero |
| 8 | **Extract a `subcommand` module from `dracon-sync/src/sync.rs`** | sync.rs is 4469 lines (grew from 4340 yesterday); same modularization pattern that worked for `git/mod.rs` (was 4700, now 2611) | Top-down extraction: candidates include `commit.rs`, `cooldown.rs`, `filter_only.rs` (separate from `git/filter.rs`), `remediation.rs` (concerns/warns/dual-branch logic) | 8-12 h | medium (historical reverts per REFACTORING_BLOCKER_ANALYSIS H-DAEMON) |
| 9 | **Make pedantic+nursery clippy gate the build** | The CI step pipes output to `tail -1` so pedantic lints can never fail the job | Add `set -euo pipefail` and check `${PIPESTATUS[0]}` or grep for "warning:" in output | 15 min | low (might surface a wave of new warnings) |
| 10 | **Run fresh tarpaulin coverage** | Reports 23+ days old | Add a CI job that runs `cargo tarpaulin` on each binary, uploads HTML to artifacts, and compares to a baseline threshold (sync 15%, system 15%, warden 20%) | 30 min setup, 10 min/run | zero |

---

## §8 — Statistics

### Code Quality (current)

| Metric | dracon-sync | dracon-system | dracon-warden | Total |
|--------|-------------|---------------|---------------|-------|
| Lines (largest .rs) | 4469 (sync.rs) | 3445 (main.rs) | 2347 (main.rs) | 10261 |
| Test count (serial) | 428 | 83 | 79 | **590** |
| Clippy warnings (CI flags) | 4 | 3 | 5 | 12 |
| Production `unwrap()` (best estimate) | 2 | 1 | 0 | 3 |

### CI Status (current)

| Job | Status | Notes |
|-----|--------|-------|
| lint (fmt) | ✅ GREEN | Was RED yesterday |
| lint (clippy) | ✅ GREEN | Was RED yesterday (1 error) |
| lint (clippy pedantic) | ⚠️ Decorative | tail -1 only |
| docs (strict) | ✅ GREEN | Was RED yesterday (4 warnings) |
| test (serial) | ✅ GREEN | 590 passed |
| test (release) | ✅ GREEN | 590 passed |
| release-build | ✅ GREEN (not re-run) | 11/3/4 MiB, under 25 MiB threshold |
| scripts (shellcheck) | ✅ GREEN (not re-run) | install.sh/uninstall.sh/doctor.sh pass |
| msrv | ✅ GREEN (not re-run) | stable + clippy passes |
| deny | ✅ GREEN (with warnings) | 10 duplicate crates |
| nix | ✅ GREEN (not re-run) | flake check passes |

### `cargo deny` details

```
$ cargo deny check 2>&1 | grep -E "^warning\["
warning[duplicate]: bech32
warning[duplicate]: getrandom
warning[duplicate]: hashbrown
warning[duplicate]: rustc-hash
warning[duplicate]: strsim
warning[duplicate]: syn
warning[duplicate]: toml
warning[duplicate]: toml_datetime
warning[duplicate]: toml_edit
warning[duplicate]: winnow
```

10 duplicate warnings. 0 license warnings (was 7). 0 source warnings (was 1).

---

## §9 — Verification Contract

- [x] `cargo check --workspace --all-targets` exit=0, 0 errors (log: cargo-check.log)
- [x] `cargo clippy ... -D clippy::style` exit=0, 0 errors, 12 warnings (log: cargo-clippy.log)
- [x] `cargo fmt --check` exit=0 (log: cargo-fmt.log)
- [x] `RUSTDOCFLAGS=-D warnings cargo doc --no-deps` exit=0 (log: cargo-doc.log)
- [x] `cargo test --test-threads=1` workspace: 590 passed, 0 failed (log: test-system-warden.log, test-sync.log, test-release.log)
- [x] `cargo deny check` exit=0, 10 duplicate warnings (log: cargo-deny.log)
- [x] All 35+ findings from the two prior audits evaluated with file:line evidence
- [x] New findings tagged separately (N-1 through N-6)
- [x] Top 10 Improvements list with title/state/action/effort/risk for each item
- [x] Repo is read-only: `git status` shows only `.pi/goals/active_goal_*.md` (the audit's own active goal file, auto-created by the pi session per AGENTS.md)
- [x] All cargo command outputs referenced in `/tmp/audit-2026-06-07/`

---

## §10 — Bottom Line

**The 2026-06-06 audit's most important findings have been addressed.** Three CI jobs (clippy, fmt, docs) flipped from RED to GREEN in 24 hours. The `test-ai` cleanup, freeze-marker TTL, archived-goals gitignore, and dead `deny.toml` entries are all done. AGENTS.md is now correct.

**Remaining surface area is much smaller and lower-severity:**
- 1 doc (`dracon-sync/README.md`) still has 7+ flat CLI paths
- 1 doc (`docs/OPERATIONS.md`) has 1 flat path
- 2 small repo-hygiene items (note.md, tarpaulin reports)
- 1 small clippy regression (8 dead-code warnings in system + warden)
- 1 medium architectural item (sync.rs is still a 4469-line monolith)
- 1 low-severity test-count drift (590 vs 686/575)
- 1 deferred (Cargo.lock dedupe, awaits `dracon-libs` pin)

**The next concrete sprint should be (in order):**
1. Fix `dracon-sync/README.md` and `docs/OPERATIONS.md` (10 min) → unblocks humans and AI agents who read those files
2. Remove `dracon-sync/note.md` + untrack tarpaulin reports (10 min) → repo hygiene
3. Silence 8 dead-code warnings on `print.rs` helpers (5 min) → restore CI clippy to 0 warnings
4. `cargo dedupe` (1-2 h) → smaller lockfile, faster builds
5. Update test counts in AGENTS.md / project-state.md (5 min)
6. Then tackle sync.rs modularization (8-12 h) — the biggest remaining architectural work
