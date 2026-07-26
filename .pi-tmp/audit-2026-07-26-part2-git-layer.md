# Audit — dracon-sync git operations layer (v0.113.1)

Scope: `dracon-sync/src/git/*.rs` (mod, ops, push, staging, multi_remote, status, diff,
branch, config, discovery, misc, urls) + call-site context in `sync.rs` / `report.rs` /
`policy.rs`. Date: 2026-07-26. Research only — no code modified.

Two findings were **empirically reproduced** (filter-repo backup-ref rewrite +
`origin` deletion; cat-file pipe deadlock). Repro scripts are described inline.

Counts: **3 HIGH, 4 MEDIUM, 9 LOW**.

---

## HIGH

### [HIGH] H1 — `rewrite_ahead_paths`: filter-repo rewrites the backup branch and deletes `origin`; F31 no-op cleanup then reports a real rewrite as "no-op", so the rewritten history is never pushed
- **Evidence**: `staging.rs:229-277` (backup branch created before `filter-repo --invert-paths --force` with no `--refs` limit), `staging.rs:308-343` (`rewrite_was_noop_then_cleanup` compares backup tree vs HEAD tree), caller `report.rs:5525-5626` (`Ok(None) => {}` — no push, no incident).
- **Mechanism** (both halves reproduced on 2026-07-26 in a scratch repo):
  1. `git filter-repo --invert-paths --force` rewrites **all refs**, including
     `refs/heads/backup/pre-sync-largeblob-fix-*`. Repro: after the rewrite the backup
     branch no longer contained `assets/big.bin` — the "backup" preserves nothing.
  2. Because backup and HEAD are rewritten identically, their trees are **always equal**,
     so `rewrite_was_noop_then_cleanup` deletes the backup and returns `Ok(None)` even
     when a real rewrite happened. The F31 test only exercises the no-op case, so this
     is untested. Caller treats `Ok(None)` as "nothing to do" → **no force-push**.
  3. filter-repo also **deletes the `origin` remote** (documented upstream behavior;
     repro confirmed: `github`/`gitlab` remotes survived, `origin` was removed).
