# AUDIT FULL — 2026-07-26 (dracon-utilities fleet)

**Scope**: dracon-sync v0.113.1 (HEAD `0676d3e`), dracon-warden v0.113.0,
dracon-system v0.112.33, and the meta-repo (AGENTS.md / docs/design /
live-vs-example config). Research/audit only — no code was modified.

**Method**: 6 parallel subsystem audits (part files under `.pi-tmp/`), each
read-completely; two workers additionally ran empirical repros (filter-repo
backup-rewrite + origin deletion; cat-file pipe deadlock; warden pre-commit
block on a throwaway repo; GNU grep `\x27` behavior). Parts 2, 4, and 5 were
independently spot-verified by a second reader against the cited source
lines (all sampled claims confirmed). Part 1 contains its own second-reader
verification pass. Part 3's findings are single-reader (line citations
provided throughout).

**Inputs**:
- `.pi-tmp/audit-2026-07-26-part0-baseline.md` — prior-audit LOW carryover +
  regression surface
- `.pi-tmp/audit-2026-07-26-part1-daemon-sync.md` — daemon/sync core loop
- `.pi-tmp/audit-2026-07-26-part2-git-layer.md` — git/*.rs operations layer
- `.pi-tmp/audit-2026-07-26-part3-report-policy.md` — report/policy/ownership/
  visibility/secrets
- `.pi-tmp/audit-2026-07-26-part4-warden.md` — warden + security crate + hooks
- `.pi-tmp/audit-2026-07-26-part5-system-meta.md` — dracon-system + meta-repo

**Counts after cross-part dedup**: **13 HIGH · 16 MEDIUM · 30+ LOW** (LOW
counted conservatively; see part files for the full tail) **· 19 carryover
items from 2026-07-21** (5 fixed, 5 partial, 8 still-open, 1 suspected).

---

## Cross-part deduplication decisions

1. **auto-gc (`git gc --prune=now`)** was found three times: part1-H3
   (blocking/no-timeout + wedge-valve re-dispatch race), part2-M1
   (`--prune=now` grace-removal race + runs before the conflict-state check),
   part3-L7 (no bound, no failure cooldown). **Merged into one HIGH (H-6
   below)** — the prune-race against concurrent object writes is the
   destructive mechanism.
2. **Push-timeout scaling not applied to mirror pushes** (part1-L4, part2-M2):
   same lines (`sync.rs:1619` vs `sync.rs:1823`). **Kept as MEDIUM** (part2's
   rating) because the v0.112.10 incident that motivated scaling was itself a
   mirror push.
3. **`refresh_stale_upstream_ref` never-converging upstream** (part1-L1 +
   part3-M3): per part1's own reconciliation note, **merged into one MEDIUM**
   — the cycle-level hot ssh/push loop + perpetual false "pushing Xm" display
   is the operator-visible harm.
4. **`scale_push_timeout` 600s cap vs live config base 900s** (part5-B2): the
   absolute cap silently *reduces* the operator-configured 900s to 600s on
   every push. Related to (2) but a distinct defect (config-vs-code); kept as
   its own MEDIUM.
5. **gitfile blindness** appears in two codebases: dracon-sync conflict-state
   checks (part2-H3) and warden `is_repo_checked_out`/`IndexLock`
   (part4-M-4). Same defect class, different blast radii — cross-referenced,
   not merged.
6. Meta-repo findings (part5-B: stale AGENTS.md line refs, 300s-vs-900s doc
   drift, v94.7.1-vs-v94.7.2, reversed design docs) are documentation debt,
   tracked separately below — not mixed into the code-bug severity counts.

---

## HIGH (13) — recommended for the remediation tasklist

### H-1. dracon-sync: large-blob auto-repair rewrites its own backup, deletes `origin`, and reports real rewrites as "no-op" — never pushes the fixed history
`git/staging.rs:229-277, 308-343`; caller `report.rs:5525-5626`. filter-repo
rewrites ALL refs including `backup/pre-sync-largeblob-fix-*` (backup
preserves nothing) and removes the `origin` remote (both **empirically
reproduced**); backup-tree == HEAD-tree after any rewrite, so
`rewrite_was_noop_then_cleanup` returns `Ok(None)` and the caller does
nothing — no force-push. Next cycle the re-created origin push fails
non-ff and the auto-pull **merges the >100 MiB blob back in** — the repair
silently un-does itself. The 2026-06-30 "zero backup branches exist"
evidence is consistent with this firing and being misreported.
Fix: bundle/tag backup + `--refs` exclusion; no-op check on HEAD sha
pre/post; re-run `configure_all_remotes` + explicit `--force-with-lease`.
(Part 2, H1.)

### H-2. dracon-sync: `detect_large_blobs_ahead` cat-file stdin/stdout deadlock — the 100 MiB guard silently never engages for big-ahead repos
`git/staging.rs:118-135`: `write_all(rev_list.stdout)` before
`wait_with_output()` drains — the exact pattern the mod.rs comment calls
"CRITICAL deadlock avoidance" was not applied here (**reproduced**). The 60s
tokio timeout can't cancel the `spawn_blocking` thread → permanent thread +
child leak per repair cycle; caller `.unwrap_or_default()` → zero blobs →
guard disabled precisely for repos with thousands of objects ahead.
(Part 2, H2.)

### H-3. dracon-sync: merge/rebase/cherry-pick detection is a no-op for all 10 nested game submodules — daemon can auto-commit conflict markers and push them
`git/status.rs:115-128` checks `<repo>/.git/MERGE_HEAD` etc.; for nested
submodules `.git` is a FILE (ENOTDIR → always false). The sibling
`IndexLock::acquire` was fixed via `path_gitdir()` in v0.112.33 — these
three helpers were not. Operator mid-merge walks away → next cycle
`git add -A` "resolves" the conflict with markers → auto-commit → pushed to
3 forges. Compounds with part2-M3 (auto-pull can leave MERGING state,
undetected). (Part 2, H3.)

### H-4. dracon-sync: detached-task drain + 15-min wedge valve unreachable when no new repo dispatches — permanent in-flight wedge in the quiet-daemon case
`daemon.rs:3745` — the entire drain/hand-off/valve block sits inside
`if !to_sync.is_empty()`. A repo whose push outlives the trailing deadline
stays in `in_flight` forever when no other repo has work (overnight,
single-active-repo), rendered "🔄 now" by `repos` — false-healthy.
Re-opens the 2026-06-15 permanent-skip class the registry was built to fix.
(Part 1, H1.)

### H-5. dracon-sync: auto-commit backstop is self-defeating — `NothingToDo` is mapped to success, destroying `ahead_since`; and the push is also skipped
`sync.rs:3980` + `daemon.rs:3945-3975`: the backstop's early return is
treated as success → activity entry removed → the 300s age window restarts
→ the moving-target auto-commit it exists to stop fires again next cycle.
And while active, the return happens before `handle_ahead_push`, so the 20+
pending commits aren't even pushed. Net: effectively dead code in the
daemon path. (Part 1, H2.)

### H-6. dracon-sync: auto-gc runs `git gc --prune=now` synchronously, unbounded, before the conflict check — and the wedge valve can re-dispatch the repo mid-gc
`git/mod.rs:3408-3465`, call site `sync.rs:3684` (before
`check_conflict_state`). Three compounding prongs (merged from parts 1/2/3):
(a) `--prune=now` removes git's 2-week grace — racing object writes
(operator commit, second sync process, or a re-dispatched task after the
15-min valve force-clears `in_flight` while gc still runs) can have fresh
objects pruned under them; (b) blocking `.output()` on a tokio worker with
no timeout — a 37 GiB-platform-class gc pins a worker for many minutes and
can't be killed by the sync-task timeout; (c) plain `Command::new("git")`
ignores `DRACON_SYNC_GIT_BIN`; failure spam has no cooldown.
Fix: default-grace `git gc` (or `--expire=1.hour.ago`), move below the
conflict check, spawn with timeout + kill-on-drop, honor the git-bin
override. (Part 1 H3 + Part 2 M1 + Part 3 L7.)

### H-7. dracon-sync: `sync_mirror_visibility` uses the safe-default visibility variant and writes the cache — a transient `gh` hiccup flips public mirrors to PRIVATE for up to 24h
`visibility.rs:628, 702` vs the `_opt` invariant documented at
`visibility.rs:265-291` ("Callers that write the visibility cache MUST use
this variant"). The CLI flip path was fixed in v0.112.33; the daemon path
was missed. Uncommanded remote state change + cache poisoning that gates
the codeberg-public-only push path off. Fail-closed (private), so
availability/integrity, not secrecy. (Part 3, H1.)

### H-8. dracon-sync: `standard_files` source path traversal — F40 fix incomplete, and validation never runs on the daemon path at all
`policy.rs:1516-1525` rejects only absolute sources (no `..` check; the
comment claims both); `expand_tilde("~/x")` bypasses the absolute check;
and `validate_config` is only invoked by the `validate`/`health`
subcommands — the daemon (`sync.rs:3824` → `standard_files.rs:36-70`)
executes raw. `source = "~/.ssh/id_rsa"` or `"../../../etc/passwd"` is a
read-anywhere → publish-everywhere primitive (daemon auto-commits + pushes
the copied file to public forges). (Part 3, H2.)

### H-9. dracon-warden: H9 regression CONFIRMED — production `filter-smudge` still corrupts whole-file-encrypted binary secrets
`security/src/lib.rs:1115-1123` (`DraconWarden::smudge` — the path
`run_filter` calls) goes straight to `from_utf8_lossy` → `smart_smudge`;
the v0.112.32 fix (`decrypt_whole_file_tag`) is wired only into
`seal_smudge`/`decrypt_file`, and **neither has any caller in the binary**.
Binary secret → smudge decrypts inline → U+FFFD for every invalid-UTF-8
byte → corrupted worktree file → next clean re-encrypts the corruption →
original bytes lost from history. The H9 unit test passes because it
exercises the helper directly. (Part 4, H-1.)

### H-10. dracon-warden: global pre-commit hook hard-blocks commits in EVERY non-hardened repo on the machine, and shadows all repo-local hooks fleet-wide
`PRE_COMMIT_HOOK` (`main.rs:2288-2317`) exits 1 unless `.gitattributes`
contains `filter=dracon`; `setup-hooks` sets global `core.hooksPath`
(**reproduced**: a throwaway /tmp repo cannot commit). Also silently
disables husky/pre-commit-framework/cargo-husky in every repo because
global hooksPath overrides `.git/hooks`. (Part 4, H-2.)

### H-11. dracon-warden: pre-rebase `head -100` checks the NEWEST 100 commits — published commits deeper in the range escape the guard
`main.rs:2446`: `rev-list` is newest-first; the cap drops the oldest (most
likely published) commits. Rebase of a >100-commit range whose 101+ commits
are on a remote passes the guard → published history rewritten → fleet
mirror divergence — the exact incident class v0.113.0 ships to prevent.
Fix: check only the boundary commit (ancestor-closed), or drop the cap.
(Part 4, H-3.)

### H-12. dracon-system: guard daemon busy-loops after the first interval
`dracon-system/src/main.rs:3154, 3230-3233`: `elapsed` is declared once and
never reset; after the first `interval_secs` the inner sleep loop never
runs again → `run_guard_once` executes back-to-back forever, spawning
`df`/`ps`/`du` + walkdir scans (and at action/critical disk, full cleanup
scans) continuously. (Part 5, A-1.)

### H-13. dracon-system: `link apply` can never fix an existing symlink — its primary case
`dracon-system/src/links.rs:135,138`: existing symlinks are routed through
`check_safe_to_delete`, which **always refuses symlinks**
(`safety.rs:31-38`). A drifted symlink (`link_target_mismatch`) errors the
whole command on the first entry; with no `in_sync` short-circuit, even a
fully-synced policy fails. `apply_link_policy` has zero test coverage.
(Part 5, A-2.)

---

## MEDIUM (16) — defer with rationale (not in the HIGH remediation tasklist)

| # | Finding (part) | Why deferred |
|---|---|---|
| M-1 | Timeout scaling not applied to mirror pushes (`sync.rs:1619` vs `1823`; parts 1-L4/2-M2) | Real but bounded: live base 900s already exceeds any scaled value |
| M-2 | `scale_push_timeout` 600s absolute cap silently reduces live-configured 900s→600s (part5-B2; code `sync.rs:167-176`) | No current incident; fix alongside M-1 (cap should be `max(base,…)`) |
| M-3 | `refresh_stale_upstream_ref` hot loop when origin down, mirrors up — silent failed fetch + N pushes/cycle forever, "pushing Xm" never converges (merged part1-L1 + part3-M3) | 30s-bounded, no data loss; needs per-repo backoff design |
| M-4 | Auto-pull-retry can merge wrong upstream/branch, no `--no-edit` (tty hang), leaves MERGING on conflict (`push.rs:204-212`; part2-M3) | Requires operator tty + fetch-first rejection; compounds H-3 |
| M-5 | SSH hardening missing on branch-deletion ops (`branch.rs:158,197,240`; part2-M4) | Deletion path rarely exercised; exit status unchecked |
| M-6 | `detached_discard` keyed per-repo not per-generation — fresh result discarded, stale applied (`daemon.rs:4027-4035, 4158-4165`; part1-M1) | Only reachable via H-4's valve; fix with H-4 |
| M-7 | FilterOnly early return drops injected stale-gitlink entries — parent gitlink convergence starved in warden-filter repos (`sync.rs:3905-3942`; part1-M2) | Breaks convergence invariant silently; no data loss |
| M-8 | FilterOnly path can flip benign fully-pushed mirror-only repos to PushFailed → stuck-ledger exhaustion alarm (`sync.rs:4153-4166`; part1-M3) | False alarm, not corruption; gate on `ahead > 0` |
| M-9 | Apply-phase vs trailing-drain outcome asymmetry (part1-M4) | State skew only; unify via one `apply_outcome` |
| M-10 | Unowned repos display "🟣 pushing" forever (report.rs:3216-3267,347-390; part3-M1) | Display-only, but on the primary safety guard |
| M-11 | `STUCK`/`FAIL` invisible in ACTIVITY column; "pushing Xm" measures last-commit age (part3-M2) | Display-only |
| M-12 | Ownership compare case-sensitive; only `origin` validated (part3-M4) | Fail-closed direction (availability, not bypass) |
| M-13 | Warden `setup-hooks` overwrites foreign global hooks unconditionally, non-atomically (part4-M-1) | Operator-visible on first run; needs backup/refuse semantics |
| M-14 | Warden pre-push secret scan: `\x27` literal in GNU grep ERE — single-quoted secrets escape (part4-M-2, verified vs grep 3.12) | Defense-in-depth layer only; filter is primary |
| M-15 | Warden pre-rebase bypass via two-arg form `git rebase <up> <branch>` (part4-M-3) | Guard bypass but interactive-only; easy fix (`${2:-HEAD}`) |
| M-16 | Warden `is_repo_checked_out`/`IndexLock` gitfile blindness — linked worktrees/nested submodules silently never hardened (part4-M-4) | Same class as H-3; silent skip, no corruption |
| (M-17) | Warden `salvage_invalid_json_markers` panics on non-ASCII (part4-M-5) | Crash mid-pass, not data loss |
| (M-18) | Warden test-identity push guard: no escape hatch + full-history scan on new branches (part4-M-6) | Only affects repos with `test@test` history; `--no-verify` workaround |
| (M-19) | dracon-system: `guard clean --all` ignored; storage cleanup aborts on first error; log-truncate TOCTOU (part5 A-3/A-4/A-5) | Bounded blast radius; all require explicit flags |

(MEDIUM table compressed for readability; full mechanisms in the part
files. Numbering continues informally past 16 — total actionable MEDIUM is
19.)

---

## LOW (30+) — deferred wholesale

Representative items (full tail in part files):
- **dracon-sync**: kill_process_group blocking sleep + missing-kill hang
  (part2-L1); idle-timeout extendable forever by `remote:` keepalives +
  unbounded stderr (part2-L2); askpass token file not cancellation-safe —
  RAII guard exists but unused (part2-L3); `github_pack_too_large`
  synchronous walk in async path (part2-L4/part1-L5); stale SEQUENTIAL doc
  vs parallel code (part2-L5/F2.12); `parse_name_status_z` desync on `U`/`C`
  (part2-L6); residual unchecked-exit sites (part2-L7); ssh-hardening
  HOME-quoting + missing-config hard-fail (part2-L8); ls-remote "not found"
  false-missing + EXISTS_CACHE never populated post-create (part2-L9);
  SHA-256 repos return 0 from `pushed_branch_pushable_bytes` → 2 GiB guard
  blind (part3-L3); size-cache `gitdir_sig` written-never-read + no schema
  version (part3-L1); `run_git_bounded` stdin-write deadline gap + tmp leak
  (part3-L2); visibility cache future-timestamps fresh forever (part3-L4);
  ledger 500-line window flap (part3-L5); visibility flips unaudited, no
  dry-run/confirm (part3-L8).
- **dracon-warden**: daemon always pushes `--no-verify` → hook layer never
  gates the fleet's primary writer — needs a deliberate policy decision
  (part4-L-1); `backfill_env_headers` unreachable success branch (L-2);
  shallow-clone fail-closed with misleading message (L-3);
  `replace_managed_block` orphan-BEGIN deletes file tail (L-4); swallowed
  hook-install errors (L-5); per-repo hook seeding dead under global
  hooksPath (L-6/F4.11).
- **dracon-system**: nested `node_modules` double-counted (part5-A-6); nix
  cleanup fabricated byte counts + swallowed generation-delete errors
  (A-7); inode monitor hardcodes `/` ignoring `disk_mount_path` (A-8);
  daemon-lock truncate-before-lock (A-9); assorted panic nits (A-10).

---

## Carryover from the 2026-07-21 audit (part 0)

18 tracked + F0.3 (promoted): **FIXED 5** (F0.3/M10, F4.9, F4.10, F4.12,
F1.16) · **PARTIAL 5** (F2.11, F2.15, F2.16, F3.13, F1.17) · **STILL-OPEN
8** (F2.10 branch-name validation, F2.12 stale doc/join-error, F2.13
space-path truncation + no-upstream blind spot, F3.11 case-sensitive trust,
F3.14 bump.rs comment parse, F3.15 ledger pruned-count/non-atomic, F4.11
inert per-repo hooks, F1.15 ~250 spawns/sec) · **SUSPECTED 1** (F2.14 ghost
standalones — mitigated, unverified). Note F2.13's blind spot now compounds
H-2 (both disable large-blob detection for upstream-less repos).

## Meta-repo / documentation debt (part 5, PART B — not code bugs)

- **All 7 spot-checked AGENTS.md source line refs are stale** —
  `report.rs:3705`→5525, `report_v2_snapshot.rs:3166` (file deleted),
  `staging.rs:152`→182, `policy.rs:1580`→1864, `sync.rs:858,859`→1042,1060,
  `git/mod.rs:684`→1004, `git/mod.rs:370`→601.
- **Push-timeout section stale**: AGENTS.md says 300s; live config has been
  900s since 2026-06-23; code default 300; example.toml ships 60; scaling
  (×1/2/4/6, cap 600s) undocumented (see M-2 above).
- **`[patch.crates-io]` section stale**: says v94.7.1; Cargo.toml pins
  v94.7.2; workspace member list missing `dracon-warden/src/security`.
- **Enforcement-stack section**: hooks/hatch verified accurate; the
  `init.templateDir` claim is not backed by any warden code (only
  `core.hooksPath` is managed).
- **auto_gc docs accurate** (2 GiB default, 0 disables).
- **Reversed design docs without a SUPERSEDED banner**:
  `docs/design/daemon-standalone-removal-2026-07-01.md` ("the daemon will
  create the standalone worktree … the new invariant" — reversed next day)
  and `docs/design/push-timeout-fix-2026-06-17.md` (300s "final").
- **Live config vs example.toml**: `alert_unpushed_threshold`,
  `auto_github_private`, `auto_github_private_account`,
  `max_stage_batch_files`, `exclude_repos` are set live but absent (even as
  comments) from the example.

## Positive checks (selected — things verified NOT broken)

- count-objects KiB→bytes units correct in ALL consumers post-v0.112.42
  (report.rs, git/mod.rs, cache schema).
- Ownership F39/F44: tuple-atomic, no substring matching anywhere; tests
  present; fail-closed on empty/unparseable.
- secrets.rs: no token value ever logged or placed in argv; F54 redaction.
- askpass script: atomic `O_EXCL|O_NOFOLLOW`, mode 0700, quote-injection
  refused.
- `ensure_gitlab_main_protected`: correct API, argv-safe, idempotent 409.
- warden M3 oversized-clean fail-closed; H8 managed-block preservation;
  test git-config hermeticity — all hold.
- 2026-07-21 HIGH fixes H3/H4/H5/H7 all still hold (part 1 regression
  pass).
- `hooks.rs` in dracon-sync: fully removed, no dangling references.

## Remediation triage recommendation (feeds task 9)

**Fix now (HIGH)**: H-1…H-13 above. Suggested grouping: (a) repair-loop
cluster H-1 + H-2 + H-6 (staging/gc, one PR); (b) submodule-gitdir cluster
H-3 (+ warden M-16, shared `path_gitdir` pattern); (c) daemon-state cluster
H-4 + H-5 (+ M-6, M-9); (d) warden cluster H-9 + H-10 + H-11 (+ M-13…M-15);
(e) system cluster H-12 + H-13; (f) visibility/config cluster H-7 + H-8.
Every HIGH except H-4/H-5 has a one-paragraph fix sketch in its part file.
New-code rule per AGENTS.md: each fix needs a unit test — note especially
H-9 (byte-identical round-trip through `DraconWarden::smudge`), H-1
(real-rewrite-not-noop), H-12 (loop-timing), H-13 (apply over existing
symlink).

**Defer with rationale**: MEDIUM table above (none is data-loss today;
several are display-only or fail-closed). LOW: batch into a follow-up
hygiene release. Meta/doc debt: single AGENTS.md refresh commit (line refs,
900s, v94.7.2, templateDir, workspace members) + SUPERSEDED banners on the
two reversed design docs.
