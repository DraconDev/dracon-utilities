# Master Roadmap — 2026-06-01

**Consolidates findings from recent audits and fixes into a single comprehensive plan.**

---

## Executive Summary

This document consolidates findings from four major investigation threads:
1. **Repo cleanup audit** (REPOS_CLEANUP_PLAN_2026-06-01.md)
2. **Sync binary fixes** (untracked/modified separation, WARN/CONCERN classification)
3. **!target/ un-ignore policy fix** (dracon-warden policy change)
4. **WARN/CONCERN repo investigations** (multiple sync passes)

### Current State (as of 2026-06-01 ~22:00)

| Metric | Value | Status |
|--------|-------|--------|
| Total repos | 34 | ✅ All healthy |
| OK repos | ~27 | ✅ Sync working |
| WARN repos | 3-5 | ⚠️ Transient (cache/real code) |
| CONCERN repos | 0-5 | ⚠️ Transient (unpushed version bumps) |
| `!target/` un-ignore lines | 0 | ✅ Fixed at source |
| Tracked target/ files | 0 | ✅ All untracked |
| `target/`/`node_modules/`/`node_modules/` build artifact dirs in 34 repos | ~249 GB untracked | ⚠️ Could reclaim with `git clean` |

---

## Section 1: Completed Fixes (May 31 - June 1, 2026)

### 1.1 Sync Binary: Untracked/Modified Separation ✅

**Problem:** The `MOD` column in `dracon-sync repos` was including untracked files. Repos with only untracked `target/` were showing 10k+ "modified" entries, causing confusion.

**Root Cause:** In `dracon-git/src/lib.rs`, the `modified_files` counter included both tracked modifications AND untracked files (via `is_wt_new()`).

**Fix Applied:**
- Added `untracked_files: usize` field to `RepoStatus` in `dracon-git/src/types.rs`
- Separated untracked from modified in both libgit2 and CLI paths
- Updated `is_clean` to also require `untracked_files == 0`
- Added "UTR" column to report table in `dracon-sync/src/report.rs`
- Updated classification logic in `report.rs`: `real_is_dirty = status.modified_files > 0 || status.staged_files > 0` (excludes untracked)

**Files Changed:**
- `dracon-libs/tools/sync/dracon-git/src/types.rs` (added `untracked_files` field)
- `dracon-libs/tools/sync/dracon-git/src/lib.rs` (2 paths updated)
- `dracon-utilities/dracon-sync/src/report.rs` (UTR column + classification fix)

**Impact:** WARN count from 16 → 4 (10+ repos that were WARN purely due to untracked artifacts are now correctly OK).

---

### 1.2 !target/ Un-Ignore Policy Fix ✅

**Problem:** The `!target/` line in `.gitignore` was un-ignoring build artifacts, causing the auto-commit daemon to commit `target/` files. Some repos had 10,000+ tracked target/ files (3+ GB each).

**Root Cause:** The dracon-warden policy had `target/`, `node_modules/`, `.cache/` in `plaintext_patterns`. The daemon generates `!{pattern}` for plaintext patterns, which un-ignored them. Any manual `.gitignore` edits were silently overwritten by the daemon on each pass.

**Fix Applied:**
- Removed `target/`, `node_modules/`, `.cache/` from `plaintext_patterns` in `~/.dracon/utilities/warden/dracon-warden.toml`
- Waited ~60s for daemon to reload and rewrite all 33 `.gitignore` files
- Manually ran `dracon-warden once` on stubborn repos (dracon-code)
- Untracked 14,191+ target/ files in 3 repos (ai-auto-writer, avid, dracon-code)
- Committed changes in 9 repos

**Files Changed:**
- `~/.dracon/utilities/warden/dracon-warden.toml` (policy)
- `.gitignore` in 33 repos (auto-rewritten by daemon)
- 9 repos committed with untrack + cleanup

**Impact:** 0 `!target/` lines remaining, 0 tracked target/ files. The fix is structural — the daemon will never re-add `!target/`.

---

### 1.3 Repo Cleanup Plan (REPOS_CLEANUP_PLAN_2026-06-01.md) ✅

**Document:** `/home/dracon/Dev/dracon-utilities/REPOS_CLEANUP_PLAN_2026-06-01.md`

