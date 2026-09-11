# Release Notes — v0.112.33 (2026-07-21) — MEDIUM sweep

**Headline**: The audit MEDIUM sweep (`AUDIT_FULL_2026-07-21.md`) —
26 fixes across 4 batches: daemon state machines (M2-M9), git layer
(M11-M19), policy/visibility/report (M20-M28), and system/tests
(M33-M34, F4.9-F4.12, M10, H2-follow-ups). **818 daemon tests**
(+21 over v0.112.31), warden 83 (+2), dracon-system 86, security
crate ~111 — all green, clippy + deny clean.

---

## Batch A — daemon state machines (M2-M9)

- **M2 (F1.9) forge-confirmed terminal state**: the auto-create block
  charged every healthy repo a `git remote` subprocess every cycle +
  `ls-remote` per remote every 300s FOREVER. New `forge_confirmed`
  set: once every auto-create remote answers `remote_repo_exists ==
  true` (or nothing to create), the repo leaves the block for the
  rest of the daemon session.
- **M3 (F1.10) bootstrap cooldown + staged hygiene**: `Ok(false)`
  (nothing to stage) re-ran the full bootstrap every 1s cycle — now
  60s cooldown. The root commit now sweeps operator-staged oversized/
  excluded content via `git rm --cached` (the shared hygiene helpers
  use `git reset HEAD`, which fails on unborn branches).
- **M4 (F1.6) MAX_FAILURES no longer abandons repos forever**: the
  gate is now a 15-minute backoff with re-probe + notification, and
  `Blocked` (merge/rebase/cherry-pick in progress — needs-human) no
  longer counts toward the budget. A repo left mid-merge overnight
  stays watched.
- **M5 (F1.11) origin failure no longer starves mirrors**: an origin
  push failure is recorded in `remote_failures` and the mirror block
  still runs; the aggregate is returned at the end.
- **M6 (F1.12) dynamic `count_ahead_commits`**: was hardcoded
  `origin/main..HEAD` (push-timeout scaling never engaged for
  mirror-only or non-main repos — exactly the repos it was built
  for). Now resolves origin/branch → mirror refs → local total
  (never-pushed). One pre-existing test updated to the correct
  semantics + 2 new tests.
- **M7 (F1.13) SIGHUP full soft-reset**: clears all cooldown maps
  (including the v0.112.30/31 additions) — the stuck ledger needs no
  explicit reload (the H5 per-cycle reload covers it).
- **M8 (F1.14) detached-task registry**: the trailing-drain timeout
  previously force-cleared `in_flight` while tasks kept running →
  duplicate concurrent `sync_repo` (index.lock contention, phantom
  ledger failures). Unfinished tasks now move to a persistent
  `detached_syncs` registry (drained through the same apply path via
  `tokio::select!`), repos STAY in-flight (no-redispatch invariant
  holds), late results apply on completion, and a 15-minute
  wedged-task valve preserves the no-permanent-skip invariant with
  stale-result discard.
- **M9 (F1.8) `stage_cooldowns` is real**: new `SyncOutcome::FilterOnly`
  for the filter-only-dirty path; the apply phase inserts a 300s
  cooldown (the map had zero insert sites; filter-noisy repos were
  re-dispatched every few seconds forever).

## Batch B — git layer (M11-M19)

- **M11 (F2.1)**: `repair_broken_tracking` now repairs the CHECKED-OUT
  branch (first-token `*` bug — verified live). Regression test.
- **M12 (F2.2)**: `rewrite_ahead_paths` filter-branch fallback argv
  rebuilt (paths inside the single shell-quoted `--index-filter`
  string + explicit `--all` rev range; the old argv could never
  succeed). Pure argv-shape tests.
- **M13 (F2.4)**: `.status()` exit codes now checked at 7 sites via
  new `std_git_checked` helper — critically, `consolidate_to_main`
  no longer proceeds to `branch -D master` + remote-delete on a
  failed `git checkout main`.
- **M14 (F2.5)**: `remote_repo_exists` is now tri-state
  (`Exists`/`Missing`/`Unknown` — transport/auth failures are NOT
  "missing") with a session-lifetime cache; no more spurious creates
  during an outage, no per-push SSH for confirmed repos.
- **M15 (F2.6)**: "Repository not found" / "Push to create is not
  enabled" / publickey failures classified as PERMANENT push
  rejections (fail fast into the H5 budget instead of burning the
  retry budget every cycle forever).
- **M16 (F2.7)**: `IndexLock` resolves the REAL gitdir (path_gitdir)
  — submodules/worktrees no longer ENOTDIR; `ensure_standard_files`
  was silently skipped every cycle for every nested submodule.
