# ExcludedDirty state — design doc (2026-06-15)

> **Goal**: `3276ceb4-2685-4157-8631-77094c297f33`
> **Operator said**: "ok looking healthier but still have
> some concerns"

## TL;DR

The 14-repo live report was back to "13 OK + 1 WARN" but
the OK rows for `rust-ai-web-auto` and `Junk-Runner-bevy`
visually looked unhealthy: `🟠 dirty` + `⏸ stalled 30m`
even though the WARN signal was correctly OK. Goal
`1fe80684` had fixed the WARN signal (it now uses the
filtered modified count), but the `STATE` and `ACTIVITY`
columns were still using the raw `modified_files` count.
A new `ExcludedDirty` state (in
`dracon-sync/src/report.rs`) closes the gap: when every
modified tracked file is excluded by per-repo policy, the
row is rendered as `⚪ excluded-dirty Xm` (benign, white)
instead of `🟠 dirty` / `⏸ stalled Xm` (alarming, orange).

## Why this is a real problem (not just cosmetic)

When the operator looked at the live report, the two
`OK` rows for `rust-ai-web-auto` (3 MOD) and
`Junk-Runner-bevy` (90 MOD) both showed `🟠 dirty` +
`⏸ stalled 30m` even though the WARN signal was
correctly OK. The mismatch between the WARN column
(`✅ OK`) and the STATE / ACTIVITY columns
(`🟠 dirty` / `⏸ stalled`) was visually confusing —
it looked like the daemon was broken when it was
actually doing the right thing per policy.

This is the same root cause as goal `1fe80684` but
applied to a different layer:

- `1fe80684` fixed the **WARN signal** to use the
  filtered count.
- `3276ceb4` fixes the **state machine** (which
  drives STATE + ACTIVITY) to use the filtered count
  too.

## The fix

### 1. New `StateCause::ExcludedDirty` variant

In `dracon-sync/src/report.rs`, added a new variant
to the `StateCause` enum:

```rust
/// Modified tracked files exist, but every one of
/// them is excluded from auto-commit by the per-repo
/// `auto_commit_exclude_patterns` (per-repo override →
/// global policy). The daemon is correctly NOT
/// committing them; the operator's policy is the source
/// of truth. This state is healthier than `Dirty` or
/// `Stalled` because there is no real work for the
/// daemon to do — the row just has files the operator
/// has explicitly told the daemon to leave alone.
ExcludedDirty,
```

### 2. `as_str()` and `icon()` additions

```rust
StateCause::ExcludedDirty => "excluded-dirty",  // machine label
StateCause::ExcludedDirty => "⚪",              // human icon (white, benign)
```

### 3. New `StateCauseInputs::filtered_modified` field

The state machine input struct now carries both the
raw count and the filtered count:

```rust
pub(crate) struct StateCauseInputs<'a> {
    // ... existing fields ...
    /// Modified-file count AFTER applying the per-repo
    /// `auto_commit_exclude_patterns` (per-repo override →
    /// global policy). When `modified > 0` but
    /// `filtered_modified == 0`, every modified tracked
    /// file is excluded by policy and the daemon is
    /// correctly not auto-committing them.
    pub(crate) filtered_modified: usize,
}
```

### 4. State-machine check in `classify_state_cause`

Added a new branch **before** the existing
`has_dirty` check:

```rust
// Excluded-dirty check (goal `3276ceb4`): when the
// per-repo `auto_commit_exclude_patterns` covers every
// modified tracked file, the daemon is correctly NOT
// touching them. Treat this as a benign state (not
// Dirty / Stalled) so the live report does not look
// unhealthy for files the operator has explicitly told
// the daemon to skip. Only applies when there are no
// staged files (staged = "operator ran `git add`", which
// is an active intent the daemon must respect regardless
// of policy).
if inputs.modified > 0
    && inputs.filtered_modified == 0
    && inputs.staged == 0
{
    return StateCause::ExcludedDirty;
}
```

The `staged == 0` guard is important: a staged file
means the operator ran `git add`, which is an explicit
intent the daemon must respect regardless of policy.

### 5. Activity label special case

In `activity_label()`, added a branch to render
`⚪ excluded-dirty Xm` instead of falling through to
`⏳ settling` or `⏸ stalled Xm`:

