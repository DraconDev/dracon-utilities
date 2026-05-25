# Full Repo Audit — 2026-05-24 (Detailed Task List)

**Status:** ✅ OK 25  ⚠️ WARN 2  ❌ CONCERN 0  👻 Ghost 2
**Goal:** Eliminate ghost repos, resolve WARNs, document decisions

---

## 📊 Summary

| Status | Count | Note |
|--------|-------|------|
| ✅ OK | 25 | All 4 remotes working |
| ⚠️ WARN | 2 | avid, cli-file-manager — dirty files |
| ❌ CONCERN | 0 | |
| 👻 Ghost | 2 | Empty dirs (to delete) |

---

## 🔧 Action Items (Loopable)

### Priority 1: Delete ghost repos (safe, no content)

| Task | Command | Outcome |
|------|---------|---------|
| Delete contextual-auto-banner | `rm -rf ~/Dev/contextual-auto-banner` | Removes empty dir |
| Delete dracon-spark-and-director | `rm -rf ~/Dev/dracon-spark-and-director` | Removes `.ralph/` artifacts |
| Delete dragon-spark-and-director | `rm -rf ~/Dev/dragon-spark-and-director` | Removes 1 shell script |

### Priority 2: Fix WARN repos (auto-resolve on next daemon cycle)

| Task | Status | Notes |
|------|--------|--------|
| avid — 1 dirty file | ⏳ Auto-commit | Daemon will commit in next pulse |
| cli-file-manager — 1 dirty file | ⏳ Auto-commit | Daemon will commit in next pulse |

### Priority 3: Verify remote matrix (manual check)

Run on all 25 OK repos:
```bash
for r in */; do
  echo "=== $(basename $r) ==="
  echo "Remotes: $(git -C $r remote | wc -l)"
  echo "Commits: $(git -C $r rev-list --count)"
done
```

Expected output: 4 remotes (origin + github + gitlab + codeberg) per repo.

### Priority 4: Verify todo.md exists and is lowercase

```bash
for r in */; do
  if [ ! -f "$r/todo.md" ]; then
    echo "MISSING todo.md: $r"
  fi
done
```

### Priority 5: Check for dual-main/master branches

```bash
for r in */; do
  branches=$(git -C $r branch | grep -E "^\* (main|master)")
  if [ $(echo "$branches" | wc -l) -gt 1 ]; then
    echo "DUAL_BRANCH: $(basename $r)"
  fi
done
```

### Priority 6: Verify all 3 mirrors have matching commits

```bash
for r in */; do
  origin_count=$(git -C $r rev-list --count origin/main)
  github_count=$(git -C $r rev-list --count github)
  gitlab_count=$(git -C $r rev-list --count gitlab)
  codeberg_count=$(git -C $r rev-list --count codeberg)
  echo "$(basename $r): origin=$origin_count github=$github_count gitlab=$gitlab_count codeberg=$codeberg_count"
done
```

### Priority 7: Audit commit message quality

Check next 5 commits per repo for `sync: N checked` format:
```bash
git -C ~/Dev/dracon-utilities log --oneline -5
```

Expected: `sync: X checked` + JSON body (when `todo_commit_messages` is active)

### Priority 8: Verify auto_create works on new repos

```bash
rm -rf /tmp/test-repo && mkdir /tmp/test-repo && cd /tmp/test-repo
git init && git commit -m "init" --allow-empty
echo "=== Before sync ===" && git remote -v
# Run daemon
cd /home/dracon/Dev/dracon-utilities
dracon-sync sync-now /tmp/test-repo
echo "=== After sync ===" && git remote -v
```

Expected: All 4 remotes created.

### Priority 9: Monitor daemon health

```bash
# Check recent events
systemctl --user status dracon-sync.service
journalctl --user -u dracon-sync.service -n 50

# Check incident ledger
cat ~/.local/state/dracon/dracon-sync-incidents.jsonl | tail -20
```

---

## ✅ Resolved This Session

| Issue | Fix |
|-------|-----|
| 2 broken origins | Fixed origins for dracon-voice-notifications + ai-vid-editor |
| todo-addict no .git | Initialized git, all 3 mirrors auto-created |
| auto_create disabled | Added `auto_github_private = true` + `todo_commit_messages = true` to policy |
| Policy config | Committed to `9d2fb51d`, daemon restarts needed to pick up changes |

---

## 🔧 How to Run This Loop

### Option 1: Manual one-offs
```bash
# Delete ghosts
rm -rf ~/Dev/contextual-auto-banner ~/Dev/dracon-spark-and-director ~/Dev/dragon-spark-and-director

# Check remote matrix
for r in ~/Dev/*/; do git -C "$r" remote | wc -l; done

# Check WARN repos
git status ~/Dev/avid ~/Dev/cli-file-manager
```

### Option 2: Scripted loop
```bash
#!/bin/bash
# audit-loop.sh

echo "=== Ghost cleanup ==="
rm -rf ~/Dev/contextual-auto-banner ~/Dev/dracon-spark-and-director ~/Dev/dragon-spark-and-director

echo "=== Remote matrix ==="
for r in ~/Dev/*/; do
  echo "$(basename $r): $(git -C "$r" remote | wc -l) remotes"
done

echo "=== WARN check ==="
git status ~/Dev/avid ~/Dev/cli-file-manager 2>/dev/null || echo "No WARNs"

echo "=== Git history ==="
git -C ~/Dev/dracon-utilities log --oneline -5

echo "=== Incident ledger ==="
cat ~/.local/state/dracon/dracon-sync-incidents.jsonl | tail -10
```

### Option 3: Daemon-driven (recommended)
```bash
# Restart daemon to pick up config changes
systemctl --user restart dracon-sync.service

# Wait for daemon to auto-commit WARN repos
sleep 2

# Check status
dracon-sync repos
```

---

## 📋 Decision Log

### 2026-05-24

| Decision | Rationale |
|-----------|------------|
| Delete ghost repos | They have no content, no git history, just noise |
| Keep WARN repos (avid, cli-file-manager) | Dirty files are legitimate; daemon auto-resolves |
| Fix broken origins | Prevents push failures and sync confusion |
| Enable todo_commit_messages | Deterministic, reproducible commit messages |
| All 3 mirrors auto-create | Ensures new repos get remotes on all platforms |
| Config outside repo | Policy lives in `~/.dracon/utilities/sync/dracon-sync.toml`, not in git |
| Daemon restarts pick up config | No need to commit policy changes to repos |

---

## 🔄 Next Steps

1. **Delete ghost repos** (3 dirs)
2. **Restart daemon** to auto-commit WARN repos
3. **Verify remote matrix** on all 25 repos
4. **Run manual audit loop** (Option 2 above)
5. **Monitor incident ledger** for any new WARNs/CONCERNs