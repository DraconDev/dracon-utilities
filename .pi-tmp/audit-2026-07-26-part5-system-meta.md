# Audit 2026-07-26 — Part 5: dracon-system (PART A) + meta-repo consistency (PART B)

Scope: read-only. dracon-system v0.112.33, dracon-sync v0.113.1, dracon-warden v0.113.0.

---

# PART A — dracon-system findings

## [HIGH] Guard daemon busy-loops after the first interval — `src/main.rs:3154,3230-3233`
`cmd_guard_daemon` declares `let mut elapsed = 0u64;` once before the outer
`while !shutdown` loop and never resets it. The inner sleep loop
(`while !shutdown && elapsed < interval { sleep(1s); elapsed += 1; }`) runs to
`elapsed == interval` exactly once; on every subsequent pass the condition is
false, so the daemon runs `run_guard_once` back-to-back with zero delay —
each pass spawns `df`, `ps`, `du`, walkdir scans, and (at action/critical
disk) full cleanup scans. Mechanism: missing `elapsed = 0` reset per outer
iteration. Fix: reset `elapsed` at the top of each outer loop pass (or use
`tokio::time::interval`). Not covered by any test (no daemon-loop timing test).

## [HIGH] `link apply` can never fix (or even re-affirm) an existing symlink — `src/links.rs:135,138`
`apply_link_policy` routes existing symlinks through
`check_safe_to_delete(&link, &[])`, which *always* bails on symlinks
("refusing to delete symlink", `src/safety.rs:31-38`). Consequences:
(a) a drifted symlink (`link_target_mismatch`) — the primary case `apply`
exists to fix — makes the whole command error out via `?` on the first entry;
(b) there is no `in_sync` short-circuit, so even a healthy symlink is
removed-and-recreated, meaning `apply` fails on fully-synced policies too.
Fix: for `file_type().is_symlink()`, `fs::remove_file(&link)` directly
(deleting a symlink never touches its target; the safety check exists to
guard `remove_dir_all` recursion), and skip entries already `in_sync`.
Not covered by tests: `links_tests.rs` only exercises structs/`evaluate_link`;
`apply_link_policy` has zero coverage.

## [MEDIUM] `guard clean --all` silently ignored; dead validation branch — `src/main.rs:3548,3357-3359`
The `--all` flag is bound to `all: _` and discarded. `do_all` is computed as
`targets.is_empty()`, so `--all --rust` cleans only rust (flag silently
ignored). Worse, `if !do_all && !targets.any()` is unreachable
(`is_empty() == !any()` by construction), so the "No cleanup targets
specified" warning can never fire. Fix: thread `all` into `CleanTargets`
(`do_all = all || targets.is_empty()`) and drop the dead branch.

## [MEDIUM] Log truncation loses recent lines (TOCTOU + rename) — `src/main.rs:1570-1650`
`truncate_log_file` with `preserve_header_lines > 0` reads the file twice,
writes a temp copy, then `rename`s it over the original. Lines appended
between the second read and the rename are silently discarded; writers
holding the old fd keep writing to an unlinked inode (data lost, space not
reclaimed until they exit). Temp path is `with_extension(...pid)` without
`O_EXCL`. Given `auto_truncate_logs` requires the explicit
`auto_cleanup_apply = true` opt-in, impact is bounded but real. Fix: open
the original with `O_TRUNC`-style in-place rewrite (copy header+prefix to a
temp, then `copy` back into the same fd and `set_len`), or
`fallocate --punch-hole`; create temp with `OpenOptions::new().create_new(true)`.

## [MEDIUM] `storage --cleanup --apply` aborts entire cleanup on first failure — `src/main.rs:2912-2919`
In `cmd_storage`, the apply loop uses `check_safe_to_delete(&path, ...)?` and
`tokio::fs::remove_dir_all(...).await?` — one protected path or one IO error
terminates the run mid-list with an error exit, after earlier deletions have
already happened (partial cleanup, no summary). The guard cleanup paths
(`auto_cleanup_rust_targets`, `try_remove_cache_dir`) correctly
log-and-continue. Fix: match the guard pattern (eprintln + continue,
collect failures into the report).

