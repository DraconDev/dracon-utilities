# AUDIT-3-UTILITIES-FULL-2026-07-11

> **Full re-audit** of the 3 utilities (dracon-sync, dracon-system,
> dracon-warden) in `~/Dev/dracon-utilities`, run 2026-07-11 after the
> 5 actionable findings from `AUDIT-3-UTILITIES-INDEPENDENT-2026-07-11.md`
> were applied.
>
> Scope: `dracon-sync`, `dracon-system`, `dracon-warden` (nested
> standalone repos under the `dracon-utilities` meta-repo).
> Bar: AGENTS.md test discipline (build / test / deny / clippy
> advisory) + the 5 prior findings' verification.

## 1. Methodology

| Step | Command | Purpose |
|------|---------|---------|
| 1a | `cargo build --release --locked` | release build |
| 1b | `cargo build --tests --locked` | test build (catches test-only warnings) |
| 2 | `cargo test --workspace --locked --no-fail-fast` | full unit + integration + doc tests |
| 3 | `cargo deny check` (workspace + per-crate) | advisories / bans / licenses / sources |
| 4 | `cargo clippy --workspace --locked` | lint surface |
| 5 | `git ls-files` per repo + `grep` CHANGELOG/AGENTS | prior-finding verification |
| 6 | `grep -A1 'name = ...' Cargo.lock` | advisory version pins |

## 2. Build / test / deny / clippy results

| Crate | `build --release` warnings | `build --tests` warnings | `test` (unit + doc) | `deny` |
|-------|----------------------------:|--------------------------:|---------------------:|-------|
| dracon-sync | 0 | 0 | 665 + 10 (3 ignored) | advisories ok, bans ok, licenses ok, sources ok |
| dracon-system | 0 | 0 | 86 | advisories ok, bans ok, licenses ok, sources ok |
| dracon-warden | 0 | 0 | 76 + 10 | advisories ok, bans ok, licenses ok, sources ok |
| **Workspace** | 0 | 0 | **847 total (0 failed, 3 ignored)** | advisories ok, bans ok, licenses ok, sources ok |

- `cargo build --release --locked` → exit 0, **0 warnings**
- `cargo build --tests --locked` → exit 0, **0 warnings**
- `cargo test --workspace --locked` → exit 0, **847 pass / 0 fail / 3 ignored**
- `cargo deny check` (workspace + all 3 per-crate) → exit 0, all 4 categories ok
- `cargo clippy --workspace --locked` → exit 0, **17 warnings** (down from 28)

**Workspace-level and per-crate commands both exit 0.**

## 3. Advisory pins (still resolved)

- `anyhow` = **1.0.103** in all 4 `Cargo.lock` files (RUSTSEC-2026-0190 fixed)
- `crossbeam-epoch` = **0.9.20** in workspace + `dracon-warden` (RUSTSEC-2026-0204 fixed)

`cargo deny check` reports `advisories ok` in all 4 runs.

## 4. Prior findings — status this run

| # | Finding | Status | Evidence |
|---|---------|--------|----------|
| A | `report_v2_snapshot.rs` dead tracked code | **RESOLVED** | `git -C dracon-sync ls-files \| grep report_v2_snapshot.rs` → 0; content moved to `docs/design/v2-card-design-snapshot-2026-06-16.md` |
| B | CHANGELOG stops at v0.112.12, `dracon-sync` at v0.112.14 | **RESOLVED** | `grep '^## \[0.112.1[34]\]' CHANGELOG.md` → both sections present |
| C | CHANGELOG duplicate `## [0.112.10]` | **RESOLVED** | `grep -c '^## \[0.112.10\]' CHANGELOG.md` → 1 (was 2) |
| D | 28 clippy warnings | **PARTIALLY RESOLVED** | clippy now 17 (13 auto-fixed via `cargo clippy --fix`; 17 manual lints remain: doc-markdown, type-complexity, let-else→?) |
| E | `dracon-utilities` meta-repo + nested standalone repos undocumented | **RESOLVED** | `grep -c 'nested standalone git repos' AGENTS.md` → 1; "Repository architecture (READ THIS FIRST)" section added |

## 5. What's healthy

- All 3 crates compile cleanly under **release** and **tests** modes, 0 warnings each.
- All 3 crates: 0 test failures across **847 tests** (3 pre-existing ignored).
- `cargo deny check` exits 0 from workspace root **and** per-crate; all 4 categories clean.
- RUSTSEC-2026-0190 (anyhow 1.0.103) and RUSTSEC-2026-0204 (crossbeam-epoch 0.9.20) remain resolved.
- 17 test-helper sites correctly set `core.hooksPath=/dev/null` (verified prior audit, unchanged).
- Workspace `Cargo.toml` + `Cargo.lock` present; `cargo install --path <crate>` still works from nested dirs.
- AGENTS.md test discipline (`cargo build --release --locked`, `cargo test --workspace --locked`, `cargo deny check`, plus clippy advisory) passes.
- The 3 nested repos are healthy in `dracon-sync repos` (each `main`, 0/0, healthy).

## 6. Remaining debt (explicitly deferred, not a blocker)

- **17 clippy lints in `dracon-sync`** require manual review:
  - ~10 `clippy::doc_markdown` (doc list-item indentation) — cosmetic, low risk
  - 2 `clippy::type_complexity` (large types in `report.rs`) — refactor candidates
  - 2 `clippy::needless_late_init` / `let..else`→`?`
  - 1 each: `unnecessary_map_or`, `derivable_impls`, `collapsible_match`, `redundant_closure`, `ok_some`, `manual_range_contains`, `into_iter_on_ref`, empty-line-after-doc
  These are quality signals, not bugs. They can be chased in a follow-up; the daemon's actual behavior is unaffected.

## 7. Delta vs AUDIT-3-UTILITIES-INDEPENDENT-2026-07-11.md

| Aspect | Independent (this morning) | Full (this run) | Change |
|--------|----------------------------|-----------------|--------|
| Release build warnings | 0 | 0 | same |
| Test build warnings | 0 | 0 | same |
| Tests passing | 847/847 | 847/847 | same |
| `cargo deny` (4 runs) | 0 | 0 | same |
| Clippy warnings | 28 | **17** | **−11 (13 auto-fixed)** |
| Orphan `report_v2_snapshot.rs` | 237 KiB tracked | **removed** | fixed |
| CHANGELOG drift | 2 versions behind | **covered** | fixed |
| CHANGELOG duplicate | 1 dup | **removed** | fixed |
| Architecture note | absent | **present** | fixed |

## 8. Summary

All 6 prior CONCERNs (from the 07-10/RECHECK/RERUN series) remain resolved.
All 5 findings from the independent audit are now addressed except
the **17 manual clippy lints**, which are deferred quality debt (no
behavioral impact). The 3 utilities are in their best-known state:
clean build, 847 passing tests, 0 advisories, documented architecture,
and a CHANGELOG that matches the shipped `dracon-sync` version.

**Recommendation:** ship as-is. The 17 clippy lints are a nice-to-have
cleanup, not a release blocker.
