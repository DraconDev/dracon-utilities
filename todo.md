# Dracon Utilities — TODO

**Audit date:** 2026-05-24

---

## 🔴 Must Fix

- [ ] **Settle GitHub Actions billing**
  - All CI jobs blocked: *"Recent account payments have failed"*
  - → `https://github.com/settings/billing`

- [ ] **Bump `git2` in dracon-libs/dracon-git** (RUSTSEC-2026-0008)
  - Currently pinned to 0.18.3; fix pending

- [ ] **Investigate `wal-backup` daemon sync loop**
  - 12+ rapid triage entries in incident ledger
  - Stale `index.lock` failure at least once

## 🟡 Should Fix

- [ ] **Verify auto_create works for all 3 platforms**
  - GitHub, GitLab, Codeberg should all auto-create remotes for new repos
  - Test with a fresh clone
  - Check incident ledger for errors

- [x] **Fix 3 CONCERN repos** (see `AUDIT.md`)
  - ✅ `pully-fully-pull-based-fleet-reconciler` — GitHub repo created, 19 commits pushed
  - ✅ `cli-file-manager` — GitHub/GitLab/Codeberg remotes added, initial commit pushed
  - ✅ `avid` — GitHub/GitLab/Codeberg remotes added, initial commit pushed (dirty state auto-resolves)

## ✅ Done

- [x] **Docs cleanup** — Archived 5 redundant docs to `ARCHIVE/`
- [x] **Full repo audit** — 27 repos, 24 OK, 3 CONCERN
- [x] **Auto-create all platforms** — `auto_create = true` for GitHub, GitLab, Codeberg
- [x] **Architecture spec** — `dracon-sync-architecture.md`
