# Project State

## Current Focus
Fix multi-remote auth infra: SSH config for Codeberg, HTTPS+PAT fallback for GitLab/Codeberg, stall prevention, mass-deletion guard hardening

## Context
Completed backlog sprint to make dracon-sync work reliably across all 3 remotes (GitHub, GitLab, Codeberg). Fixed SSH config to include custom key path so Codeberg pushes work. Added HTTPS+PAT fallback for GitLab and Codeberg when SSH fails. Fixed stall bug where committed-but-still-behind repos would never pull before pushing. Fixed repo discovery descending into subdirs of already-discovered repos. Hardened mass-deletion guard with secondary check, incident logging, and `--force` bypass.

## Completed
- [x] SSH config fix: `GIT_SSH_HARDENING` → function with `-F $HOME/.dracon/secrets/ssh/config` for Codeberg SSH key discovery
- [x] Codeberg push verified: SSH handshake + git push both succeed
- [x] Codeberg PAT created + stored: `~/.dracon/utilities/sync/secrets/codeberg.env`
- [x] GitLab PAT configured and stored: `~/.dracon/utilities/sync/secrets/gitlab.env`
- [x] HTTPS+PAT fallback: `gitlab_https_url()` + `codeberg_https_url()` in push transport fallbacks
- [x] `GIT_TERMINAL_PROMPT=0` + `BatchMode=yes` on all push commands (no interactive prompts)
- [x] Post-commit pull check: if still behind upstream after commit, pull before pushing
- [x] Repo discovery fix: `continue` after `.git` dir found prevents descending into subdirs
- [x] Mass-deletion guard: fixed invalid `git ls-files --count`, added secondary >50% check, incident logging
- [x] `--force` flag: `dracon-sync sync-now --force <repo>` bypasses mass-deletion guard
- [x] Alert threshold: `alert_unpushed_threshold` field (default 10) + Prometheus counter
- [x] Stall fixes: repaired dracon-code (304 commits merged), browser-extensions-shared (node_modules), tiles (debug.log)
- [x] Docs: AGENTS.md (test count 358, incident response), CHANGELOG.md, README.md (safety, PAT setup, alert docs)
- [x] All 358 tests passing (was 351) — added 8 boundary tests + alert threshold tests
- [x] 42 orphan repos identified from old suffix loop bug (left for manual cleanup)

## In Progress
- (none)

## Blockers
- Orphan repo deletion blocked by GitHub sudo mode; user prefers to leave them

## Next Steps
1. Monitor multi-remote sync stability (GitHub + GitLab + Codeberg)
2. Consider periodic `gh auth refresh` automation for `delete_repo` scope
