# Audit 2026-07-26 Part 3 — dracon-sync reporting + policy surface

Scope: `dracon-sync/src/{report,policy,ownership,exclude,visibility,secrets,role,nix}.rs` (+ `sync.rs` /
`git/mod.rs` where they consume the audited surface). Version under audit: v0.113.1 (HEAD `0676d3e`).
Read-only audit; no code modified.

`hooks.rs`: **verified fully removed.** No file in `src/`, no `mod hooks` in `main.rs` (modules listed at
`main.rs:5-23`), no Cargo references. All remaining "hooks" mentions are correct references to
warden's `core.hooksPath` management (test hermeticity + `sync.rs:3681` comment). Only doc-rot: the
meta-repo `AGENTS.md` forbidden-actions section still cites `report_v2_snapshot.rs:3166` (file merged
into `report.rs`); see L6.

---

## HIGH

### H1 — `sync_mirror_visibility` violates the `get_github_visibility_opt` cache-poison invariant
- **Evidence**: `visibility.rs:254-256` (`get_github_visibility` = `_opt(...).unwrap_or(true)`),
  `visibility.rs:265-291` (`_opt` doc: *"Callers that write the visibility cache MUST use this variant:
  the safe-default path poisons the cache to 'private' on any transient `gh` hiccup"*),
  `visibility.rs:628` (`let github_private = get_github_visibility(&owner, &gh_repo);` — the bool
  variant), `visibility.rs:702` (`update_visibility_cache(repo_path, github_private);` — unconditional
  cache write "even on partial failures").
- **Mechanism**: On any transient `gh` failure (network, auth expiry, rate limit) the daemon-side
  visibility sync (a) drives `github_private = true` (safe default) into `set_gitlab_visibility` /
  `set_codeberg_visibility` — i.e. it **flips public mirrors to private without an operator command** —
  and (b) writes "private" into the visibility cache for up to `interval_hours` (24h), which gates the
  codeberg-public-only push path off. This is exactly the M25/F3.7 bug class the `_opt` variant was
  added (v0.112.33) to prevent; the CLI flip path (`main.rs:520-534`) was fixed to use `_opt` but the
  daemon path was missed. Fail direction is closed (private), so this is an availability/integrity bug,
  not a secrecy leak — but it is an uncommanded, unaudited remote state change.
- **Suggested fix**: Use `get_github_visibility_opt` in `sync_mirror_visibility`; on `None`, skip both
  the mirror flips AND the cache write (return early). Add a regression test mirroring
  `test_cache_written_on_parseable_origin_even_when_tokens_missing` for the gh-failure case.

### H2 — `standard_files` source path traversal: F40 fix incomplete + validation never enforced at runtime
- **Evidence**: `policy.rs:1516-1525` — validate rejects only `source_path.is_absolute()`; the comment
  says *"(not absolute, not `..`)"* but no `ParentDir` check exists for source (target gets the full
  check at 1498-1514). `policy.rs:80-88` — `StandardFileConfig::source_path()` calls `expand_tilde`;
  `policy.rs:91-104` — `expand_tilde("~/x")` → `$HOME/x` (absolute). `main.rs:805,1141` —
  `validate_config` is invoked only by the `validate` and `health` subcommands; the daemon's execution
  path (`sync.rs:3824` → `standard_files::ensure_standard_files`) loads `SyncPolicy` raw and copies
  `source_path` → `repo.join(target)` with no validation (`standard_files.rs:36-70`).
- **Mechanism**: Two independent gaps compound: (1) validation gap — `source = "../../../etc/passwd"`
  or `source = "~/.ssh/id_rsa"` (raw string `"~/..."` is NOT `Path::is_absolute`, so it passes
  validation) escape the sync base dir; (2) enforcement gap — even a value `validate_config` *would*
  reject still executes in the daemon, because nothing calls `validate_config` on the daemon's load
  path. The copied file lands in every watched repo and is then auto-committed + auto-pushed to public
  forges by the daemon's commit-all policy. A config typo or a write to the policy file is a
  read-anywhere → publish-everywhere primitive under the daemon's UID.
- **Suggested fix**: (a) reject `ParentDir`/`RootDir`/`Prefix` components in source the same way target
  is checked; (b) resolve tildes *before* validation (validate the expanded path); (c) enforce at the
  point of use — have `ensure_standard_files` itself skip (and log) any source/target that fails the
  component check, so validation is not dependent on the operator running `dracon-sync validate`.

---

## MEDIUM

### M1 — Unowned repos display "🟣 pushing Xm (N ahead)" forever (false-active)
- **Evidence**: `report.rs:3216-3267` — `push_status` is computed from flags/ahead *before* the
  ownership override (`report.rs:3332-3344`), and the override rewrites only `state_cause`, never
  `push_status`. `report.rs:347-360` — `activity_label` returns `🟣 pushing...` for
  `push_status == "PENDING"` *before* the `StateCause::Unowned` arm at `report.rs:383-390`.
  `report.rs:1734-1740` — `repo_is_active` returns true for `"PENDING"` regardless of `Unowned`.
- **Mechanism**: An unowned repo (`auto_skip_unowned`, e.g. HEAD author not yet whitelisted — the exact
  2026-07-25 ai-auto-writer/browser-extensions-shared scenario) with origin+upstream and ahead>0 gets
  `push_status=PENDING`, so the ACTIVITY column shows "pushing" and the row counts as ACTIVE — while
  the daemon intentionally never touches it. The 🚫 unowned label is unreachable in ACTIVITY; only the
  HINT reveals the truth. This is a false-healthy display on the daemon's primary safety guard.
- **Suggested fix**: Check `StateCause::Unowned` first in `activity_label`; exclude `Unowned` in
  `repo_is_active`; optionally force `push_status = "SKIPPED"` when the ownership override fires.

### M2 — `push_status == "STUCK"` / `"FAIL"` invisible in ACTIVITY; "pushing Xm" label lies about duration
- **Evidence**: `report.rs:347-378` — `activity_label` handles `"PENDING"` and `"PUSH_STUCK"` but not
  `"STUCK"` or `"FAIL"` (set at `report.rs:3224-3228` from the `STUCK_PUSH` flag). The PUSH cell
  renderer *does* handle them (`report.rs:4562-4563`), so the two columns contradict.
- **Mechanism**: A clean repo with a recorded recent push failure (`STUCK_PUSH` flag →
  `push_status="STUCK"`) falls through to the clean-repo arms and shows "🟢 synced Xm" / "⚪ idle Xm"
  in ACTIVITY while the PUSH cell shows 🛑 STUCK — false-healthy. Separately (the known cosmetic
  issue): the doc comment at `report.rs:310-312` claims "pushing Xm" = push-in-progress duration, but
  Xm is parsed from `row.last_when` (last-commit age, `report.rs:350-354`). For a repo whose push
  silently never converges (see M3), the label grows unboundedly ("pushing 4000m") while implying an
  active push. The label should change: e.g. `⏳ unpushed N (last commit Xm)` — Xm should be anchored
  to when the ahead state began (daemon activity map), not last-commit age.
- **Suggested fix**: Add `"STUCK" | "FAIL"` arms to `activity_label` (🛑, same as PUSH_STUCK); reword
  the PENDING label and fix the doc comment.

### M3 — v0.113.1 `refresh_stale_upstream_ref`: unbounded re-push hot loop when origin is down but mirrors are up
- **Evidence**: `sync.rs:4102-4146` — the refresh fetch is fire-and-forget (`let _ = ...`,
  `sync.rs:4140-4145`), failure is silent and unrecorded. `sync.rs:4176-4180` — refresh runs after
  every *successful* push. `sync.rs:4160-4173` — `should_push = ahead > 0` re-fires every cycle while
  the upstream ref stays stale.
- **Mechanism**: The daemon pushes to NAMED mirror remotes; if the branch's configured upstream remote
  (`branch.<name>.remote`, typically `origin`) is unreachable while the mirrors are up, the mirror
  pushes keep succeeding (so no failure budget accrues, no STUCK classification, no backoff) and the
  convergence fetch to the dead origin fails silently every cycle — a 30s-timeout fetch plus N ssh
  push handshakes per repo per cycle, forever, and `↑N` / "pushing Xm" displayed forever (M2's label
  makes this look like progress). The v0.113.1 fix converges the display only when the upstream remote
  is reachable; the down-origin case has no bound.
- **Suggested fix**: Record refresh failures to the incident ledger and apply a per-repo cooldown
  (e.g. reuse `stage_cooldown_secs` pattern) before re-attempting the fetch; and/or when `origin`'s
  URL equals a successfully-pushed mirror's URL, update `refs/remotes/origin/<branch>` locally from
  the mirror's tracking ref instead of fetching (zero-network convergence). Surface "upstream
  unreachable" in the report instead of infinite PENDING.

### M4 — ownership trust compare is case-sensitive; only `origin` is validated
- **Evidence**: `ownership.rs:295-302` — `th == host && to == owner` exact string equality.
  `ownership.rs:263-271` — only `inputs.origin_url` is checked against `trusted_remote_hosts`;
  `ownership.rs:524-530` (`git_origin_url` reads only `remote get-url origin`).
- **Mechanism**: (a) DNS hostnames and forge usernames are case-insensitive, but the tuple compare is
  case-sensitive: `git@gitlab.com:DraconDev/repo.git` vs trusted `gitlab.com/dracondev` →
  `untrusted_origin` → repo flipped to 🚫 unowned and silently dropped from sync. The 2026-07-02 fix
  for this class added uppercase entries to `default_trusted_remote_hosts` rather than normalizing
  case — any new casing variant (a fresh clone via a differently-cased URL) re-triggers it. This is a
  fail-closed (availability) bug, not a bypass — the F39 tuple-atomicity itself is correct, no
  substring matching remains anywhere in trust evaluation (verified `classify`/`is_trusted_origin`/
  `parse_origin`). (b) The guard validates only `origin`, but the daemon pushes to the named mirror
  remotes from `policy.remotes`; a repo-local `github`/`gitlab` remote URL pointing at attacker infra
  is never ownership-checked (the daemon does rewrite these from policy in the normal flow, so the
  exposure is a repo whose local config was edited out-of-band).
- **Suggested fix**: Compare host and owner with `eq_ignore_ascii_case` (add a regression test with
  mixed-case URLs). Consider validating the effective push remote URLs (post-`policy.remotes`
  resolution) in the ownership pass, not just `origin`.

---

## LOW

### L1 — Size cache: `gitdir_sig` written but never read; no schema version; stale TTL comments
- **Evidence**: `report.rs:2976` computes `gitdir_signature(&repo)` (a stat syscall per repo) and
  stores it (`report.rs:3020`), but the lookup arm (`report.rs:2984-2990`) checks only
  `cached_at_secs` freshness + `missing_objects.is_some()` — `gitdir_sig` is never consulted anywhere
  on the read path (grep confirms: only writes/tests). Comments at `report.rs:2589`, `2611-2613`,
  `2616-2619`, `2971-2974` still claim "a mismatch forces recomputation". TTL comments at
  `report.rs:2593-2600` ("30s"), `2654` ("24h size-cache TTL"), `2854` ("= 30s"), `2969` are stale
  post-v0.112.42 (TTL is 3600s).
- **Mechanism**: The documented mtime-invalidation design is not implemented — the cache is TTL-only.
  Consequences are benign (sizes drift slowly; the push path measures fresh at `sync.rs:1638` with
  `precomputed_size=None`), but a repo crossing the 2 GiB guard mid-TTL shows a stale/absent
  PACK_SIZE_WARNING for up to 1h. No schema/version field on `CachedRepoSize`: the v0.112.42 KiB→bytes
  unit fix relied on manual cache deletion on the deployed machine; on any other machine the old
  wrong-unit entries were served for up to TTL (self-healing, but silent). Future schema/unit changes
  have no invalidation lever other than "wait an hour" or manual `rm`.
- **Suggested fix**: Either reinstate the sig check for entries older than a short grace window, or
  delete the dead field/comments; add a `schema_version: u32` to the cache file and discard mismatches
  on load (cheap insurance for the next unit fix).

### L2 — `run_git_bounded`: deadline not enforced during stdin write; tmp leak on spawn failure
- **Evidence**: `report.rs:2683-2690` — `stdin.write_all(stdin_data)` runs *before* the deadline loop;
  a child that stops reading (hangs before consuming stdin) blocks `write_all` indefinitely and the
  4s bound never fires. `report.rs:2662-2672` — tmp file is created before `spawn()`; on spawn failure
  `.ok()?` returns with the file leaked (the `TmpCleanup` guard is installed after spawn).
- **Suggested fix**: Write stdin from a thread (pattern already used in
  `pushed_branch_pushable_bytes`, `git/mod.rs:118-125`) or poll-write with the deadline; move the
  guard creation above the spawn.

### L3 — `pushed_branch_pushable_bytes` silently returns 0 for SHA-256 repos, bypassing the 2 GiB guard
- **Evidence**: `git/mod.rs:88-95` — SHA collection filters `sha.len() == 40`; a SHA-256 object-format
  repo yields an empty list → `shas.is_empty()` → `return 0` → `github_pack_too_large` computes
  `(0 >= LIMIT, 0)` = not-too-big for any repo size (`git/mod.rs:58-66`).
- **Mechanism**: The report's PACK_SIZE_WARNING and the GitHub-skip guard both go blind on SHA-256
  repos. Not exploitable today (fleet is SHA-1) but a silent failure mode rather than the conservative
  `u64::MAX` used for every other error path in the same function.
- **Suggested fix**: Accept 40- or 64-hex SHAs (or use `cat-file --batch-check` with object names from
  `rev-list` unfiltered), and return `u64::MAX` (conservative) when the list is empty but the branch
  has commits.

### L4 — Visibility cache freshness: future timestamps are "fresh" effectively forever
- **Evidence**: `visibility.rs:122-128` — `now.saturating_sub(ts) < interval_secs`; a cache file with
  a future `ts` (clock skew, restored backup, manual edit) saturates to 0 and stays fresh until wall
  clock catches up — the daemon will not re-query GitHub for the whole skew duration.
- **Suggested fix**: Treat `ts > now + small_grace` as stale.

### L5 — Push-failure map window can flap under ledger churn
- **Evidence**: `report.rs:1302-1304` — last-500-lines window AND 10-minute cutoff. With 35+ repos and
  chatty ledger entries, >500 lines can accrue inside 10 minutes, evicting in-window push failures →
  `STUCK_PUSH` flag / CONCERN classification flaps off while the failure is still recent.
- **Suggested fix**: Read until the oldest line's ts is below the cutoff (bounded loop), or raise the
  window and document the interaction.

### L6 — Doc-rot / dead references
- Meta-repo `AGENTS.md` cites `report_v2_snapshot.rs:3166` and `report.rs:3705` line numbers; the
  former file no longer exists (merged). `report.rs:2593`, `2612`, `2654`, `2854`, `2969` stale TTL
  numbers (see L1). `report.rs:310-312` false "push has been in progress" doc (see M2).
- `secrets.rs:186-197` — `warn_if_world_readable` checks `mode & 0o044` (group OR other read) but the
  message says only "world-readable"; either the check or the message is wrong.
- `report.rs:3361-3363` — `delta < 1 → "1s"` displays "1s" for same-second (and, via
  `saturating_sub`, future) ledger timestamps; harmless but wrong-looking in the DAEMON column.

### L7 — `maybe_auto_gc` runs `git gc --prune=now` synchronously with no timeout in the sync path
- **Evidence**: `git/mod.rs:3431-3435` — `.output()` with no bound; called from
  `sync.rs:3684` on every non-dry-run `sync_repo`. Units (KiB→bytes) and the 0-disable semantics are
  correct (`git/mod.rs:3388-3395, 3409`), and serde backcompat is fine (`policy.rs:518-519` +
  `default_auto_gc_garbage_threshold_bytes`), but a gc on a multi-GiB repo can run for many minutes
  inside the repo's sync cycle. Also note `gc` failure → `eprintln` every cycle with no cooldown
  (ledger-spam class the codebase usually guards against).
- **Suggested fix**: Run under `repo_sync_timeout_secs` (or a dedicated bound), and record a ledger
  entry with cooldown on gc failure.

### L8 — Visibility flips have no audit-ledger record and no confirmation/dry-run
- **Evidence**: `main.rs:250-271` (MakePublic/MakePrivate have no `--dry-run` / `--yes`),
  `main.rs:469-540` (executes immediately, prints to stdout), `visibility.rs:507-602` — no call to
  `record_sync_alert` / incident ledger anywhere on the flip path. `dry_run` *is* respected on the
  daemon-side sync (`sync.rs:347`), so the dry-run question is clean everywhere except that the flip
  CLI has no dry-run mode at all.
- **Mechanism**: `make-public` is a one-way secrecy-relevant operation; a typo'd repo name flips a
  private repo public with no confirmation and leaves no trace in the incident ledger that the
  operator audits.
- **Suggested fix**: Write an incident-ledger entry (scope "visibility", old→new state, per-remote
  results) on every flip; add a `--yes` requirement or `--dry-run` default for `make-public`.

---

## Things checked and found CLEAN

- **count-objects units (v0.112.42 regression sweep)**: all consumers now correct —
  `measure_git_size_via_count_objects` multiplies by 1024 (`report.rs:712-717`),
  `parse_count_objects_garbage_bytes` multiplies by 1024 (`git/mod.rs:3388-3394`),
  `github_pack_too_large` LIMIT and all size comparisons are in bytes (`git/mod.rs:44-66`), cache
  stores bytes. Cache read/write schema agree (serde round-trip; `#[serde(default)]` on new fields
  gives graceful upgrade).
- **Ownership F39/F44**: no substring matching remains; tuple-atomic `(host, owner)` compare; F44
  asymmetric-trust flags either-signal-untrusted with explicit asymmetry detail; regression tests
  present (`ownership.rs:691-730`). Empty-string/empty-list trusted entries fail closed; unparseable
  URLs fail closed. (Case-sensitivity is M4 above.)
- **F40 target-side validation**: absolute/`..`/root/prefix rejection for `standard_files[].target`
  is correct with tests (`policy.rs:2872-2907`). (Source side is H2.)
- **`auto_gc_garbage_threshold_bytes`**: serde-defaulted, old configs load, 0 disables, default 2 GiB,
  units correct.
- **secrets.rs**: no token value is ever logged — warnings carry env-name/path only; tokens passed to
  curl via stdin (`-H @-`), never argv; control-char refusal on both env and file paths; group/world-
  writable dir refusal; F54 credential redaction in ownership detail (`ownership.rs:328-361`).
- **hooks.rs**: fully removed, no dangling code references (see header note).
- **probe_missing_objects**: path-strip before `cat-file` present, both subprocesses bounded,
  timeout→0 (fail-safe against false BROKEN_HISTORY), counts only `<sha> missing` lines.
- **visibility dry_run**: daemon-side `maybe_sync_visibility_and_metadata` respects `ctx.dry_run`
  (`sync.rs:347`). (CLI flips have no dry-run at all — L8.)
