# AUDIT FULL — 2026-07-26 (post v0.113.1)

**Scope**: dracon-sync v0.113.1 (daemon, sync, git layer, report/policy/ownership/visibility/secrets),
dracon-warden v0.113.0 (filter, hooks, harden), dracon-system v0.112.33, meta-repo docs/config consistency.

**Method**: 6 parallel audit subagents, one per subsystem, briefed on the historical bug-pattern
catalog (raw-vs-effective accessors, uncached network in hot loops, false-healthy states, swallowed
errors, unit mismatches, early-return starvation, pipe deadlocks). Detail files:
`.pi-tmp/audit-2026-07-26-part{0-baseline,1-daemon-sync,2-git-layer,3-report-policy,4-warden,5-system-meta,5-sync-core-addendum}.md`.
Every HIGH below was **independently re-verified against the code by the orchestrator** (or
empirically reproduced by the auditing agent — marked REPRO).

**Counts**: **13 HIGH, 23 MEDIUM, ~30 LOW** (LOW consolidated; includes 8 still-open + 5 partial
carried forward from AUDIT_FULL_2026-07-21). Regression checks: all four 2026-07-21 HIGH fixes
(push-failure-as-synced, notification cooldowns, stuck-ledger split-brain, ls-remote hot loop) **hold**.

**Headline theme**: the daemon's *protective* machinery is where the sharpest edges are — the
auto-repair path (H6, H7), the auto-gc (H3), the wedge valve (H1), the backstop (H2), and warden's
enforcement hooks (W-H2, W-H3) all have defects that either silently disable the protection or
make it fire on the wrong targets.

---

## HIGH

### SYNC-H1. Quiet-daemon wedge: detached-task registry + 15-min valve unreachable when no new repo dispatches
`daemon.rs:3745` — `if !to_sync.is_empty()` wraps the ENTIRE spawn/apply/trailing-drain/hand-off/
wedge-valve block (valve at :4153-4166). A repo whose push outlives the trailing deadline stays in
`in_flight`; the no-redispatch check skips it every cycle; if no OTHER repo dispatches (overnight,
single active repo), the block never runs → the finished task is never applied, the wedged task is
never force-cleared, and `save_in_flight` makes `repos` report it as actively-processing forever
(false-healthy). Re-opens the 2026-06-15 permanent-skip class the registry was built to fix.
**Fix**: hoist detached-registry drain + wedge valve out of the `to_sync` gate; run them every cycle.

### SYNC-H2. Auto-commit backstop is self-defeating (and suppresses the push that would drain the backlog)
`sync.rs:3980` — backstop returns `NothingToDo` BEFORE `handle_ahead_push`; the daemon apply phase
maps `NothingToDo → success → activity.remove` (`daemon.rs:3830-3835` + success cleanup), which
**destroys `ahead_since`** — the very signal `is_backstop_active` needs (≥300s). Next cycle the
entry is recreated fresh → daemon commits anyway. The backstop only ever skips ONE dispatch per
300s window, and during that skip the pending 20+ commits aren't even pushed. Effectively dead code.
**Fix**: distinct outcome (or Blocked) so the apply phase retains `ahead_since`; call
`handle_ahead_push` before returning so the backlog drains.

### SYNC-H3. `maybe_auto_gc`: blocking, no timeout, runs before conflict-check, prune-race via wedge valve
`sync.rs:3684` → `git/mod.rs:3408-3465` — synchronous `std::process::Command::output()` for both
`count-objects -v` and `git gc --prune=now` inside the async sync task: no timeout, no kill-on-drop,
no spawn_blocking, plain `"git"` (ignores `DRACON_SYNC_GIT_BIN`). Runs BEFORE `check_conflict_state`.
A multi-GiB gc pins a worker thread for minutes; the (SYNC-H1) wedge valve can force-clear and
re-dispatch the repo while the old `gc --prune=now` still runs → `--prune=now` (no mtime grace)
racing a live commit/push = the classic prune-race against in-flight object writes. gc failure also
re-logs every cycle with no cooldown. (Merges part1-H3 + part2-M1 + part3-L7.)
**Fix**: run via bounded tokio spawn (or spawn_blocking) with a hard timeout; move below the
conflict check; use the shared git builder; consider plain `git gc` (2-week grace) for the
unattended path.

