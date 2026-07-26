# Audit 2026-07-26 — Part 0: Baseline (prior-audit LOW carryover + operator decisions + regression surface)

Scope: verify the 19 LOW findings from `AUDIT_FULL_2026-07-21.md` against current code
(dracon-sync v0.113.1, dracon-warden v0.113.0, dracon-system), report the two deferred
operator-decision items, and list the safety-critical regression surface from
dracon-sync v0.112.34–v0.113.1 and dracon-warden v0.113.0.

Note on count: the audit's "19 LOW" includes F0.3, which was promoted to M10 and fixed in
v0.112.33 (pre-commit identity guard, extended v0.112.36 to honor `owned = true`). 18 findings
are actually tracked below.

## 1. LOW findings status table

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| F0.3 | (promoted to M10 pre-audit-close) | FIXED | v0.112.33 M10 pre-commit identity guard; v0.112.36 honors `owned = true` override |
| F2.10 | Branch-name validation gaps in first SSH push + detached-HEAD retry inconsistency | STILL-OPEN | `git/push.rs:103-106,164-167`: SSH refspec is bare `HEAD` (no branch-name check) and detached-HEAD hardcodes `HEAD:refs/heads/main` in both first-push and retry paths; only the HTTPS fallback validates via `is_safe_branch_name` (push.rs:130) |
| F2.11 | Blocking sleep + no SIGKILL in kill_process_group | PARTIAL | SIGKILL added v0.112.22 (F47): `git/ops.rs:25-49` SIGTERM → 2s → SIGKILL. Still-open prong: `std::thread::sleep(2s)` blocks the async executor thread on the timeout path (ops.rs:44, called from async loop ops.rs:188) |
| F2.12 | Stale parallel-push doc comment; join errors lose remote name | STILL-OPEN | `git/multi_remote.rs:622-638` doc says "SEQUENTIAL (not concurrent)" while :648-672 spawns parallel `tokio::spawn`s; :668-669 join error recorded as `("unknown", Err("join error"))` — remote name still lost |
| F2.13 | Space-paths truncated in detect_large_blobs_ahead; no-upstream = "no blobs" | STILL-OPEN | `git/staging.rs:142-159`: parses cat-file output with `split_whitespace()` + `parts.next()` — paths with spaces truncated; rev-list failure (incl. no upstream) returns `Ok(Vec::new())` (staging.rs:121-123) |
| F2.14 | SUSPECTED ghost standalone candidates in discovery | OPEN-SUSPECTED (mitigated) | Downstream filter exists: `git/discovery.rs:70 is_duplicate_standalone_for_nested` skips standalones when the nested submodule is canonical; daemon no longer materializes standalones (removed 2026-07-08). No dedicated verification test found — still unverified per the audit's deferral |
| F2.15 | Stale index.lock bricks repo after SIGKILL | PARTIAL | Startup-only cleanup exists: `daemon.rs:2263-2290` removes `.git/index.lock` after a `fuser` check at startup. Still-open prongs: no mid-cycle recovery (a SIGKILLed git bricks the repo until daemon restart), and the cleanup probes the literal `<repo>/.git/index.lock` (broken for submodule/worktree gitdirs — the M16 fix covered IndexLock in status.rs, not this path) |
| F2.16 | Parser nits (4 prongs) | MIXED | (a) `path.trim()` corrupts space-filenames: FIXED v0.112.33 — production parsing is NUL-delimited `parse_name_status_z`; `parse_name_status_line` with `path.trim()` is now `#[cfg(test)]`-only (diff.rs:55-88). (b) `count_unpushed_vs_mirrors` hardcodes `main`: STILL-OPEN (status.rs:329-352, `refs/remotes/{github,gitlab,codeberg}/main`). (c) `has_codeberg_tracking_ref` transient-failure flap: STILL-OPEN (multi_remote.rs:113-123, single rev-parse, failure → false → codeberg excluded). (d) orphan-origin `-N` heuristic false-positives: STILL-OPEN (misc.rs:41-66 still flags any single-digit `-N` suffix; a legit `name-3` repo would false-positive) |
| F3.11 | Trust matching case-sensitive despite comment (fail-safe direction) | STILL-OPEN | `ownership.rs:173,189-193`: exact `==` string comparisons for trusted emails/authors; no case-folding anywhere in ownership.rs |
| F3.13 | role.rs F55 precedence not implemented — display-only | PARTIAL | v0.112.22 (F55) added the full-path check, but `role.rs:165-179` combines all three matchers with `\|\|` inside first-match-wins iteration — a basename/path-tail collision with an earlier row still beats a full-path match on a later row. No strict precedence; impact remains display-only |
| F3.14 | bump.rs version line with trailing comment parses to None | STILL-OPEN | `bump.rs:1-21`: F43 (v0.112.21) strips trailing `;`, but `version = "1.0.0" # comment` still fails the closing-`"` check → None |
| F3.15 | Ledger pruned-count wrong on >100MB branch; non-atomic rewrite; stale doc constant | STILL-OPEN | `report.rs:1182-1196`: >100MB branch returns `Ok(lines.len())` (kept, not pruned); both rewrite branches use plain `std::fs::write` (non-atomic); 100MB constant hardcoded |
| F4.9 | security-subcrate env mutex dropped immediately (F45/F46 class in warden tests) | FIXED | v0.112.33 — `dracon-warden/src/security/tests/common.rs:7-81`: guard now holds `ENV_MUTEX` for its whole lifetime (4 "FIXED 2026-07-21 (v0.112.33, audit F4.9)" sites) |
| F4.10 | is_git_tracked_dir checks basename at repo ROOT — nested tracked dirs deleted without --allow-tracked | FIXED | v0.112.33 — `dracon-system/src/main.rs:2516+`: "FIXED 2026-07-21 (v0.112.33, audit F4.10)" comment; ls-files now uses path relative to repo root |
| F4.11 | Per-repo hooks inert under global hooksPath | STILL-OPEN (now by-design tension) | `dracon-warden/src/main.rs:2563-2603 install_hooks_for_repo` still seeds `.git/hooks/*`, which git ignores when global `core.hooksPath` is set — and warden 0.113.0 made global hooksPath the canonical fleet layer. The per-repo seeding is dead weight (or a trap) unless a repo sets local hooksPath |
| F4.12 | load_system_policy swallows read errors → defaults | FIXED | v0.112.33 — `dracon-system/src/main.rs:2438-2454`: read errors now propagated ("FIXED 2026-07-21 (v0.112.33, audit F4.12)") |
| F1.15 | Per-cycle subprocess inventory ~250 spawns/sec + unthrottled log-spam paths | STILL-OPEN | No changelog entry v0.112.34–v0.113.1 addresses daemon per-cycle spawn reduction or log throttling (v0.112.31 throttled notifications only; v0.112.40-42 perf work targeted the `repos` CLI, not the daemon loop) |
| F1.16 | SUSPECTED restore_excluded_paths --worktree discards operator edits | FIXED (operator decision made) | v0.112.34 — `sync.rs:1442-1510`: default now unstages only (edits preserved); opt-in `revert_excluded_to_head = true` restores old revert-to-HEAD behavior; documented in AGENTS.md:172 |
| F1.17 | 6-prong batch | MIXED | (a) batch limit per-partition not union: STILL-OPEN — `sync.rs:3165-3178` comment claims "union" but code does `regular.take(max_batch)` AND `gitlink.take(max_batch)` (up to 2× limit). (b) unbounded webhook threads: STILL-OPEN — `sync.rs:215` bare `std::thread::spawn` per failure. (c) `count_unpushed_vs_configured_remotes` returns 1 not a count: STILL-OPEN — `daemon.rs:61-103` returns literal `1` on any mismatch/failure. (d) dead insert branch, (e) blocking subprocess in async, (f) detached-HEAD guard SUSPECTED: not individually relocated in this pass; no fix evidence in changelogs → presumed still-open |

