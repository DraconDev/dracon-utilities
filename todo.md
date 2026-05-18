# Dracon Utilities — TODO

## Done ✅
- [x] Clone race bug: daemon now skips repos with active index.lock + 15s grace period for newly discovered repos
- [x] Mass deletion guard: tiered thresholds (85%+, 70%+ ≥5 files, 10+ absolute)
- [x] `repo_diff_entries` root cause fix: returns ALL diff entries when dirty, not just when staged
- [x] Status reporting honesty: WARN uses real dirty state, not `effective_dirty`
- [x] GitHub transport: HTTPS+PAT (removed `insteadOf` URL rewrite)
- [x] Push timeouts: 60s per push, 120s per repo
- [x] AI provider timeouts: 10s per provider, TransientFailed 5-min cooldown
- [x] AI skip for >20 file diffs
- [x] dirty_since tracking: 5s max delay for actively-edited repos
- [x] Desktop notifications for push failures (30-min rate limit)
- [x] Systemd sandboxing: 14 directives per service
- [x] Module extraction: git.rs (11 modules), system/main.rs (5 modules)
- [x] CI/CD: 7 jobs, 15 checks
- [x] All mirrors synced (GitHub, GitLab, Codeberg)
- [x] Guard never-kills invariant enforced

## Remaining 🔧
- [ ] Monitor daemon stability with all fixes over 24h
- [ ] Fix nvidia/openrouter AI providers returning "parse response" errors
- [ ] Guard/storage extraction from system/main.rs (deeply coupled, low priority)
- [ ] Switch GitLab/Codeberg to HTTPS+PAT for consistency
- [ ] Warden security lib extraction (3,500 lines, 75 pub fn)
- [ ] Improve journald reliability (tracing-journald crate?)
- [ ] Handle untracked-only repos in daemon detection
- [ ] Test that clone-into-Dev always works (automated test)