### SYNC-H4. `sync_mirror_visibility` violates the cache-poison invariant: transient `gh` hiccup flips public mirrors to private
`visibility.rs:628` uses `get_github_visibility` (bool, safe-default `true`) and `visibility.rs:702`
unconditionally writes the cache "even on partial failures" — while `get_github_visibility_opt`
exists precisely for cache-writing callers (its doc says the bool path poisons the cache). On any
transient gh failure the daemon (a) drives `github_private=true` into `set_gitlab_visibility` /
`set_codeberg_visibility` — **uncommanded remote state change flipping public mirrors private** —
and (b) caches "private" for 24h, gating the codeberg-public-only push path off. The CLI flip path
was fixed to use `_opt` (v0.112.33); the daemon path was missed.
**Fix**: use `_opt`; on `None` skip both the flips and the cache write.

### SYNC-H5. `standard_files` source path traversal: read-anywhere → publish-everywhere
`policy.rs:1516-1525` — target gets full component validation (absolute/`..`/root/prefix);
**source only gets `is_absolute()`**, and `expand_tilde("~/x")` runs at USE time
(`policy.rs:80-104`) — so `source = "~/.ssh/id_rsa"` passes validation (raw `~/...` is not
`is_absolute`) and resolves outside the sync base. Compounding: `validate_config` is only invoked
by the `validate`/`health` subcommands (`main.rs:805,1141`) — the daemon's execution path
(`sync.rs:3824` → `standard_files::ensure_standard_files`) never validates at all. The copied file
lands in every watched repo and is auto-committed + auto-pushed to public forges.
**Fix**: validate the tilde-EXPANDED source for ParentDir/RootDir/Prefix; enforce component checks
inside `ensure_standard_files` itself (point-of-use), not just in `validate_config`.

### SYNC-H6. `rewrite_ahead_paths` destroys its own backup, deletes `origin`, reports real rewrites as "no-op" — repair silently un-does itself (REPRO)
`staging.rs:229-277` — backup branch created, then `git filter-repo --invert-paths --force` with
NO `--refs` limit. Reproduced: (1) filter-repo rewrites ALL refs — the "backup" branch loses the
blob too, preserving nothing; (2) backup and HEAD are rewritten identically →
`rewrite_was_noop_then_cleanup` (compares backup tree vs HEAD tree) ALWAYS reports no-op → deletes
the backup, returns `Ok(None)` → caller (`report.rs:5525-5626`) does nothing — **no force-push**;
(3) filter-repo deletes the `origin` remote (documented upstream behavior). Next cycle
`ensure_origin_for_vscode` re-adds origin, the push fails non-ff, and `push_with_retries`' auto-pull
(`git pull --no-rebase origin HEAD`) **merges the pre-rewrite history back** — the >100 MiB blob
returns to local history and is pushed to all mirrors. Note: the 2026-06-30 audit's "zero backup
branches exist" evidence is consistent with this path having fired and been misreported as no-op.
**Fix**: bundle-based backup (or `--refs` excluding the backup); no-op check = pre vs post HEAD sha;
after rewrite, re-run remote configuration and push `--force-with-lease` explicitly.

### SYNC-H7. `detect_large_blobs_ahead`: cat-file stdin/stdout pipe deadlock — the 100 MiB guard silently never engages (REPRO)
`staging.rs:118-135` — `stdin.write_all(&rev_list.stdout)` completes BEFORE
`cat_file.wait_with_output()` starts draining: with ~4000 objects ahead, cat-file's 64 KiB stdout
pipe fills, it stops reading stdin, parent's `write_all` blocks forever. Reproduced (>15s hang).
The documented fixed pattern in `git/mod.rs:99-113` (writer thread) was not applied here. The 60s
tokio timeout returns Err, but `spawn_blocking` is uncancellable → the thread stays blocked and the
cat-file child leaks **every repair cycle**; caller does `.unwrap_or_default()` → zero entries →
the large-blob guard never engages for exactly the repos with thousands of objects ahead.
**Fix**: mirror the mod.rs writer-thread pattern (or pipe rev-list fd directly into cat-file stdin).

