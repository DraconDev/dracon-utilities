# Audit 2026-07-26, part 1 — dracon-sync core loop (daemon.rs / sync.rs / main.rs)

Scope: `dracon-sync/src/{daemon.rs (4474), sync.rs (9449, prod code ends ~4210), main.rs (2093)}` at v0.113.1,
plus the directly-implicated helpers `git/mod.rs::maybe_auto_gc`, `git/ops.rs::run_git_with_timeout_env_progress`,
`git/branch.rs::current_branch`, `git/config.rs::git_ssh_hardening`, `git/mod.rs::github_pack_too_large`,
`git/multi_remote.rs::push_mirror_remotes`, and `policy.rs` defaults. Read completely.

---

## HIGH

### H1. Detached-task registry and the 15-min wedged-task valve are unreachable when no new repo dispatches — permanent in-flight wedge in the quiet-daemon case

- Evidence: `daemon.rs:3745` `if !to_sync.is_empty() {` wraps the ENTIRE parallel/apply/trailing-drain/hand-off/wedge-valve block. The trailing drain select over `detached_syncs` (`daemon.rs:4006-4020`), the detached-registry hand-off (`daemon.rs:4132-4144`), and the wedged-task safety valve (`daemon.rs:4153-4165`) all live inside that gate.
- Mechanism: repo R's push outlives the trailing deadline (default 120s) → R stays in `in_flight`, handle moves to `detached_syncs`. On every subsequent cycle R is skipped by the no-redispatch check (`in_flight.contains(&repo) → continue`). If no OTHER repo has work (overnight, single active repo — common with ~35 watched repos), `to_sync` is empty → the whole block is skipped → the finished/wedged task is never polled, never applied, and the 15-minute valve never fires. R stays in `in_flight` forever: never re-dispatched until some other repo happens to dispatch or the daemon restarts. This re-opens the exact 2026-06-15 permanent-skip bug class the M8/F1.14 registry was built to fix, and the "safety valve" the comment promises does not actually log or clear in this scenario. Compounding: `save_in_flight(&in_flight)` runs every cycle with fresh `written_at`, so `repos` renders R as actively-processing ("🔄 now") forever — false-healthy.
- Suggested fix: hoist detached-registry maintenance out of the `if !to_sync.is_empty()` gate: every cycle, (a) non-blockingly drain `detached_syncs` (`while let Ok(Some(r)) = tokio::time::timeout(Duration::ZERO, detached_syncs.next()).await` or `poll_next`), applying results through the same path; (b) run the wedge valve unconditionally.

### H2. Auto-commit backstop is self-defeating: `NothingToDo` is treated as success and clears `ahead_since`; and while active it also suppresses the push

- Evidence: `sync.rs:3980` `if ctx.backstop_active { return Ok(SyncOutcome::NothingToDo); }` (inside the dirty+auto_commit branch, BEFORE the `handle_ahead_push` at `sync.rs:4080`). Daemon main apply phase maps `NothingToDo → sync_success = true` and then does `activity.remove(&repo)` (`daemon.rs:3945-3975`, success cleanup at ~3960-3972). `ahead_since` lives on the activity entry; `is_backstop_active` requires `ahead_since` age ≥ `auto_commit_backstop_min_age_secs` (default 300s, threshold 20 — `policy.rs:1134-1140`).
- Mechanism: cycle A: repo ahead>20 for >300s, backstop fires → `NothingToDo` → apply phase removes the activity entry → `ahead_since` destroyed. Cycle B (~1s later): entry re-created with `ahead_since = now` → backstop inactive for another 300s → the daemon auto-commits again — the moving-target the backstop exists to prevent. The backstop only ever skips ONE dispatch per 300s window. Additionally, while backstop IS active the early return happens before `handle_ahead_push`, so the pending 20+ commits are not even pushed during the skipped cycle (the one thing that would drain the backlog). Net effect: the moving-target protection is effectively dead code in the daemon path.
- Suggested fix: return a distinct outcome (or `Blocked`) from the backstop branch so the apply phase retains the activity entry and its `ahead_since`; and let the push phase still run (call `handle_ahead_push` before returning) so the backlog drains.

### H3. `maybe_auto_gc`: synchronous, no timeout, blocking a tokio worker — and the wedge valve can re-dispatch a repo while its `git gc --prune=now` is still running

