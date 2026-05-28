# Dracon Utilities Audit — 2026-05-28

## Status: IN PROGRESS

## Recently Completed

### ✅ F1: New Branch Auto-Push (sync.rs)
**Problem:** New local branches with no upstream tracking were never pushed. `ahead == 0` blocked the push, and `auto_pull_merge` didn't detect new branches since there's no tracking ref to compare against.

**Fix:** Two changes in `sync.rs`:
1. `push_with_blob_check` — pushes even when `ahead == 0` if the branch has no upstream tracking
2. `handle_ahead_push` — same logic applied after auto_pull_merge

**Files changed:** `dracon-sync/src/sync.rs`
**Build:** ✅ passes `cargo check`
**Installed:** ✅ daemon restarted with new binary (sha256: 865be256b724663d58d200675a29ea6c41d8006b3de4e568a211b4074f3b7911)
**Tested:** Created `test-auto-push` branch → the sync daemon picked it up and pushed it to all 4 remotes (origin, github, codeberg, gitlab) on the next cycle.

---

## Audit Tasklist

### Category: Core Functionality

- [ ] **F1** ✅ (done above) New branch auto-push

- [ ] **F2** Verify `auto_pull_merge` correctly handles the case where a local branch exists but remote tracking hasn't been set up yet — does it create the tracking ref or just skip?

- [ ] **F3** Investigate the `push_with_retries` code path — when multiple mirrors are configured, does a failure on one mirror prevent subsequent mirrors from being tried? Check `push_to_mirror` and `push_with_retries` interaction.

- [ ] **F4** The `IndexLock` coordination mechanism in `harden_repo` and `ensure_standard_files` — verify that `O_EXCL` atomic create correctly coordinates between warden/sync and git operations. Confirm the lock is released on both success and error paths.

### Category: Safety Guards

- [ ] **G1** Mass deletion guard in `sync.rs` — confirm the three thresholds (85%+, 70%+with5+files, 10+absolute) correctly handle symlinks and gitlinks (submodules). Currently `is_gitlink_unchanged` filters some entries — does this affect the accuracy of `missing_count` vs `total_tracked`?

- [ ] **G2** The `filter_only_cleared` flag from `compute_diff_entries` — verify it's being used correctly. It's currently discarded (`_`). This flag indicates that changes ARE present but were all filtered out by clean/smudge filters. When `filter_only_cleared` is `true` and `status.is_clean` is `false`, this should trigger a cooldown, not a commit attempt.

- [ ] **G3** The `stuck repo` mechanism — check `daemon::is_repo_stuck` and `stuck_list` management. When a push times out, is the repo correctly marked as stuck? When does it become unstuck automatically?

- [ ] **G4** The `repair-concerns` and `repair-warns` commands — audit the repair heuristics. Are they actually fixing the underlying problems or just masking symptoms? Do any repair operations have the potential to make things worse if the repo is in an unexpected state?

### Category: Per-Remote Logic

- [ ] **R1** The `repo_name_map` feature for dot-prefixed repos on GitLab — test `.dracon` → `dracon-home` mapping end-to-end with a real push to GitLab.

- [ ] **R2** `auto_github_private` — verify that GitHub repo creation via `gh repo create` handles the case where the repo already exists (it should reuse, not create `repo-1` suffix). The AGENTS.md explicitly bans the suffix pattern.

- [ ] **R3** `sync_visibility` and `sync_metadata` for cross-platform mirror sync — verify the cache in `~/.local/state/dracon/visibility-sync/` is correctly populated by `get_visibility_and_metadata` and the TTL check works.

- [ ] **R4** Codeberg/Forgejo push-to-create disabled — confirm that attempts to push to a non-existent Codeberg repo produce a clear error rather than silently failing.

### Category: Process Monitoring (dracon-system)

- [ ] **S1** Auto-renice graduated thresholds — verify that when a process crosses `180% / 300% / 500%` CPU or `4GB / 8GB` RSS thresholds, it gets the correct nice value (5/10/15). Also verify that `release_after_secs` of being non-heavy correctly un-renices the process back to nice 0.

- [ ] **S2** `proactive_cleanup_percent` — the guard only removes `target/` dirs older than `rust_target_max_age_days` (14 days default). Verify this doesn't interfere with active builds — a `target/` dir that's actively being written to should have a recent `Cargo.lock` or similar marker.

- [ ] **S3** The guard log rotation at `guard_log_max_mb` — confirm the rotation happens atomically (rename old log, create new one) and doesn't lose events during the rename.

- [ ] **S4** Protected path handling — `/` requires exact match but all other system paths use ancestor matching. Confirm that `/home` correctly protects `/home/dracon/Dev` etc. Also verify that user-protected paths from config are canonicalized before comparison.

