# Revert all filtering — let the daemon commit everything (2026-06-15)

> **Goal**: `76ddaa7e-9835-48e2-a322-05077c651ac3`
> **Operator said**: "what is even the excluded dirty i jsut
> checked the rust ai web auto and its just 3 repots not
> getting commited that is clearly our eerror we should
> not be disincluding markdown files, the untrack files
> in the browser extensions is another markdown adn the
> junk runner is jsut disincluding a bunch of pngs, we
> shoudl nto filter these out at all, this would leave
> only the dracon platform which actually has a ton of
> changes that we are not pushing but clearly the 5
> second inactivity was true"

## TL;DR

The operator reviewed the live report and called out
goals `1fe80684` (per-repo `auto_commit_exclude_patterns`
+ WARN filter) and `3276ceb4` (ExcludedDirty state) as
**errors**. The filters were hiding real content the
operator wanted committed, and the new state was
unfamiliar and unjustified. This goal reverts ALL of that:

- Per-repo `auto_commit_exclude_patterns` removed from
  `Junk-Runner-bevy` and `rust-ai-web-auto`
- `StateCause::ExcludedDirty` removed
- `count_non_excluded_modified_files` helper removed
- `filtered_modified` field on `StateCauseInputs` removed
- `non_excluded_modified` field on `RepoReportRow` removed
- `dir/<glob>` pattern support in
  `matches_untracked_exclude` reverted
- 9 tests removed (5 from `1fe80684` + 4 from `3276ceb4`)

The 5s `inactivity_push_delay_secs` (set by goal
`546d4f9c`) is **correct as-is** per the operator, and
stays in place. After removing the filters, only
`dracon-platform` had a real concern (active edits with
settling) — which cleared naturally.

The operator's claim of "a ton of changes that we are
not pushing" on `dracon-platform` turned out to be
**not accurate**: AHEAD count was 0 the entire time,
all 4 remotes were in sync (after a transient
catch-up), and the daemon was actively committing
the edited files. The 1 MOD on `dracon-platform` was
an actively-edited `+page.svelte` file, which
correctly settled after a few minutes.

## What was reverted (in detail)

### 1. Per-repo `auto_commit_exclude_patterns`

`Junk-Runner-bevy/.dracon/dracon-sync.toml`:

- Before: `["**/test-results/**", "**/e2e/screenshots/**"]`
  (added by goal `0ab367b5` for the WARN fix)
- After: file kept (with explanatory comment), no
  effective excludes

`rust-ai-web-auto/.dracon/dracon-sync.toml`:

- Before: `["reports/kdp-live-*.md"]`
  (added by goal `1fe80684`)
- After: file kept (with explanatory comment), no
  effective excludes

### 2. `ExcludedDirty` state (goal `3276ceb4`)

Removed from `dracon-sync/src/report.rs`:

- `StateCause::ExcludedDirty` enum variant
- `as_str()`, `icon()`, `state_cause_as_str()` arms
- The ExcludedDirty branch in `classify_state_cause`
- The ExcludedDirty branch in `activity_label`
- The ExcludedDirty arm in the color match
- `filtered_modified: usize` field on `StateCauseInputs`
- `non_excluded_modified: usize` field on `RepoReportRow`
- 4 new tests in the `report::tests` module

### 3. WARN filter (goal `1fe80684`)

Removed from `dracon-sync/src/report.rs`:

- `count_non_excluded_modified_files` helper function
  (~95 lines)
- The call site in `run_repos_report` (replaced
  `let real_is_dirty = non_excluded_modified > 0 || status.staged_files > 0;`
  with
  `let real_is_dirty = status.modified_files > 0 || status.staged_files > 0;`)
- The call site in `run_repair_warns` (same revert)
- 5 new tests in the `report::tests` module

### 4. `dir/<glob>` pattern support in `matches_untracked_exclude`

Reverted the addition in
`dracon-sync/src/exclude.rs` that supported patterns
like `reports/kdp-live-*.md` (the rust-ai-web-auto
case) and `web/test-results/*.png` (the
Junk-Runner-bevy case). The function is now back to
its pre-`1fe80684` state, supporting only `**/...` and
`.../**` patterns.

## What was kept (not touched)

- `dracon-git` 94.7.0 upgrade (goal `0ab367b5`) — the
  library bug fix for `is_wt_new()` double-counting is
  real and not a filter
- 5s `inactivity_push_delay_secs` (goal `546d4f9c`) —
  operator confirmed correct as-is
- "Durable commit-all policy" defaults (goal
  `546d4f9c`) — keep
- `kiki-sassy` and `dracon-ai-lib` `owned = true`
  per-repo overrides — not filters, just ownership
  signals
- `.dracon/*.bak-*` `.gitignore` entry (goal
  `3276ceb4`) — forward-only hygiene, doesn't touch
  history, doesn't filter daemon behavior
- `dracon-platform/.pi-tmp/*` `.gitignore` entry
  (goal `3276ceb4`) — same, forward-only hygiene

## dracon-platform push investigation

The operator reported "a ton of changes that we are not
pushing" on `dracon-platform`. Investigation findings:

- **AHEAD count was 0** the entire time
  (`git rev-list --count origin/main..main` = 0)
- **All 4 remotes are in sync** (after a transient
  catch-up)