- Evidence: call site `sync.rs:3684` `crate::git::maybe_auto_gc(repo, policy.auto_gc_garbage_threshold_bytes);` (sync fn inside the async sync task). Implementation `git/mod.rs:3408-3465`: `std::process::Command::new("git")` + `.output()` for both `count-objects -v` and `git gc --prune=now --quiet` — no timeout, no kill-on-drop, no `spawn_blocking`, and plain `"git"` (ignores `DRACON_SYNC_GIT_BIN`). Default threshold 2 GiB (`policy.rs:1084`).
- Mechanism: (a) A gc on a repo that just crossed 2 GiB garbage (dracon-platform measured 37 GiB) can run for many minutes, pinning one multi-thread worker for the duration; 4 simultaneous gc'ing repos pin 4 workers. (b) Worse: the sync task can't be killed while blocked in `.output()`. Trailing drain (120s) defers it to the detached registry; the H1 valve (when reachable) force-clears `in_flight` at 15 min and the repo RE-DISPATCHES while the old `gc --prune=now` is still running → `gc --prune=now` concurrent with a fresh commit/push of the same repo. `--prune=now` removes the mtime grace period, so this is the classic prune-race against in-flight object writes (the very tmp_pack_* objects this feature cleans up are created by pushes). Same-repo concurrency within one task is correctly serialized (gc before commit/push) — the valve is what breaks it. (c) Also races the operator's own concurrent git work in the same repo.
- Suggested fix: run gc via `run_git_with_timeout`-style spawn (tokio, kill-on-drop, bounded e.g. 600s) or `spawn_blocking`; skip gc when the repo is in `in_flight`/detached registries older than the gc start; honor `DRACON_SYNC_GIT_BIN`; consider `git gc` without `--prune=now` (default 2-week grace) for the unattended path.

---

## MEDIUM

### M1. `detached_discard` is keyed per-repo, not per-task-generation — a fresh result can be discarded and the stale one applied

- Evidence: wedge valve `daemon.rs:4158-4165` (`detached_discard.insert(repo)`), discard check `daemon.rs:4027-4035` (`if detached_discard.remove(&repo) { ...; continue; }`).
- Mechanism: wedged task W for repo R is force-cleared; R re-dispatches as task N; both handles sit in `detached_syncs` (FuturesUnordered = completion order). If N's fresh result arrives FIRST, `detached_discard.remove(&R)` consumes the marker and N's real result is thrown away; when W's stale result later arrives the marker is gone and it is APPLIED as if fresh (failure_count, Synced-log, activity removal). Exactly inverted. Additionally the discard only drops the returned outcome — the stale task's side effects are not discardable: `record_push_success`/`record_push_failure` inside `sync.rs` write the disk ledger directly (`sync.rs:4173-4203`), so a late stale `record_push_success` can erase a stuck entry the fresh task just created.
- Suggested fix: tag each dispatch with a generation counter (repo → u64); carry it in the task result; discard only results from generations older than the current one. Accept that ledger side effects can't be rolled back, or route them through the apply phase.

### M2. Filter-only early return drops injected stale-gitlink entries — parent gitlink convergence starved for filter-noisy parents

- Evidence: stale-gitlink injection `sync.rs:3905-3935` (appends `Modified` DiffFile entries, sets `status.is_clean = false`), then `sync.rs:3942` `if filter_only_cleared { ... return Ok(SyncOutcome::FilterOnly); }` — `filter_only_cleared` was computed inside `compute_diff_entries` BEFORE the injection, so injected gitlink entries never reach the partition/stage/commit pipeline.
- Mechanism: a parent repo whose only natural dirty entries are filter noise (warden `filter=dracon` repos — exactly the fleet pattern) AND whose submodule gitlinks are stale hits the FilterOnly return every cycle; `stage_gitlink_updates` never runs; the parent's gitlink drifts from the shared gitdir's `main` indefinitely, breaking the convergence invariant AGENTS.md says the daemon enforces. v0.113.1 fixed the push leg of this early-return starvation (junk-runner) but not the commit leg. Pre-existing, same bug class.
- Suggested fix: in the `filter_only_cleared` branch, when stale-gitlink entries were injected, fall through to the normal `!status.is_clean && policy.auto_commit` pipeline (only-gitlink commits are legitimate), or partition the injected entries through `stage_gitlink_updates` before returning FilterOnly.

### M3. Filter-only path (v0.113.1) can now flip a benign repo to PushFailed / stuck-ledger exhaustion for pushes that were never needed

