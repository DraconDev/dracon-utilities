# Audit 2026-07-26, part 5 — core-loop addendum (FilterOnly / refresh_stale_upstream_ref verification + gap hunt)

Scope: independent third-reader pass over `dracon-sync/src/sync.rs` core loop
(`compute_diff_entries`, the v0.113.1 FilterOnly branch, `handle_ahead_push`,
`refresh_stale_upstream_ref`, `push_background`, `auto_pull_merge`,
`clean_staged_paths`) and the daemon↔warden hook interplay, after part1's two
reads. Part1's findings were re-verified against source where cited below; this
file adds what part1 did NOT cover.

---

## NEW FINDINGS

### N1 [MEDIUM]. `compute_diff_entries`: a failed `git diff HEAD` is misclassified as "filter-only" — dirty repo silenced for 300s with no error recorded

- **Evidence**: `sync.rs:657` `let diff_output = crate::git::git_diff_head_files(repo).await.unwrap_or_default();` then `sync.rs:661-670`: `if diff_output.is_empty() && !entries.is_empty()` + all-Modified → `entries.clear(); status.is_clean = true; filter_only_cleared = true;`. The error modes are real: `git/diff.rs:14-45` returns Err on non-zero exit AND on the 30s `tokio::time::timeout` — both become an EMPTY set via `unwrap_or_default()`, indistinguishable from "diff ran clean, filter equalized".
- **Mechanism**: dirty repo (all entries Modified) + transient diff failure (30s timeout on a huge/repo-locked tree, spawn failure) → the commit pipeline is skipped entirely AND — because v0.112.33 made `FilterOnly` insert the 300s stage cooldown (sync.rs:3937-3965 comment) — the repo's real changes go uncommitted for 5 minutes per transient failure, reported as a benign FilterOnly, `failure_count` untouched. In the mixed case (untracked files present, so `has_non_modified` routes to the `retain` branch, sync.rs:672-679), an empty-from-error `diff_output` silently DROPS all Modified entries for the cycle while untracked ones are committed. The v0.113.1 push leg still runs, so committed work pushes — only the commit of dirty changes is lost for the window.
- **Fix**: propagate the error (`let diff_output = ... .await?;`) so a diff failure surfaces as a sync failure, or on Err fall back to trusting the libgit2 entries (skip the filter-only determination entirely — false positives are far costlier than false negatives here). Non-UTF8 pathnames are also lossy-mangled by `from_utf8_lossy` in diff.rs:24 (U+FFFD path never matches `e.path` → Modified entry dropped); use `OsStr`/`PathBuf::from(OsString)` on unix.

### N2 [MEDIUM]. Residual `[DRACON_SECRET:...]` ciphertext committed inside dracon-sync SOURCE comments — and no current warden command can repair it

- **Evidence**: `src/sync.rs:3375`, `src/sync.rs:4172` (`...the test \`test_sync_repo_mirror_failu[DRACON_SECRET:YWdlLWVu...]\`.`), `src/daemon.rs:1110`, `src/daemon.rs:1132` (same pattern, `test_record_push_failu...`). Present in HEAD blob AND worktree; `git status` clean (clean-filter fixpoint). Introduced by `817ecb2` (2026-06-21, the source-encryption-incident era — the commit also added the zero-byte `.plaintext` siblings). `src/git/mod.rs:686` is a legitimate intended test fixture (written into a temp `github.env` by a test), not corruption.
- **Mechanism**: during the 2026-06-15..21 incident window, warden's clean filter in-situ-encrypted a substring of these Rust comments (a scanner pattern matched part of a test name — e.g. `re_...`/`tly_...`-style API-key regexes match snake_case identifiers), and the tag form was committed to history on all mirrors. Checkout never decrypted it (repo has no `.gitattributes`/filter config), leaving literal ciphertext in the worktree. It compiles (all 4 are inside comments), but: (a) the comments are destroyed — the doc cross-references are unreadable; (b) repair is IMPOSSIBLE with current tooling: `scrub_markers` only scans `*.json` (main.rs:1724), `resmudge` only scans `protected_patterns` paths (main.rs:1865), and `WardenSecurity::decrypt_path`/`decrypt_file` (the H9-fixed, binary-safe path!) has NO CLI caller in the warden binary — confirmed by grep, only `decrypt_file`←`decrypt_path` internal reference. Cross-ref part4 H-1: the only whole-file-safe decryption paths are dead code.
- **Fix**: add a warden subcommand that decrypts arbitrary-marker files by explicit path (the machinery exists, unwired), then repair these 4 comments; or restore the comment text from `817ecb2^` and commit the fix. Add a fleet-wide grep for `\[DRACON_SECRET:` in tracked non-secret files to the audit runbook.

