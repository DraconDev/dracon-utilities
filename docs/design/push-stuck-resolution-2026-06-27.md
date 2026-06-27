# PUSH_STUCK Divergence Resolution — 2026-06-27

**Date**: 2026-06-27 (BST)
**Author**: pi (operator-instructed resolution of the PUSH_STUCK state on `dracon-platform`)
**Mode**: operator's git state will be modified (rebase or force-push, per operator's decision); daemon source unchanged
**Trigger**: `dracon-sync repos` showed `dracon-platform` in ❌ CONCERN with `PUSH_STUCK` status. Investigation found a true divergence: codeberg-side commit `6a7cf69324` not in local history; local ahead=1238, behind=1. The divergent commit is a substantial work product (51 files, +454/-90, 41 binary files) including a security fix and security hardening.
**Related docs**:
- `docs/design/repo-remote-visibility-2026-06-27.md` — v1 PUSH-TO column (2026-06-27 morning)
- `docs/design/repo-remote-visibility-v2-2026-06-27.md` — v2 card redesign (2026-06-27 midday)
- `docs/design/auto-create-size-investigation-2026-06-27.md` — size-based auto_create skip investigation
- `AGENTS.md` — operator's commit policy (NEVER force-push on >5-commits-ahead without explicit override)

**Evidence files** (under `docs/design/audit-2026-06-26/`):
- `push-stuck-divergence-evidence.txt` (5,435 bytes) — full divergence investigation output
- `push-stuck-repos-before.txt` (6,451 bytes) — `dracon-sync repos` capture BEFORE resolution
- `push-stuck-repos-after.txt` (TBD, post-resolution) — `dracon-sync repos` capture AFTER resolution

---

## Section 1 — Divergence investigation

### 1.1 State summary

| Field | Value |
|---|---|
| Local HEAD (initial) | `f2bf55aeceff5468d22410ef52ecf71e64578062` (2026-06-27 13:57:30) |
| Local HEAD (latest check) | `8c95d44c356377d217554af94458182e1aab38a5` (2026-06-27 16:24:xx) |
| Codeberg `main-temp` | `6a7cf69324074e35cff9e64f4aa3ef15d6c3b4e5` (2026-06-26 21:17:34) |
| Merge-base | `8fc02238f509c7e5e48106f474e65e5e7e1e603b` (2026-06-26 21:15:42) |
| Local commits past merge-base | 1350 (grew from 1238 → 1321 → 1340 → 1350 during operator-decision wait) |
| Codeberg commits past merge-base | 1 |
| Stash count | 22 (including `divergence-resolution-stash` from 2026-06-19 and several `ovh-*` stashes) |
| Daemon push failures | 205+ (up from 157 at investigation time) |
| Daemon backstop status | **ACTIVE** — daemon is skipping auto-commit for dracon-platform due to >300s push pending |
| Divergent commit ancestor of local HEAD? | **NO** (confirmed by `git merge-base --is-ancestor`) |
| Local HEAD descendant of codeberg tip? | **NO** (confirmed by `git merge-base --is-ancestor`) |
| `git push` status | **REJECTED as non-fast-forward** (confirmed by `git push --dry-run`) |

### 1.2 The divergent codeberg commit (`6a7cf69324`)

- **Author**: DraconDev <dracsharp@gmail.com>
- **Date**: Fri Jun 26 21:17:34 2026 +0100
- **Stats**: 51 files changed, +454/-90 lines, 41 binary files
- **Subject** (semantic decomposition):
  - `fix-ovh-access-key-id-misconfig` — **SECURITY FIX** (infrastructure credential)
  - `add-migration-safety-doc` — documentation
  - `tighten-gitignore-explicit-denylist` — **SECURITY HARDENING**
  - `rename-allowed-paths-to-migration-todo` — refactor
  - `annotate-migration-todo-per-game` — refactor
  - `verify-policy-baseline` — audit
  - `regenerate-roads` — 8 road tile regenerations
  - `wire-map2d-to-v11` — code change (Map2D.svelte)
  - `blend-town-thumbnails` — 15 town tile regenerations
  - `final-verify-and-commit` — final commit