- Evidence: `sync.rs:3958` `let push_ok = handle_ahead_push(&mut ctx, &svc).await?;` in the `filter_only_cleared` branch; `sync.rs:4160-4166` `should_push = current_status.ahead > 0 || !branch_has_upstream || upstream_ref_missing`; `sync.rs:4153` `svc.get_status().await?`.
- Mechanism: (a) For a mirror-only repo (no upstream) that is FULLY pushed, `!branch_has_upstream` makes `should_push` true forever, so every 300s stage-cooldown cycle now does a real push attempt. Any remote failure (one forge down) is written to the stuck ledger by `record_push_failure` (`sync.rs:4191-4197`); `consecutive_failures` accumulates to `push_max_retries` → `StuckDecision::Exhausted` → "🛑 push-stuck, auto-push paused" desktop alarm for a repo that was fully synced and previously benign. (b) The `?` on `svc.get_status()` converts what used to be a clean FilterOnly into `Err` (failure_count increment, no cooldown insert) on a transient libgit2 hiccup. The outcome propagation itself (PushFailed recorded + mapped) is CORRECT — the concern is that the attempt now exists for zero-ahead repos.
- Suggested fix: in the filter-only branch, gate the push on positive evidence of unpushed work (`current_status.ahead > 0 || upstream_ref_missing || count_unpushed_vs_mirrors > 0`) rather than `!branch_has_upstream` alone; treat `get_status` failure here as FilterOnly-with-debug-log instead of `?`.

### M4. Main apply phase vs trailing-drain asymmetry for identical outcomes

- Evidence: main apply: `NothingToDo → sync_success = true → activity.remove + failure_count = 0` (`daemon.rs:3922-3930`, ~3965-3972); trailing drain: `NothingToDo` only logs, entry retained (`daemon.rs:4057-4062`). Trailing `PushFailed` increments `failure_count` but never sends the desktop notification the main phase sends (`daemon.rs:4076-4084` vs `3890-3910`).
- Mechanism: the same `SyncOutcome` mutates daemon state differently depending on whether the task finished within ~2s (apply deadline = `pulse*2`) or within the trailing window. Activity retention — and therefore `ahead_since`/`dirty_since`/backstop semantics (see H2) — becomes timing-dependent. Push failures that complete late are silent on the desktop.
- Suggested fix: extract one `apply_outcome(repo, outcome, ...)` used by both phases.

---

## LOW

### L1. `refresh_stale_upstream_ref` (v0.113.1): unthrottled fetch on never-converging upstreams; blocking sync subprocesses in async fn

