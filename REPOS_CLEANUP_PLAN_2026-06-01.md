# Repo Cleanup Plan — 2026-06-01

## Executive Summary

21 of 34 repos are in WARN state. Investigation reveals:

- **17 repos**: Only untracked build artifacts (target/, node_modules/, .dracon/ data dirs)
- **4 repos**: Have actual file changes (goal management, spec work, deletions)

**No security concerns found.** No commits or pushes were made during analysis.

## Per-Repo Recommendations

### Category A: Build Artifacts Only (17 repos — bulk fix needed)

These repos have only untracked `target/`, `node_modules/`, or `.dracon/` directories. No code changes present. Action: update .gitignore to exclude these patterns, then run `dracon-sync repair-warns --apply`.

| # | Repo | Untracked | Size | Branch | Recommendation |
|---|------|-----------|------|--------|----------------|
| 1 | dracon-terminal-engine | target/, crates/cargo-dracon/target/, crates/dracon-macros/target/ | 51G | main | DISCARD: add `target/` to .gitignore |
| 2 | dracon-utilities | target/ | 15G | main | DISCARD: target/ (plus active goal file change — commit) |
| 3 | dracon-platform | target/ | 8.7G | main | DISCARD: add `target/` to .gitignore |
| 4 | rust-ai-web-auto | target/ | 4.5G | main | DISCARD: target/ (plus 1 deleted file — verify intentional) |
| 5 | dracon-ai-lib | target/, .dracon/ | 2.7G | main | DISCARD: add `target/` and `.dracon/` to .gitignore |
| 6 | avid | target/ | — | main | DISCARD: add `target/` to .gitignore |
| 7 | cli-file-manager | target/, cfm-lib/target/ | — | main | DISCARD: add `target/` to .gitignore |
| 8 | ai-auto-writer | target/ | — | main | DISCARD: add `target/` to .gitignore |
| 9 | Junk-Runner-bevy | target/ | — | tauri2 | DISCARD: add `target/` to .gitignore |
| 10 | browser-extensions-shared | node_modules/, cursor-style/node_modules/, wxt-shared/node_modules/ | 951M | main | DISCARD: add `node_modules/` to .gitignore |
| 11 | respec-spec-reconciler | node_modules/ | 231M | autoresearch/evolutionary-reconciler-2026-05-30 | INVESTIGATE: stale branch with 16 large commits |
| 12 | ai-auto-repo-rot-scanner-todo-agent | target/ | — | main | DISCARD: add `target/` to .gitignore |
| 13 | opencode-auto-review-completed-todos | node_modules/ | 61M | main | DISCARD: add `node_modules/` to .gitignore |
| 14 | pully-fully-pull-based-fleet-reconciler | pully-types/target/ | — | main | DISCARD: add `target/` to .gitignore |
| 15 | dracon-demons | target/ | — | main | DISCARD: add `target/` to .gitignore |
| 16 | dracon-voice-notifications | target/ | — | main | DISCARD: add `target/` to .gitignore |
| 17 | wal-backup | target/ | — | main | DISCARD: add `target/` to .gitignore |
| 18 | pi-auto-review | node_modules/ | 264M | main | DISCARD: add `node_modules/` to .gitignore |
| 19 | video-factory | target/, web/node_modules/ | — | main | DISCARD: add `target/` and `node_modules/` to .gitignore |
| 20 | video-uploader | target/ | — | main | DISCARD: add `target/` to .gitignore |
| 21 | dracon-code | target/, examples/phase2/example2/target/ | — | main | DISCARD: add `target/` to .gitignore |

### Category B: Real Content Changes (3 repos)

#### B1. dracon-utilities (branch: main, last: 80 seconds ago)
- **1 file changed**: `.pi/goals/active_goal_2026060112445269_mpv55vx0-ly82en.md` (5 insertions, 5 deletions)
- **Recommendation**: COMMIT — this is the current goal file. Auto-commit should handle it via daemon.