- **M17 (F2.8)**: `cli_diff_entries`/`git_name_status_entries` use
  `-z` (raw NUL-delimited paths — non-ASCII filenames no longer
  dropped) and propagate non-zero exits as Err (with the unborn-repo
  fallback preserved in `repo_diff_entries`).
- **M18 (F2.9)**: `remove_stale_remotes` is scoped to daemon-managed
  remotes (`dracon.managed-<name>` config marker stamped by
  `ensure_remote`, or names in the current policy list) — operator-
  added remotes (`backup`, forks) survive.
- **M19 (F2.3)**: `is_safe_git_path` rejects `..` at ANY depth (was
  first-two-components only).

## Batch C — policy/visibility/report (M20-M28)

- **M20 (F3.2)**: TOML ordering check now warns when a bare key
  inside `[[remotes]]`/`[[standard_files]]` is not a known table
  field — catches top-level fields silently absorbed into the last
  table entry. **Live-verified on the operator's own config**
  (`standard_files_auto` warning now surfaces).
- **M21 (F3.3)**: `config validate` prints warnings unconditionally
  (was success-path-only).
- **M22 (F3.4)**: `expand_tilde("~/x")` → `$HOME/x` (was `/x` via
  `Path::join` absolute-replacement semantics).
- **M23 (F3.5)**: visibility cache only written when the GITHUB leg
  succeeded; on failure the OBSERVED state is cached (or skipped
  when unknown).
- **M24 (F3.6)**: `make-public`/`make-private` collects ALL basename
  matches and bails on ambiguity with full paths; prints the
  resolved path before flipping.
- **M25 (F3.7)**: `refresh-visibility` uses the new
  `get_github_visibility_opt` (error-aware) — gh failures count as
  errors (real counter) and skip the cache write instead of
  poisoning every repo to private.
- **M26 (F3.8)**: `parse_github_owner_repo` is host-verified
  (ssh://, userinfo, port forms handled; gitlab/codeberg origins
  return None instead of wrong-forge pairs).
- **M27 (F3.10)**: `.env`-file secrets get the same F52 control-char
  refusal as env vars (curl header injection surface).
- **M28 (F3.12)**: exclude-pattern matcher rewritten — single-`/`
  relative patterns (`reports/kdp-live-*.md`) no longer silently
  dead, and segment-exact matching replaces the raw-substring arm
  (`**/scratch/**` no longer excludes `unscratched/`,
  `**/tmp/**` no longer excludes `tmpl/`, `**/~/**` no longer
  excludes any path containing `~`).

## Batch D — system/tests (M33-M34, F4.9-F4.12, M10, H2)

- **M33 (F4.7)**: guard's active-build protection walks ALL Cargo.toml
  ancestors + ancestor-aware matching (workspace-member case:
  `ws/target` protected when cargo runs from a member) + a 60s mtime
  backstop (catches `--manifest-path` builds from unrelated CWDs).
  Both cleanup paths fixed.
- **M34 (F4.8)**: ps-output samples are verified out-of-band via
  `/proc/<pid>/comm` (argv newline injection can't fabricate heavy
  processes), and renice requires comm + starttime match (PID-reuse
  window closed). Live-verified against real processes.
- **F4.10**: `is_git_tracked_dir` queries the repo-RELATIVE path
  (was basename-at-root — nested tracked dirs were deletable without
  `--allow-tracked`).
- **F4.12**: `load_system_policy` propagates read errors (was:
  silently ran on defaults when the policy file was unreadable).
- **F4.9**: security-subcrate test guards now HOLD the env mutex for
  their lifetime (was: locked in a local binding dropped on return —
  parallel HOME mutation across tests) + restore-overwrite instead
  of remove-then-set.
- **M10 (F0.3)**: pre-commit identity guard — `stage_commit_and_push`
  verifies the effective committer identity (user.email + user.name)
  is in trusted_emails/trusted_authors BEFORE committing. Turns the
  F0.1 post-hoc lockout into a pre-commit guard. 11 test policies
  updated to declare the trusted test identity.
- **H2 follow-up (F0.1)**: warden's pre-push hook now rejects pushes
  containing commits authored by test identities
  (`test@test`, `test@test.com`, `test@example.com`) in the PUSHED
  range (historical commits unaffected). 2 behavioral tests.

---

## Test discipline

- `cargo test --workspace --locked` ✅ all green: dracon-sync 818
  (+21), dracon-warden 83 (+2), dracon-security ~111, dracon-system 86
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean

## Audit remediation status (AUDIT_FULL_2026-07-21.md)

All 10 HIGH remediated (v0.112.31: H1, H3-H7, H10, M1; v0.112.32:
H8, H9). All 34 MEDIUM remediated (v0.112.32: M29-M32; v0.112.33:
M2-M28, M33, M34, F4.9-F4.12, M10). LOW findings and the two
operator-decision items (F1.16 restore semantics, live config
cleanup) remain.
