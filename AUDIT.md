# Full Repo Audit — 2026-05-24

**Status:** ✅ OK 25  ⚠️ WARN 2  ❌ CONCERN 0  👻 Ghost 2
**Total:** 27 (+ 2 ghost dirs)

---

## 📊 Summary

| Status | Count | Notes |
|--------|-------|-------|
| ✅ OK | 25 | All 3 mirrors + origin, no issues |
| ⚠️ WARN | 2 | avid, cli-file-manager — dirty files, daemon auto-commits |
| 👻 Ghost | 2 | Empty dirs, no .git (see below) |
| ❌ CONCERN | 0 | |

---

## 🔧 Fixed This Session

| Issue | Repo | Fix |
|-------|------|-----|
| Broken origin | `dracon-voice-notifications` | origin pointed to `kiki-sassy-desktop-announcer` → fixed to `dracon-voice-notifications` |
| Broken origin | `ai-vid-editor` | origin pointed to `ai-gui-auto-video-editor` → fixed to `ai-vid-editor` |
| No .git | `todo-addict` | Initialized git, all 3 mirrors auto-created on sync |
| Missing config | all new repos | Added `auto_github_private = true` + `todo_commit_messages = true` to policy |

---

## 👻 Ghost Repos (No .git)

These dirs were discovered as git repos by the daemon but contain no commits:

### `contextual-auto-banner/`
- **State:** Empty directory (0 files)
- **Action needed:** Delete from disk, or add content + init

### `dracon-spark-and-director/`
- **State:** `.ralph/` directory only (audit artifacts from a Ralph loop)
- **Action needed:** Delete from disk, or move .ralph elsewhere and init

### `dragon-spark-and-director/`
- **State:** 1 file (`autoresearch.sh` — unrelated shell script)
- **Action needed:** Delete from disk, or init as a real repo

---

## ⚠️ WARN Repos (Dirty Files)

Daemon auto-commits these — no manual action needed:

| Repo | Dirty | Last Commit |
|------|-------|-------------|
| `avid` | 1 file | `chore(sync): update .gitignore` (19h ago) |
| `cli-file-manager` | 1 file | `chore: init cli-file-manager` (15h ago) |

---

## ✅ All 27 Real Repos: Complete Remote Matrix

| Repo | Origin | GitHub | GitLab | Codeberg |
|------|--------|--------|--------|----------|
| .dracon | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| ai-auto-repo-rot-scanner-todo-agent | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| ai-auto-writer | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| ai-vid-editor | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| avid | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| azumi | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| browser-extensions-shared | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| cli-file-manager | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| dracon-code | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| dracon-demons | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| DraconDev | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| dracon-libs | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| dracon-platform | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| dracon-rust-ui | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| dracon-terminal-engine | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| dracon-utilities | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| dracon-voice-notifications | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| Junk-Runner-bevy | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| opencode-auto-review-completed-todos | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| pi-auto-review | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| pully-fully-pull-based-fleet-reconciler | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| respec | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| sqlite-embedded-continuous-wal-backup-to-object-storage | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| tiles | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| todo-addict | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| video-factory | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| video-uploader | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |
| wal-backup | ✅ HTTPS | ✅ SSH | ✅ SSH | ✅ SSH |

---

## 🔧 Action Items

- [ ] **Delete ghost repos**: contextual-auto-banner, dracon-spark-and-director, dragon-spark-and-director
- [x] **Fix broken origins**: dracon-voice-notifications ✅, ai-vid-editor ✅
- [x] **Fix auto_create**: added auto_github_private + todo_commit_messages to policy ✅
- [ ] **Commit policy change**: sync daemon restart needed to pick up new config