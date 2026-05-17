
# Dracon Utilities Cleanup — Remaining Items

## Goal
Complete all remaining cleanup items for the dracon-utilities project after the major refactoring and startup cleanup work.

## Checklist

### Code Fixes
- [ ] Fix `get_diff()` in dracon-libs — uses libgit2 directly, no CLI fallback on NUL byte errors (causes tiles sync failures)
- [ ] Fix nvidia AI provider `max_tokens` bug (generates negative values like -53106)
- [ ] Verify all .gitignore changes are correct (warden-managed block + our additions)
- [ ] Ensure `scripts/cleanup-github-orphans.sh` is tracked and executable
- [ ] Run full clippy on all crates and fix remaining warnings
- [ ] Verify all 694 tests pass on release build
- [ ] Deploy release binaries to all 3 services

### Daemon Hardening
- [ ] Add `repair_broken_tracking` to the once-per-cycle loop (not just startup) — new tracking breaks can appear during runtime
- [ ] Consider: daemon should verify push actually succeeded (ahead count drops) after reporting "ok: pushed"

### Documentation
- [ ] Update CHANGELOG.md with startup cleanup, broken tracking repair, and orphan cleanup script entries
- [ ] Update AGENTS.md with `repair_broken_tracking` mention and orphan cleanup script

### Manual Items (inform user, don't automate)
- [ ] User needs to run `gh auth refresh -h github.com -s delete_repo` then `./scripts/cleanup-github-orphans.sh --apply` to delete 83 orphan repos
- [ ] Monitor daemon stability with Restart=always over 24h

## Constraints
- All 694 tests must pass after every change
- No breaking changes to function signatures
- Release build must pass before deployment
- Use `pub(crate) use module::*;` re-export pattern