#### B2. rust-ai-web-auto (branch: main, last: 4 minutes ago)
- **1 file deleted**: `.pi/goals/active_goal_2026060112380730_mpv4x744-pzio9k.md`
- **1 file added**: `.pi/goals/archived/goal_2026060112440792_mpv4x744-pzio9k.md`
- **1 untracked**: `target/`
- **Recommendation**: COMMIT — goal file was archived. Verify archive file exists before committing.

#### B3. respec-spec-reconciler (branch: autoresearch/evolutionary-reconciler-2026-05-30, last: 16 hours ago)
- **Stale branch** with 16 commits >1000 line changes to `src/spec_parser.ts` and `SPEC.md`
- **49,101 lines diverged from main**
- **Recommendation**: INVESTIGATE — this is the real outlier. Review the branch to determine if work should be merged or discarded.
- Recent commits (last 5):
  - `723da998`: spec_parser.ts, SPEC.md DELTA:+2647/-15
  - `e36bb79a`: spec_parser.ts, SPEC.md DELTA:+2071/-15
  - `f2db7cbb`: SPEC.md DELTA:+212/-0
  - `8710c3ba`: spec_parser.ts, SPEC.md DELTA:+8132/-15 ← **8K outlier**
  - `ecc69f15`: spec_parser.ts, SPEC.md DELTA:+2243/-15

### Category C: No Action Needed (13 OK repos)

The following 13 repos are confirmed clean: DraconDev, .dracon, youtube-video-uploader, volume-and-video-pro, tiles-tui-file-manager, SamAI, git-seal, obs-wayland-hotkey, kittentts-showcase, test-auto-create, opencode-auto-force-resume, opencode-auto-continue, dracon-libs.

## Recommended Action Plan

### Step 1: Bulk .gitignore Fix (17 repos)

For each repo in Category A, add or update `.gitignore` with these patterns:

```gitignore
# Build artifacts
target/
**/target/
node_modules/
**/node_modules/
# Local data
.dracon/
```

This can be automated using `dracon-sync repair-warns --apply` after the .gitignore patterns are in place.

### Step 2: Commit Goal File Changes (2 repos)

```bash
# For dracon-utilities
cd ~/Dev/dracon-utilities
git add .pi/goals/active_goal_2026060112445269_mpv55vx0-ly82en.md
git commit -m "chore(goal): update active goal state"

# For rust-ai-web-auto
cd ~/Dev/rust-ai-web-auto
git add .pi/goals/
git commit -m "chore(goal): archive completed goal"
```

### Step 3: Investigate respec-spec-reconciler Stale Branch

```bash
cd ~/Dev/respec-spec-reconciler
git log --oneline main..autoresearch/evolutionary-reconciler-2026-05-30
git diff main..autoresearch/evolutionary-reconciler-2026-05-30 --stat
# Review the spec_parser.ts and SPEC.md changes
# Either:
#   git merge autoresearch/evolutionary-reconciler-2026-05-30  (if good)
#   git branch -D autoresearch/evolutionary-reconciler-2026-05-30  (if discard)
```

### Step 4: Run Repair

```bash
dracon-sync repair-warns --apply
```

## Risk Assessment

- **Low risk**: Bulk .gitignore fixes are safe — they only prevent tracking, don't delete data
- **Medium risk**: Committing goal file changes — these are internal .pi/goals/ files, no external impact
- **Medium risk**: Merging/discarding respec-spec-reconciler branch — need to review the 8K spec changes first

## Estimated Impact

- **Disk space recovery**: ~75 GB (after removing untracked target/ and node_modules/ from .gitignore)
- **WARN count reduction**: From 21 to 0 (after repair)
- **Sync health**: All repos will be in clean state

## Files Referenced

- Policy: `~/.dracon/utilities/sync/dracon-sync.toml`
- Sync tool: `dracon-sync` (in `~/.local/bin/`)
- Incident ledger: `~/.local/state/dracon/dracon-sync-incidents.jsonl`
- This plan: `REPOS_CLEANUP_PLAN_2026-06-01.md`