Categorized 34 repos:
- **Category A** (15 repos): Pure build artifacts, DISCARD with .gitignore fixes
- **Category A-Mixed** (4 repos): Build artifacts + content changes, COMMIT real + .gitignore
- **Category B** (1 repo): Real content only — respec-spec-reconciler stale branch (49K line divergence)
- **Category D** (1 repo): Data dir only — dracon-ai-lib .dracon/
- **Category C** (13 repos): No action needed

**Status:** Most of this is now handled by the sync binary fix and !target/ policy fix. The respec-spec-reconciler stale branch still needs human review.

---

## Section 2: Outstanding Issues (June 1, 2026)

### 2.1 Stale Branch: respec-spec-reconciler 🔴

**Status:** Not addressed. Branch `autoresearch/evolutionary-reconciler-2026-05-30` is 49,101 lines diverged from main with 16+ commits of 2-8K line spec changes.

**Recommended Action:**
```bash
cd ~/Dev/respec-spec-reconciler
git log --oneline main..autoresearch/evolutionary-reconciler-2026-05-30
# Review the 16+ large commits
# Either:
#   git merge autoresearch/evolutionary-reconciler-2026-05-30  (if good)
#   git branch -D autoresearch/evolutionary-reconciler-2026-05-30  (if discard)
```

**Priority:** Medium — not breaking anything but taking up disk space.

---

### 2.2 Disk Space Reclamation (~249 GB available) 🟡

**Status:** `target/` and `node_modules/` directories exist on disk but are no longer tracked. Could reclaim with `git clean -fdx`.

**Caveat:** `.gitignore` only stops tracking — doesn't reclaim disk. `git clean -fdx` permanently deletes untracked files.

**Recommended Action (optional, manual):**
```bash
# For each repo with untracked target/ or node_modules/:
cd ~/Dev/<repo>
git clean -fdx   # WARNING: permanently deletes untracked files
```

**Size breakdown:**
- 16 target/ dirs: ~248 GB
- 4 node_modules/ dirs: ~1.2 GB
- 2 .dracon/ dirs: ~228 KB

**Priority:** Low — optional, manual, irreversible.

---

### 2.3 Sync System Improvements (Future) 🟢

These are observations from the investigations that could be improved:

1. **Auto-commit timing**: `inactivity_push_delay_secs=5` causes WARN/CONERN to flicker. The daemon's `untrack target/` commit in respec-spec-reconciler-style repos creates a 49K divergence. Consider increasing delay or adding sanity checks before auto-committing large changes.

2. **Better error messages**: The `run repair-warns --apply` hint could include estimated disk space saved.

3. **Pre-commit sanity check**: Before auto-committing changes > 1000 lines, daemon could prompt or log a warning. This would have caught the respec-spec-reconciler stale branch issue earlier.

4. **Stale branch detection**: A periodic check for branches with > 10K line divergence that haven't been touched in > 24h could surface abandoned experiments.

---

## Section 3: Deferred Tasks (From tasks.md)

These are large refactoring tasks that were deferred from the initial audit. Each requires 2-4 hours of focused work in a dedicated session.

| Task | Description | Current State | Effort |
|------|-------------|---------------|--------|
| H-SEC-LIB | Complete security lib split | 3 modules extracted (scanner, environment, keys), lib.rs still 2,854 lines | 4-6h |
| H-DAEMON | Extract cooldown manager | CooldownManager struct created, NOT integrated into daemon | 2-4h |
| L-HEALTH-ENDPOINT | Add daemon health check socket | Not started | 3-5h |
| L-ASYNC-UNIFY | Unify sync git calls to async | Not started (30+ sync calls to convert) | 4-6h |

**Total estimated effort:** 13-21 hours of focused refactoring work.

---

## Section 4: Architecture Insights (For Future Reference)

### 4.1 Daemon Ecosystem

The system has two main daemons:
- **dracon-warden**: Manages `.gitignore` (DRACON MANAGED BLOCK) and `.gitattributes` for encryption. Reads `~/.dracon/utilities/warden/dracon-warden.toml`.
- **dracon-sync**: Manages git sync, auto-commit, auto-push. Reads `~/.dracon/utilities/sync/dracon-sync.toml`.

**Important:** Manual edits to daemon-managed files in `.gitignore` will be **silently overwritten** by the daemon. Always change the policy file, not the managed files directly.

