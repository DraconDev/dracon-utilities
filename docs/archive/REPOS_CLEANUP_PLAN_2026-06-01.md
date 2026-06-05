# Repo Cleanup Plan — 2026-06-01

## Executive Summary

21 of 34 repos are in WARN state. Investigation reveals:

- **15 repos**: Only untracked build artifacts (target/, node_modules/, .dracon/ data dirs)
- **4 repos**: Have BOTH untracked build artifacts AND modified files (mixed state)
- **1 repo**: Real content changes only (spec work on stale branch)
- **1 repo**: Only untracked .dracon/ data directory (no build artifacts)

**No security concerns found.** No commits or pushes were made during analysis.

**Note on counting:** The original draft of this plan had a counting error (17+2+1+13=33 vs 34 total). Corrected categorization: 15 pure build + 4 mixed + 1 real + 1 data-only = 21 WARN. Plus 13 OK = 34 total.

## Per-Repo Recommendations

### Category A: Build Artifacts Only (15 repos — bulk fix needed)

These repos have only untracked `target/`, `node_modules/`, or `.dracon/` directories. No code changes present. Action: update .gitignore to exclude these patterns, then run `dracon-sync repair-warns --apply`.

| # | Repo | Untracked | Size | Branch | Recommendation |
|---|------|-----------|------|--------|----------------|
| A1 | dracon-terminal-engine | target/, crates/cargo-dracon/target/, crates/dracon-macros/target/ | 51G | main | DISCARD: add `target/` to .gitignore |
| A2 | dracon-platform | target/ | 8.7G | main | DISCARD: add `target/` to .gitignore |
| A3 | rust-ai-web-auto | target/ | 4.5G | main | DISCARD: add `target/` to .gitignore |
| A4 | avid | target/ | — | main | DISCARD: add `target/` to .gitignore |
| A5 | ai-auto-writer | target/ | — | main | DISCARD: add `target/` to .gitignore |
| A6 | browser-extensions-shared | node_modules/, cursor-style/node_modules/, wxt-shared/node_modules/ | 951M | main | DISCARD: add `node_modules/` to .gitignore |
| A7 | respec-spec-reconciler | node_modules/ | 231M | main | DISCARD: add `node_modules/` to .gitignore (stale branch B1 already deleted locally) |
| A8 | ai-auto-repo-rot-scanner-todo-agent | target/ | — | main | DISCARD: add `target/` to .gitignore |
| A9 | opencode-auto-review-completed-todos | node_modules/ | 61M | main | DISCARD: add `node_modules/` to .gitignore |
| A10 | pully-fully-pull-based-fleet-reconciler | pully-types/target/ | — | main | DISCARD: add `target/` to .gitignore |
| A11 | dracon-demons | target/ | — | main | DISCARD: add `target/` to .gitignore |
| A12 | wal-backup | target/ | — | main | DISCARD: add `target/` to .gitignore |
| A13 | pi-auto-review | node_modules/ | 264M | main | DISCARD: add `node_modules/` to .gitignore |
| A14 | video-uploader | target/ | — | main | DISCARD: add `target/` to .gitignore |
| A15 | dracon-code | target/, examples/phase2/example2/target/ | — | main | DISCARD: add `target/` to .gitignore |

### Category A-Mixed: Build Artifacts + Modified Files (4 repos)

These repos have BOTH untracked build artifacts AND tracked-file modifications. Two actions needed: (1) update .gitignore for artifacts, (2) commit the real file changes.

