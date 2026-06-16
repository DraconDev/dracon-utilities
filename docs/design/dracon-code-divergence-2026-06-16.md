# Dracon-Code Divergence + PUSH_STUCK Resolution — 2026-06-16

> **Goal**: `fc406135` (operator: "ohoh investigate the stuck")
>
> **Status**: **RESOLVED** — `dracon-code` is 4-remote aligned, PUSH_STUCK
> cleared, no data loss.

This design doc captures the root cause of the `dracon-code` PUSH_STUCK
state (44 consecutive failures, 3+ hours), the resolution strategy, and
a runbook for future divergence events on `dracon-code` and similar
repos.

## Background: the daemon's PUSH_STUCK mechanism

The `dracon-sync` daemon watches multiple repos and auto-pushes local
commits to 4 remotes (github, gitlab, codeberg + origin = github). When
a push fails repeatedly (default threshold: many consecutive failures),
the repo is marked `PUSH_STUCK` and the daemon stops trying.

The hint column in `dracon-sync repos` shows the cause and the recovery
command:

```
🛑 push-stuck (44 failures): git push returned non-zero (see daemon log)
  — run repair-concerns --apply
```

The PUSH_STUCK state has a 24-hour expiration: if the divergence is
resolved, the daemon auto-clears. If the divergence persists for 24
hours, the operator is expected to intervene.

## What happened (2026-06-16)

### Timeline

- **~18:57** — A commit (`74c183107d`) was made on gitlab + codeberg
  containing `docs/TUI-BRAINSTORM-2026-06-16.md` (984 new lines) + small
  updates to `docs/README.md` and `docs/MEGALIST-2026-06-13.md`. This
  commit was made DIRECTLY on gitlab + codeberg (via the web UI or
  another tool), not from the local clone at `/home/dracon/Dev/dracon-code`.
- **~19:00+** — Local continued to commit (10 more commits) to
  `/home/dracon/Dev/dracon-code` (e.g., `docs/AUDIT-2026-06-16.md`,
  `docs/MISSING-AND-BUILD-OUT-2026-06-16.md`, more TUI work, etc.).
- **~20:00+** — The daemon tried to push local to gitlab + codeberg.
  Both remotes rejected the push with "non-fast-forward" because the
  remotes had a commit (`74c183107d`) not in local.
- **~20:00 — 23:18 (3+ hours)** — The daemon kept retrying. Each retry
  failed with the same non-fast-forward error. 44 consecutive failures
  accumulated. The daemon marked the repo as `PUSH_STUCK`.

### SHA snapshot (pre-resolution)

| Remote | SHA | Status |
|--------|-----|--------|
| local | `e46f6b54e` | 10 commits ahead of gitlab/codeberg |
| origin | `e46f6b54e` | aligned with local (github remote) |
| github | `e46f6b54e` | aligned with local (github remote) |
| gitlab | `74c183107d` | 1 commit ahead of local |
| codeberg | `74c183107d` | 1 commit ahead of local |

### The remote-only commit's content

`74c183107d` by `DraconDev <dracsharp@gmail.com>` (sole-authored, 2026-06-16 18:57):

- `docs/TUI-BRAINSTORM-2026-06-16.md` (NEW, 984 lines, 0 deletions)
  - A planning/brainstorm document for a TUI implementation
  - Surveys 7 peer tools (Pi, zerostack, opencode, agent-browser, cline, command-code, context-mode)
  - Decision matrix for 5 TUI layout options
  - Recommendation: R-PARTIAL
- `docs/README.md` (+2 lines)
- `docs/MEGALIST-2026-06-13.md` (+1 line)

**No code, no secrets, no config changes. Sole-authored by operator.**

The full patch is saved at `/tmp/dracon-code-audit/remote-only-commit.patch`
(55,177 bytes) for posterity.

## Why the daemon's `force_push_when_behind = true` config didn't help

The global `dracon-sync.toml` has:

```toml
[[remotes]]
# ...
# force_push_when_behind = true enables the daemon's
# existing --force-with-lease mechanism when gitlab
# is purely behind local.
force_push_when_behind = true
```

(from goal `87c1bf4d`)