### 4.2 Sync Classification Logic (Fixed)

Before fix: `real_is_dirty = !status.is_clean` (included untracked in dirty check)
After fix: `real_is_dirty = status.modified_files > 0 || status.staged_files > 0` (excludes untracked)

**Implication:** A repo with only untracked build artifacts is now correctly OK, not WARN.

### 4.3 Warden Policy Structure

The dracon-warden policy has these key sections:
- `protected_patterns`: Files matching these are tracked AND encrypted (e.g., `*.env`, `config.json`)
- `plaintext_patterns`: Files matching these are tracked but NOT encrypted (e.g., `Cargo.lock`, `target/` was here)
- `hygiene_patterns`: Files matching these are ignored entirely (e.g., `target/`, `node_modules/`, `*.log`)

**Conflict to avoid:** A pattern in BOTH `plaintext_patterns` AND `hygiene_patterns` creates a `!pattern` un-ignore in the managed block, which re-includes the pattern for tracking. This was the root cause of the target/ tracking issue.

---

## Section 5: Lessons Learned

1. **Always check the daemon's source of truth, not the symptom.** The `!target/` was being re-added by the daemon from the policy file. Editing `.gitignore` directly was futile.

2. **Sync binary classification must distinguish tracked vs untracked.** Including untracked in `MOD` count caused false positives for repos with only build artifacts.

3. **Stale branches with large divergence are a warning sign.** The respec-spec-reconciler's 49K line divergence should have triggered a warning before auto-committing.

4. **Auto-commit daemon can cause large unintended changes.** The version-bump process in 3 repos committed 4341+ line Cargo.lock updates. Consider adding confirmation prompts for large changes.

5. **Evidence in goal files is truncated to ~200 chars.** For detailed investigations, write to a standalone markdown file (like REPOS_CLEANUP_PLAN_2026-06-01.md) for full context.

---

## Section 6: Recommended Next Steps (Priority Order)

1. **🔴 High Priority: Review respec-spec-reconciler stale branch**
   - 49K line divergence, 16+ commits
   - Either merge or discard
   - ~30 min manual work

2. **🟡 Medium Priority: Document the fix in AGENTS.md**
   - Add a section about the untracked/modified distinction
   - Add a warning about editing daemon-managed files
   - ~15 min

3. **🟡 Medium Priority: Add the respec-spec-reconciler investigation to REPOS_CLEANUP_PLAN**
   - Or create a new investigation doc for the stale branch
   - ~30 min

4. **🟢 Low Priority: Disk space reclamation (optional)**
   - `git clean -fdx` on repos with untracked target/
   - ~249 GB potential recovery
   - ~30 min for 30+ repos
   - **WARNING: irreversible, backup first**

5. **🟢 Low Priority: Tackle the 4 deferred refactoring tasks (when time permits)**
   - H-SEC-LIB, H-DAEMON, L-HEALTH-ENDPOINT, L-ASYNC-UNIFY
   - 13-21 hours of focused work

---

## Section 7: File Index (All Relevant Documents)

### Plans & Audits
- `/home/dracon/Dev/dracon-utilities/REPOS_CLEANUP_PLAN_2026-06-01.md` (9.2 KB) — Initial repo cleanup plan
- `/home/dracon/Dev/dracon-utilities/MASTER_ROADMAP_2026-06-01.md` (this file) — Master consolidation
- `/home/dracon/Dev/dracon-utilities/tasks.md` (22.6 KB) — Detailed task list

### Configuration Files
- `~/.dracon/utilities/sync/dracon-sync.toml` — Sync policy
- `~/.dracon/utilities/warden/dracon-warden.toml` — Warden policy (modified to remove target/ from plaintext_patterns)

### Source Code
- `dracon-utilities/dracon-libs/tools/sync/dracon-git/` — Git service (modified for untracked separation)
- `dracon-utilities/dracon-sync/src/` — Sync binary (modified for WARN/CONCERN classification)
- `dracon-utilities/dracon-warden/src/main.rs` — Warden binary (source of !target/ generation logic)

### Repos
- 34 repos in `~/Dev/` — All healthy with no `!target/` and no tracked target/ files

---

**Document version:** 1.0
**Date:** 2026-06-01
**Author:** Pi AI agent (consolidating findings from 4 audit threads)