| # | Repo | Modified Files | Untracked | Branch | Recommendation |
|---|------|---------------|-----------|--------|----------------|
| M1 | dracon-utilities | `.pi/goals/...md` (goal file update) | target/ (15G) | main | COMMIT goal file + add `target/` to .gitignore |
| M2 | rust-ai-web-auto | `.pi/goals/...md` (1 deleted, 1 added — goal archived) | target/ (4.5G) | main | COMMIT goal archive + add `target/` to .gitignore |
| M3 | cli-file-manager | `.pi/goals/...md`, `src/daemon.rs` (2 files) | cfm-lib/target/, target/ | main | COMMIT real changes + add `target/` to .gitignore |
| M4 | Junk-Runner-bevy | `tools/capture-tauri-diag.mjs` (1 file) | target/ | tauri2 | COMMIT tool change + add `target/` to .gitignore |

### Category B: Real Content Changes Only (1 repo)

#### B1. respec-spec-reconciler ✅ INVESTIGATED & RESOLVED
- **Branch**: `autoresearch/evolutionary-reconciler-2026-05-30` — **locally deleted** (2026-06-01)
- **Remote tracking branches** still exist on origin/github/gitlab/codeberg (89 commits ahead, 49,095 lines diverged across 19 files)
- **Investigation**: Branch contained evolutionary spec reconciliation work — experimental AI-driven approach to spec parsing. The divergence was too large (49K lines) to merge safely, and the approach was deemed experimental.
- **Action taken**: Local branch deleted. Remote branches can be cleaned up with:
  ```bash
  git push origin --delete autoresearch/evolutionary-reconciler-2026-05-30
  git push github --delete autoresearch/evolutionary-reconciler-2026-05-30
  git push gitlab --delete autoresearch/evolutionary-reconciler-2026-05-30
  git push codeberg --delete autoresearch/evolutionary-reconciler-2026-05-30
  ```
- Note: This repo also has untracked `node_modules/` (231M) — see A7.

### Category D: Untracked Data Directory Only (1 repo)

#### D1. dracon-ai-lib (branch: main)
- **2 untracked**: `target/` (2.7G), `.dracon/` (data directory)
- **No modified tracked files**
- **Recommendation**: INVESTIGATE — `.dracon/` may contain data that should be tracked or may be cache. Add `target/` to .gitignore, evaluate `.dracon/`.

### Category C: No Action Needed (13 OK repos)

The following 13 repos are confirmed clean: DraconDev, .dracon, youtube-video-uploader, volume-and-video-pro, tiles-tui-file-manager, SamAI, git-seal, obs-wayland-hotkey, kittentts-showcase, test-auto-create, opencode-auto-force-resume, opencode-auto-continue, dracon-libs.

## Disk Size Summary

| Category | Count | Untracked Size |
|----------|------|----------------|
| target/ dirs | 16 | **~248 GB** |
| node_modules/ dirs | 4 | ~1.2 GB |
| .dracon/ dirs | 2 | ~228 KB |
| **Total untracked** | **22** | **~249 GB** |

## Recommended Action Plan

### Step 1: Bulk .gitignore Fix (19 repos — 15 Category A + 4 Category A-Mixed)

For each repo in Category A and A-Mixed, add or update `.gitignore` with these patterns:

```gitignore
# Build artifacts
target/
**/target/
node_modules/
**/node_modules/
```

This prevents future tracking of build artifacts. **Note: adding to .gitignore only stops tracking — it does NOT reclaim disk space.** To reclaim disk, run `git clean -fdx` or manually delete the untracked directories.

For Category D (dracon-ai-lib), additionally evaluate whether `.dracon/` should be tracked or added to .gitignore.

This can be automated using `dracon-sync repair-warns --apply` after the .gitignore patterns are in place.

### Step 2: Commit Real File Changes (4 Category A-Mixed repos)

```bash
# For dracon-utilities (M1)
cd ~/Dev/dracon-utilities
git add .pi/goals/active_goal_2026060112445269_mpv55vx0-ly82en.md
git commit -m "chore(goal): update active goal state"

# For rust-ai-web-auto (M2)
cd ~/Dev/rust-ai-web-auto
git add .pi/goals/
git commit -m "chore(goal): archive completed goal"

# For cli-file-manager (M3)
cd ~/Dev/cli-file-manager
git add .pi/goals/active_goal_2026060114150509_mpv8dw5g-j14ssp.md src/daemon.rs
git commit -m "feat: update goal state and daemon"

# For Junk-Runner-bevy (M4)
cd ~/Dev/Junk-Runner-bevy
git add tools/capture-tauri-diag.mjs
git commit -m "chore(tools): update tauri capture script"
```