### SYNC-H8. Conflict-state detection is a no-op for nested submodules (all 10 nested game repos)
`status.rs:115-128` — `is_rebase_in_progress` / `is_merge_in_progress` / `is_cherry_pick_in_progress`
check `repo.join(".git").join("MERGE_HEAD")` etc. For nested-on-`main` submodules `.git` is a FILE
(verified: junk-runner's is a 55-byte gitdir pointer) → ENOTDIR → always false →
`check_conflict_state` never blocks. Operator mid-merge with conflicts walks away; daemon
`git add -A` stages conflicted files (silently "resolving" with markers), commits, pushes to all
forges. The sibling bug in `IndexLock::acquire` was fixed in v0.112.33 via `path_gitdir()`; these
three helpers were missed. Compounds with SYNC-M7 (auto-pull can leave MERGING state).
**Fix**: resolve the real gitdir via `crate::git::path_gitdir(repo)` and check the state files there.

### WARDEN-H1. H9 regression CONFIRMED: production filter-smudge still corrupts whole-file-encrypted binary secrets
`security/src/lib.rs:1115-1123` — `DraconWarden::smudge` (the path `run_filter` actually calls)
goes straight to `String::from_utf8_lossy` → `smart_smudge`, whose decrypt arm pushes
`from_utf8_lossy(&plaintext)` (filter.rs:361). The v0.112.32 H9 fix (`decrypt_whole_file_tag`)
is only wired into `seal_smudge` and `decrypt_file` — and `seal_smudge`/`seal_clean`/`decrypt_path`
have NO callers in the binary. The H9 unit test exercises the helper directly, so it passes while
production stays broken: binary secret → whole-file `[DRACON_SECRET:<b64>]` (ASCII, passes the NUL
check) → smudge decrypts inline → every invalid-UTF-8 byte → U+FFFD → corrupted worktree → next
clean re-encrypts the corruption → original bytes lost from history.
**Fix**: call `decrypt_whole_file_tag` FIRST in `DraconWarden::smudge`/`Warden::smudge`; byte-
identical round-trip test through the production entry point.

### WARDEN-H2. Global pre-commit hook hard-blocks commits in EVERY non-hardened repo on the machine (REPRO)
`main.rs:2288-2317` — `PRE_COMMIT_HOOK` exits 1 unless the repo's `.gitattributes` contains
`filter=dracon`; `run_setup_hooks` sets GLOBAL `core.hooksPath` (`main.rs:2497-2510`). Reproduced:
a throwaway /tmp repo cannot `git commit` ("Warden filter missing from .gitattributes"). Any
third-party clone, scratch repo, or new repo outside warden's roots is broken until hardened (or
`--no-verify`). The same global hooksPath also silently DISABLES all per-repo hooks fleet-wide
(husky, pre-commit framework) by shadowing `.git/hooks`.
**Fix**: no-op unless the repo is warden-managed (marker check); chain/preserve repo-local hooks.

### WARDEN-H3. pre-rebase `head -100` checks the NEWEST 100 commits — published commits deeper in the range escape
`main.rs:2446` — `git rev-list "$upstream"..HEAD | head -100`: rev-list is newest-first, so the cap
drops the OLDEST commits — precisely those most likely already published. Rebase with >100 commits
where 101+ are on a remote → guard passes → published history rewritten → divergent fleet mirrors
(the exact incident class v0.113.0 ships to prevent). Verified: line 2446 as described.
**Fix**: remote containment is ancestor-closed — check only the boundary commit (`tail -1`) or drop
the cap; one `git branch -r --contains` call instead of 100 subprocesses.

