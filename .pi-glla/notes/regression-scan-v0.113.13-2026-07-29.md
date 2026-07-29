# Post-completion regression scan — v0.113.12 + v0.113.13

> **Date**: 2026-07-29 ~18:25 UTC (reviewer follow-up after the janitor
> spot-check close-out)
> **Scope**: regressions from v0.113.12 (legend footer) and v0.113.13
> (repos table v2 + exclusion-aware dirty semantics).

## Results

| # | Check | Result |
|---|-------|--------|
| 1 | `cargo test --workspace --locked` | **1192 passed, 0 failed** |
| 1 | `cargo clippy --workspace --locked -- -D warnings` | **clean** |
| 2 | Rich tier (COLUMNS=200) | renders; legend present (2 grep hits = header + body) |
| 2 | Compact tier (COLUMNS=100) | renders; legend absent (0 hits) — width-gate works |
| 2 | `repos --json` | parses (`python3 -m json.tool` OK) |
| 2 | `repos junk-runner` detail | renders full detail view |
| 3 | Classifier not over-suppressing | junk-runner live row: `⏳ dirty 0m · 1 excl · 4 mod` — excluded file and 4 committable files correctly separated in ONE row |
| 4 | Daemon health | `active`; 0 ERROR/panic lines since 12:07 UTC restart; 0 real 🟣 PENDING rows (the 1 grep hit was the legend's own PUSH explainer line) |
| 5 | Session residue | meta-repo clean (only the expected untracked nested-repo dirs) |

**Verdict: PASS — no regressions from either release.**

## Non-regression observation (flagged for operator decision)

`convos` shows 🟡 WARN `⏳ dirty 3h · 1-2 mod`. Investigation:

- The dirt is 2 affiliate markdown files in `extension-profit/affiliate/`
  with mtimes 1-4 min old — a convos-side agent loop is **actively
  churning them right now**.
- The daemon's settle logic is deliberately deferring ("daemon handles
  after changes settle" hint); it last committed convos at 15:48 local
  and has committed OTHER repos (junk-runner 5 files at 19:17) since.
- The dirty-clock measures first-dirty time, so 3h of continuous churn
  escalates to WARN even though nothing is stalled.

This is **NOT a v0.113.13 regression** — v0.113.13 only changed *which
files count* as dirty, not the dirty-clock WARN escalation; convos
would WARN identically on v0.113.11. But it is the same *family* as
the junk-runner false WARN we just fixed: **WARN fires while the
daemon is intentionally not committing** (there: policy exclusion;
here: settle-deferral during active churn).

Candidate follow-up (operator's call): reset/suppress the dirty-clock
while a repo's dirty files were modified within the settle window —
i.e. WARN only when dirt is both committable AND the files have been
quiet long enough that the daemon *should* have committed them.
Deferred — out of scope for a regression scan.
