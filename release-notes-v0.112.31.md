# Release Notes — v0.112.31 (2026-07-21)

**Headline**: The audit-driven HIGH batch (`AUDIT_FULL_2026-07-21.md`)
— 8 fixes closing the daemon's failure-visibility and
policy-enforcement holes found after the v0.112.28–30 cluster. **797
daemon tests** (+14 over v0.112.30), clippy + deny clean.

---

## Fixes (audit finding IDs in parentheses)

### 1. Push failure is no longer reported as `🔁 synced` (H3/F1.3)

New `SyncOutcome::PushFailed`. Both push paths
(`stage_commit_and_push`, `handle_ahead_push`) previously swallowed
push failures after writing the disk ledger, returning a success
outcome — the apply phase logged `🔁 synced`, reset `failure_count`,
and dropped the activity entry. Now the apply phase counts the
failure (no synced log, `failure_count` increments, activity retained
for backstop/stuck logic). A mirror-leg failure (origin succeeded)
also returns `PushFailed` — the sync isn't fully healthy until ALL
forges are current. Two pre-existing tests that asserted the old
false-healthy outcomes were updated to the new semantics.

### 2. Throttled notifications actually expire (H4/F1.1)

Every notification throttle used `Entry::Vacant` + stored a deadline
that **nothing ever read** — each notification (ownership-skip,
stuck-retry, push-failure, Stuck-Ahead/Behind, Mirror-Degraded) fired
exactly ONCE per daemon lifetime. New `notify_throttled` helper
(get-and-compare + re-arm) at all 7 call sites; the map is cleared on
SIGHUP. Regression test: fires → suppresses → re-fires after deadline.

### 3. Stuck-push ledger unified; `push_max_retries` enforced (H5/F1.2)

Three interlocking defects: (a) the loop's map was loaded once at
startup while runtime failures went to a disk ledger the loop never
re-read — now reloaded every cycle; (b) the retry path deleted the
entry before dispatching, resetting `consecutive_failures` to 1 every
5 minutes — the entry now persists with a new `last_retry_at` stamp
throttling attempts; (c) `push_max_retries` was report-display-only —
the new `StuckDecision::Exhausted` arm stops auto-push when the
budget is spent and tells the operator exactly how to resume
(`dracon-sync unstuck` / `repair-concerns --apply`). Startup logs a
summary of repos entering the daemon already stuck. Pure
`stuck_decision` function + 5 new tests.

### 4. Directory-expansion can no longer bypass the 100 MiB limit (H6/F1.5)

`stage_existing_files` recursed into untracked directories and staged
every file inside wholesale — size limit and all per-file exclude
patterns only applied to top-level entries. A 500 MiB file nested in
a new dir sailed into history, violating AGENTS.md's documented hard
exclusion. New `stage_existing_files_filtered` re-applies the per-file
policy (`max_stage_file_bytes`, `exclude_file_patterns`,
`untracked_exclude_patterns`, effective `auto_commit_exclude_patterns`)
to every expanded path; all 3 production call sites use it (the old
signature survives as a `#[cfg(test)]` wrapper). 2 regression tests
(oversized-nested, pattern-nested).

### 5. Local-first ahead counting; no more per-cycle SSH for broken repos (H7/F1.4)

The v0.112.30 ahead-override still ran `git ls-remote` (SSH) per
remote on every 1s cycle for every repo whose upstream tracking ref
was missing — permanently, for every repo with a broken push. But a
missing tracking ref already implies every commit is unpushed, so the
purely-local `count_all_head_commits` now answers first (new
`any_mirror_tracking_ref_exists` helper distinguishes "synced vs
mirror" from "never pushed"). The `ls-remote` fallback survives only
for the residual edge, behind a 300s per-repo cooldown.

### 6. Codeberg API URL fixed — the v0.112.29 GitLab bug's twin (H10/F3.1)

`CODEBERG_API_REPOS = ".../repos/{}/{}"` +
`.replace("{}", "{owner}/{repo}")` substituted BOTH slots →
`/api/v1/repos/o/r/o/r` → 404 on every call (the v0.112.29 note
claiming "harmless" was wrong). `make-public --include-codeberg` can
never have worked; daemon codeberg visibility/metadata sync silently
404'd. Single-placeholder template (same shape as the GitLab fix) +
pinning test + live verification against the real codeberg API
(corrected URL → 200, old doubled URL → 404).

### 7. Ownership verdict re-detects on a 10-minute TTL (H1/F0.2)

The cached `RepoActivity.ownership` verdict previously never
refreshed while Unowned (verified live: the daemon skipped its own
source repo for 25 minutes after the config was fixed, until SIGHUP).
New `ownership_needs_redetect`: Unowned/Unknown verdicts re-detect
after 600s; Owned verdicts stay sticky (a transient git error must
not flip a good repo into skip mode). A repo that recovers gets a
`✅ ownership restored` log + ledger alert. The skip log now also
carries the SIGHUP/restart recovery hint (landed with fix 2).

### 8. Mirror failures are tracked and named (M1/F1.7+F3.9)

`mirror_consecutive_fails` was initialized empty and NEVER written —
the "Mirror Degraded" notification was dead code. The apply phase
(both sites) now populates it from `remote_failures`. The stuck-push
ledger's `last_error` now names the failing remotes
(`git push returned non-zero (remotes: bad-mirror)`) instead of the
generic "see daemon log" — the `repos` HINT says WHICH forge is
failing. Helper + integration test (ledger names the remote).

---

## Test discipline

- `cargo test --workspace --locked` ✅ **797 daemon** (+14), 0 failed
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean

## Files changed

- `dracon-sync/src/sync.rs` — PushFailed outcome, push-failure
  propagation, `stage_existing_files_filtered` (+ re-filter),
  `failing_remote_names`, 4 tests updated/added
- `dracon-sync/src/daemon.rs` — `notify_throttled` (+7 sites, SIGHUP
  clear), stuck-push reload/`last_retry_at`/`StuckDecision`,
  ahead-override local-first reorder + `ls_remote_cooldowns`,
  ownership TTL re-detect + recovery alert, `mirror_consecutive_fails`
  wiring, 7 tests
- `dracon-sync/src/git/status.rs` — `any_mirror_tracking_ref_exists`
  (+2 tests)
- `dracon-sync/src/visibility.rs` — codeberg single-placeholder
  template (+ pinning test)
- `dracon-sync/src/main.rs` — sync-now PushFailed arm
