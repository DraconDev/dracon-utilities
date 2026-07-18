# Audit Part 1 — dracon-sync core (sync.rs / daemon.rs / git/{mod,multi_remote,push}.rs)
# Source-level audit, 15,491 LOC, 2026-07-18
# For inclusion into AUDIT_FULL_2026-07-18-POSTFIX.md

## CRITICAL findings

### F1 — `git show HEAD:<path>` argument injection (CRITICAL)
**File:** `dracon-sync/src/sync.rs:2687-2692, 2736-2738` (inside `auto_resolve_unmerged_if_safe`)

```rust
let head_output = Command::new("git")
    .args([
        "-C", &repo.to_string_lossy(),
        "show", &format!("HEAD:{}", path),
    ])
```

- **What:** `path` is read from `git ls-files --unmerged` output (i.e. arbitrary working-tree-relative paths). It is interpolated directly into `git show` with NO `--` separator. If a path begins with `-` or `--`, git treats it as an option. A file named `--upload-pack=evil` or `-c` (followed by an attacker-controlled string) that reaches the merge-unmerge path becomes a git-flag-injection vector. The daemon runs as the operator (full FS + SSH access).
- **Why it matters:** Realistic because operator-level merge conflicts land on files they did not author; a pre-existing file with a leading-dash name in any watched repo is sufficient.
- **Fix:** Use `["-C", &repo, "show", "HEAD", "--", path.as_str()]` so paths prefixed with `--` get caught. Even simpler: use libgit2 `blob_data` via `dracon_git` instead of `Command::new("git", ...)`.
- **Verification:** Add a unit test that creates a file named `--upload-pack=evil` with unmerged stage 3 and asserts (a) the resolved HEAD blob content matches the expected hash. Without the fix, `git show HEAD:--upload-pack=...` exits with error and `head_output` is `None` (the daemon silently skips that path, hiding the bug).

### F2 — Commit-msg content injection via `MERGE_MSG` / `REVERT_HEAD` and goal-metadata fields (CRITICAL)
**File:** `dracon-sync/src/sync.rs:2382, 2468-2471, 2483, 2589, 2597, 2605`

The `compute_blast_radius` function reads `.git/MERGE_HEAD` / `REVERT_HEAD` only as a file-existence gate (prefixing `MERGE:` / `REVERT:` into the commit message). It also calls `extract_goal_metadata` (sync.rs:1879-1895) which reads `.pi/goals/*.md`, parses JSON out of the top of the file, and embeds `status`, `pause_reason` (truncated), `tokens_used`, `evidence`, `skip_reason` RAW into the commit message.