- [ ] **S5** Process monitoring sustain time — a process must use >`process_cpu_percent` for >`process_sustain_secs` before being flagged. Are there any race conditions where a burst of heavy CPU could be missed or a short-lived heavy process could be incorrectly flagged?

### Category: Warden / Encryption

- [ ] **W1** The `DRACON_SECRET` marker detection in `scrub-markers` — verify the regex correctly identifies all variants (`DRACON_SECRET`, `DRACON_SECRET_x001_`, etc.) and doesn't produce false positives.

- [ ] **W2** The `clean filter` / `smudge filter` git operations — verify that the filter correctly handles binary files, large files (> `max_stage_file_bytes`), and files that were already encrypted in a previous run (idempotency).

- [ ] **W3** `resmudge` command — this fixes ciphertext stuck in the working tree. Verify it correctly identifies when a file has ciphertext that should be plaintext and re-runs the smudge filter on those files.

- [ ] **W4** The `IndexLock` usage in `harden_repo` — multiple writes happen (`apply_overwrite_file`, `publish_repo_pubkey`) before the lock is released. Confirm that if a write fails partway through, the lock is still released (via `Drop` or similar).

### Category: Testing

- [ ] **T1** Serial test reliability — the AGENTS.md notes ~10-20 tests fail unpredictably with default parallelism due to `PATH` mutations and shared global state (TCP listeners, locks). Confirm the recommended `--test-threads=1` approach is documented and the failures are understood.

- [ ] **T2** Add test coverage for new branch auto-push (`F1`) — a test that creates a branch with no upstream and verifies that `push_with_blob_check` would attempt the push.

- [ ] **T3** Add test for `filter_only_cleared` cooldown scenario (`G2`).

### Category: Operational State

- [ ] **O1** Incident ledger retention enforcement at startup — verify it runs before any other startup work. If the ledger grows very large, does the startup prune cause a noticeable delay?

- [ ] **O2** `visibility-sync/` cache cleanup — orphan `.last` files for deleted repos are removed at startup. Confirm this doesn't interfere with cache reads that might happen concurrently during the daemon loop.

- [ ] **O3** The `stuck_push_repos.json` file — when a repo is marked stuck, are all remotes documented or just the one that failed? If the push times out on `gitlab` but succeeds on `origin`, does the repo still get marked as stuck?

- [ ] **O4** The `IndexLock` stale lock cleanup at startup — `stale index.lock` removal runs every ~5 minutes during the daemon loop. Confirm at startup it runs before any git operations.

### Category: Configuration / Policy

- [ ] **P1** TOML field ordering — AGENTS.md explicitly warns that `standard_files` must appear BEFORE any section headers. Add a note to `dracon-sync.example.toml` warning about this or add policy validation for field ordering.

- [ ] **P2** `validate-config` command — does it catch all configuration errors and warnings including field ordering issues, invalid paths, missing required sections?

- [ ] **P3** The default values for `proactive_cleanup_percent`, `rust_target_max_age_days`, etc. — confirm they're sensible defaults and docs in AGENTS.md match the actual defaults in code.

### Category: Secret Management

- [ ] **K1** Token resolution — `load_secret("NAME")` checks env var first, then scans `*.env` files. Confirm that env vars set during tests (via `EnvRestorer`) don't leak between tests.

- [ ] **K2** The `GH_TOKEN` env var — verify it's used as a fallback when `gh auth` isn't configured. Does it take precedence over `gh auth` credentials or the other way around?

### Category: Release Pipeline

- [ ] **L1** `auto_tag` — verify it creates annotated tags (not lightweight) since releases need the extra metadata.

- [ ] **L2** `auto_release` — dry-run publish runs before real publish. Does a failed dry-run prevent the release from being created?

- [ ] **L3** The Nix flake auto-update PR — confirm it correctly updates the `version` field in `flake.nix` and opens a PR against the correct branch.

- [ ] **L4** Publish targets with registry pre-check — "registry pre-check skips already-published versions." Verify this check is fast and doesn't require a network call for every sync cycle.

---

## Quick Wins (Low Effort, High Value)

- [ ] **Q1** Add `filter_only_cleared` handling to `sync.rs` — when `true`, skip staging and apply cooldown instead of treating it as needing a commit.
- [ ] **Q2** Document `DRACON_SYNC_GIT_BIN` env var override in sync's `--help` output.
- [ ] **Q3** Add `sha256sum` of the installed binary to `install.sh` output for verification.

## Next Steps

1. **Immediate:** Address `G2` (filter_only_cleared cooldown) as it directly affects the new auto-push feature's correctness.
2. **Next sprint:** `T2` (test for new branch push) and `T3` (test for filter_only_cleared cooldown).
3. **Follow-up:** `F2`, `F3` to understand push retry mechanics and mirror failure handling.
