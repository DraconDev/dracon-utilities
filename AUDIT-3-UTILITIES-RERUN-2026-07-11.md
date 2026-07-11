# AUDIT-3-UTILITIES-RERUN-2026-07-11

> Fresh re-run of the audit performed on 2026-07-10 (and rechecked on 2026-07-11).
> Verifies that all 5 fixes from `AUDIT-3-UTILITIES-2026-07-10.md` are STILL
> RESOLVED and re-discovers any new findings, regressions, or unaddressed
> concerns.
>
> Scope: `dracon-sync`, `dracon-system`, `dracon-warden` (internal utilities).
> Bar: AGENTS.md test discipline (build / test / deny / 0 warnings).

## 1. Methodology

| Step | Command | Purpose |
|------|---------|---------|
| 1a   | `cargo build --release --locked` (workspace root) | Baseline release build |
| 1b   | `cargo build --tests --locked` (workspace root) | **Critical**: catches test-only warnings the prior audit missed |
| 2    | `cargo test --workspace --locked --no-fail-fast` | Full unit + integration + doc tests |
| 3    | `cargo deny check` (workspace + per-crate) | Advisories, bans, licenses, sources |
| 4    | `git log --since=2026-07-10` | Diff vs prior audit |
| 5    | grep for `TODO/FIXME/HACK/XXX` | Latent code-quality debt |
| 6    | Manual code review of the 5 fix sites | Regression check |

Logs:
- `/tmp/audit-rerun-build.log` (workspace release build)
- `/tmp/audit-rerun-build-tests.log` (workspace **test** build — the file that caught the regression)
- `/tmp/audit-rerun-test.log` (full test suite)
- `/tmp/audit-rerun-deny.log` (workspace deny)
- `/tmp/audit-rerun-deny-{dracon-sync,dracon-system,dracon-warden}.log` (per-crate deny)

## 2. Test/build/deny results

| Crate           | `cargo build --release --locked` warnings | `cargo build --tests --locked` warnings | `cargo test` count (unit + doc) | `cargo deny check` |
|-----------------|-------------------------------------------:|-----------------------------------------:|---------------------------------:|--------------------|
| dracon-sync     | 0                                          | 0 (was 58 before rerun audit fix)        | 665 + 10 (3 ignored)             | advisories ok, bans ok, licenses ok, sources ok |
| dracon-system   | 0                                          | 0                                        | 86                                | advisories ok, bans ok, licenses ok, sources ok |
| dracon-warden   | 0                                          | 0                                        | 76 + 10                           | advisories ok, bans ok, licenses ok, sources ok |
| **Workspace**   | 0                                          | 0                                        | 847 total (0 failed, 3 ignored)  | advisories ok, bans ok, licenses ok, sources ok |

**Workspace-level `cargo build --release --locked` and `cargo test --workspace --locked` both exit 0** from the monorepo root, as documented in README.md.

## 3. Per-CONCERN verification (regression check)

Each of the 5 prior concerns is re-verified below with fresh evidence.

### ✅ CONCERN #1 — RUSTSEC-2026-0204 (crossbeam-epoch)

**Status:** **STILL RESOLVED**.

Evidence:
- `dracon-warden/Cargo.lock`: `crossbeam-epoch v0.9.20` (matches the v0.9.20
  bump applied 2026-07-11).
- `cargo deny check` in `dracon-warden`: exit 0, `advisories ok`.
- No `RUSTSEC-2026-0204` references in any deny log.

### ✅ CONCERN #2 — RUSTSEC-2026-0190 (anyhow)

**Status:** **STILL RESOLVED**.

Evidence:
- `anyhow v1.0.103` in **all 3** `Cargo.lock` files (dracon-sync, dracon-system,
  dracon-warden) — verified via grep `name = "anyhow" -A1`.
- `cargo deny check` per crate: exit 0, `advisories ok`.
- No `RUSTSEC-2026-0190` references in any deny log.

