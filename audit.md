# Dracon Utilities Audit — 2026-05-28

## Status: IN PROGRESS

---

## Recently Completed

- [x] **F1**: New branch auto-push — `push_with_blob_check` and `handle_ahead_push` now push branches with no upstream tracking
- [x] **G2**: filter_only_cleared — now returns `NothingToDo` immediately instead of falling through to staging logic
- [x] **Audit itself**: All 40 items reviewed, detailed findings documented below

---

## Core Functionality

- [ ] **F2**: Verify `auto_pull_merge` correctly handles new branches with no upstream
  - **Finding**: `auto_pull_merge` skips new branches by design (requires `has_upstream=true`, which new branches don't have). `handle_ahead_push` handles the push instead. No action needed.

- [ ] **F3**: Investigate `push_with_retries` and mirror failure handling
  - **Finding**: Mirrors are tried independently via `push_to_all_remotes`. Each failure increments `mirror_consecutive_fails`. Origin failures tracked separately via `failure_count`. No action needed.

- [ ] **F4**: Verify `IndexLock` coordination in `harden_repo` and `ensure_standard_files`
  - **Finding**: `O_EXCL` atomic create, proper `Drop` release. No issues.

---

## Safety Guards

- [ ] **G1**: Mass deletion guard — confirm three thresholds handle symlinks/gitlinks correctly
  - ⚠️ **Critical finding**: Mass deletion guard is REMOVED from production code (confirmed in `main.rs:847` metric stub + daemon comment). AGENTS.md is stale. **Update AGENTS.md** to remove the mass deletion guard section.

- [ ] **G3**: Audit `stuck repo` mechanism — `daemon::is_repo_stuck`, stuck/unstuck lifecycle
  - **Finding**: Repo becomes stuck on diverged+3fails OR clean+ahead+3originfails. Retried every 5 min. Minor: up to ~30s delay after auto-retry before repo re-enters activity loop. **No action needed.**

- [ ] **G4**: Audit `repair-concerns` and `repair-warns` repair heuristics
  - **Finding**: Repair runs inside sync timeout. Unresolvable issues trigger cooldown. Concern: slow repairs could cause sync timeouts. **Verify no repair op exceeds sync timeout.**

---

## Per-Remote Logic

- [ ] **R1**: Test `repo_name_map` for `.dracon` → `dracon-home` on GitLab end-to-end
  - **Not verified**. Code validates map entries but mapping during push-to-create not tested. **Manual test needed.**

- [ ] **R2**: Verify `auto_github_private` reuses existing repos instead of creating suffixed names
  - ⚠️ **Potential issue**: Code calls `gh repo create` without checking existence first. Falls back to origin URL on failure. **Test with existing repo name.**

- [ ] **R3**: Verify `visibility-sync/` cache TTL and population logic
  - **Finding**: Per-repo `.last` files with 24h TTL. Startup prune removes orphans. Works correctly. **No action needed.**

- [ ] **R4**: Confirm Codeberg push-to-create produces clear error
  - **Finding**: Correctly documented and implemented. Git transport error on non-existent repo is clear. **No action needed.**

---

## Process Monitoring (dracon-system)

- [ ] **S1**: Verify graduated renice thresholds (180%/300%/500% CPU, 4GB/8GB RSS) → nice 5/10/15
  - **Finding**: `graduated_nice_value` correctly implemented and well-tested. **No issues.**

- [ ] **S2**: Verify `proactive_cleanup_percent` protects active builds via PID exclusion + age heuristics
  - **Finding**: `detect_active_rust_builds` excludes running cargo/rustc from cleanup. 14-day age heuristic for target dirs is conservative. **No issues.**

- [ ] **S3**: Verify guard log rotation at `guard_log_max_mb`
  - ⚠️ **Not found**: Rotation logic not identified in `main.rs`. **Find implementation or add rotation.**

- [ ] **S4**: Verify protected path canonicalization before ancestor matching
  - **Not verified**: Canonicalization call site not found in `main.rs` grep. **Check `check_safe_to_delete_guard` or equivalent.**

- [ ] **S5**: Audit process monitoring sustain time race condition
  - **Finding**: Gap < sampling_interval (30s) won't reset `heavy_since`. Design choice, low severity. **No action needed.**

---

## Warden / Encryption

- [ ] **W1**: Verify `DRACON_SECRET` regex catches all variants including `_x001_`
  - **Finding**: User-facing format is `[DRACON_SECRET:keyname]` (brackets required). Tests comprehensive. AGENTS.md `_x001_` variants are internal. **No action needed.**

- [ ] **W2**: Verify clean/smudge filter handles binary, large files, already-encrypted idempotency
  - **Finding**: `smart_smudge` no-ops on plaintext, `smart_clean` re-encrypts safely. **No issues.**

- [ ] **W3**: Verify `resmudge` correctly identifies and fixes ciphertext stuck in working tree
  - **Finding**: `resmudge_repo` calls `smart_smudge`, correctly replaces markers. **No issues.**

- [ ] **W4**: Verify `IndexLock` in `harden_repo` released on both success and error paths
  - **Finding**: Unconditional `remove_file` in `Drop`. Multiple writes coordinated. **No critical issues.**

---

## Testing

- [ ] **T1**: Verify serial test reliability — confirm `--test-threads=1` workaround documented
  - **Finding**: Issue understood, workaround documented in AGENTS.md. **No action needed.**

- [ ] **T2**: Add test for new branch auto-push (F1 coverage)
  - **Not implemented**. **Add test** creating branch with no upstream, verify `push_with_blob_check` attempts push.

- [ ] **T3**: Add test for `filter_only_cleared` cooldown (G2 coverage)
  - **Not implemented**. **Add test** with clean/smudge filter producing no-diff modified entries, verify `NothingToDo` returned.

---

## Operational State

- [ ] **O1**: Verify incident ledger retention runs at startup before daemon loop
  - ⚠️ **Minor risk**: `read_to_string` loads entire ledger to memory. No size guard. Large corrupted file could OOM. **Add size guard** (e.g., skip/hint if > 100MB).

- [ ] **O2**: Verify `visibility-sync/` cache cleanup doesn't interfere with concurrent reads
  - **Finding**: Per-repo `.last` files. Orphan removal only. Latent race on same-file read/write not observed but possible. **Low severity.**

- [ ] **O3**: Verify `stuck_push_repos.json` is repo-level, not per-remote
  - **Finding**: Correctly repo-level. Origin failure drives stuck marking; mirror failures drive `mirror_consecutive_fails` notifications. **No action needed.**

- [ ] **O4**: Verify `IndexLock` stale lock cleanup runs at startup + periodic (every ~5 min)
  - **Finding**: Both startup cleanup and periodic `cycle_count.is_multiple_of(300)` call `repair_broken_tracking`. **No issues.**

---

## Configuration / Policy

- [ ] **P1**: Add TOML field ordering check to `validate-config`
  - ⚠️ **Silent mis-parse risk**: `standard_files` after a section header is silently ignored. No validation catches this. **Add validation** or update example config with warning.

- [ ] **P2**: Verify `validate-config` catches field ordering issues
  - **Finding**: Only validates values and required fields, not TOML structure. **P1 above addresses this.**

- [ ] **P3**: Verify all AGENTS.md defaults match code defaults
  - **Finding**: All checked defaults match (50% proactive, 14-day rust target, 60s repair cooldown, 1MB guard log, 120s release). **No discrepancies.**

---

## Secret Management

- [ ] **K1**: Verify `load_secret` env precedence and `EnvRestorer` test isolation
  - **Finding**: Env var checked first, then `.env` files. `EnvRestorer` correctly restores on drop. **No issues.**

- [ ] **K2**: Verify `GH_TOKEN` env var takes precedence over `gh auth`
  - **Finding**: Correctly checked first in `load_secret`. **No issues.**

---

## Release Pipeline

- [ ] **L1**: Verify `auto_tag` creates annotated tags (not lightweight)
  - **Finding**: Uses `git tag -a -m` (annotated). **No issues.**

- [ ] **L2**: Verify `auto_release` dry-run logic before real publish
  - **Finding**: GitHub Releases use `gh release create` directly — no dry-run built into gh itself. Publish to crates.io/npm likely has dry-run. **Acceptable limitation.**

- [ ] **L3**: Test Nix flake auto-update PR creation (integration test)
  - **Not verified**. **Integration test needed** with live GitHub token.

- [ ] **L4**: Verify registry pre-check is fast (not a network call per cycle)
  - ⚠️ **Found issue**: `version_exists_on_registry` makes a network call per publish target per bump cycle. No local caching. Could slow down release pipeline. **Add cache or accept limitation.**

---

## Quick Wins

- [ ] **Q1**: Add `filter_only_cleared` handling to `sync_repo` — DONE in G2 above

- [ ] **Q2**: Document `DRACON_SYNC_GIT_BIN` env var in sync `--help` output
  - **Not done**. Add to clap argument docs. **Low effort.**

- [ ] **Q3**: Add `sha256sum` of installed binary to `install.sh` output
  - **Not done**. Add `sha256sum ~/.local/bin/dracon-sync` to install output. **Low effort.**

- [ ] **Q4**: Add TOML field ordering warning to `dracon-sync.example.toml`
  - **Not done**. Add comment: `# NOTE: standard_files must appear before any section headers`. **Low effort.**

- [ ] **Q5**: Add size guard to `enforce_retention_at_startup` (O1 above)
  - **Not done**. Check file size > 100MB before calling `read_to_string`. **Low effort.**

---

## Priority Order

1. **[ ] G1**: Update AGENTS.md — mass deletion guard description is stale
2. **[ ] P1**: Add TOML field ordering validation to `validate-config`
3. **[ ] R2**: Manual test `auto_github_private` with existing repo name
4. **[ ] T2**: Add test for new branch auto-push
5. **[ ] T3**: Add test for `filter_only_cleared` cooldown path
6. **[ ] S3**: Find guard log rotation implementation
7. **[ ] Q3**: Add `sha256sum` to `install.sh`
8. **[ ] Q2**: Document `DRACON_SYNC_GIT_BIN` in `--help`
9. **[ ] S4**: Verify canonicalization in protected path check
10. **[ ] O1**: Add size guard to incident ledger startup prune
