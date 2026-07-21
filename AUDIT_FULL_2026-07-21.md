# AUDIT — dracon-utilities (2026-07-21)

> **Scope**: dracon-sync 0.112.30 (daemon.rs, sync.rs, git/*, policy, ownership,
> visibility, secrets, exclude, report, main), dracon-warden 0.112.12,
> dracon-system 0.112.12.
> **Method**: 4 parallel audit agents briefed on the v0.112.28–30 bug patterns
> (raw-vs-effective accessors, multi-placeholder `str::replace`, unreachable
> code behind guards, uncached network calls in the 1s loop, false-healthy
> states, cache invalidation, swallowed errors) + a live incident investigated
> during kickoff. Every HIGH and the load-bearing MEDIUMs were independently
> spot-verified against the code or live state (see Verification appendix).
> **Detail files**: `.pi-tmp/audit-2026-07-21-part0..4.md`.
>
> **Totals: 63 findings — 10 HIGH, 34 MEDIUM, 19 LOW** (2 marked SUSPECTED;
> cross-auditor duplicates merged: F1.7≡F3.9 counted once).

---

## Executive summary

The v0.112.28–30 fixes were correct as far as they went, but the audit shows
they treated symptoms of four systemic weaknesses:

1. **The daemon's failure-visibility stack is broken end-to-end.** Push
   failure is reported as `🔁 synced` (H3), throttled notifications fire
   exactly ONCE per daemon lifetime because the cooldown deadlines are never
   read (H4), the stuck-push ledger is split-brain between a load-once memory
   map and a runtime-written disk file with the retry budget never enforced
   (H5), and the mirror-degraded detector is dead code (M1). A repo can fail
   to push for days with a single log line and one notification.
2. **Policy enforcement has real holes.** The 100 MiB hard size limit and all
   per-file exclude patterns are bypassed when an untracked DIRECTORY is
   expanded for staging (H6). Warden's `harden_repo` overwrites whole
   .gitignore/.gitattributes files — it provably deleted operator rules in
   THIS repo's history (commit `3a67685f`) and will do it again on the next
   manual `dracon-warden once` (H8).
3. **The hot-loop fixes are incomplete.** SSH `git ls-remote` still runs every
   1s cycle for never-pushed repos (H7), healthy repos pay `ls-remote` per
   remote every 5 min forever (M2), and the ownership verdict is cached
   forever — verified live during this audit (H1).
4. **Secret-handling edges leak or corrupt.** Warden smudge corrupts
   whole-file-encrypted BINARY secrets via `String::from_utf8_lossy` (H9),
   and filter-clean passes plaintext through to git for >10 MiB inputs (M3).

Also found live during kickoff: a v0.112.30 test ran `git config` with a
positional path and poisoned the LIVE dracon-sync repo's identity (H2, fixed
same-day in commit `5e32547`).

---

## HIGH findings

### H1 (F0.2) — Ownership verdict cached indefinitely; operator remediation never detected
`daemon.rs:2591-2620`. `RepoActivity.ownership` is detected once; the unowned
`continue` precedes every `activity.remove` site, so the verdict never
refreshes. **Verified live**: after cleaning the poisoned config AND landing a
trusted-author commit, the daemon kept skipping its own source repo for ~25
minutes until SIGHUP. The skip log doesn't mention the cache.
**Fix**: TTL-based re-detection while Unowned (5–15 min); extend the skip log
with the SIGHUP recovery hint; clear note in `ownership --explain`.

### H2 (F0.1) — Test-infrastructure hazard: git-with-positional-path mutates the LIVE repo
`multi_remote.rs` test (fixed in `5e32547`). `git config name value <path>`
sets config in the CWD repo — under `cargo test` that's the real package root.
The daemon committed `35a72d7` as `test <test@test>` and the ownership guard
then locked the repo. **Fixed same-day**; workspace grep found no other
instances. **Follow-up**: pre-push hook or CI check rejecting commits authored
by `test@test*`; a `test_git_cmd` wrapper that forbids positional repo paths
on config/ref-mutating subcommands.

### H3 (F1.3) — Push failure is reported as `🔁 synced`
`sync.rs:3036-3070`, `sync.rs:3661-3684`, `daemon.rs:3086-3151`. On push
failure, `stage_commit_and_push`/`handle_ahead_push` only `eprintln` +
`record_push_failure` (disk-only, see H5) and return success, so the apply
phase logs `🔁 synced`, resets `failure_count`, and drops the activity entry.
False-healthy — the exact class v0.112.30 fixed for ahead-detection. **Verified**
by reading the call chain.
**Fix**: propagate push failure as a distinct `SyncOutcome::PushFailed`
(commit-success + push-failure must count as failure for accounting).

### H4 (F1.1) — All throttled notifications fire exactly ONCE per daemon lifetime
`daemon.rs:2230` + 6 insert sites. Every notification uses
`Entry::Vacant` + stores `Instant::now() + 1800s` — and **nothing ever reads
the deadline**. No expiry check, no retain, SIGHUP doesn't clear the map.
Ownership-skip reminders, stuck-retry alerts, push-failure notifications,
Stuck-Ahead/Behind, Mirror-Degraded: all fire once ever. **Verified** by grep —
zero reads of the stored Instant.
**Fix**: `if map.get(&key).is_none_or(|until| now >= *until) { notify; insert }`;
clear on SIGHUP.

### H5 (F1.2) — Stuck-push ledger split-brain; `push_max_retries` never enforced
`daemon.rs:2231` (load-once map), `1660-1680` (disk-only record), `2659-2691`
(retry resets budget). Runtime failures go to a disk file the loop never
re-reads; the 5-min backoff never fires for post-startup failures; the retry
deletes the entry so `consecutive_failures` resets to 1 each time;
`push_max_retries` is only used by report.rs for display. The entire
stuck-push escalation story is illusory. **Verified** by grep.
**Fix**: shared map (or reload ledger each cycle), `last_retry_at` instead of
delete, enforce `consecutive_failures < push_max_retries` at dispatch.

### H6 (F1.5) — Directory-expansion bypasses the 100 MiB hard limit and all per-file excludes
`sync.rs:665-905` + `exclude.rs:1297-1304`. libgit2 collapses fully-untracked
subtrees into one DIRECTORY entry; `should_stage_entry` approves dirs
wholesale; `stage_existing_files` then recursively stages every file inside
with NO size check, no `exclude_file_patterns`, no
`auto_commit_exclude_patterns`, no `untracked_exclude_patterns`. A 500 MiB
`assets/video.mp4` inside a new `assets/` dir sails through — violating
AGENTS.md's documented hard exclusion. **Verified** by reading both functions.
**Fix**: re-run the per-file predicate (at minimum size + pattern checks) on
each expanded path; regression test with oversized file nested in untracked dir.

### H7 (F1.4) — `git ls-remote` (SSH) every 1s cycle for never-pushed repos — v0.112.29 class still live
`daemon.rs:2766-2797` → `daemon.rs:61-106`. The ahead-override fallback chain
runs `count_unpushed_vs_configured_remotes` (one SSH `ls-remote` per remote)
every cycle with no cooldown, for exactly the repos whose pushes are broken.
Runs before the MAX_FAILURES gate, so even "abandoned" repos keep churning.
**Fix**: `count_all_head_commits` (purely local) BEFORE any `ls-remote` in the
`upstream_ref_missing` arm; 300s cooldown otherwise.

### H8 (F4.1) — Warden `harden_repo` wipes operator .gitignore/.gitattributes content outside the managed block
`dracon-warden/src/main.rs:1103-1108, 660-696, 568-613`.
`build_gitignore_block_with_existing` returns ONLY the managed block;
`apply_overwrite_file` overwrites the whole file with it; the surgical
`replace_managed_block` is `#[cfg(test)]`-only. **Verified live**: commit
`b69d9c2c` (operator's 8-line nested-repo section) → commit `3a67685f`
(warden harden deleted exactly those 8 lines). The operator's re-added
section (2026-07-15) survives only because no harden pass has run since.
No systemd timer exists — fires on next manual `dracon-warden once`.
**Fix**: promote `replace_managed_block` to production for both files; keep
the atomic temp+rename write; regression test with content before AND after
the block.

### H9 (F4.2) — Warden smudge corrupts whole-file-encrypted BINARY secrets
`dracon-warden/src/security/src/modules/filter.rs:361`. Whole-file-encrypted
binary files (DER `.key`, SQLite under `secrets/**`, `.kdbx`) decrypt through
`String::from_utf8_lossy` → invalid bytes become U+FFFD → working-tree file
silently corrupted; the corrupted file is later re-cleaned and re-encrypted,
so the corruption propagates into history and the original bytes are lost.
**Fix**: if the entire content is one secret tag, decrypt and write RAW BYTES
(no UTF-8 conversion); round-trip test with random non-UTF-8 bytes.

### H10 (F3.1) — CODEBERG_API_REPOS: the v0.112.29 GitLab bug's twin — every Codeberg API call 404s
`visibility.rs:260, 328, 770`. `".../repos/{}/{}"` +
`.replace("{}", "{owner}/{repo}")` substitutes BOTH slots →
`/api/v1/repos/dracondev/x/dracondev/x` → 404. My v0.112.29 note claiming
"harmless" was wrong. **Verified**: URL construction reproduced; live
`make-public --include-codeberg` fails on the codeberg leg.
`make-public/make-private --include-codeberg` can never succeed; daemon-side
codeberg visibility/metadata sync silently 404s.
**Fix**: single-placeholder template (same shape as the GitLab fix); add the
missing URL-pinning test.

---

## MEDIUM findings (34)

### Failure-visibility & state machine (daemon)

- **M1 (F1.7≡F3.9)** — `mirror_consecutive_fails` is write-never: the
  "Mirror Degraded" notification is dead code; mirror failures surface only as
  opaque "git push returned non-zero" PUSH_STUCK hints, and `is_repo_stuck`
  then blocks manual `sync-now` even when origin is healthy.
- **M2 (F1.9)** — Auto-create-on-discovery block: every healthy repo pays
  `git remote` per cycle + `ls-remote` per remote every 300s FOREVER; no
  "creation confirmed, stop checking" terminal state. ~100 SSH conns/5min
  across the fleet, existing solely for the first-boot window.
- **M3 (F1.10)** — Bootstrap gaps: `Ok(false)` gets no cooldown (empty repos
  re-run the whole bootstrap every 1s); the root commit skips staged-hygiene
  (`unstage_oversized_paths`/`unstage_excluded_paths`) — an operator-staged
  300 MiB file in a fresh repo is swept in unchecked.
- **M4 (F1.6)** — `MAX_FAILURES` permanently abandons repos after 5 failures
  with no re-probe and no notification; needs-human `Blocked` states (merge in
  progress) count toward the budget — a repo left mid-merge overnight falls
  off the sync radar precisely when the operator needs it watched.
- **M5 (F1.11)** — Origin push failure short-circuits ALL mirror pushes for
  the cycle — one broken forge orphans the repo on all forges.
- **M6 (F1.12)** — `count_ahead_commits` hardcodes `origin/main..HEAD` —
  push-timeout scaling never engages for mirror-only or non-main repos
  (exactly the repos the scaling was built for).
- **M7 (F1.13)** — SIGHUP clears an inconsistent subset of state (not the new
  v0.112.30 cooldowns, not stuck_push_repos reload).
- **M8 (F1.14)** — Trailing-drain timeout clears `in_flight` while tasks keep
  running → duplicate concurrent sync_repo on the same repo → index.lock
  contention + phantom ledger failures.
- **M9 (F1.8)** — `stage_cooldowns` map is dead (never inserted); filter-only
  repos are re-dispatched every few seconds forever.
- **M10 (F0.3)** — No pre-commit identity check: the daemon commits with
  whatever `user.email` the repo config says; the ownership guard locks out
  only AFTER poisoned commits reach the mirrors.

### git operations layer

- **M11 (F2.1)** — `repair_broken_tracking` never repairs the CHECKED-OUT
  branch (first-token `*` bug). **Verified.**
- **M12 (F2.2)** — `rewrite_ahead_paths` filter-branch fallback builds a
  broken argv (can never succeed on hosts without git-filter-repo).
- **M13 (F2.4)** — `.status()` exit codes ignored at ~6 sites;
  `consolidate_to_main` proceeds to `branch -D master` + push-delete even when
  `git checkout main` failed.
- **M14 (F2.5)** — `remote_repo_exists`: uncached per-push SSH call; ANY error
  (network down) looks like "repo missing" → spurious create attempts.
- **M15 (F2.6)** — "Repository not found" / "Push to create is not enabled"
  not classified as permanent → deleted remote repos retry forever (directly
  relevant to the v0.112.28 codeberg posture).
- **M16 (F2.7)** — `IndexLock` hardcodes `<repo>/.git/index.lock` → ENOTDIR on
  every nested submodule → standard files silently skipped every cycle for all
  10 game submodules.
- **M17 (F2.8)** — `cli_diff_entries`/`git_name_status_entries` don't use `-z`
  → non-ASCII filenames silently dropped from diffs; diff failure reads as
  "clean".
- **M18 (F2.9)** — `remove_stale_remotes` deletes OPERATOR-ADDED remotes (any
  non-origin remote not in policy). The v0.112.30 codeberg exclusion relies on
  it — scope it to daemon-managed remotes (e.g. a `dracon.managed` marker).
- **M19 (F2.3)** — `is_safe_git_path` only checks the first two components for
  `..` — deep traversal passes (defense-in-depth gap, not live).

### Policy / visibility / report

- **M20 (F3.2)** — TOML footgun is live TODAY: `standard_files_auto = true` in
  the operator's own config is silently absorbed into the last
  `[[standard_files]]` entry; `[extra_remotes]` is silently dropped. The
  ordering checker's `in_table` logic can't see it. **Verified by parsing the
  live config.**
- **M21 (F3.3)** — `config validate` hides ALL warnings on the success path
  ("✅ Policy is valid" while withholding every collected warning).
- **M22 (F3.4)** — `expand_tilde("~/x")` → `/x` (filesystem root) via
  `Path::join` semantics. Dormant (live config uses relative paths).
  **Verified by compiling the function.**
- **M23 (F3.5)** — `make-public`/`make-private` writes the visibility cache on
  ANY remote success even when the github leg failed → codeberg gate can start
  pushing a still-private repo to a world-visible forge (or stop pushing a
  still-public one).
- **M24 (F3.6)** — `make-public` resolves repos by basename first-match-wins —
  two repos sharing a basename → flips the WRONG repo (privacy incident shape).
- **M25 (F3.7)** — `refresh-visibility` reports errors as "refreshed" and
  poisons the cache to private on any `gh` hiccup; `errors` counter hardcoded 0.
- **M26 (F3.8)** — `parse_github_owner_repo` misparses `ssh://`/userinfo URLs
  and accepts non-GitHub origins as GitHub → wrong-forge visibility lookups
  feed the private safe-default.
- **M27 (F3.10)** — F52 control-char check covers env vars only; `.env`-file
  secrets are unvalidated (curl header injection surface via mid-line `\r`).
- **M28 (F3.12)** — Exclude patterns: single-`/` patterns without `**` (e.g.
  `reports/kdp-live-*.md`) are silently dead; the `rel.contains()` arm
  overmatches (`**/tmp/**` excludes `foo/tmpl/x`, `**/~/**` excludes any path
  containing `~`).

### Warden / system

- **M29 (F4.3)** — V1-fallback gate not wired: `set_allow_v1_fallback` has
  zero callers and no policy field — legacy V1 ciphertexts are undecryptable
  and the documented migration path doesn't exist.
- **M30 (F4.4)** — `setup-hooks --local` runs `git config local ...` (missing
  `--`) — always fails after hooks are already written (partial application).
  **Verified live** ("key does not contain a section: local").
- **M31 (F4.5)** — filter-clean passes plaintext through to git for >10 MiB
  inputs and refused paths — a 15 MiB `.env` lands in history unencrypted,
  silently.
- **M32 (F4.6)** — pre-push hook word-splits filenames — paths with spaces
  bypass the secret scan.
- **M33 (F4.7)** — guard's active-build protection misses workspace members —
  `ws/target` deleted mid-build when cargo runs from a member crate.
- **M34 (F4.8)** — guard's `ps` parsing is vulnerable to argv newline
  injection → can renice an arbitrary victim PID (bounded: deprioritize only).

---

## LOW findings (19)

F0.3→M10 promoted. Remaining: F2.10 (branch-name validation gaps in first SSH
push + detached-HEAD retry inconsistency), F2.11 (blocking sleep + no SIGKILL
in kill_process_group), F2.12 (stale parallel-push doc comment; join errors
lose remote name), F2.13 (space-paths truncated in detect_large_blobs_ahead;
no-upstream = "no blobs"), F2.14 SUSPECTED (ghost standalone candidates),
F2.15 (stale index.lock bricks repo after SIGKILL), F2.16 (parser nits:
path.trim() corrupts space-filenames; count_unpushed_vs_mirrors hardcodes
main; has_codeberg_tracking_ref transient-failure flap; orphan-origin `-N`
heuristic false-positives), F3.11 (trust matching case-sensitive despite
comment — fail-safe direction), F3.13 (role.rs F55 precedence not
implemented — display-only), F3.14 (bump.rs version line with trailing
comment parses to None), F3.15 (ledger pruned-count wrong on >100MB branch;
non-atomic ledger rewrite; stale doc constant), F4.9 (security-subcrate env
mutex dropped immediately — F45/F46 class persists in warden tests), F4.10
(is_git_tracked_dir checks basename at repo ROOT — nested tracked dirs
deleted without --allow-tracked), F4.11 (per-repo hooks inert under global
hooksPath), F4.12 (load_system_policy swallows read errors → defaults),
F1.15 (per-cycle subprocess inventory ~250 spawns/sec + unthrottled log-spam
paths), F1.16 SUSPECTED (restore_excluded_paths --worktree discards operator
edits — needs operator-intent decision), F1.17 (batch limit per-partition not
union; dead insert branch; blocking subprocess in async; unbounded webhook
threads; detached-HEAD guard SUSPECTED; count_unpushed_vs_configured_remotes
returns 1 not a count).

---

## Recommended remediation order

**v0.112.31 (daemon failure-visibility + policy holes — the HIGH batch):**

1. **H3** push failure ≠ "synced" — route a distinct outcome to the apply
   phase. Small, local, closes the false-healthy class for good.
2. **H4** notify cooldowns actually expire — one-line pattern fix at 6 sites
   + SIGHUP clear.
3. **H5** stuck-push ledger: shared map, don't reset budget on retry, enforce
   `push_max_retries`.
4. **H6** re-filter expanded directory files (size + patterns) — AGENTS.md
   hard-exclusion enforcement.
5. **H7** local-only `count_all_head_commits` before any `ls-remote` in the
   missing-ref arm + cooldown.
6. **H10** codeberg URL single-placeholder fix + pinning test (trivial; same
   shape as v0.112.29).
7. **H1** ownership verdict TTL re-detection + skip-log recovery hint.
8. **M1** wire `mirror_consecutive_fails` from `remote_failures` (or delete
   the dead code); pass failing remote name into `record_push_failure`.

**v0.112.32 (warden batch):**

9. **H8** `replace_managed_block` in production for .gitignore/.gitattributes.
10. **H9** whole-file-encrypted payloads round-trip as bytes.
11. **M29** wire `allow_v1_fallback` policy field (or scoped CLI flag).
12. **M30** `setup-hooks --local` missing `--` (one-line).
13. **M31** filter-clean fail-closed for oversized/refused inputs.
14. **M32** pre-push hook NUL-delimited iteration.

**v0.112.33 (daemon MEDIUM sweep + system + tests):**

15. M2–M9, M11–M19 (incl. F2.1 checked-out branch repair, M15 permanent
    push-failure classes, M16 IndexLock gitdir resolution, M18 managed-remote
    scoping).
16. M20–M28 (incl. config validate warnings, visibility-cache write
    discipline, exclude-pattern matcher fixes).
17. M33–M34 (guard workspace-awareness + ps injection hardening),
    F4.9/F4.10/F4.12, H2 follow-ups (commit-identity CI guard, M10 pre-commit
    identity check).

**Explicitly deferred / needs operator decision:**
- F1.16 (`restore_excluded_paths --worktree` discards operator edits to
  excluded paths) — SUSPECTED; is exclusion meant as "don't commit" or
  "must equal HEAD"?
- F2.14 (ghost standalone candidates in discovery) — SUSPECTED, verify
  downstream filtering first.
- Live config cleanup: move `standard_files_auto = true` above the
  `[[standard_files]]` blocks and delete or implement `[extra_remotes]`
  (M20) — operator's config edit, not a code change.

---

## Verification appendix (spot-checks performed on agent claims)

| Claim | Method | Result |
|---|---|---|
| F4.1 warden wiped operator .gitignore | `git show b69d9c2c` / `git show 3a67685f` | ✅ exact 8 lines added then deleted |
| F4.1 code path | read `build_gitignore_block_with_existing` + `apply_overwrite_file` + `#[cfg(test)] replace_managed_block` | ✅ confirmed |
| F4.4 `git config local` always fails | live test in temp repo | ✅ "key does not contain a section: local" |
| F2.1 checked-out branch skipped | read branch.rs:321-328 | ✅ first token `"*"` → `""` → skip |
| F3.1 codeberg URL doubled | python `str.replace` reproduction | ✅ 4-segment path → 404 |
| F3.2 live config absorption | `tomllib` parse of live dracon-sync.toml | ✅ `standard_files_auto` in last table entry; `extra_remotes` dropped |
| F1.1 notify deadline never read | grep `remote_notify_cooldowns` | ✅ 6 insert sites, zero expiry reads |
| F1.3 push failure → synced | read sync.rs:3055-3072 + daemon apply phase | ✅ Ok(false)/Err arms fall through to Synced |
| F1.2 push_max_retries unenforced | grep | ✅ report.rs display only |
| F1.5 dir expansion unfiltered | read exclude.rs:1297 + sync.rs:770-815 | ✅ no size/pattern checks on expanded files |
| F0.1/F0.2 ownership cache | live incident (config fix → 25 min skip → SIGHUP → recovery) | ✅ reproduced end-to-end |