### SYS-H1. dracon-system guard daemon busy-loops after the first interval
`dracon-system/src/main.rs:3154,3230-3233` — `elapsed` declared once before the outer loop, never
reset; inner `while elapsed < interval` runs once, then every subsequent pass spins
`run_guard_once` back-to-back with zero delay (spawning df/ps/du/walkdir scans continuously).
**Fix**: reset `elapsed = 0` per outer iteration (or `tokio::time::interval`); add a timing test.

### SYS-H2. `link apply` can never fix (or even re-affirm) an existing symlink
`dracon-system/src/links.rs:135,138` — existing symlinks route through
`check_safe_to_delete(&link, &[])` which ALWAYS bails on symlinks → (a) drifted symlinks (the
primary case `apply` exists to fix) error out the whole command; (b) no `in_sync` short-circuit —
even healthy symlinks fail. Zero test coverage on `apply_link_policy`.
**Fix**: for symlinks use `fs::remove_file` directly (never touches the target); skip `in_sync`.

---

## MEDIUM (23; deduplicated — part1-L1+part3-M3 merged into M5; part1-L4+part2-M2 merged into M6)

| # | Finding | Location | Summary |
|---|---|---|---|
| M1 | detached_discard generation bug | daemon.rs:4158,4027 | per-repo (not per-generation) discard: fresh result discarded, stale one applied; ledger side effects not discardable |
| M2 | FilterOnly drops injected stale-gitlink entries | sync.rs:3905-3962 | parent gitlink convergence starved for filter-noisy parents — commit leg of the v0.113.1 starvation class |
| M3 | FilterOnly push on zero-ahead mirror-only repos | sync.rs:3958,4160 | `!branch_has_upstream` → push attempt every 300s → spurious failure-ledger entries → false 🛑 Exhausted |
| M4 | apply/trailing-drain outcome asymmetry | daemon.rs:3922-3972 vs 4057-4084 | same SyncOutcome mutates state differently by timing; late PushFailed never notifies |
| M5 | refresh_stale_upstream_ref unbounded on dead origin | sync.rs:4102-4180 | fire-and-forget silent failure; 30s fetch + N pushes per cycle forever when origin down but mirrors up; no ledger/cooldown |
| M6 | mirror pushes don't get scaled timeout | sync.rs:1821-1830 | origin gets scaled (≤600s), mirrors get raw base — the exact stall the scaling prevents |
| M7 | auto-pull-retry hazards | push.rs:204-212 | `git pull --no-rebase origin HEAD`: HEAD = remote default branch (may differ); origin may be foreign; no `--no-edit` (tty hang); conflict leaves MERGING (invisible per SYNC-H8) |
| M8 | SSH hardening missing on branch deletions | branch.rs:158,197,240 | no GIT_SSH_COMMAND/GIT_TERMINAL_PROMPT; stderr nulled; exit unchecked |
| M9 | unowned repos display "🟣 pushing" false-active | report.rs:3216-3267,347-390 | ownership override never rewrites push_status; 🚫 label unreachable in ACTIVITY |
| M10 | STUCK/FAIL invisible in ACTIVITY + label lies | report.rs:347-378 | ACTIVITY shows "synced" while PUSH cell shows 🛑 STUCK; "pushing Xm" = last-commit age (doc wrong) |
| M11 | ownership compare case-sensitive; only origin validated | ownership.rs:295-302,263-271 | mixed-case URL → false unowned (fail-closed); mirror remote URLs never ownership-checked |
| M12 | compute_diff_entries `unwrap_or_default` → false FilterOnly | sync.rs:657 | transient `git diff HEAD` failure → empty set → dirty repo misclassified filter-only → commit skipped + 300s cooldown, no error |
| M13 | residual `[DRACON_SECRET:…]` ciphertext in dracon-sync source comments | sync.rs:3375,4172; daemon.rs:1110,1132 | 2026-06-21 incident residue; no warden command can repair (scrub=json-only, resmudge=protected-only, decrypt_path uncalled) |
| M14 | scale_push_timeout 600s cap silently reduces live 900s base | sync.rs:149-176 | live config base=900 → `min(900×k, 600)` = 600 always; cap should be base-aware |
| M15 | setup-hooks overwrites foreign global hooks, non-atomic | warden main.rs:2467-2475 | unconditional fs::write ×3; no existence check/backup/temp+rename; clobbers operator's previous hooksPath |
| M16 | pre-push secret-scan regex: `\x27` literal in GNU ERE | warden main.rs:2406 | single-quoted secrets escape the scan; negated class refuses values containing x/2/7 (verified GNU grep 3.12) |
| M17 | pre-rebase bypass via two-arg form | warden main.rs:2442-2446 | `git rebase main feature` → empty `$1..HEAD` range → published feature commits rewritten |
| M18 | harden_repo silently skips gitfile repos | warden main.rs:1039-1062 | `.git` file → read fails → skip with no log: no managed blocks/filter/pubkey for worktrees+submodules |
| M19 | salvage_invalid_json_markers panics on non-ASCII | warden main.rs:1648-1656 | byte-offset slicing of &str panics; `bytes[i] as char` mojibake; crashes whole warden pass |
| M20 | test-identity push guard: no escape hatch, full-history scan on new branches | warden main.rs:2419-2428,2376-2382 | any historical test@ commit → branch unpushable without --no-verify; outside DRACON_ALLOW_REWRITE guard |
| M21 | `guard clean --all` silently ignored + dead validation | system main.rs:3548,3357 | `--all --rust` cleans only rust; "no targets" warning unreachable |
| M22 | log truncation TOCTOU + rename loses lines | system main.rs:1570-1650 | lines appended between read and rename discarded; writers keep unlinked inode |
| M23 | storage --cleanup --apply aborts on first failure | system main.rs:2912-2919 | one protected path/IO error kills the run mid-list after partial deletions |

