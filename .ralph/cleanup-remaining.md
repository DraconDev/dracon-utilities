# Dracon Utilities Cleanup — Remaining Items

## Goal
Complete all remaining cleanup items for the dracon-utilities project after the major refactoring and startup cleanup work.

## Checklist

### Code Fixes
- [x] Fix `get_diff()` in dracon-libs — added CLI fallback on libgit2 errors (nul bytes, binary blobs)
- [x] Fix nvidia AI provider `max_tokens` bug — added `max_tokens: Some(256)` to ChatRequest
- [x] Verify all .gitignore changes are correct (warden-managed block + our additions for /plan/, /plans/, /.ralph/, scratch files)
- [x] Ensure `scripts/cleanup-github-orphans.sh` is tracked (git add) and executable (755)
- [x] Run full clippy on all crates and fix remaining warnings — **0 warnings remaining**
- [x] Verify all 694 tests pass on release build
- [x] Deploy release binaries to all 3 services — all active (running)

### Daemon Hardening
- [x] Add `repair_broken_tracking` to the periodic loop (every ~300 cycles / ~5 min) — not just startup
- [ ] ~~Consider: daemon should verify push actually succeeded (ahead count drops)~~ — deferred, complex to implement without race conditions (daemon may commit during push verification)

### Documentation
- [x] Update CHANGELOG.md with startup cleanup, broken tracking repair, dead code cleanup, orphan cleanup script, AI max_tokens fix, get_diff fallback
- [x] Update AGENTS.md with startup cleanup section, broken tracking repair, and GitHub orphan cleanup script

### Manual Items (inform user, don't automate)
- [ ] User needs to run `gh auth refresh -h github.com -s delete_repo` then `./scripts/cleanup-github-orphans.sh --apply` to delete 83 orphan repos
- [ ] Monitor daemon stability with Restart=always over 24h

## Constraints
- All 694 tests must pass after every change ✅
- No breaking changes to function signatures ✅
- Release build must pass before deployment ✅
- Use `pub(crate) use module::*;` re-export pattern ✅