### N3 [LOW]. `refresh_stale_upstream_ref` fetches ALL refspecs of the remote and fires on vacuous push success

- **Evidence**: `sync.rs:4137-4145` `git fetch <remote>` (no `<branch>` arg — fetches every configured refspec for that remote); `sync.rs:4174-4178` runs it after ANY `Ok(true)` from `push_background`, including the vacuous case (`push_results` empty because every remote was excluded → `all_ok` on empty iterator = true → `Ok(!origin_failed)` = true).
- **Mechanism**: extends part1-L1: a repo with many branches/tags on the upstream remote pays a full fetch (30s-bounded) after every successful push, not just the one branch needed; and repos whose remotes are all policy-excluded still fetch upstream each cycle.
- **Fix**: `git fetch <remote> <short-branch>`; skip the refresh when `push_results` was empty.

---

## VERIFIED (part1 holds / no issue)

- **Part1 M2 (gitlink injection starved by FilterOnly return)**: holds. Injection at `sync.rs:3905-3935` mutates `entries`/`status` AFTER `compute_diff_entries` computed `filter_only_cleared` (`sync.rs:3868-3872`); the `sync.rs:3942` early return bypasses staging/commit of the injected gitlinks. Line numbers exact.
- **Part1 M3 (`!branch_has_upstream` → perpetual should_push in FilterOnly leg)**: holds; `sync.rs:4160-4166`. Additionally verified the aggregate return of `push_background` is correct: mirror failure → `return Ok(false)` (`sync.rs:1844`) → `record_push_failure` → `PushFailed`; excluded remotes (github-2GiB-skip, codeberg-public-only) are NOT in `push_results` so they don't poison `all_ok` (`sync.rs:1837`).
- **CR-2 regression (filter-only re-detection via CLI fallback)**: properly guarded — `sync.rs:691` `if entries.is_empty() && !filter_only_cleared` skips the fallback, and the fallback itself (`cli_diff_entries` via porcelain) is clean-filter-aware.
- **Warden global-hook interplay**: the daemon is FULLY hook-immune — commits use `git commit --no-verify` (`sync.rs:3615`, doc comment `sync.rs:3454`) and every push path uses `push --no-verify` (`git/push.rs:22,41,65,113`). So warden's v0.113.0 global pre-commit/pre-push/pre-rebase stack (part4) cannot break daemon commits/pushes — and equally provides zero enforcement for the fleet's primary writer. Cross-ref part4 L-1; this is the sync-side confirmation.
- **`auto_pull_merge`** (`sync.rs:444-522`): correct gating (behind>0 && clean), timeout bounded by `pull_op_timeout_secs`, MergeConflict/failure/timeout all propagate Err — aborts the sync pass rather than committing on top of a half-merged tree. Note the daemon merges, never rebases, so no interplay with warden's pre-rebase guard even for interactive-equivalent flows.
- **`is_backstop_active`** (`sync.rs:130-148`): `threshold == 0` disables; strict `>` comparison. Part1 H2 (the `NothingToDo`-clears-`ahead_since` self-defeat) is an apply-phase issue, verified there.
- **`clean_staged_paths`** (`sync.rs:524-566`): dry_run correctly skips all three mutations; oversized-unstage uses the 100 MiB policy bound.