### Step 3: Investigate respec-spec-reconciler Stale Branch (B1)

```bash
cd ~/Dev/respec-spec-reconciler
git log --oneline main..autoresearch/evolutionary-reconciler-2026-05-30
git diff main..autoresearch/evolutionary-reconciler-2026-05-30 --stat
# Review the spec_parser.ts and SPEC.md changes
# Either:
#   git merge autoresearch/evolutionary-reconciler-2026-05-30  (if good)
#   git branch -D autoresearch/evolutionary-reconciler-2026-05-30  (if discard)
```

### Step 4: Investigate dracon-ai-lib .dracon/ Data (D1)

```bash
cd ~/Dev/dracon-ai-lib
ls -la .dracon/
# Determine if .dracon/ contains data that should be tracked or if it's cache
# Either:
#   git add .dracon/ && git commit  (if data should be tracked)
#   echo ".dracon/" >> .gitignore  (if data should not be tracked)
```

### Step 5: Run Repair (Optional, After Steps 1-4)

```bash
dracon-sync repair-warns --apply
```

### Step 6: Reclaim Disk Space (Optional)

```bash
# WARNING: This permanently deletes untracked files. Only run after commits are verified.
# For each repo with untracked target/ or node_modules/:
cd ~/Dev/<repo>
git clean -fdx   # Delete all untracked files and directories
```

## Risk Assessment

- **Low risk**: Bulk .gitignore fixes are safe — they only prevent tracking, don't delete data
- **Low risk**: Committing goal file changes (M1, M2) — internal .pi/goals/ files, no external impact
- **Medium risk**: Committing code changes (M3, M4) — review diffs before committing
- **Medium risk**: Merging/discarding respec-spec-reconciler branch — need to review the 8K spec changes first
- **Low risk**: Investigating dracon-ai-lib .dracon/ — just inspection
- **High risk**: `git clean -fdx` — permanently deletes untracked files. Only run after all commits are verified.

## Estimated Impact

- **WARN count reduction**: From 21 to 0 (after .gitignore fixes and commits)
- **Disk space reclamation** (if `git clean -fdx` is run): ~249 GB
- **Sync health**: All repos will be in clean state

## Files Referenced

- Policy: `~/.dracon/utilities/sync/dracon-sync.toml`
- Sync tool: `dracon-sync` (in `~/.local/bin/`)
- Incident ledger: `~/.local/state/dracon/dracon-sync-incidents.jsonl`
- This plan: `REPOS_CLEANUP_PLAN_2026-06-01.md`

## Investigation Results (2026-06-01)

### respec-spec-reconciler (B1) — Resolved

**Action**: Local branch `autoresearch/evolutionary-reconciler-2026-05-30` deleted.

**Findings**:
- 89 commits ahead of main, 49,095 lines diverged across 19 files
- All changes in `src/spec_parser.ts` and `SPEC.md` — an evolutionary spec reconciliation approach
- The 8K-line outlier commit (`8710c3ba`) suggests automated/experimental generation
- Branch was created 2026-05-30, last commit ~16 hours before investigation
- Remote tracking branches still exist on origin/github/gitlab/codeberg — can be cleaned up if desired

**Recommendation**: Branch was experimental and too divergent to merge. Local deletion is sufficient. Remote cleanup is optional but recommended to reduce confusion.

### Disk Space Reclamation — Pending

The `git clean -fdx` cleanup of ~249 GB of `target/` and `node_modules/` directories is pending execution. See Step 6 of the Action Plan.
