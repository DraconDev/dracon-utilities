
# Autoresearch Ideas — System Audit (2026-05-21)

## Investigated & Deferred

- **Codeberg push-to-create disabled**: Forgejo on codeberg.org doesn't allow `git push` to create repos. 
  The `dracon-home` repo needs to be created manually on Codeberg, or push-to-create enabled in user settings.
  **Impact**: Low. GitHub and GitLab both working fine.

- **Incident ledger retention**: 2,739 lines of historical incidents from Jan 1, 2026. All push failures are 
  historical (Jan 1) not current. Policy already has `incident_ledger_max_lines = 10000` and 
  `incident_ledger_max_age_days = 30`. Ledger will self-prune over time.

- **Mass deletion guard counter**: `dracon_sync_mass_deletion_guard_blocked_total` is always 0 since the guard 
  was removed. Could be removed from metrics entirely, but keeping for backward compat.

- **Warden plaintext_patterns allowlist too restrictive**: The `is_allowed_plaintext_pattern` validator in 
  `dracon-warden/src/main.rs` only allows ~14 patterns. Cannot add things like `*.toml`, `*.md`, or directory 
  patterns like `utilities/`. This is intentional security (plaintext = escape hatch for encryption). 
  The warden's own logic handles the `.dracon` case correctly via `-filter` directives in `.gitattributes`.

## Not Investigated (Out of Scope)

- GPU/system-level performance for heavy builds
- NixOS rebuild times
- Network latency to GitHub/GitLab/Codeberg
- Memory usage of daemon under high repo load

## Performance Findings (2026-05-21)

- **Metric clarification**: `sync_cycle_ms` (41,408ms) measures the `once` command — a FULL sequential stress test that processes all 24 repos + pushes to all 3 remotes. This is NOT the daemon iteration time.
- **Real daemon iteration**: ~156ms per clean cycle (pulse_interval=1s, inactivity_delay=5s). Daemon is fast.
- **Per-repo overhead**: ~35ms per `git status` call. Negligible.
- **Push overhead**: ~584ms per push (network-bound). Largest cost in dirty-repo cycles.
- **No optimization warranted**: Daemon is well-optimized. The 41s is expected for the sequential `once` command.

## Verified Working (No Action Needed)

- **Warden encryption**: Working correctly. New `secrets/**` files get `[DRACON_SECRET:...]` blobs in git index on `git add`. Smudge filter decrypts on checkout.
- **.dracon discovery fix**: `discover_git_repos()` now correctly finds dot-prefixed watch roots (`.dracon`).
- **GitLab sync**: `.dracon` pushes successfully to `dracondev/dracon-home` on GitLab.
- **All 602 tests passing**: No regressions.