- **What:** Any operator-readable file under `.pi/goals/archived/` or in the staged diff is reflected into commit-message text. No escaping of `` ` ``, `[`, `]`, `|`. `sanitize_task_name` does sanitize markdown checkbox text, but goal-metadata fields (`pause_reason`, `evidence`, `skip_reason`) are used RAW.
- **Why it matters:** A committed file containing backtick-embedded strings propagates unescaped into every pushed forge. Downstream tools that parse commit subjects for routing misfire. Lower severity than F1 but still a content-injection vector.
- **Fix:** Sanitize each goal_metadata field the same way `sanitize_task_name` does.
- **Verification:** Add a goal file containing `{"pauseReason": "evil` ` rm -rf / `"}` and assert the pushed commit message does not contain unescaped backticks adjacent to CR/LF.

## HIGH findings

### F3 — `repo.join(".git/index.lock")` check is racy (HIGH)
**File:** `dracon-sync/src/daemon.rs:2514-2525` (loop body, index.lock check), `daemon.rs:1810-1825` (`run_startup_cleanup`)

The daemon only checks `lock.exists()` — not "is the lock held by another process?". Between `exists()` returning `false` and the daemon calling `git commit`, another process can create the lock. `in_flight` is populated AFTER all per-repo checks pass (`daemon.rs:2866-2870`), so two cycles can race on the same repo path in the gap.

- **Fix:** Acquire an exclusive file lock (`fs2::FileExt::lock_exclusive`) on `<repo>/.git/` BEFORE entering the per-repo sync logic, and hold it across `sync_repo`. Drop when the task completes. Reuse `IndexLock::acquire` (sync.rs:3281) which is already used for `standard_files`.

### F4 — `trailing_drain_deadline_secs=120s` combined with re-dispatch pattern defeats MAX_FAILURES=5 (HIGH)
**File:** `dracon-sync/src/daemon.rs:503-525, 2752-2920`

The MAX_FAILURES guard counts ONLY failures applied inside the apply-phase loop (`daemon.rs:2849`). The trailing-drain path (`daemon.rs:2891-2933`) does NOT increment `failure_count` — only `entry.failure_count += 1` for `Blocked` and `Err`. The trailing-drain timeout (120s) means a repo that consistently times its pushes past 120s will get retried every cycle forever, never accumulating enough `failure_count` to trip MAX_FAILURES.

- **Fix:** Increment `failure_count` on trailing-drain timeout (treat missing-after-deadline as a failure), OR compute `total_attempts_since_first_dispatch` and trip after N cycles.

### F5 — `auto_create_all_remotes` race: another push can land during the create-then-push window (HIGH)
**File:** `dracon-sync/src/git/multi_remote.rs:98-128, 148-170`

The `auto_create_all_remotes` call uses `remote_repo_exists` (multi_remote.rs:498-507), which does `ls-remote HEAD`. Two simultaneous daemon invocations on the same repo both see "doesn't exist" → both call `gh repo create` → one wins (201), the other gets "already exists" (handled at 480-482). The remaining hole: between `auto_create_all_remotes` finishing and `push_to_all_remotes` starting, two cycles' `push_mirror_remotes` calls can interleave. If cycle A keeps `["github","codeberg"]` and cycle B keeps `["github","gitlab","codeberg"]`, cycle A's `remove_stale_remotes` call drops `gitlab` while cycle B is still trying to push to it.

- **Fix:** Serialize `push_mirror_remotes` per-repo across concurrent cycles using the same `in_flight` HashSet that `daemon.rs:2786` maintains, OR remove `remove_stale_remotes` from `push_mirror_remotes` and move to a one-shot setup pass.

### F6 — `create_repo_on_codeberg` token path-error swallows auth-rejection reason, leaks token (HIGH/security)
**File:** `dracon-sync/src/git/multi_remote.rs:475-489`

```rust
let body = response.text().await.unwrap_or_default();
anyhow::bail!("codeberg repo create failed ({}): {}", status, body);
```

Codeberg error responses for invalid tokens / unauthorized requests may include the token in the `message` field (sometimes echoed in headers). Worse, on a 401, the operator's codeberg token can end up in the daemon log via `eprintln!("⚠️ auto-create failed for {} on {}: {}", repo_name, remote_name, e);` at multi_remote.rs:108-111.

- **Fix:** Strip `Authorization:\s*Bearer\s+\S+` from the error-body string before formatting into `anyhow::bail!`.

### F7 — Permanent-rejection detection misses GitHub verified-email / GitLab project-owner / GitLab LFS-quota patterns (HIGH)
**File:** `dracon-sync/src/git/push.rs:255-263`

```rust
pub(crate) fn is_permanent_push_rejection(err_msg: &str) -> bool {
    err_msg.contains("pre-receive hook declined")
        || err_msg.contains("protected branch")
        || err_msg.contains("not allowed to push")
        || err_msg.contains("deny updating")
        || err_msg.contains("hook declined")
}
```

Misses:
- GitHub verified-email rejection: `You can only push commits that were created with your verified email.` (missed)
- GitLab project-owner denial: `GL-200: Push to this repository has been denied by the project owner.` (missed)
- GitLab branch-push rule: `branch is not in the list of allowed branches` (missed)
- GitLab LFS-quota: `you have exceeded your LFS storage limit` (missed)

The `is_permanent_push_rejection` short-circuit is the only mechanism preventing the daemon from burning its 3-retry budget on every cycle for repos with a real policy issue. Without comprehensive matching, those repos get a HINT column entry per cycle (filed every 60s).

- **Fix:** Add a `PUSH_PERMANENT_REJECTION_PATTERNS` const array (regex) for GH error codes (GH001 size, GH005 email, GH006 protected, GH013 not allowed; GL-200/300; CB-PRIVATE-001).

## MEDIUM findings

### F8 — `push_to_all_remotes` says "SEQUENTIAL" in doc but runs PARALLEL — comment lies (MEDIUM, code clarity + correctness)
**File:** `dracon-sync/src/git/multi_remote.rs:280-318`

The doc comment at multi_remote.rs:280-295 claims SEQUENTIAL after goal `87c1bf4d`. But the actual implementation at lines 312-318 spawns each push in its own `tokio::spawn`. This is **parallel** execution. The race-condition fix the comment claims to describe (one remote lagging, another accepting a fast-forward that the slow remote rejects) is NOT mitigated by the current code.

- **Fix:** Either restore actual sequential execution via `for remote in sorted` without spawn, OR update the doc comment and re-derive whether the original race is still resolved.

### F9 — `gh` / `glab` commands invoked with `Command::new("glab")` — no command-injection review of `repo_name` (MEDIUM)
**File:** `dracon-sync/src/git/multi_remote.rs:449-460` (`create_repo_on_gitlab`), `misc.rs:30-37` (`gh_cmd`)

`repo_name` is derived from `repo.file_name()` (multi_remote.rs:72). An operator repo at `/home/user/-oops/` produces `repo_name="-oops"`. `glab repo create -oops` interprets `-oops` as a flag. GitHub CLI similarly: `gh repo create --evil-flag`.

- **Fix:** Sanitize `repo_name` to `[a-zA-Z0-9._-]+` before passing to `glab`/`gh`.

### F10 — `pushed_branch_pushable_bytes` unbounded SHA String allocation (MEDIUM)
**File:** `dracon-sync/src/git/mod.rs:80-166`

The function writes SHAs to cat-file's stdin from a SEPARATE thread (lines 138-144) which avoids the 64 KiB stdin-pipe deadlock. Good. But the SHAs list grows unbounded — `shas.push_str(sha); shas.push('\n')` — for a repo with hundreds of thousands of reachable objects (dracon-platform has ~1.5M objects in one branch).

- **Fix:** Stream SHAs directly to cat-file stdin in batches (256 SHAs at a time), or use libgit2 `odb_read` via `dracon_git`.

### F11 — `push_with_retries` always invokes `push_with_transport_fallbacks` after exhausting retries, even on timeout-killed process (MEDIUM)
**File:** `dracon-sync/src/git/push.rs:191-197`

After exhausting `attempts`, the code falls through to a SINGLE attempt at `push_with_transport_fallbacks` which sequentially attempts HTTPS push to github → gitlab → codeberg. For a remote that consistently times out, this means the daemon hits the remote at 1× SSH + N× SSH retries + 3× HTTPS attempts before returning Err. On a 600s timeout, that's up to ~50min of daemon time consumed.

- **Fix:** Check the LAST attempt's error before falling through; if it was a timeout, skip the HTTPS fallback chain.

### F12 — `freeze_reason` checked once per cycle, but `SIGHUP` between check and sync-task spawn does not re-check (MEDIUM)
**File:** `dracon-sync/src/daemon.rs:2466-2472`

After the freeze check passes, the loop body iterates `for repo in repos` and spawns `sync_repo` tasks without re-checking freeze. A 1-second SIGHUP-style race window per cycle means up to N (sync_repo tasks in flight) repos commit during a freeze.

- **Fix:** Check `freeze_reason` at the head of the spawn loop AND inside `sync_repo` before each commit.

### F13 — `in_flight: HashSet<PathBuf>` not protected against races from `tokio::spawn` closure (MEDIUM, re-dispatch edge case)
**File:** `dracon-sync/src/daemon.rs:2858-2870, 2895-2905`

`in_flight` is a `HashSet<PathBuf>` mutating from the main loop while tasks spawned earlier are still running. Rust's single-threaded async model means no data race, BUT the next iteration of the cycle can read `in_flight` while a previous spawn is still running. A single repo's two `sync_repo` tasks may both pass the `in_flight.contains(&repo) == false` check if they happen to be dispatched on adjacent cycles.

- **Fix:** Before dispatching a repo, check `in_flight.contains(&repo) || last_drain_pending.contains(&repo)`. Track a `recently_dispatched: HashMap<PathBuf, Instant>` with a 120s expiry.

### F14 — `is_push_rejected` matcher matches `"rejected"` too broadly (MEDIUM)
**File:** `dracon-sync/src/git/push.rs:247-253`

The blanket `"rejected"` substring catches any git message with that word, including server-side error responses like "remote rejected: user has been banned" (permanent) and "push rejected: too many requests" (transient). When such errors match, `push_with_retries` triggers an auto-pull that is GUARANTEED to fail.

- **Fix:** Match only the well-formed `[rejected] <branch> -> <target> (<reason>)` pattern, OR match the specific substrings `non-fast-forward`, `fetch first`, `first parent`, `current tip`.

### F15 — `verbosity` (VERBOSITY AtomicU8) checked at module scope but no CLI parsing sets it on `-v` (MEDIUM)
**File:** `dracon-sync/src/daemon.rs:14-23`

No CLI parsing in this file sets `VERBOSITY` from `--verbose`/`-v`. If `main.rs` does not set VERBOSITY on `-v`, the daemon runs at the default level always.

- **Verification:** Run `dracon-sync daemon -v`; tail -f the journal; confirm `🐛` debug lines emit. (OUT OF SCOPE FOR THIS AUDIT — needs main.rs check.)

### F16 — `compute_blast_radius` accepts arbitrarily-long paths with no length cap (MEDIUM)
**File:** `dracon-sync/src/sync.rs:1992-2030, 2438-2444`

A path that exceeds the embedding budget will truncate the commit message strangely. If 10 large paths overflow, the `format!(...)` string can exceed 64KB (libgit2 commit-message cap) and `svc.commit(&msg).await?;` at sync.rs:2921 fails silently.

- **Fix:** Slice metric vectors to a hard cap (e.g. 5 elements, 1KB total) and add a debug log when truncated.

## LOW findings (selected)

### F17 — HTTP codeberg client uses default `reqwest::Client` — no timeouts (LOW)
**File:** `dracon-sync/src/git/multi_remote.rs:475-493`

`reqwest::Client::new()` uses defaults: connect timeout 0, overall request timeout 0. A flaky Codeberg blocks the daemon indefinitely.

- **Fix:** `reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?`.

### F18 — `run_startup_cleanup` uses `fuser <lock-file>` to detect live hold — non-portable (LOW)
**File:** `dracon-sync/src/daemon.rs:1817-1825`

`fuser` is Linux-specific and not in all distros. On macOS or in minimal containers, this silently falls through to `unwrap_or(false)` → always treats the lock as free → removes it.

- **Fix:** Use `lsof` as a fallback, OR use `flock(fd, LOCK_EX | LOCK_NB)` from `fs2` which is cross-platform.

### F19 — `run_startup_cleanup` spurious lock removals due to fuser race (LOW)
**File:** `dracon-sync/src/daemon.rs:1818-1822`

fuser returns 0 also for transient races (process started and exited in the window). On a busy CI, this can cause spurious lock removals.

- **Fix:** Add a small retry delay (`std::thread::sleep(Duration::from_millis(50))`) before locking.

## INFO / non-issues (verified clean)

- **Path traversal in `PathBuf::join`:** No untrusted user input goes through a `PathBuf::join` boundary in sync.rs / daemon.rs / git/. The daemon receives paths from `discover_git_repos`, which validates via `is_safe_git_path` (discovery.rs:633 per audit grep).
- **Credential leakage in push error messages:** `run_child_inner` (ops.rs:82-150) collects stderr via `BufReader::lines` and embeds it in the `child_status_result` Error. AUTH errors from `git push` (e.g., `Permission denied (publickey)`) don't contain the token. Verified by reading ops.rs:140-150.
- **Index/lock handling:** `IndexLock::acquire` (sync.rs:3281) uses flock semantics correctly. The other lock path at daemon.rs:1818 uses `exists()` only — the gap is F3.
- **Force-push detection:** `force_push_when_behind = false` is respected in `multi_remote.rs:217-227` (no `--force-with-lease` unless `force_when_behind=true`). Test `test_push_to_named_remote_no_auto_force_when_disabled` (mod.rs:2570+) confirms. ✅
- **Commit-all policy:** `untracked_exclude_patterns = []` global default + per-repo overrides (sync.rs:3310+) — wired. ✅
- **Size limit:** `max_stage_file_bytes = 104857600` enforced in `unstage_oversized_paths` (sync.rs:418) before staging. ✅
- **Push timeout:** `push_op_timeout_secs = 300` applied via `scale_push_timeout` (sync.rs:81-103) which scales with `ahead_commits`. Default 300 base × multiplier × cap 600. ✅
- **Auto-repair:** `auto_repair_concerns = true` (default) wired in `run_once` (daemon.rs:2174) and via `repair_concerns.rs`. ✅
- **Subprocess leaks:** `kill_on_drop(true)` is set on every `tokio::process::Command::spawn` (ops.rs:201, 217); pushes that time out call `kill_process_group` (ops.rs:17-26). No unbounded `spawn-without-wait` patterns. ✅
- **panic! / unreachable! / todo! in production paths:** Zero hits across all 5 files. Clean.
- **`unsafe` blocks:** Zero in all 5 files. Clean.
- **TODO/FIXME/XXX/HACK comments:** Zero hits. Comments are detailed but no `TODO` markers.

## Summary

| Severity | Count | Notable |
|---|---|---|
| **CRITICAL** | 2 | F1 (git-show arg injection), F2 (commit-msg content injection) |
| **HIGH** | 5 | F3 (lock race), F4 (FAILURES bypass), F5 (concurrent remove_stale), F6 (token leak in error body), F7 (incomplete permanent-rejection patterns) |
| **MEDIUM** | 9 | F8 (sequential-comment lies), F9 (glab/gh command injection), F10 (unbounded RSS), F11 (timeout-then-HTTPS fallback), F12 (freeze-check race), F13 (in_flight race edge case), F14 (broad "rejected" matcher), F15 (verbosity may not be wired), F16 (no commit-msg length cap) |
| **LOW** | 3 | F17 (reqwest no timeout), F18 (fuser non-portable), F19 (fuser race) |
| **INFO** | clean | AGENTS.md policy fields all enforced |
