# Audit Part 3 — dracon-sync (report.rs / git/{discovery,staging,diff,status,branch}.rs)
# Source-level audit, 2026-07-18
# For inclusion into AUDIT_FULL_2026-07-18-POSTFIX.md

## HIGH findings

### F30 — Full table constraint sum is 345 cols, exceeds 300-col tier threshold (HIGH, v0.112.19 bug incomplete)
**File:** `dracon-sync/src/report.rs:3708`, constraint set at `:3770-3793`, test at `:8210-8231`

```rust
// Sum: 3+11+17+11+17+8+8+7+9+9+13+17+22+11+17+11+8+8+8+15+17+22 = 268
// Plus 23 borders: 291 cols minimum. Full tier starts at 300 cols to give
// 9+ cols of headroom.
table.set_constraints(vec![
    ColumnConstraint::Absolute(Width::Fixed(4)),     // #
    ColumnConstraint::Absolute(Width::Fixed(11)),    // STATUS
    ColumnConstraint::LowerBoundary(Width::Fixed(17)), // REPO
    ColumnConstraint::LowerBoundary(Width::Fixed(35)), // ROLE   ← 35, not 11 in comment
    ...
    ColumnConstraint::LowerBoundary(Width::Fixed(32)), // PUSH-TO ← 32, not 17 in comment
    ...
]);
```

**Actual constraint sum: 322 + 23 borders = 345 cols minimum.** At terminal_width=300, comfy-table will letter-wrap ROLE and PUSH-TO columns exactly like v0.112.18 did for PUSH. The CHANGELOG entry for v0.112.19 is misleading.