## LOW (consolidated ~30 — see part files for full detail)

**dracon-sync**: unthrottled refresh fetch on never-converging upstreams (part1-L1); stage batching
union 2× documented limit (part1-L2, = 2026-07-21 F1.17 partial); GIT_SSH_COMMAND dispatch-local to
Command::Daemon (part1-L3); github_pack_too_large synchronous+uncached for ≥2 GiB repos (part1-L5,
part2-L4); kill_process_group blocking sleep + missing-`kill` hang (part2-L1); idle-timeout resettable
forever by `remote:` keepalives + unbounded stderr (part2-L2); askpass token file not cancellation-safe
(part2-L3); push_to_all_remotes stale SEQUENTIAL doc (part2-L5); parse_name_status_z desyncs on U/C
records (part2-L6); unchecked-exit residuals diff.rs/multi_remote.rs (part2-L7); git_ssh_hardening
fails hard if config missing + unquoted HOME (part2-L8); ls_remote "not found" matches GitLab
permission-mask + EXISTS_CACHE never populated on create + HTTPS fallback no credential + first-SSH-
failure not screened (part2-L9); rev-list @{u} empty on upstream-less repos disables large-blob
detection (part2-L9); refresh fetches ALL refspecs + fires on vacuous push success (addendum N3);
size-cache gitdir_sig write-only + no schema version + stale TTL comments (part3-L1); run_git_bounded
stdin-write deadline gap + tmp leak (part3-L2); SHA-256 repos bypass 2 GiB guard via len==40 filter
(part3-L3); visibility cache future-timestamp freshness (part3-L4); push-failure map 500-line flap
(part3-L5); doc-rot/dead refs incl. warn_if_world_readable message mismatch (part3-L6); visibility
flips: no ledger record, no --dry-run/--yes on make-public (part3-L8).

**dracon-warden**: daemon pushes always `--no-verify` — hooks never gate the fleet's primary writer
(L-1, needs a deliberate decision); backfill_env_headers unreachable success branch (L-2); shallow-
clone fail-closed with misleading message (L-3); replace_managed_block malformed-block deletes file
tail (L-4); swallowed errors in hook-install/stale-removal/resmudge (L-5); per-repo hook installs
dead under global hooksPath + installer disagreement (L-6).