- **Blast radius**: on the next sync cycle `ensure_origin_for_vscode` re-adds `origin`;
  the origin push then fails non-fast-forward (local was rewritten, remote wasn't), and
  `push_with_retries`' auto-pull (`git pull --no-rebase origin HEAD`, push.rs:208)
  **merges the pre-rewrite history back in** — the >100 MiB blob returns to local
  history and is pushed to all mirrors. The repair silently un-does itself and pollutes
  history with a merge. Note: the 2026-06-30 audit's evidence "zero
  `backup/pre-sync-largeblob-fix-*` branches exist" is *consistent with this path firing
  and being misreported as no-op* — absence of backups is not proof it never fired.
- **Suggested fix**: create the backup as a bundle (`git bundle create`) or tag it and
  pass `--refs` to filter-repo excluding the backup ref; do the no-op check by comparing
  pre-rewrite HEAD sha vs post-rewrite HEAD sha, not backup-tree vs HEAD-tree; after a
  successful rewrite, re-run `configure_all_remotes` (origin is gone) and push with
  `--force-with-lease` explicitly instead of relying on the auto-pull path.

### [HIGH] H2 — `detect_large_blobs_ahead`: cat-file stdin/stdout pipe deadlock (sibling of the mod.rs deadlock that "broke dracon-platform's push"), fails open with a permanent thread+process leak
- **Evidence**: `staging.rs:118-135` — `stdin.write_all(&rev_list.stdout)` completes
  *before* `cat_file.wait_with_output()` starts draining stdout. Compare with the
  documented fix in `git/mod.rs:99-113` ("CRITICAL deadlock avoidance … feed stdin from
  a SEPARATE thread"). The fixed pattern was not applied here.
- **Mechanism** (reproduced 2026-07-26): with ~4000 objects in `@{u}..HEAD`
  (~160 KiB rev-list output), cat-file's 64 KiB stdout pipe fills (nobody reading), it
  stops reading stdin, and the parent's `write_all` blocks forever. Python replica of the
  exact write-then-read pattern hung >15 s ("DEADLOCK CONFIRMED"). In production the
  `tokio::time::timeout(60s)` fires and returns Err, but `spawn_blocking` is
  **uncancellable**: the thread stays blocked on `write_all` and the cat-file child is
  leaked **every repair cycle**. Caller `report.rs:5469-5471` does `.unwrap_or_default()`
  → zero entries → the 100 MiB large-blob guard silently never engages for exactly the
  repos with thousands of objects ahead (the ones that need it).
- **Suggested fix**: mirror the mod.rs pattern (writer thread + concurrent stdout
  drain), or pipe rev-list output directly into cat-file via
  `.stdin(Stdio::from(rev_list_stdout_file))` / a single shell-free pipeline, or use
  `git cat-file --batch-check` with `--buffer` reading from the rev-list child fd.

### [HIGH] H3 — conflict-state detection is a no-op for nested submodules (`.git` is a file), so the daemon can stage/commit/push through an operator's in-progress merge/rebase in all 10 nested game repos
- **Evidence**: `status.rs:115-128` — `is_rebase_in_progress` / `is_merge_in_progress` /
  `is_cherry_pick_in_progress` all check `repo.join(".git").join("MERGE_HEAD")` etc.
  For the canonical nested-on-`main` submodule layout, `<repo>/.git` is a **file**, so
  these paths are ENOTDIR and `.exists()` is always false. `check_conflict_state`
  (sync.rs:387-408) therefore never blocks for nested submodules. The sibling bug in
  `IndexLock::acquire` was fixed in v0.112.33 (M16/F2.7) by resolving the real gitdir
  via `path_gitdir()` (status.rs:40-49) — these three helpers were not.
- **Mechanism**: operator runs `git merge`/`git rebase` in a nested game repo, hits a
  conflict, walks away. Daemon cycle: `check_conflict_state` passes → `git add -A`
  stages the conflicted files (**silently "resolving" the conflict with markers**) →
  auto-commit → auto-push to github/gitlab/codeberg. The operator's merge state is
  destroyed and conflict markers are published. Compounds with finding M3 (auto-pull can
  leave a repo in MERGING state — undetected for nested repos).
- **Suggested fix**: resolve the real gitdir once via `crate::git::path_gitdir(repo)`
  (already `pub(crate)` since v0.112.33) and check `MERGE_HEAD` / `rebase-merge` /
  `rebase-apply` / `CHERRY_PICK_HEAD` under it, falling back to the `.git` dir.

---

## MEDIUM

### [MEDIUM] M1 — `maybe_auto_gc`: `git gc --prune=now` runs before the conflict-state check, unbounded and blocking, on a live repo
- **Evidence**: `git/mod.rs:3408-3456`; call site `sync.rs:3683-3687` runs it **before**
  `check_conflict_state` (sync.rs:3687).
- **Mechanism**:
  1. `--prune=now` removes git's 2-week mtime grace. git-prune(1)/git-gc(1) warn that
     pruning without a grace period is unsafe concurrently with other git writers: a
     racing `git commit` (operator in a terminal, agent loop, or a second
     `dracon-sync once` process) writes loose objects *then* updates the ref — gc in
     that window prunes the just-written objects → broken ref / corruption. It also
     expires unreachable reflog entries, destroying the amend/rebase safety net.
  2. Ordering: a repo mid-rebase/merge/cherry-pick gets gc'd even though the sync is
     correctly blocked two statements later.
  3. `std::process::Command::output()` blocks the tokio worker thread with **no
     timeout**. A gc on a 30+ GiB gitdir can run far past `repo_sync_timeout_secs`
     (420 s); the sync-task timeout cannot cancel a blocking call, so the gc leaks into
     the next cycle (gc.pid lock makes the second invocation fail noisily).
  4. Uses plain `Command::new("git")` instead of the `git_cmd()` builder (binary
     override / env consistency).
  Units and the `0`-disables semantic are correct (KiB→bytes multiply at mod.rs:3391 is
  right; threshold check at mod.rs:3409,3421 is right; dry_run respected).
- **Suggested fix**: use plain `git gc` (default grace) or `git prune --expire=1.hour.ago`;
  move the call below `check_conflict_state`; run it via `spawn_blocking` (or
  `tokio::process`) with a hard timeout; use the shared `git_cmd()` builder.

### [MEDIUM] M2 — push timeout scaling is computed but not applied to the mirror pushes
- **Evidence**: `sync.rs:1618-1627` computes `scaled_timeout =
  scale_push_timeout(policy.push_op_timeout_secs, ahead_count)` and uses it only for the
  origin push (sync.rs:1692). The mirror path passes the **unscaled** base:
  `push_mirror_remotes(repo, &policy.remotes, policy.push_op_timeout_secs, …)`
  (sync.rs:1821-1830).
- **Mechanism**: the scaling exists because a large-ahead push sits in the negotiate
  phase past the base timeout (comment sync.rs:1609-1617). For a repo 28+ commits ahead
  whose origin is codeberg, origin gets 600 s but the github/gitlab pushes get the raw
  300 s (or less per config) — the exact stall the scaling was added to prevent, on the
  forges where the big packs actually go. `scale_push_timeout` itself (sync.rs:167-178)
  is arithmetically fine (capped at 600 s).
- **Suggested fix**: pass `scaled_timeout` to `push_mirror_remotes`.

### [MEDIUM] M3 — auto-pull-retry can merge the wrong upstream / wrong branch, and can hang a tty on a merge editor
- **Evidence**: `push.rs:204-212` — on `[rejected] (fetch first)`, runs
  `git pull --no-rebase origin HEAD` once, then retries the push.
- **Mechanism**:
  1. `HEAD` as a fetch refspec resolves to the **remote's default branch**, which may
     differ from the branch being pushed (e.g. local `main` vs remote HEAD → `master`):
     merges the wrong branch into the pushed branch.
  2. `origin` is not necessarily the daemon's mirror — operators can set a custom origin
     (fork, internal gitlab). On fetch-first the daemon merges that foreign tip into
     local and, if the merge is clean, pushes the result to all three mirrors. (Unrelated
     histories are refused by git, but a fork sharing history merges cleanly.)
  3. No `--no-edit`: git opens `$EDITOR` for a merge commit when stdin is a tty. Tokio
     `Command` inherits stdin (`ops.rs:219-243` never sets it), so `dracon-sync once` /
     `repair-concerns --apply` from a terminal can hang inside vim.
  4. On conflict the pull leaves the repo in MERGING state — per H3, undetected for
     nested submodules, so the next cycle `git add -A` "resolves" it with markers.
- **Suggested fix**: pull an explicit `refs/heads/<branch>` matching the pushed refspec,
  add `--no-edit`, verify `origin` URL matches the expected mirror before auto-merging,
  and abort (`git merge --abort`) on conflict.

### [MEDIUM] M4 — SSH hardening (v0.112.41) not applied to the branch-deletion network ops
- **Evidence**: `branch.rs:158` and `branch.rs:197`
  (`git push origin --delete master`), `branch.rs:240`
  (`git push origin --delete <other>`) — plain `std_git_command()` spawns with no
  `GIT_SSH_COMMAND`, no `GIT_TERMINAL_PROMPT=0`, stderr nulled.
- **Mechanism**: these bypass the `-F ~/.dracon/secrets/ssh/config` key/config selection
  (wrong key → `Permission denied (publickey)`), and without BatchMode an unknown-host
  or passphrase prompt can block a tty-run CLI; failures are invisible (stderr null,
  exit status unchecked). Every other network spawn (push/pull/ls-remote in push.rs,
  multi_remote.rs:1012) does carry the hardening, so this is a coverage gap, not design.
- **Suggested fix**: route these through `run_git_with_timeout_env_progress` with the
  standard `GIT_SSH_COMMAND`/`GIT_TERMINAL_PROMPT` env, and check the exit status.

---

## LOW

### [LOW] L1 — `kill_process_group`: blocking sleep in async context; missing `kill` binary → push future hangs forever
- `ops.rs:28-51`: `std::thread::sleep(2s)` between SIGTERM/SIGKILL stalls an executor
  thread. If `kill` is absent (sandbox), the early `return` skips SIGKILL entirely; an
  orphaned ssh then holds the stderr pipe open and `stderr_task.await` (ops.rs:186)
  never completes — the push future never resolves. Fix: async sleep; always follow
  `start_kill()` with a bounded `wait()` that doesn't depend on stderr EOF.

### [LOW] L2 — idle-timeout can be extended forever by `remote:` lines; stderr buffered unboundedly
- `ops.rs:91` (`^remote:\s+\S` resets the deadline — a hung/hostile remote emitting
  keepalives makes the push immortal, defeating the 600 s cap) and `ops.rs:127-145`
  (full stderr accumulated in one String; progress with `\r` is a single giant "line").
  Fix: cap total wall-clock in addition to idle, cap stderr retention (keep tail).

### [LOW] L3 — askpass token file cleanup is not cancellation-safe
- `push.rs:29-46, 59-86`: explicit `remove_file` after the push await; a
  cancelled/panicked task leaves `/tmp/dracon-git-askpass-*.sh` containing the forge
  token (mode 0700) until reboot. The `AskpassScript` RAII guard exists
  (`ops.rs:462-488`) but is deliberately unused. Fix: use the guard at all call sites.

### [LOW] L4 — `github_pack_too_large` refinement runs synchronously in the async push path, uncached
- `git/mod.rs:40-71`, call site `sync.rs:1633` (passes `precomputed_size=None`): for
  every ≥2 GiB `.git`, `rev-list --objects <branch>` + `cat-file --batch-check` walks
  the whole branch inline (tens of seconds on dracon-platform-class repos), blocking a
  worker thread every cycle; the report.rs size cache is not consulted. Fix:
  `spawn_blocking` + share the `REPO_SIZE_CACHE`.

### [LOW] L5 — `push_to_all_remotes` doc comment says SEQUENTIAL (with a PUSH_STUCK race rationale); the code is parallel
- `multi_remote.rs:538-560` vs `566-592`. Either the 2026-06-20 re-parallelization
  re-introduced the documented fast/slow-remote race, or the comment is stale. Reconcile.

### [LOW] L6 — `parse_name_status_z` desyncs on `U`/`C` records
- `diff.rs:92-137`: unknown statuses don't consume their path operands, so a following
  record's path is read as a status — a path starting with `M`/`A`/`D`/`R` then corrupts
  the staged-set. Reachable during conflicted merges (`U path`) via
  `git_name_status_entries(["diff","--cached","--name-status"])` (sync.rs:3607).
  Fix: consume one path for all single-path statuses, two for `R`/`C`.

### [LOW] L7 — residual unchecked-exit sites (the v0.112.33 M13/F2.4 class)
- `diff.rs:225-240` (`untracked_entries`), `diff.rs:255-273` (`staged_paths`,
  `tracked_paths`): non-zero exit reads as "empty" (one-cycle blindness; for
  `staged_paths` it also silently skips unstage enforcement).
  `multi_remote.rs:392-395` (`remove_stale_remotes`: `remote remove` exit ignored),
  `multi_remote.rs:231-256` (`ensure_origin_for_vscode`: `remote add` / `config` exits
  ignored). Fix: `std_git_checked` everywhere.

### [LOW] L8 — `git_ssh_hardening` failure modes
- `config.rs:9-13`: `-F <config>` makes **every** ssh op fail hard if
  `~/.dracon/secrets/ssh/config` is missing (file exists on this host — verified); HOME
  is interpolated unquoted into a shell-parsed `GIT_SSH_COMMAND` (spaces/metachars in
  HOME break or inject). Fix: tolerate a missing config file; single-quote the path.

### [LOW] L9 — assorted minor
- `multi_remote.rs:960-971` `ls_remote_indicates_missing`: bare `contains("not found")`
  also matches GitLab's permission-masked private-repo response → `Missing` → spurious
  (harmless, already-exists) create attempts each cycle; `EXISTS_CACHE` is never
  populated after a *successful create* (only after a successful ls-remote), so the
  round-trip repeats. `multi_remote.rs:1036-1043`.
- `push.rs:13-24`: the GitHub HTTPS fallback carries **no credential at all** (unlike
  GitLab/Codeberg askpass) — dead path unless a system credential helper exists.
- `multi_remote.rs:464-492`: first SSH failure in `push_to_named_remote` is not screened
  for permanent/pack-too-large before the HTTPS fallback → a 2 GiB pack rejection
  re-packs over HTTPS before failing fast.
- `ensure_gitlab_main_protected` (multi_remote.rs:756-806) is otherwise correct: args
  are passed as argv (no shell injection), glab missing/failing is non-fatal, the
  endpoint/params (`POST projects/:id/protected_branches`, `push_access_level=40`,
  `merge_access_level=40`, `allow_force_push=false`) match the GitLab API, and the 409
  "already exists" idempotency path is handled. Nits: `repo_name` is interpolated into
  the URL path with only the `/` encoded (spaces etc. would break the endpoint), and
  the 409 check depends on the English phrase "already exists" appearing in stderr.
- `staging.rs:105-165`: `rev-list @{u}..HEAD` silently returns `Ok(vec![])` when no
  upstream is configured (rev-list fails) → large-blob detection silently disabled on
  upstream-less repos; `@{u}` can also point at the "wrong" mirror.
  `staging.rs:163-164`: double `.with_context(...)?` produces a duplicated message.

---

## Positive checks (no issue found)

- `parse_count_objects_garbage_bytes` unit handling is correct (KiB → bytes, *1024) —
  the v0.112.42 sibling in report.rs was not repeated here. `size-garbage:` prefix match
  can't collide with `size:`/`size-pack:` (checked ordering-independent).
- `git_askpass_script` (ops.rs:362-435): atomic `O_EXCL|O_NOFOLLOW` create with mode
  0700, single-quote tokens refused (F59). No token ever appears in argv anywhere in the
  layer (env vars / askpass file / Authorization header only); tokens are not logged.
- `is_safe_branch_name` / `is_safe_git_path` gate every branch/path that reaches argv;
  `restore_paths` (F32) and unstage paths validate before spawning.
- `pushed_branch_pushable_bytes` (mod.rs:73-176) uses the correct writer-thread pattern
  and saturating sums; `github_pack_too_large` fast path avoids subprocesses for small
  repos.
- `diagnose_divergence` + `--force-with-lease`: stale local tracking refs make the lease
  fail, not overwrite — the force path is safe; divergence parse failures default to
  `Divergent` (no force).
- `run_child_inner` progress predicate is the F48-tightened regex; deadline math uses
  `saturating_duration_since` (no underflow).
- `.gitmodules` parsing (discovery.rs) can't inject: values are data-only, candidate
  paths are anchored under watch roots.