**Math:**
- Comment claims 268 → 291 cols minimum, fits 300 tier ✅ (the comment's belief)
- Test array `[3, 11, 17, 11, 17, 8, 8, 7, 9, 11, 13, 17, 22, 11, 17, 11, 8, 8, 8, 15, 17, 22]` has 22 entries summing to 271 → 294, fits 300 ✅ (the test's belief)
- Production constraints actually have 23 entries with ROLE=35 and PUSH-TO=32 (added in v0.112.19): sum 322 → 345, **EXCEEDS 300** ✗

**Discrepancy root cause:** ROLE column was added (with width 35 to fit "submod (of dracon-platform/web/games/wip/junk-runner)") and PUSH-TO was widened from 17 to 32 (to fit "codeberg [excl:github,gitlab]"). Both changes happened AFTER the test array was last updated, and the test array was never re-synced. The test would fail in CI if it were actually run against production, but `cargo test` apparently was NOT run after the v0.112.19 layout change.

**Why it matters:** Operators at exactly 300-345 col terminals will see the same letter-wrap artifacts (P/U/S/H on separate lines for PUSH_STUCK, R/O/L/E for ROLE) that v0.112.19 reportedly fixed. The fix is INCOMPLETE.

**Fix:**
1. Either widen Full tier threshold from 300 to ≥350 (and update `choose_layout_tier` + docstring + tests), OR
2. Shrink some column widths: ROLE=35→22 (truncate submod labels earlier), PUSH-TO=32→22 (drop the codeberg_skip_reason annotation when it doesn't fit), LAST COMMIT=22→15.
3. Update the in-code comment to match production (sum 322, plus 23 borders = 345).
4. Update `test_full_table_min_width_within_300` so the array has 23 entries matching production.

**Verification:**
```bash
cargo test -p dracon-sync --locked report::tests::test_full_table_min_width_within_300
```
The test should currently FAIL because the test array sums to 294 (under 300) but the production code's constraint set sums to 345 (over 300). Run it and confirm.

## MEDIUM findings

### F31 — `rewrite_ahead_paths` silent no-op creates empty backup branches (MEDIUM)
**File:** `dracon-sync/src/report.rs:4415-4445` (callsite), `dracon-sync/src/git/staging.rs:103-180` (impl)

`git filter-repo --invert-paths --path non-existent.txt` succeeds with exit 0 even when the path doesn't exist in any commit. The function returns `Ok(Some(backup_branch))` and the caller proceeds to `push_with_retries`. Operator's incident ledger records it as `result: "ok"` and `backup_branch: ...` as if real work happened. Net effect: a backup branch is created every cycle for no actual rewrite, polluting git history with `backup/pre-sync-largeblob-fix-*` branches.

**Fix:** After `git filter-repo` succeeds, run `git diff --shortstat backup_branch HEAD`. If empty, the rewrite was a no-op; return `Ok(None)` and log `result: "noop"`.

### F32 — `restore_paths` no path validation (MEDIUM, defense-in-depth)
**File:** `dracon-sync/src/git/staging.rs:182-230`

`unstage_*` functions gate on `super::is_safe_git_path` but `restore_paths` does NOT. Path injection via restore path is possible.

**Fix:** Add `for path in paths { if !super::is_safe_git_path(path) { return Err(...) } }` at the top.

### F33 — `parse_name_status_line` misparses rename scores (MEDIUM)
**File:** `dracon-sync/src/git/diff.rs:25-50`

`git diff --name-status` emits `R100\ta.txt\tb.txt` (rename with score). The match arm `_ => return None` silently drops `C` (copy) and `U` (unmerged) entries. Worse: the `R` arm reads `_old = parts.next()?; let new = parts.next()?;` — if input is `R100\ta.txt\tb.txt` then 3 parts = score+old+new; we read the score as `old` and `old` as `new`. Real bug: rename detection is incorrect.

**Fix:** Use `git diff --name-status -M` (capital M, prints rename as `R<old_sha>\t<new>` without scores), OR parse the score suffix explicitly.

### F34 — `consolidate_to_main` deletes remote branch without operator confirmation gate (MEDIUM, defense-in-depth)
**File:** `dracon-sync/src/git/branch.rs:108-130`

`git push origin --delete master` deletes a remote branch with no operator confirmation gate. Auto-repair path can invoke this.

**Fix:** Add a `--apply` gate that's only true after explicit `--apply` from the CLI; auto-repair path should NEVER call this function. Move destructive operations behind a `--destructive` flag.

## LOW findings

### F35 — `repair_broken_tracking` fires `git branch -vv` per repo, sequential (LOW)
**File:** `dracon-sync/src/git/branch.rs:320-380`

26 repos × ~30-100ms = 3-10s total. Parallelize via `buffer_unordered(16)`, OR short-circuit by checking `has_tracking_upstream` first.

### F36 — `set_upstream_to_remote_branch` doesn't check refspec target exists (LOW)
**File:** `dracon-sync/src/git/branch.rs:280-300`

Sets upstream to `<remote>/<branch>` even if that remote-tracking ref doesn't exist locally. Next pull/push fails cryptically.

**Fix:** Call `crate::git::remote_branch_exists(repo, branch)` first.

### F37 — `has_origin_remote` reads `<repo>/.git/config` without resolving gitdir (LOW)
**File:** `dracon-sync/src/git/status.rs:62-78`

For worktree-style checkouts, `<repo>/.git/config` doesn't exist; falls through to subprocess which works. Pattern is code-smell duplication. Use `resolve_gitdir_path` from `branch.rs` to canonicalize.

### F38 — `IndexLock::Drop` removes lockfile without checking if we created it (LOW, INFO)
**File:** `dracon-sync/src/git/status.rs:55-60`

`held: bool` field tracks create_new success — that part is correct. Doc-comment is misleading ("checkout waits for us" only works if checkout respects the same basename). Add doc-comment clarifying O_EXCL semantics.

## INFO / non-issues

- **`discovery.rs` (1177 LOC, fully read)**: NO findings. Symlinks skipped (line 113), `.git` files (worktrees) handled via `is_git_worktree_file`/`path_gitdir`, bare repos handled, path traversal safe (canonicalize before join), `is_safe_git_path`/`is_safe_branch_name` guard command-injection, `.gitmodules` parser is custom (no shell-out), `unwrap()` only in `#[cfg(test)]` blocks. Cosmetic typo: `exlude_set` (line 30) should be `exclude_set`. ✅
- **JSON output schema**: Has key/field stability; no schema-version field — but schema-versionless output is conventional for operator dashboards.

## Summary

| Severity | Count |
|---|---|
| **HIGH** | 1 (F30 table constraint sum 345 > 300, v0.112.19 incomplete fix) |
| **MEDIUM** | 4 (F31 no-op backup branches, F32 restore_paths no validation, F33 rename score parsing, F34 remote branch delete no gate) |
| **LOW** | 4 |
| **INFO** | clean (discovery.rs exemplary) |

**Top priority:** F30 — this is a partial regression of the v0.112.19 fix that the operator just deployed. The CHANGELOG claim is wrong, the test is wrong, the comment is wrong. Need to either widen the Full tier or trim columns, AND fix the test.