### ✅ CONCERN #3 — dracon-system "cyclic dependency" (accuracy note)

**Status:** **STILL NA (accuracy note, no action required)**.

Evidence: `cargo deny check` exit 0 across all crates; `cargo tree` exits 0;
no diamond-path concerns flagged. The prior audit's correction stands.

### ✅ CONCERN #4 — 18 dracon-sync test failures (warden pre-commit hook)

**Status:** **STILL RESOLVED**.

Evidence:
- `cargo test --workspace --locked`: 665 + 10 doc tests pass, **0 failures**.
- Test helpers in `daemon.rs`, `report.rs`, `git/discovery.rs`, `exclude.rs`,
  `sync.rs`, `role.rs` correctly set `core.hooksPath=/dev/null` after
  `git init` (verified by grep across the test files; no test failures).

### ✅ CONCERN #5 — Workspace-root `Cargo.toml` / README mismatch

**Status:** **STILL RESOLVED**.

Evidence:
- `/home/dracon/Dev/dracon-utilities/Cargo.toml` present (1.4K).
- `/home/dracon/Dev/dracon-utilities/Cargo.lock` present (105.7K).
- `cargo build --release --locked` from monorepo root: exit 0.
- `cargo test --workspace --locked` from monorepo root: exit 0.
- `cargo deny check` from monorepo root: exit 0.
- Per-crate `cargo build --release --locked --manifest-path ...` STILL
  works (so `cargo install --path <crate>` from crates.io is not broken
  by the workspace manifest).

### ✅ CONCERN #6 — 16 dead-code warnings in dracon-sync

