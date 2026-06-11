# Branch-reconciliation approval log

Format: ts | scope | user_decision | evidence_ref | applied

2026-06-11T18:19:38+01:00 | init | n/a | branch-reconciliation goal created | n/a
2026-06-11T18:25+01:00 | public-release | user picked 'merge to main, delete branch' | ask_user_question 1/4 | pending-execute
2026-06-11T18:25+01:00 | folder-auto-banner | user says 'we were done with it that is why i deleted' — keep trash, log policy | ask_user_question 2/4 | documented
2026-06-11T18:25+01:00 | one-mil-girls | user picked 'commit 3 src changes as-is' | ask_user_question 3/4 | pending-execute
2026-06-11T18:25+01:00 | Junk-Runner-bevy | user picked 'keep tauri2, log divergence' | ask_user_question 4/4 | documented

# ===== Applied actions 2026-06-11 =====

## public-release → merged to main, deleted on all 4 remotes
  - paused daemon: dracon-sync pause
  - stashed 2 uncommitted evidence files
  - git checkout main (was 43d7505d)
  - git merge --no-ff public-release → 242783df (10 new files, 3856 insertions)
  - git commit f99961ce for the 2 uncommitted + 1 untracked evidence files
  - git push origin main: 43d7505d..f99961ce OK
  - git push github main: up-to-date (already pushed by daemon)
  - git push gitlab main: 43d7505d..f99961ce OK
  - git push codeberg main: 43d7505d..f99961ce OK
  - git push origin --delete public-release: OK
  - git push gitlab --delete public-release: OK
  - git push codeberg --delete public-release: OK
  - git push github --delete public-release: already deleted by daemon
  - git branch -d public-release: OK (was 91a5664e)
  - git remote prune github: cleaned stale remotes/github/public-release tracking ref

## folder-auto-banner → policy block noted, no action
  - user: 'we were done with it that is why i deleted'
  - trashed copy at ~/.local/share/Trash/files/folder-auto-banner (2.3 GB) preserved
  - deletion date: 2026-06-06T23:34:17
  - daemon's stale /home/dracon/Dev/folder-auto-banner reference will be self-cleaned on next startup_cleanup

## one-mil-girls → committed 3 src changes, pushed to origin/github/codeberg; GitLab blocked by protected-branch policy
  - git add 'src/lib/engine/characters.test.ts' 'src/lib/engine/saveLoad.test.ts' 'src/lib/stores/saveLoad.svelte.ts'
  - git commit 6bf75d9 '3 file(s) in src [src/lib/engine/characters.test.ts, src/lib/engine/saveLoad.test.ts, src/lib/stores/saveLoad.svelte.ts] DELTA:+4/-4'
  - decoded diff was pure cosmetic comment edits (shim→stub, comment rewording) — not destructive
  - git push origin main: 33f1eb1..6bf75d9 OK
  - git push github main: up-to-date (daemon already pushed)
  - git push codeberg main: 33f1eb1..6bf75d9 OK
  - git push gitlab main: REJECTED — 'You are not allowed to push code to protected branches on this project.'
  - 31 untracked files in docs/audit/2026-06-11-cleanup/ left in place (not deleted, not added)

## Junk-Runner-bevy → no action, tauri2 kept as the working branch
  - user: 'keep tauri2, log divergence'
  - ahead/behind origin/main: 2604/1 (no local main divergence)
  - stash list: empty
  - documented in evidence/junk-runner-bevy-state.md

## Resulting state
  - main on all 4 remotes is at f99961ce (the merge + post-merge evidence commit)
  - public-release is gone from all 4 remotes and local
  - one-mil-girls main on origin/github/codeberg is at 6bf75d9; gitlab still at 2026-06-05 ancestor
  - folder-auto-banner: trashed (no auto-cleanup needed; user already deleted)
  - Junk-Runner-bevy: untouched