- **New files**: `_shared-assets/gen-walk-cycles.py`, `visual-audit/play-v2.png` (995 KB)
- **Major data update**: `web/music/libs/data/cookbook.json` (+178 lines)

### 1.3 Reachability analysis

- **Permanent ref containing the divergent commit**: only `refs/remotes/codeberg/main-temp`
- **Reflog entries**: 3 (entries 0, 1249, 1255) — recovery window ~30-90 days
- **Other clones/backups found**: NONE
  - `/home/dracon/Dev/dracon-platform` — the working repo (divergent commit NOT in any local branch)
  - `/home/dracon/Downloads/1/dracon-platform` — NOT a git repo (just a 94MB directory of files)
  - `/home/dracon/dracon/backups/*.bundle` — bundles for other repos (dracon-code, rust-ai-web-auto, etc.), no dracon-platform bundle
- **Recovery if force-pushed**: only via local reflog for ~30-90 days, then truly gone

### 1.4 Pre-resolution working tree state

```
 M web/games/.env.ovh                              (tracked, recently committed, unstaged modification)
 M web/games/wip/hellhunter/.pi/goals/active_goal_*.md  (unstaged modification to a goal file)
?? web/games/wip/darklord/.tmp-audit/             (untracked audit scripts directory)
```

- `.env.ovh` is TRACKED (not gitignored) and has been committed multiple times recently (with `ENV:` in the commit subject). This is the operator's intentional pattern, NOT a violation of AGENTS.md (which forbids `.env`, not `.env.ovh`).
- The goal file modification is a working-tree-only change to an active goal file.
- `.tmp-audit/` contains 3 audit scripts (`audit-1.mjs`, `bounding-box.mjs`, `capture-interactive.mjs`) — safe to leave untracked.

**For option (a) rebase**, these must be stashed or committed first (rebase requires a clean working tree).

---

## Section 2 — Three resolution options

### 2.1 Option (a) — Rebase: `git pull --rebase codeberg main-temp`

**What happens**: Local's 1238 commits are replayed ON TOP of the divergent codeberg commit. The divergent commit's work (security fixes, content) is preserved and becomes part of the local history. Then the daemon can push the new local HEAD to codeberg as a normal fast-forward.

**Preconditions**: Working tree must be clean (currently has uncommitted changes — must be stashed first).

**Irreversibility**: Medium. The rebase creates new commit SHAs for the 1238 local commits. The old SHAs are reachable from the reflog for ~30-90 days. `git rebase --abort` works until conflicts are resolved.

**Files at risk**: All 51 files from the divergent commit are also modified in the local 1350 commits. **ACTUAL CONFLICT COUNT (verified via read-only `git merge-tree --write-tree` on 2026-06-27): 16 total conflicts** — 7 text content conflicts, 1 rename/rename conflict, 8 binary file conflicts. See `audit-2026-06-26/push-stuck-mergetree-conflicts.txt` for the authoritative list and `audit-2026-06-26/push-stuck-manual-conflicts.txt` for manual-conflict details.

  - **13 mechanical conflicts** (5 archived goal files, 1 rename/rename, 8 binary PNGs): all can be resolved by taking "ours" (local HEAD) — these are auto-generated state files or binary tiles where "ours" is the newer version
  - **1 trivial manual conflict** (`cookbook.json`): different content additions at different line ranges + an `updatedAt` timestamp. Take local's `updatedAt`, keep both sets of additions. Estimated 1-2 minutes.
  - **1 REAL design conflict** (`Map2D.svelte`): fundamental v10-vs-v11 tile set disagreement. Local reverted v11 because "v11 was just diagonal stripes — stripey void on the adventure map. v10 is the painted set the user wants." Codeberg has v11 seamless + 3-variant harmonic. **Requires human judgment on which tile approach is correct.**
  - **1 actually auto-merged** (`proceduralSprites.ts`): initial classification was wrong; both sides made identical changes to the mixHex helper and streak rendering. Local's additional gradient block change is a "one-side-changed" case that git resolves cleanly. No action needed.