## [LOW] Nested `node_modules` double-counted; reclaim stats inflated — `src/main.rs:1453-1530`
`clean_old_node_modules` WalkDir does not prune a matched `node_modules`
subtree, so `a/node_modules/b/node_modules` is matched twice: `reclaimed`
sums both (inner is inside outer's `du`), and after the outer is deleted the
inner removal fails with a printed error. Fix: `filter_entry` to skip
descending into matched dirs, or track and subtract.

## [LOW] Nix cleanup reports fabricated byte counts; generation-delete errors conditionally swallowed — `src/main.rs:1393-1443`
`clean_nix_garbage` sets `reclaimed = delete_count * 1 MiB` (pure fiction;
also counts "deleting" lines from `--dry-run`), and returns the collected
`nix-env --delete-generations` errors only when `reclaimed == 0` — if gc
succeeds, failed generation deletion is silently dropped (swallowed error).
Fix: parse "freed" from nix output or report path counts honestly; always
surface `errs` (warn, don't conflate with success).

## [LOW] Inode monitoring hardcodes `/`, ignoring `disk_mount_path` — `src/main.rs:1060-1061,1102`
`inode_use_percent`/`get_inode_info` run `df -Pi /` while the guard's disk
thresholds use `guard.disk_mount_path` (default `/nix` on NixOS). On a
system where `/nix` is a separate mount, inode warnings track the wrong
filesystem. Fix: pass `&guard.disk_mount_path` through.

## [LOW] `acquire_daemon_lock` truncates before locking — `src/main.rs:74-99`
Opens the lock file with `.truncate(true)` *before* `lock_exclusive()`
succeeds, truncating the file a live daemon holds (content is cosmetic, the
flock is on the inode, so impact is minor), and discards the underlying io
error ("lock file is held by another process" hides EACCES etc.). Fix:
open without truncate, lock, then `set_len(0)` (already done post-lock) and
include the source error.

## [LOW] Panic/robustness nits
- `resolve_bin` second lock uses `.unwrap()` (poison panics) while the first
  uses `into_inner()` — inconsistent (`src/main.rs:1348-1355`).
- `shorten_event_time` fallback `ts[..19]` panics on non-ASCII input of len > 19
  (`src/events.rs:264-267`); use `chars().take(19)`.
- `zram` "Compression ratio" prints `compr/orig` as a percent (semantically
  the inverse of a ratio; the `(Yx)` form is correct) (`src/zram.rs:84-91`).

## Destructive-path test coverage assessment (`guard_tests.rs`, `tests.rs`, `links_tests.rs`)
COVERED: `graduated_nice_value` (all tiers/clamps), `disk_state` thresholds,
`df`/`ps` parsing, `should_notify` cooldowns, `predict_fill_time`,
`check_safe_to_delete` + `check_safe_to_delete_guard` (protected roots, user
paths, symlink rejection), `renice_process_with_bin` success/failure, one
live `run_guard_once` smoke test.
NOT COVERED (gaps that would have caught the findings above):
- daemon interval/elapsed loop (HIGH #1),
- `apply_link_policy` / force-replace backup path (HIGH #2),
- `auto_cleanup_rust_targets` / `proactive_cleanup_rust_targets` deletion
  paths — incl. the v0.112.33 workspace-member protection (M33) and the
  60s mtime backstop, neither has a regression test,
- `truncate_log_file`,
- `manage_sync_freeze` marker write/remove,
- `check_heavy_processes` renice → un-renice state machine and the
  starttime PID-reuse reset (M34),
- `clean_old_node_modules` nesting, `clean_nix_garbage` reporting.

---

# PART B — meta-repo stale claims (AGENTS.md / docs/design vs code)

## B1. Stale source line references in AGENTS.md (spot-checked 7 of ~8; ALL 7 stale)
| AGENTS.md claim | Actual (dracon-sync v0.113.1) |
|---|---|
| `report.rs:3705` rewrite_ahead_paths call (2 occurrences) | `src/report.rs:5525` |
| `report_v2_snapshot.rs:3166` call site | **file no longer exists** (removed/merged; grep: 0 matches repo-wide) |
| `git/staging.rs:148-244` / `:152` fn definition | `src/git/staging.rs:182` |
| `policy.rs:1580` `auto_repair_concerns: true` default | `src/policy.rs:1864` |
| `sync.rs:858,859` `git add -A --` / `-f --` | `src/sync.rs:1042, 1060` |
| `git/mod.rs:684+` `force_push_when_behind = false` | `src/git/mod.rs:1004` (RemoteConfig default literal) |
| `git/mod.rs:370+` `test_load_secret_or_legacy_pat_*` | `src/git/mod.rs:601` |
(The only un-located-but-still-true ref: `default_push_op_timeout_secs` in
policy.rs exists at :1080 — name-level claim OK.)

## B2. Push-timeout section is stale (AGENTS.md "Push timeouts", ~line 100)
- AGENTS.md: "`push_op_timeout_secs = 300` … matches the daemon's own code
  default". Reality: code default is still 300 (`policy.rs:1080`), but the
  **live global config has `push_op_timeout_secs = 900`** since 2026-06-23
  (`~/.dracon/utilities/sync/dracon-sync.toml:158`, with a full changelog
  comment), and `repo_sync_timeout_secs = 960` (code default is 420,
  `policy.rs:1091`). AGENTS.md's "300s" claim is 5 weeks out of date.
- **Timeout scaling is real and undocumented**: `scale_push_timeout`
  (`src/sync.rs:149-176`, call site :1619) multiplies base ×1/×2/×4/×6 by
  commits-ahead, **capped at 600s** — NOT 900s. AGENTS.md never mentions it.
- **Bonus daemon bug surfaced by this**: the 600s cap is absolute, so with
  the live base of 900s the scaler *reduces* every push to 600s
  (`min(900×k, 600)`), silently weakening the operator's configured 900s
  (and `repo_sync_timeout_secs=960` headroom). The cap should be
  `max(base, …)`-aware. Tests (`sync.rs:7398+`) only cover base=60.
- `dracon-sync.example.toml:78` still ships `push_op_timeout_secs = 60`
  with comments referencing 60s — now 3-way inconsistent with code default
  (300) and live (900).

## B3. `[patch.crates-io]` status section stale (AGENTS.md last section)
AGENTS.md: "points at …?tag=v94.7.1 … when the operator publishes
v94.7.1 to crates.io, remove". Actual `Cargo.toml:33`:
`tag = "v94.7.2"` (updated 2026-07-25, with a comment explaining the v94.7.2
git2-0.21 transport fix). The design doc
`docs/design/incident-amend-race-and-trust-2026-07-25.md` follow-up
correctly says v94.7.2 — only AGENTS.md lags.
Also stale one line up: AGENTS.md says workspace members are the 3 crates;
actual members list has 4 entries (adds `dracon-warden/src/security`,
since v0.112.32).

## B4. History-rewrite ENFORCEMENT stack vs warden code — mostly accurate, one wrong claim
VERIFIED ACCURATE (dracon-warden v0.113.0 `src/main.rs`):
- hooks `pre-push` (secrets scan + non-fast-forward + branch-deletion
  refusal, :2317-2366) and `pre-rebase` (:2437-2451) exist;
- `DRACON_ALLOW_REWRITE=1` escape hatch present in both (:2356, :2441);
- install = `setup-hooks` writing to `~/.config/git/hooks` + global
  `core.hooksPath` (:2460-2515); stale `.pre-dracon` chaining artifacts
  from the dracon-sync experiment are cleaned up (:2477-2483) — consistent
  with the "moved from dracon-sync to warden" story.
INACCURATE: "warden-owned via core.hooksPath **+ init.templateDir**"
(AGENTS.md enforcement stack item 1; repeated in the incident design doc).
No warden code sets or manages `init.templateDir` (grep: 0 matches in
`src/`). The live global git config does have
`init.templateDir=/home/dracon/.git-templates` (with hook copies), but its
installer is untracked/manual — warden's `setup-hooks` would not recreate
it, and nothing keeps it in sync. Doc should say "core.hooksPath only" (or
warden should own the templateDir too).

## B5. `auto_gc_garbage_threshold_bytes` — documentation ACCURATE
Code default = 2 GiB, 0 disables (`policy.rs:1084-1088`,
`git/mod.rs:3408-3411` early-return on 0), invoked from `sync.rs:3684`.
AGENTS.md claim ("default 2 GiB, 0 disables") and example.toml comment
match the code. No action.

## B6. Design docs whose design was REVERSED without an in-doc note
- `docs/design/daemon-standalone-removal-2026-07-01.md` — ends with "the
  daemon will create the standalone worktree directly on `main` … This is
  the new invariant." REVERSED the next day by the 2026-07-02 nested-on-main
  migration (standalones eliminated; materialization code removed
  2026-07-08, goal `730eaf2a`). The doc carries no SUPERSEDED banner;
  AGENTS.md does document the reversal, but the doc itself actively
  misinforms.
- `docs/design/push-timeout-fix-2026-06-17.md` — presents 300s as the final
  fix; superseded by 900s on 2026-06-23 (per live-config changelog and
  `gitlab-storage-and-divergence-2026-06-23.md`) and by the
  `scale_push_timeout` code (600s cap). No UPDATE note in the doc;
  AGENTS.md still cites it as the current rationale.
- (Checked: the dracon-sync per-repo hooks design was reversed *inside*
  `incident-amend-race-and-trust-2026-07-25.md` itself, :246-249 — that one
  is properly documented; no orphan hooks doc found.)

## B7. Live global config vs `dracon-sync.example.toml` (keys only; no values printed)
In live config but NOT mentioned anywhere in the example (even as comments):
- `alert_unpushed_threshold`
- `auto_github_private`
- `auto_github_private_account`
- `max_stage_batch_files`
- `exclude_repos`
All five exist as real `SyncPolicy` fields in `policy.rs` — the example
(which is the documented config reference) is missing them.
Live keys that ARE documented in the example only as commented prose
(acceptable but easy to miss): `system_repo`, `standard_files`,
`standard_files_auto`, `sync_metadata`, `sync_visibility`,
`sync_visibility_interval_hours`, `trusted_emails`, `trusted_authors`.
Example-only keys (present in example, unset live → code defaults apply;
all still valid policy fields, not stale): `auto_stage_untracked`,
`sem_max_concurrent_sync`, `stage_cooldown_secs`, `stage_op_timeout_secs`.

## B8. Minor AGENTS.md freshness notes
- "Recent audit-driven changes (2026-07-19, v0.112.21)" with test counts
  733/915 is historical-by-label but is the newest section; nothing covers
  v0.112.31–v0.113.1 daemon changes (e.g. excluded-path semantics
  v0.112.34 IS present, but the 900s timeout, timeout scaling, security
  crate workspace membership, and v94.7.2 are not).
- "Debounce window (3-second)" claim not verified in this pass (out of the
  assigned spot-check scope).