**Tally (18 tracked findings; F0.3 counted as fixed-via-promotion):** FIXED 5
(F0.3/M10, F4.9, F4.10, F4.12, F1.16) · PARTIAL/MIXED 5 (F2.11, F2.15, F2.16, F3.13,
F1.17) · STILL-OPEN 8 (F2.10, F2.12, F2.13, F3.11, F3.14, F3.15, F4.11, F1.15) ·
OPEN-SUSPECTED 1 (F2.14). Total: 5+5+8+1 = 19.

## 2. Operator-decision items status

1. **F1.16 restore semantics — DECIDED and shipped (v0.112.34, 2026-07-22).** Operator chose
   "exclusion = don't auto-commit" as the default: excluded files are unstaged after each
   commit but worktree edits are preserved. Opt-in hygiene enforcement ("must equal HEAD")
   via `revert_excluded_to_head = true` in `.dracon/dracon-sync.toml`. 2 regression tests;
   documented in AGENTS.md (line 172). No longer open.
2. **Live config cleanup — DONE (2026-07-22).** Verified in the live policy
   `/home/dracon/.dracon/utilities/sync/dracon-sync.toml`: `standard_files_auto = true` moved
   ABOVE the `[[standard_files]]` blocks (line 282, with a comment citing audit M20/F3.2),
   and the zombie `[extra_remotes]` table deleted (line 298 comment: "DELETED 2026-07-22
   (audit M20/F3.2 cleanup)"). Both operator edits confirmed present.
3. **F2.14 (also in the deferred list) — verification still pending.** Downstream filtering
   (`is_duplicate_standalone_for_nested`, discovery.rs:70) exists, but no one has confirmed
   the ghost-candidate class is dead; treat as suspected-open with a mitigating filter.

## 3. Regression surface — safety-critical changes since the audit

### dracon-sync v0.112.34 → v0.113.1

- v0.112.34: excluded-path post-commit handling inverted — default is now unstage-only
  (`unstage_paths_git`), `--worktree` restore only behind `revert_excluded_to_head`
  (data-deletion behavior change, sync.rs:1442).
- v0.112.36: pre-commit identity guard (M10) now accepts per-repo `owned = true` overrides —
  a commit-gate relaxation; `Blocked` outcomes cool down 300s (retry cadence change).
- v0.112.39: `rewrite_ahead_paths` auto-repair now refuses to rewrite damaged gitdirs
  (missing-objects pre-flight, staging.rs:206); `probe_missing_objects` + `BROKEN_HISTORY`
  state (report.rs); hygiene_patterns now exclude screenshot dirs from auto-commit (commit
  policy change); operator ran an orphan cutover + force-push on deathrun.
- v0.112.40: `repos` size probe switched from `du -sb` to `git count-objects -v` — changed
  the semantics feeding `github_pack_too_large`'s 2 GiB push guard (report.rs).
- v0.112.41: daemon now sets `GIT_SSH_COMMAND` process-wide (all CLI fetch/pull inherit the
  hardened ssh config); dracon-git v94.7.2 enables git2 ssh/https transports + `ssh_cred()`
  agent/key fallback — fetch/pull auth path changed.
- v0.112.42: `REPO_SIZE_CACHE_TTL_SECS` 30→3600 (freshness of pack-size data); CRITICAL unit
  fix — count-objects sizes were read as bytes, actually KiB (×1024) — the 2 GiB pack guard
  had been silently disabled since v0.112.40.
- v0.113.0: NEW `auto_gc_garbage_threshold_bytes` (default 2 GiB) — the daemon now runs
  `git gc --prune=now` itself when `size-garbage` exceeds the threshold (first daemon-initiated
  gc/prune — destructive-op class); gitlab auto-create now protects `main`
  (`allow_force_push=false`) on create and re-ensures on exists.
- v0.113.0 (ownership note): the planned no-history-rewrite hook layer moved OUT of
  dracon-sync to dracon-warden — dracon-sync's per-repo `src/hooks.rs` experiment was added
  (fab7a26) then deleted (d11f5b4) to avoid ping-ponging hook ownership.
- v0.113.1: `filter_only_cleared` early-return reordered — FilterOnly path now runs
  `handle_ahead_push` FIRST (push-ordering change; fixes 10h push starvation); new
  `refresh_stale_upstream_ref` runs a bounded `git fetch <upstream>` after successful pushes
  when the tracking ref disagrees with HEAD (new network op in the push path).

### dracon-warden v0.113.0 (2026-07-25)

- pre-push hook now refuses non-fast-forward ref updates (amend/rebase of pushed commits can
  never push) and branch deletions — hard push gate, fleet-wide via global hooksPath.
- NEW pre-rebase hook refuses rebasing any commit contained in a remote-tracking branch.
- `DRACON_ALLOW_REWRITE=1` env escape hatch bypasses both guards.
- `setup-hooks` (global + local) installs all three hooks and deletes stale `.pre-dracon`
  chaining artifacts from the dracon-sync per-repo hook experiment.
- `install_hooks_for_repo` also seeds `pre-rebase` (only-if-missing; foreign hooks untouched).
- pre-push tests corrected to use git's real all-zeros new-ref sentinel (the ff-guard
  correctly rejects the old empty-tree-SHA test fixture).
- (Context, pre-existing from the audit batch: v0.112.32 filter-clean fail-closed +
  whole-file binary encryption round-trip; v0.112.33 pre-push test-identity rejection.)

### Cross-cutting notes for other agents

- Global `core.hooksPath` (warden) now owns all hook enforcement; per-repo `.git/hooks`
  seeds (F4.11) are inert under it — any hook-behavior audit must read the global hooks dir.
- Two behavior classes changed silently-defaulted knobs: gc --prune=now (v0.113.0) and
  excluded-path preservation (v0.112.34) — both are opt-out/opt-in config-gated.
- The KiB/bytes unit bug (v0.112.42) means pack-size gating logic changed twice in 2 days —
  push-guard behavior at the 2 GiB boundary deserves a dedicated regression check.
