# Full Repo Audit — AI-to-AI Version Control

**Date:** 2026-05-23
**Total Repos:** 26

---

## 📊 Summary

| Status | Count |
|--------|-------|
| ✅ Has GitHub Remote | 25 |
| ❌ No Remote | 1 |

**Total:** 26 repos audited

---

## 🎯 Audit Tasks

### Primary Audit: Verify All Repos Have GitHub Remotes

For each repo in `~/Dev`, verify that `git remote -v` shows both `github` and `codeberg` remotes.

**Command to run:**
```bash
cd ~/Dev && find . -name ".git" -type d 2>/dev/null | while read repo; do
  echo "=== $(basename "$repo") ==="
  git -C "$repo" remote -v
done
```

**Expected Result:**
- 25 repos should show:
  ```
  codeberg    git@codeberg.org:dracondev/<repo>.git (fetch)
  codeberg    git@codeberg.org:dracondev/<repo>.git (push)
  github      git@github.com:DraconDev/<repo>.git (fetch)
  github      git@github.com:DraconDev/<repo>.git (push)
  ```

- 1 repo (auto-ai-video-processor-folder-watcher-daemon-cli) shows **NO REMOTE**

---

## 🔍 Current Findings

### ✅ OK — Has GitHub Remote (25 repos)

1. ai-auto-repo-rot-scanner-todo-agent
2. ai-auto-writer
3. ai-vid-editor
4. azumi
5. browser-extensions-shared
6. dracon-code
7. dracon-demons
8. DraconDev
9. dracon-libs
10. dracon-platform
11. dracon-rust-ui
12. dracon-terminal-engine
13. dracon-utilities
14. dracon-voice-notifications
15. dragon-spark-and-director
16. Junk-Runner-bevy
17. opencode-auto-review-completed-todos
18. pi-auto-review
19. pully-fully-pull-based-fleet-reconciler
20. respec
21. sqlite-embedded-continuous-wal-backup-to-object-storage
22. tiles
23. todo-addict
24. video-factory
25. video-uploader
26. wal-backup

### ❌ CONCERN — No Remote (1 repo)

1. **auto-ai-video-processor-folder-watcher-daemon-cli**
   - Location: `~/Dev/auto-ai-video-processor-folder-watcher-daemon-cli`
   - Status: Has `.git` but no remotes configured
   - Expected: Should have auto-created GitHub private repo
   - Action: Investigate why auto_create didn't work

---

## 🛠️ Action Items

### 1. Fix auto-ai-video-processor-folder-watcher-daemon-cli

This repo has no remote configured. The auto_create should have created one.

**Check:**
```bash
cd ~/Dev/auto-ai-video-processor-folder-watcher-daemon-cli
git remote -v
```

**Expected output (if working):**
```
github  git@github.com:DraconDev/auto-ai-video-processor-folder-watcher-daemon-cli.git (fetch)
github  git@github.com:DraconDev/auto-ai-video-processor-folder-watcher-daemon-cli.git (push)
```

**If empty:**
1. Check `~/.dracon/utilities/sync/dracon-sync.toml` has `auto_create = true` for GitHub remote
2. Check that `gh` CLI is authenticated: `gh auth status`
3. Manually create the repo: `gh repo create auto-ai-video-processor-folder-watcher-daemon-cli --private`
4. Add remote: `git remote add origin https://github.com/DraconDev/auto-ai-video-processor-folder-watcher-daemon-cli.git`

---

### 2. Verify Auto-Create is Working

Test with a fresh clone to confirm auto_create works:

```bash
cd /tmp && rm -rf test-auto-create && git clone https://github.com/DraconDev/test-auto-create.git test-auto-create
cd test-auto-create
git remote -v
```

**Expected:** Should show github remote immediately (auto_create should have created it).

---

### 3. Document Findings in Repo-Specific TODOs

For each repo, update its `TODO.md` with:
- Current remote status
- Any issues found
- Next steps

---

## 📝 Notes

- **auto_create = true** was added to `~/.dracon/utilities/sync/dracon-sync.toml`
- GitHub has 30 repos under `DraconDev` account
- Incident ledger shows auto_create attempts working
- The CONCERN repo (`auto-ai-video-processor-folder-watcher-daemon-cli`) needs manual investigation

---

## 🔁 For Later AI Agent Loop

This file is designed for the AI to:
1. Read this audit
2. Loop through each task
3. Check repo status
4. Document findings in repo-specific TODOs
5. Fix broken auto_create logic if needed

The AI agent can use this as a checklist to verify all repos have proper remote configuration before syncing.