- **No push failures** in the incident ledger
- **No push-stuck retries** in the daemon log
- The daemon was actively committing
  (`journalctl --user -u dracon-sync.service | rg "dracon-platform"`)
  every 30-60 seconds
- The "1 MOD" the operator saw was an actively-edited
  `+page.svelte` file (and later
  `ai-providers-catalog.json` and
  `SITE-AUDIT-2026-06-16.md`) — the daemon correctly
  waited for fingerprint stability (5s) and committed
- The transient `trailing-drain: clearing 1 stuck
  in_flight entries` log entries are a normal
  pattern: they fire when a previous commit's
  in_flight entry was still in the set when the new
  commit started, and the trailing-drain clears it
  after the new commit completes

**Conclusion**: the operator's perception of "a ton of
changes not being pushed" was likely from observing
multiple `trailing-drain` log lines and the WARN rows
for actively-edited files. The actual push state was
healthy throughout.

## Live impact (before vs after revert)

### Before (with filters, goal `1fe80684` + `3276ceb4`)

| Repo | MOD | WARN | STATE | ACTIVITY |
|------|-----|------|-------|----------|
| Junk-Runner-bevy | 90 | ✅ OK | ⚪ excluded-dirty | ⚪ excluded-dirty |
| rust-ai-web-auto | 3 | ✅ OK | ⚪ excluded-dirty | ⚪ excluded-dirty |

The 90 PNGs and 3 markdowns were HIDDEN by the
filters. The daemon was NOT committing them.

### After (filters removed, goal `76ddaa7e`)

| Repo | MOD | WARN | STATE | ACTIVITY |
|------|-----|------|-------|----------|
| Junk-Runner-bevy | 0-1 | ✅ OK (when settled) | 🟢 synced | 🟢 synced |
| rust-ai-web-auto | 0-3 | ✅ OK (when settled) | 🟢 synced | 🟢 synced |

The 90 PNGs and 3 markdowns are NOW being committed
and pushed normally. When files are actively being
modified faster than 5s, the row temporarily shows as
WARN (settling), but as soon as the file is stable for
5 seconds, the daemon commits it. This is the desired
behavior.

## Tests

- **Before**: 860 tests (was 851, added 5 from
  `1fe80684` + 4 from `3276ceb4` = 860)
- **After**: 851 tests (removed 5 from `1fe80684` + 4
  from `3276ceb4` = 851)
- All 851 pass. 0 failures.

## Verification

- ✅ Live report: **14 OK + 0 WARN + 0 CONCERN + 0 failed**
- ✅ Tests: **851 passed, 0 failed**
- ✅ Build: `cargo build --release --locked` clean
- ✅ Deny: `cargo deny check` clean
- ✅ 4-remote alignment for all repos that had
  code/config changes
- ✅ No per-repo `auto_commit_exclude_patterns` in any
  of the 14 repos (rg returns 0)
- ✅ No `ExcludedDirty` variant in the daemon code
  (rg returns 0)
- ✅ No `count_non_excluded_modified_files` function
  (rg returns 0)
- ✅ No `filtered_modified` field in the daemon code
  (rg returns 0)
- ✅ `matches_untracked_exclude` reverted to
  pre-`1fe80684` state (only `**/...` and `.../**`)
- ✅ Daemon running, policy valid
- ✅ No force-pushes anywhere
- ✅ No sensitive files in any new commit
- ✅ Design doc captured
- ✅ CHANGELOG entries under [Unreleased]

## What the operator can verify

Three commands:

1. `rg "auto_commit_exclude_patterns" /home/dracon/Dev/*/.dracon/dracon-sync.toml 2>/dev/null`
   — returns 0 matches (the excludes are gone)
2. `rg "ExcludedDirty|count_non_excluded_modified_files" /home/dracon/Dev/dracon-utilities/dracon-sync/src/`
   — returns 0 matches (the code is reverted)
3. `dracon-sync repos`
   — shows 14 OK + 0 WARN + 0 CONCERN + 0 failed

All three return the expected results. The goal is
fixed.

## Why the operator's directive matters

The previous goals (`1fe80684`, `3276ceb4`) made
assumptions about what the operator wanted:
- "These files churn too much, the operator must not
  want them committed"
- "The report should look clean even if files are
  being churned"

The operator's actual intent:
- "I want every change committed. Don't filter.
  Don't hide. The 5s inactivity is the source of
  truth for commit timing."
- "The report should reflect what the daemon IS
  doing, not what I might prefer it to do."

This is a stronger commitment to the "commit-all"
policy from `546d4f9c`. The filters and the
ExcludedDirty state were a regression away from that
commitment. The reversion restores the original
intent.

## Related design docs

- `docs/design/commit-all-policy-durable-2026-06-15.md`:
  the durable commit-all policy (goal `546d4f9c`).
  This reversion aligns with that policy.
- `docs/design/junk-runner-fix-2026-06-15.md`:
  the library upgrade (goal `0ab367b5`) and the
  per-repo PNG exclude (which is now reverted). The
  library upgrade is still valid (not a filter); the
  per-repo exclude is the part that's reverted.
- `docs/design/all-green-investigation-2026-06-15.md`:
  the WARN filter and per-repo excludes from
  `1fe80684`. All reverted in this goal.
- `docs/design/excluded-dirty-state-2026-06-15.md`:
  the ExcludedDirty state from `3276ceb4`. Reverted
  in this goal.