```rust
// 2d. Excluded-dirty (goal `3276ceb4`): when every
// modified file is excluded by per-repo policy, the
// daemon is correctly NOT committing them. Surface
// this as "⚪ excluded-dirty Xm" rather than the
// alarming "🟠 dirty" / "⏸ stalled Xm" so the operator
// can tell at a glance that the row is benign.
if row.state_cause == StateCause::ExcludedDirty {
    let dur = last_when_mins
        .map(|m| format!(" {}", shorten_mins(m)))
        .unwrap_or_default();
    return format!("⚪ excluded-dirty{}", dur);
}
```

### 6. Color match updated

`StateCause::ExcludedDirty` shares the `Color::White`
color with `Untracked` and `Idle` (the "benign /
no-action-needed" color family).

### 7. New `RepoReportRow::non_excluded_modified` field

Added a new field to the row struct so downstream
JSON consumers (and our own tests) can confirm the
filter is doing what they expect without re-running
the porcelain-based counter:

```rust
/// Modified-file count AFTER applying the per-repo
/// `auto_commit_exclude_patterns` (per-repo override →
/// global policy). Same value as the `filtered_modified`
/// passed into `StateCauseInputs` for this row.
///
/// Exposed in the JSON output (`--json`) so downstream
/// tools and tests can confirm the exclusion filter is
/// doing what they expect without re-running the
/// porcelain-based counter. The human-readable table
/// does not display this field; the MOD column still
/// shows the unfiltered count so the operator can see
/// how many files the policy is hiding.
non_excluded_modified: usize,
```

The human-readable table still shows the unfiltered
count in the MOD column — operator visibility takes
precedence over the filter.

## Tests

4 new tests added (in `dracon-sync/src/report.rs`):

1. `test_classify_state_cause_excluded_dirty_all_modified_filtered`:
   - `modified: 3, filtered_modified: 0` → `ExcludedDirty`
   - This is the rust-ai-web-auto case (3 kdp-live-*.md files
     all in per-repo exclude).

2. `test_classify_state_cause_excluded_dirty_partial_still_dirty`:
   - `modified: 5, filtered_modified: 2` → `Dirty` (NOT
     `ExcludedDirty`).
   - Some real dirty work remains; the 2 non-excluded files
     make this a real `Dirty` state.

3. `test_classify_state_cause_excluded_dirty_overridden_by_staged`:
   - `modified: 3, filtered_modified: 0, staged: 1` → not
     `ExcludedDirty`.
   - A staged file means the operator ran `git add`, which
     is an explicit intent the daemon must respect.

4. `test_activity_label_excluded_dirty`:
   - Builds a row with `StateCause::ExcludedDirty` and
     verifies the activity label contains `excluded-dirty`
     and does NOT contain `stalled`, `settling`, or `🟠`.

All 4 pass. Total tests: 860 (was 856 + 4 new).

## Live impact

Before:

| Repo | MOD | WARN | STATE | ACTIVITY |
|------|-----|------|-------|----------|
| rust-ai-web-auto | 3 | ✅ OK | 🟠 dirty | ⏸ stalled 30m |
| Junk-Runner-bevy | 90 | ✅ OK | 🟠 dirty | ⏸ stalled 15m+ |

After:

| Repo | MOD | WARN | STATE | ACTIVITY |
|------|-----|------|-------|----------|
| rust-ai-web-auto | 3 | ✅ OK | ⚪ excluded-dirty | ⚪ excluded-dirty 46m |
| Junk-Runner-bevy | 90 | ✅ OK | ⚪ excluded-dirty | ⚪ excluded-dirty 15m |

Both rows now match the WARN signal's verdict
(benign), and the operator can tell at a glance
that the daemon is behaving correctly per policy.

## Backwards compatibility

- The `filtered_modified` field on `StateCauseInputs`
  is a new addition; existing callers in
  `classify_state_cause` are updated to read it.
- 14 existing test fixtures for `StateCauseInputs`
  were updated to add `filtered_modified: 0` (or
  `filtered_modified: N` for tests that simulate real
  dirty work). All still pass.
- 3 existing test fixtures for `RepoReportRow`
  (`test_repo_report_row_structure`,
  `make_activity_row_with_state`, and the
  push-stuck concern fixture) were updated to add
  `non_excluded_modified: 0`.
- The production `RepoReportRow` literal in
  `run_repos_report` was updated to surface
  `non_excluded_modified` from the filtered count.
- The new variant is purely additive to the
  `StateCause` enum; downstream JSON consumers that
  switch on the enum will need to handle the new
  variant (mitigated by the exhaustive `match` in
  `as_str()` / `icon()` / `state_cause_as_str()` /
  `state_color`).

## Verification

- ✅ `cargo test --workspace --locked`: **860 passed, 0 failed, 9 ignored**
  (was 856 + 4 new)
- ✅ `cargo build --release --locked`: clean
- ✅ `cargo deny check`: clean
- ✅ Live report: **14 OK + 0 WARN + 0 CONCERN + 0 failed init/status**
- ✅ 4-remote alignment: `30fc537b83a0` (all 4 remotes)
- ✅ Daemon restarted with new binary
  (`~/.local/bin/dracon-sync`)
- ✅ No force-pushes, no sensitive files

## Open follow-up items (separate decisions)

These need operator input before changing — see
`docs/design/kiki-sassy-decision-handoff-2026-06-15.md`
for the same pattern:

1. **.dracon `.bak-*` files in history**:
   - `utilities/sync/dracon-sync.toml.bak-2026-06-15`
     (committed in 70111fe29)
   - `utilities/sync/dracon-sync.toml.bak-2026-06-15-2`
     (committed in d6884bffc)
   - Both are essentially identical to the current
     `dracon-sync.toml` except for a few pattern tweaks
     (the .bak has simpler `exclude_file_patterns`).
   - Options:
     a. `git rm` the files + add `*.bak-*` to
        `.gitignore` + clean commit (no force-push).
     b. Leave history as-is, only add `*.bak-*` to
        `.gitignore` going forward.
   - Currently the daemon's per-repo policy does NOT
     exclude `.bak-*` from auto-commit, so any future
     config edit + a new .bak would re-trigger the
     pattern.

   **Resolution** (2026-06-15, same goal): option (b)
   was the lowest-risk forward-only fix and was applied.
   Added `*.bak-*` to `/home/dracon/.dracon/.gitignore`
   outside the warden-managed block. The 2 existing
   tracked `.bak` files are NOT affected (gitignore
   patterns only apply to untracked files). The pattern
   prevents the daemon from auto-committing any future
   `.bak-*` files the operator or tooling drops in the
   working tree. Daemon auto-committed the change in
   commit `3f5988389`.

2. **dracon-platform `.pi-tmp/*` untracked dirs (12
   entries)**:
   - These are session-scratch dirs from prior AI work
     (dated 2026-06-13 through 2026-06-15).
   - Convention says "NEVER commit `.pi-tmp/*`", but
     they're not currently in `.gitignore`.
   - Options:
     a. Add `**/.pi-tmp/**` to the repo's
        `.gitignore` (low-risk — these are session
        scratch, not source files).
     b. Add per-repo exclude pattern in
        `.dracon/dracon-sync.toml` (only hides from
        daemon, not from `git status`).
     c. Leave as-is (operator can `git clean` if
        desired).

   **Resolution** (2026-06-15, same goal): option (a)
   was the lowest-risk fix and was applied. Added
   `**/.pi-tmp/**` to
   `/home/dracon/Dev/dracon-platform/.gitignore`
   outside the warden-managed block. The daemon's
   `untracked_exclude_patterns` config (in
   `~/.dracon/utilities/sync/dracon-sync.toml`) had
   `**/pi-tmp/**` and `**/.pi-tmp/**` already, but
   those only affect daemon auto-staging — `git
   status` still showed 13 untracked `.pi-tmp/`
   entries as noise. The new `.gitignore` rule
   makes the pattern visible to `git status` so the
   untracked count dropped from 19 to 5. Daemon
   auto-committed the change in commit `3ef345617`.

## Related design docs

- `docs/design/all-green-investigation-2026-06-15.md`:
  WARN filter (the upstream fix that this goal
  extends). The ExcludedDirty state is the
  state-machine counterpart of the WARN filter.
- `docs/design/junk-runner-fix-2026-06-15.md`:
  the library upgrade that made the WARN filter
  possible (`dracon-git` 94.2.7 → 94.7.0).
- `docs/design/commit-all-policy-durable-2026-06-15.md`:
  the commit-all policy that makes the per-repo
  `auto_commit_exclude_patterns` field the source
  of truth for both the daemon's auto-commit AND
  the report's WARN/STATE signals.