**AGENTS.md implications**: ✅ NO force-push needed. The local becomes a descendant of codeberg, so a subsequent `git push` is a normal fast-forward.

**Recovery procedure if it goes wrong**: `git rebase --abort` returns to the pre-rebase state. The reflog preserves the old SHAs.

**Estimated effort**: 5-10 minutes for the mechanical + trivial conflicts. The Map2D.svelte design decision is the actual time sink — the operator (or developer) needs to decide between v10 painted and v11 seamless. The decision itself can be made in seconds if the operator knows which approach they want, but if a screenshot comparison is needed to verify "v11 is stripey void" vs "v10 is painted" then add 10-30 minutes for visual verification.

**Approval text needed**: `rebase`

### 2.2 Option (b) — Force-push: `git push --force-with-lease codeberg main-temp`

**What happens**: The local's 1238 commits are pushed to codeberg, making codeberg match the local HEAD. The divergent commit `6a7cf69324` is DESTROYED on codeberg (orphaned, but still in the local reflog for ~30-90 days).

**Preconditions**: None (working tree state doesn't matter for push).

**Irreversibility**: HIGH. The divergent commit is lost from codeberg immediately. It's only recoverable from the local reflog for ~30-90 days, then truly gone (unless a clone exists elsewhere — none found).

**Files at risk**: ALL 51 files of the divergent commit's work. This includes:
- **SECURITY FIX** (`fix-ovh-access-key-id-misconfig`) — losing this leaves the operator vulnerable
- **SECURITY HARDENING** (`tighten-gitignore-explicit-denylist`) — losing this reverts the security improvement
- 41 binary content files (terrain/road/town tiles) — expensive to regenerate
- 10 text files (cookbook.json data, code changes, docs)

**AGENTS.md implications**: ❌ **EXPLICITLY FORBIDDEN** without override. AGENTS.md says: "NEVER force-push to repos with > 5 commits ahead". This repo is **1238 commits ahead**. Force-pushing here is the exact scenario AGENTS.md warns about, same risk class as the 2026-06-21 force-push incident.

**Recovery procedure if it goes wrong**:
- The divergent commit is in the local reflog (entries 1249, 1255) — recoverable via `git reset --hard 6a7cf69324` within ~30-90 days
- After the reflog expires, the commit is truly gone unless someone has a clone
- There are NO other clones/backups found

**Approval text needed**: `force-push — I explicitly override the AGENTS.md 'NEVER force-push on >5-commits-ahead' rule for this specific case` (exact text required)

### 2.3 Option (c) — Accept permanent stuck state

**What happens**: No action taken. The PUSH_STUCK status in `dracon-sync repos` persists. The divergent commit remains on codeberg, unreachable from the local working copy. The daemon keeps trying to push and failing on every cycle.

**Preconditions**: None.

**Irreversibility**: Low. The divergent commit is safe on codeberg. The operator can revisit this decision at any time.

**Files at risk**: None (no files are modified or lost).

**AGENTS.md implications**: ✅ No AGENTS.md rules violated.

**Side effects**:
- PUSH_STUCK classification persists (CONCERN status in `dracon-sync repos`)
- The divergent commit becomes orphaned on codeberg
- The operator must remember "there's a security fix on codeberg I don't have locally"
- The 5-failure backoff counter resets on every daemon restart, so the daemon will keep trying
- The hint in `dracon-sync repos` will continue to suggest `repair-concerns --apply`

**Approval text needed**: `accept stuck`

### 2.4 Tradeoff comparison

| Dimension | (a) Rebase | (b) Force-push | (c) Accept stuck |
|---|---|---|---|
| Preserves divergent commit | ✅ Yes | ❌ No (reflog only) | ✅ Yes (orphaned) |
| Preserves security fix | ✅ Yes | ❌ No | ⚠️ On codeberg only |
| AGENTS.md compliant | ✅ Yes | ❌ Needs override | ✅ Yes |
| Local commits preserved | ✅ Yes (new SHAs) | ✅ Yes (same SHAs) | ✅ Yes |
| Working tree handling | Must stash | No change needed | No change needed |
| Conflict resolution needed | ⚠️ ~10 text files | None | None |
| Time to complete | ~5-30 min | ~10 sec | 0 sec |
| Reversibility | Medium (reflog) | Low (reflog only) | High (revisit anytime) |
| Risk class | Low | HIGH | None |

---

## Section 3 — Operator's decision

**Decision**: [TO BE FILLED IN BY OPERATOR — one of: rebase / force-push — I explicitly override the AGENTS.md 'NEVER force-push on >5-commits-ahead' rule for this specific case / accept stuck]

**Timestamp**: [TO BE FILLED IN]

**Rationale**: [TO BE FILLED IN]

---

## Section 4 — Implementation steps

[TO BE FILLED IN AFTER OPERATOR'S DECISION — exact commands and their output]

### 4.1 If option (a) rebase

```
# 1. Stash working tree changes
cd /home/dracon/Dev/dracon-platform
git stash push --include-untracked --message "pre-rebase-stash-2026-06-27" -- \
  web/games/.env.ovh \
  web/games/wip/hellhunter/.pi/goals/active_goal_2026062702305590_mqvoohsw-p75onv.md \
  web/games/wip/darklord/.tmp-audit/

# 2. Run the rebase
git pull --rebase codeberg main-temp

# 3. Resolve conflicts (expected on ~10 text files)
# ... (conflict resolution details)

# 4. Restore stashed changes
git stash pop

# 5. Verify state
git rev-parse HEAD
git rev-list --count codeberg/main-temp..HEAD  # should be 0 after daemon pushes
git rev-list --count HEAD..codeberg/main-temp  # should be 0

# 6. Daemon pushes the rebased local to codeberg (automatic)
```

### 4.2 If option (b) force-push

```
# WARNING: This destroys the divergent commit on codeberg
# AGENTS.md override required (captured in Section 3)

cd /home/dracon/Dev/dracon-platform
git push --force-with-lease codeberg main-temp
```

### 4.3 If option (c) accept stuck

```
# No implementation steps — just document the decision
# Optional: add a note to .dracon/dracon-sync.toml explaining the conscious stuck-state decision
```

---

## Section 5 — Post-resolution verification

[TO BE FILLED IN AFTER IMPLEMENTATION]

### 5.1 Post-resolution state

- [ ] `dracon-sync repos` shows new status (PUSH_STUCK → OK for a/b, or PUSH_STUCK with stuck-state note for c)
- [ ] `git rev-list --count codeberg/main-temp..HEAD` = expected value
- [ ] `git rev-list --count HEAD..codeberg/main-temp` = expected value
- [ ] Daemon transitions from PUSH_STUCK to OK (for a/b)
- [ ] `push-stuck-repos-after.txt` captured

### 5.2 No collateral damage

- [ ] `git remote -v` in `/home/dracon/Dev/dracon-platform` shows the same remotes as before this goal
- [ ] Other 14 watched repos are untouched
- [ ] `~/.dracon/secrets/pat/*.env` files are untouched (mode 600, contents unchanged)

---

## Section 6 — Lessons learned

[TO BE FILLED IN AFTER IMPLEMENTATION]

For future PUSH_STUCK situations:
- **Check the merge-base first** — if the local is a fast-forward of the remote, a regular `git push` works (no force needed)
- **Inspect the divergent commit's content** — force-pushing a security fix is different from force-pushing a typo fix
- **Check the reflog** — recovery window is ~30-90 days, so force-push is not always immediately catastrophic
- **Document the decision** — even for option (c), the conscious stuck-state decision should be recorded

---

**Status**: Awaiting operator's decision. Investigation complete, three options presented, design doc skeleton ready.**