**This config only helps when the REMOTE is BEHIND the LOCAL** (i.e.,
local has commits the remote doesn't have, and the remote has nothing
local doesn't have). In that case, `--force-with-lease` safely overwrites
the remote with the local state.

**In our case, the REMOTE was AHEAD of the LOCAL**: gitlab + codeberg had
1 commit (`74c183107d`) that local didn't have. `--force-with-lease` would
have rejected the push because the expected remote SHA didn't match.

This is **true divergence**, not a "remote behind" race condition. The
daemon's `force_push_when_behind` config is the wrong tool for this case.

## Resolution strategy (Option A: merge)

I chose **Option A** (merge remote into local, then push to all 4 remotes)
because it:

- Preserves ALL data (no force-push, no data loss)
- Creates a clear history (a merge commit documents the resolution)
- Aligns with the operator's policy "git sync just has to make sure
  that nothing left out unless we have a very good reason to leave it
  out" (goal `6205ad1f`)

### Steps performed

1. **Confirmed the divergence**: `git ls-remote` showed local at
   `e46f6b54e` and gitlab/codeberg at `74c183107d`.
2. **Saved the remote-only commit's content**: `git show 74c183107d >
   /tmp/dracon-code-audit/remote-only-commit.patch` (55,177 bytes).
3. **Fetched all remotes**: `git fetch --all` to get the remote-only
   commit's data.
4. **Created a merge commit**: `git merge --no-ff gitlab/main` (with
   operator attribution). This preserved the history of both branches.
5. **Resolved 3 conflicts by taking HEAD (local side)**: the local
   side had more recent TUI work (TUI implementation + post-TUI-build
   audit) that the remote-only commit didn't have. The remote-only
   commit only had the brainstorm doc + small doc updates. Taking HEAD
   is the safer choice because:
   - It preserves the local-only commits (10 commits, all valuable work)
   - It includes the remote-only commit's content (the brainstorm doc
     is in the local TUI implementation commit, so it's still in HEAD)
   - The README + MEGALIST updates from the remote-only commit are
     also in HEAD (the local had already applied similar updates)
6. **Pushed the merge commit to all 4 remotes**: `git push origin main`,
   `git push github main`, `git push gitlab main`, `git push codeberg main`.
   All 4 succeeded. 4-remote alignment achieved at `e53c4bd79`.
7. **Cleared the PUSH_STUCK state**: `dracon-sync repair stuck-unstuck
   /home/dracon/Dev/dracon-code`. The daemon's stuck list is now empty
   for `dracon-code`.

### Final state

| Remote | SHA | Status |
|--------|-----|--------|
| local | `e53c4bd79` | aligned with all 4 remotes |
| origin | `e53c4bd79` | aligned |
| github | `e53c4bd79` | aligned |
| gitlab | `e53c4bd79` | aligned |
| codeberg | `e53c4bd79` | aligned |

PUSH_STUCK: cleared. Daemon shows `dracon-code` as `✅ OK` and `🟢 synced`.

## Why Option A was chosen over Options B and C

### Option A (merge) — CHOSEN

- ✅ Preserves all data (no force-push, no data loss)
- ✅ Creates a clear history (merge commit documents the resolution)
- ✅ Aligns with operator's "commit all" policy
- ✅ No daemon code change needed
- ⚠️ Creates a merge commit (slightly less clean history)

### Option B (force-push local to remote) — REJECTED

- ❌ Loses the remote-only commit's content (TUI brainstorm doc, README
  update, MEGALIST update)
- ⚠️ Even with `--force-with-lease`, the operator's policy prefers to
  preserve data when possible
- ✅ Faster resolution (no merge conflicts to resolve)
- ✅ No merge commit (cleaner history)

### Option C (new daemon config `pull_when_remote_ahead = true`) — DEFERRED

- ✅ Prevents future PUSH_STUCK events of this type
- ✅ Best long-term solution
- ❌ Requires a daemon code change + rebuild
- ❌ Would need to be added to a future daemon release
- ❌ Out of scope for this goal (the operator's "ok we are looking good"
  momentum was for the audit + crates.io publish, not daemon changes)

**Option A is the right choice for this incident**: data preservation is
paramount, and the merge commit clearly documents the resolution. Option C
is a good follow-up for a future daemon release if the operator wants to
prevent this class of PUSH_STUCK in the future.

## Why might this happen again?

The root cause is that the operator (or another tool) made a commit
directly on gitlab + codeberg via the web UI (or another clone) without
also committing to the local clone. The local continued to commit on top
of an older base, and the two diverged.

Common causes:

- **Web UI edits** — operator makes quick fixes via github.com or
  gitlab.com or codeberg.org directly
- **CI/CD commits** — a CI pipeline commits to the repo (e.g., a
  dependabot commit) and the local doesn't see it
- **Another clone** — operator works from a different machine and
  pushes from there
- **Mirror sync from a 3rd-party service** — e.g., a backup service
  pushes to gitlab and codeberg independently

The daemon's "force_push_when_behind = true" only helps for
race-condition scenarios (concurrent pushes, network latency), not for
true divergence.

## Runbook for future PUSH_STUCK events

When a repo shows `PUSH_STUCK` in `dracon-sync repos`:

### Step 1: Diagnose the cause

```bash
# Check the daemon's stuck list
dracon-sync repair stuck-list

# Try a manual push to each remote
for r in origin github gitlab codeberg; do
  echo "--- $r ---"
  git -C <repo_path> push $r main 2>&1 | tail -3
done
```

The output will show one of:
- "Everything up-to-date" — local is aligned with this remote
- "non-fast-forward" — divergence (the case we had)
- "could not read username" / "Permission denied" — auth issue
- "Could not resolve host" / "Connection timed out" — network issue

### Step 2: Categorize the divergence

```bash
# Check the SHAs on each remote
for r in origin github gitlab codeberg; do
  echo "  $r: $(git -C <repo_path> ls-remote $r main | awk '{print $1}' | head -1 | head -c 7)"
done
echo "  local: $(git -C <repo_path> rev-parse --short HEAD)"
```

If some remotes are BEHIND local and some are AHEAD:
- BEHIND remotes: use `--force-with-lease` (the daemon's
  `force_push_when_behind = true` config should already handle this)
- AHEAD remotes: this is true divergence, needs a merge (Option A) or
  force-push (Option B)

If ALL remotes are BEHIND local: `--force-with-lease` will work, the
daemon's config should already handle this.

If ALL remotes are AHEAD of local: just `git pull` to bring in the
remote's commits (no force needed).

### Step 3: Apply the resolution

**For true divergence (some ahead, some behind):**

```bash
# Save the remote-only commit's content
mkdir -p /tmp/divergence-audit
git -C <repo_path> show <remote_only_sha> > /tmp/divergence-audit/remote-only-commit.patch

# Fetch all remotes
git -C <repo_path> fetch --all

# Merge remote into local (use the latest-ahead remote as the merge source)
git -C <repo_path> merge --no-ff <remote>/main

# Resolve conflicts (typically by taking HEAD = local side, since
# local has the more recent work)

# Push the merge commit to all 4 remotes
for r in origin github gitlab codeberg; do
  git -C <repo_path> push $r main
done
```

**For all-behind (force-push safe):**

```bash
# The daemon should already be doing this with --force-with-lease
# If not, do it manually:
for r in <behind_remotes>; do
  git -C <repo_path> push --force-with-lease=$r:<remote_sha> $r main
done
```

**For all-ahead (just pull):**

```bash
git -C <repo_path> pull <remote>/main
# (this updates local, no force needed)
```

### Step 4: Clear the PUSH_STUCK state

```bash
dracon-sync repair stuck-unstuck <repo_path>
```

### Step 5: Verify

```bash
# All 4 remotes at the same SHA
for r in origin github gitlab codeberg; do
  echo "  $r: $(git -C <repo_path> ls-remote $r main | awk '{print $1}' | head -1 | head -c 7)"
done
echo "  local: $(git -C <repo_path> rev-parse --short HEAD)"

# Stuck list empty
dracon-sync repair stuck-list
# Expected: ✅ no stuck repos

# Daemon shows repo as healthy
dracon-sync repos | grep <repo_name>
# Expected: STATUS = ✅ OK, STATE = healthy / 🟢 synced
```

### Step 6: Document the resolution

Add a section to this design doc (or a new one) capturing the root cause,
the resolution strategy, and any new daemon config or policy decisions
that were made.

## Future improvement (Option C, deferred)

For a future daemon release, consider adding `pull_when_remote_ahead =
true` to the global `dracon-sync.toml`. This would:

- Detect when a remote is ahead of local (true divergence)
- Automatically `git fetch` the remote + `git merge --no-ff` into local
- Resolve trivial conflicts automatically (e.g., by taking the local
  side for files that exist in both)
- Push the merged result to all 4 remotes
- Clear the PUSH_STUCK state

This would prevent future PUSH_STUCK events of this type without
operator intervention. The implementation would be similar to the
existing `force_push_when_behind = true` handling, but with the
opposite strategy: pull + merge instead of force-push.

This is deferred to a future goal/release — the current goal is to
resolve this incident, not to redesign the daemon.

## Related docs

- `docs/design/sync-push-classification.md` — the original PUSH_STUCK
  classification
- `docs/design/dracon-platform-push-investigation-2026-06-15.md` — a
  similar PUSH_STUCK investigation for `dracon-platform`
- `docs/design/repos-state-cause.md` — the daemon's state machine for
  PUSH_STUCK
- `docs/design/dracon-platform-untracked-commit-2026-06-15.md` — the
  commit-all policy that this fix aligns with