**dracon-system**: nested node_modules double-counted (LOW); fabricated nix reclaim bytes +
conditionally swallowed generation errors (LOW); inode monitoring hardcodes `/` ignoring
disk_mount_path (LOW); acquire_daemon_lock truncates before locking (LOW); resolve_bin poison-unwrap,
shorten_event_time non-ASCII panic, zram ratio semantics (LOW nits). Test-coverage gaps enumerated in
part5 (daemon loop timing, apply_link_policy, rust-target cleanup paths, truncate_log_file,
manage_sync_freeze, renice state machine).

**Carried forward from 2026-07-21 (verified 2026-07-26)**: STILL-OPEN — F2.10 push.rs branch-
validation gaps; F2.12 stale SEQUENTIAL doc + lost remote names; F2.13 space-path truncation;
F3.11 case-sensitive trust (now M11); F3.14 bump.rs comment parsing; F3.15 non-atomic ledger
rewrite; F4.11 per-repo hook seeding inert under global hooksPath; F1.15 per-cycle spawn/log-spam.
PARTIAL — F2.11 SIGKILL added but blocking 2s sleep remains (= part2-L1); F2.15 index.lock startup-
only + literal `.git/index.lock` broken for submodule gitdirs; F2.16 hardcoded `main` in mirror
counting + codeberg flap; F3.13 no true precedence; F1.17 batch-limit union bug (= part1-L2).

## Meta-repo / docs findings (part5-B)

- **B1**: 7 of 7 spot-checked AGENTS.md source line refs stale (report.rs:3705→5525;
  report_v2_snapshot.rs GONE; staging.rs:152→182; policy.rs:1580→1864; sync.rs:858→1042/1060;
  git/mod.rs:684→1004; git/mod.rs:370→601).
- **B2**: push-timeout doc stale — live config is 900s (since 2026-06-23), AGENTS.md says 300;
  scale_push_timeout undocumented; example.toml still says 60. (Code bug from this = M14.)
- **B3**: `[patch.crates-io]` section says v94.7.1, actual is v94.7.2; workspace members list
  missing `dracon-warden/src/security`.
- **B4**: AGENTS.md claims warden owns `init.templateDir` — no warden code touches it (the live
  templateDir is manually installed, untracked). Claim should be "core.hooksPath only".
- **B5**: auto_gc docs accurate. ✓
- **B6**: two reversed design docs lack SUPERSEDED banners (daemon-standalone-removal-2026-07-01;
  push-timeout-fix-2026-06-17).
- **B7**: 5 live config keys undocumented in example.toml (alert_unpushed_threshold,
  auto_github_private, auto_github_private_account, max_stage_batch_files, exclude_repos).
- **B8**: AGENTS.md "Recent audit-driven changes" section stops at v0.112.21.

## Verified CLEAN (regression sweep)

- 2026-07-21 HIGH fixes H3/H4/H5/H7 all hold (with tests).
- count-objects unit handling correct everywhere post-v0.112.42 (×1024 both consumers).
- Ownership F39/F44: tuple-atomic, no substring matching, fail-closed on empty/unparseable.
- secrets.rs: no token ever logged/argv'd; askpass atomic O_EXCL|O_NOFOLLOW 0700.
- is_safe_branch_name/is_safe_git_path gate all argv-bound branches/paths.
- force-with-lease divergence path safe (stale lease fails closed; parse failure → Divergent).
- ensure_gitlab_main_protected: correct API/params, idempotent 409, argv-only.
- hooks.rs fully removed from dracon-sync (no dangling references).
- Warden: oversized-clean fails closed (M3 fix holds); harden_repo managed blocks preserve operator
  content (H8 fix holds); hook edge cases (new-branch zero-sha, tag moves, annotated-tag peeling,
  missing-object fail-closed, unpublished pull --rebase) all correct.
- v0.113.1 FilterOnly fix: borrow/scope fine, PushFailed correctly recorded+mapped,
  refresh_stale_upstream_ref cannot hang (30s idle timeout, prompt disabled, detached-HEAD safe).
