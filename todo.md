# Dracon Utilities — TODO

## Done ✅
- [x] Clone race bug: IndexLock acquires `.git/index.lock` (git's own protocol) before working-tree writes in both warden and sync daemons. O_EXCL atomic, no TOCTOU race, no heuristics.
- [x] Global git filter.dracon path: fixed ~/.cargo/bin → ~/.local/bin
- [x] Mass deletion guard: REMOVED entirely — IndexLock fixes root cause (clone race). Git revert is the safety net. Prometheus counter kept as always-0 for compat.
- [x] `repo_diff_entries` root cause fix: returns ALL diff entries when dirty, not just when staged
- [x] Status reporting honesty: WARN uses real dirty state, not `effective_dirty`. MOD/STG always show real counts.
- [x] GitHub transport: HTTPS+PAT (removed `insteadOf` URL rewrite)
- [x] Push timeouts: 60s per push, 120s per repo
- [x] AI provider timeouts: 10s per provider, TransientFailed 5-min cooldown
- [x] AI skip for >20 file diffs
- [x] dirty_since tracking: 5s max delay for actively-edited repos
- [x] Desktop notifications for push failures (30-min rate limit per repo)
- [x] **Sustained-state desktop notifications** (new):
  - Stuck Ahead: repo has unpushed commits for >10 min (30-min rate limit)
  - Stuck Behind: upstream has unpulled changes for >30 min (30-min rate limit)
  - Mirror Degraded: mirror push fails 3+ consecutive cycles (30-min rate limit per mirror)
- [x] Systemd sandboxing: 14 directives per service
- [x] Module extraction: git.rs (11 modules), system/main.rs (5 modules)
- [x] CI/CD: 7 jobs, 15 checks
- [x] All mirrors synced (GitHub, GitLab, Codeberg)
- [x] Guard never-kills invariant enforced
- [x] `git add -A -f` bug: removed unconditional `-f`. `partition_gitignored()` splits paths — non-ignored → `git add -A`, tracked-but-ignored → `git add -A -f`, untracked-and-ignored → skip. Prevents target/.output/ force-add.
- [x] Install script: stops daemons before binary copy, PATH scan, process verification
- [x] Orphan gitlink cleanup: dracon-code examples/phase3+4 removed, browser-extensions cinematic-pages converted from submodule to subtree (regular tracked files)
- [x] Subtree convention: repos-with-repos-inside use subtree (no .git dirs in children), clone gets everything
- [x] Time display: seconds ≥60 show as minutes ("84s" → "1m24s"), minutes ≥60 show as hours+minutes
- [x] .ralph/ session dirs gitignored where found
- [x] Startup cleanup: stale stuck repos, incident ledger retention, visibility cache orphans, broken tracking refs, stale .git/index.lock, guard log rotation

## Remaining 🔧
- [ ] Monitor daemon stability with all fixes over 24h — clone race should be eliminated, git add should never force-add build artifacts
- [ ] Fix nvidia/openrouter AI providers returning "parse response" errors
- [ ] Guard/storage extraction from system/main.rs (deeply coupled, low priority)
- [ ] Switch GitLab/Codeberg to HTTPS+PAT for consistency
- [ ] Warden security lib extraction (3,500 lines, 75 pub fn)
- [ ] Improve journald reliability (tracing-journald crate?)
- [ ] Handle untracked-only repos in daemon detection
- [ ] Test that clone-into-Dev always works (automated test)

## Key Decisions 📋
- **GitHub HTTPS+PAT as primary**: More reliable than SSH
- **Subtree over submodule**: Clone gets everything, no `--recurse-submodules` needed
- **Mass deletion guard removed**: IndexLock fixes root cause. Git revert is the safety net.
- **Guard NEVER kills**: Only renices. Code-level invariant.
- **Sync = transport only**: NOT a CI orchestrator
- **Real dirty counts**: MOD/STG columns always show actual file counts, not filtered zeros
- **Short push timeouts (60s)**: Long timeouts blocked entire daemon
- **dirty_since > fingerprint-based delays**: 5s max delay for actively-edited repos
- **partition_gitignored()**: Prevents build artifacts from being force-added while still re-staging tracked-but-ignored files

## Test Count 🧪
- dracon-sync: 456 tests (0 failures)
- dracon-system: 81 tests
- dracon-warden: 65 tests
- Total: 602 tests, serial execution (`--test-threads=1`)