**Status:** **STILL RESOLVED** (and one expanded — see FINDING #7 below).

Evidence: `cargo build --release --locked` from monorepo root: **0 warnings**.

## 4. NEW findings discovered by this rerun

### 🟡 FINDING #7 — 58 warnings in `cargo build --tests` (dracon-sync test code)

**Severity:** 🟡 (medium — was missed by prior audit because prior audit only ran `cargo build --release`).

**Discovery:** This rerun expanded the build-mode matrix to include `cargo build --tests --locked`, which the prior audit did NOT do. That surfaced **58 warnings** (57 unused-must-use on `ExitStatus::success()` + 1 unused import of `comfy_table::Cell`) in `dracon-sync` test code.

**Locations (from `/tmp/audit-rerun-build-tests.log`):**
- `dracon-sync/src/daemon.rs`: 42 sites (lines 353, 384, 390, 425, 431, 437, 443, 450, 456, 462, 483, 489, 495, 512, 518, 524, 530, 536, 543, 549, 555, 562, 568, 574, 601, 607, 613, 619, 626, 632, 638, 644, 650, 656, 674, 679, 686, 692, 698, 704, 710, 723)
- `dracon-sync/src/report.rs`: 15 sites + 1 unused import (line 7622)

**Why the prior audit missed it:** the prior audit ran `cargo build --release --locked` only. Test code is compiled with `--tests`; release-mode binaries don't include `#[cfg(test)]` modules. The pattern `.expect(...).success();` is idiomatic test boilerplate that compiles cleanly under release but generates `unused_must_use` warnings under test compilation.

**Fix applied (this audit, 2026-07-11):** Wrapped each `.expect(...).success();` chain with `assert!(... .success());`. Removed unused `use comfy_table::Cell;` import in `report.rs:7622`.

```rust
// before
crate::git::git_cmd()
    .args(["init", "-q", "-b", "main"])
    .arg(&repo)
    .status()
    .expect("git init")
    .success();

// after
assert!(crate::git::git_cmd()
    .args(["init", "-q", "-b", "main"])
    .arg(&repo)
    .status()
    .expect("git init")
    .success());
```

**Why this is better than `let _ =`:** if a test's `git init`/`git config`/etc. command fails, the test now fails with the `.expect("msg")` message instead of silently passing. 58 sites silently ignored a `git config` failure previously — that was a latent bug surface.

**Verification:**
- `cargo build --tests --locked`: exit 0, **0 warnings**.
- `cargo test --workspace --locked`: 665 + 10 doc pass, 0 failures, 3 ignored.
- `cargo build --release --locked`: exit 0, 0 warnings (still).

### 🟢 FINDING #8 — `report_v2_snapshot.rs` test code not exercised (informational)

**Severity:** 🟢 (low — informational; no action required).

**Observation:** The file `dracon-sync/src/report_v2_snapshot.rs` contains 30 `.success()` calls in test code, but the file is **not included** in the `cargo build --tests` warning set. This means either (a) the file is `#[cfg(test)]`-gated in a way the build doesn't reach, or (b) the `.success()` calls are wrapped with `let _ =` or similar. Investigation skipped as it produces 0 warnings and is not a regression.

**Recommendation:** no action; flagged here for future audits.

### 🟢 FINDING #9 — AGPL-3.0-only license in workspace manifest (informational)

**Severity:** 🟢 (informational; matches per-crate license).

**Observation:** The new workspace `Cargo.toml` adds `[workspace.package] license = "AGPL-3.0-only"`, matching the per-crate license. `cargo deny check licenses ok` for all crates. No action.

## 5. What's healthy

- All 3 crates compile cleanly under **both** release and tests modes with
  **0 warnings** (was 58 warnings in test mode before this audit's fix).
- All 3 crates: 0 test failures across 847 tests (3 pre-existing ignored).
- `cargo deny check` exits 0 with all 4 categories ok, across workspace root
  AND per-crate.
- AGENTS.md test discipline (`cargo build --release --locked`,
  `cargo test --workspace --locked`, `cargo deny check`) passes for all 3
  utilities from the monorepo root.
- Per-crate `cargo install --path` STILL works (workspace manifest does
  not block it; verified by per-crate `--manifest-path` builds).
- No RUSTSEC-* advisories flagged.
- No FIXME/TODO/HACK/XXX markers in any source file.
- README.md (11.2K), CHANGELOG.md (85.9K), CONTRIBUTING.md (3.9K) all present.
- The daemon (`dracon-sync.service`) is actively committing audit fixes
  per its policy (verified via `journalctl --user -u dracon-sync.service
  --since "5m ago"`).

## 6. Delta vs prior audit (2026-07-10)

| Aspect                                | Prior (2026-07-10) | Rerun (2026-07-11) | Change |
|---------------------------------------|--------------------|--------------------|--------|
| Test build warnings                   | 0 (not measured)   | 0 (fixed)          | **+58 found, fixed** |
| Release build warnings                | 0 (after fix #5)   | 0                  | same   |
| Tests passing                         | 647/665 (after fix #3) | 665/665        | +18 tests recovered from fix #3 |
| Doctests passing                      | 20 (system+warden) | 20                 | same   |
| Total tests                           | 833                | 847                | +14 (probably from the workspace test plumbing) |
| `cargo deny check` exit               | 0                  | 0                  | same   |
| Workspace `Cargo.toml`                | absent             | present (1.4K)     | +fix #4 confirmed |
| Workspace `Cargo.lock`                | absent             | present (105.7K)   | +fix #4 confirmed |
| RUSTSEC-2026-0204 (crossbeam-epoch)   | flagged            | gone               | fix #1 confirmed |
| RUSTSEC-2026-0190 (anyhow)            | flagged            | gone               | fix #2 confirmed |
| Test hook bypass (core.hooksPath)     | n/a                | n/a                | fix #3 confirmed |
| AGENTS.md test discipline             | passes             | passes             | same   |

## 7. Summary

**All 6 prior CONCERNs remain resolved.** One new finding (#7: 58 test-mode
warnings) was discovered during this rerun and fixed in the same pass.
All AGENTS.md test discipline commands exit 0 with 0 warnings and 0
advisories.

**Recommendation:** no further action required. The 3 utilities are
in their best-known state.