- Evidence: `sync.rs:4102-4150`. The 4 `output()` closure calls are synchronous `std::process` inside an async fn (config×2, rev-parse×2). Fetch fires whenever `upstream_ref != HEAD`, with no negative cache.
- Mechanism: timeout (30s idle, `run_git_with_timeout_env_progress`) and `GIT_TERMINAL_PROMPT=0` are correct — no hang, no prompt. Detached HEAD handled (`current_branch` filters `"HEAD"`, returns `Option`), missing branch config handled (empty-config early returns), no feedback loop with the 300s cooldown (the fetch doesn't dirty the tree). But a repo whose configured upstream remote never carries the branch (push goes to mirrors only) takes a 30s-bounded SSH fetch after EVERY successful push, forever. Minor worker-thread blocking from the sync subprocesses.
- Suggested fix: record a per-repo "upstream unconvergeable" marker after N failed refreshes and stop fetching; move the config/rev-parse probes behind one `git` invocation or `spawn_blocking`.

### L2. Stage batching takes `max_batch` from EACH list — union can be 2× the documented limit

- Evidence: `sync.rs:3164-3174`: `regular_paths.into_iter().take(take)` and `gitlink_paths.into_iter().take(take)` with `take = max_batch`, despite the comment "batch-limit applies to the union of regular and gitlink paths".
- Mechanism: worst-case staged files per commit = 2 × `max_stage_batch_files`. Cosmetic unless the batch limit exists to bound lock/IO time, in which case the bound is silently doubled.
- Suggested fix: `let take_r = take.min(regular_paths.len()); let take_g = take - take_r;` or similar.

### L3. v0.112.41 `GIT_SSH_COMMAND` hardening is dispatch-local to `Command::Daemon`

- Evidence: `main.rs` `Command::Daemon` arm (~line 1155): `if std::env::var_os("GIT_SSH_COMMAND").is_none() { set_var(...) }`. `Command::Once` / `Command::SyncNow` run the same fetch/pull code without it.
- Mechanism: correct for the systemd unit (the broken case), but the fix lives at the command-dispatch layer rather than at the fetch/pull site, so any future entry point (or `once` run under `systemd-run` for debugging, which is exactly how the incident was reproduced) hits the unhardened path again. Also honors an inherited-but-broken `GIT_SSH_COMMAND` (the `is_none()` guard).
- Suggested fix: set the default inside the fetch/pull spawn helper (`spawn_git_command*`) rather than in main, or document that `once` is interactive-only.

### L4. Push-timeout scaling applies to the origin push but not to mirror pushes

- Evidence: `sync.rs:1630-1710`: `scaled_timeout = scale_push_timeout(...)` used for `push_with_retries(repo, scaled_timeout, ...)` (origin), but `push_mirror_remotes(repo, &policy.remotes, policy.push_op_timeout_secs, ...)` receives the unscaled base.
- Mechanism: the v0.112.10 lesson (28-commit binary push >60s) was a MIRROR push; with today's base of 300s the exposure is only for >20-ahead pushes (which scale to 600s on origin but stay 300s on mirrors). A large push can therefore time out on mirrors while origin gets the scaled budget.
- Suggested fix: pass `scaled_timeout` to `push_mirror_remotes`.

### L5. `push_background`'s `github_pack_too_large` is a synchronous, potentially-expensive call inside the async push path

- Evidence: `sync.rs:1652` → `git/mod.rs:40-148`: fast-path on `measure_git_size_bytes` (cheap), but for `.git` ≥ 2 GiB it runs `rev-list --objects <branch>` + `cat-file --batch-check` synchronously on every push attempt.
- Mechanism: only the already-oversized repos pay it (they're skipped from github anyway), so each of their push cycles blocks a worker thread for the full object walk — seconds to tens of seconds for a huge branch — on every attempt, forever, until the repo shrinks.
- Suggested fix: cache the measurement (it can't shrink without a gc/rewrite, both of which the daemon would notice) or `spawn_blocking`.

---

## Regression checks on the 2026-07-21 HIGH fixes — all HOLD

- **H3 (push failure reported as synced)**: holds. `SyncOutcome::PushFailed` is produced by both push paths (`sync.rs:3360-3370` stage_commit_and_push, `sync.rs:4189-4203` handle_ahead_push) and mapped to failure in BOTH the main apply phase (`daemon.rs:3888-3910`) and the trailing drain (`daemon.rs:4076-4084`). The v0.113.1 FilterOnly path propagates it too (`sync.rs:3958-3962`). See M3 for the new false-positive vector.
- **H4 (notification cooldown deadlines never read)**: holds. `notify_throttled` (`daemon.rs:2090-2110`) reads the stored deadline, re-fires after expiry, and re-arms; regression test present. All call sites converted.
- **H5 (stuck ledger split-brain)**: holds. Ledger reloaded from disk every cycle (`daemon.rs:2780-2782`), retry path persists `last_retry_at` instead of deleting the entry (`daemon.rs:3385-3395`), `Exhausted` arm enforces `push_max_retries` (`stuck_decision`, `daemon.rs:520-545`).
- **H7 (ls-remote every 1s cycle for never-pushed repos)**: holds. Local-first fallback chain + `ls_remote_cooldowns` 300s throttle (`daemon.rs:3455-3490`).

## Answers to the specific v0.113.1/v0.113.0/v0.112.41 review questions

1. FilterOnly + `handle_ahead_push`: borrow/scope is fine (`&mut ctx`, `&svc` disjoint). `PushFailed` from the new path IS correctly recorded (record_push_failure inside handle_ahead_push + apply-phase mapping). But yes, it can flip previously-benign repos to PushFailed — see M3. `refresh_stale_upstream_ref` cannot hang the sync path (30s idle timeout + prompt disabled), handles detached HEAD and missing branch config, and has no cooldown feedback loop — but is unthrottled on never-converging upstreams (L1).
2. v0.112.41 GIT_SSH_COMMAND: correct for the daemon arm; dispatch-local limitation at L3.
3. v0.113.0 auto-gc: cannot run concurrently with a push of the same repo WITHIN one task (strictly sequential), but the no-timeout blocking call plus the wedge valve creates the cross-task concurrency window — H3. Threshold parse is correct (KiB→bytes ×1024, default 2 GiB). `dry_run` correctly skips.
4. in_flight bookkeeping: panic → JoinError → converted to Err result → applied → removed (no leak). Timeout kill of git subprocesses → task returns Err → removed. The leak is the gating bug H1. The wedge valve logs and clears ONLY when reachable (H1); when it fires, the discard mechanism has the generation bug M1.
5. Stuck ledger after v0.113.1: PushFailed from the FilterOnly path is recorded identically to other paths (M3 covers the over-recording edge).
