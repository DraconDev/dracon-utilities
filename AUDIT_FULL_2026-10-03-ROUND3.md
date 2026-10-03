# Full audit — ROUND 3 (2026-10-03)

Method: read-only source inspection of every cited body across six area
reports, plus falsification critic re-verification (live /tmp probes only,
no repo writes) and locked/offline `cargo test` subsets executed by the
researchers. Baselines: `AUDIT_FULL_2026-10-02-ROUND2.md`,
`AUDIT_FIXES_2026-10-02.md`, `AGENTS.md` contracts.
Date: 2026-10-03.
Areas merged: sync-git, sync-daemon, warden, system, report-display, critic.

Result: 43 new findings — 1 HIGH, 3 MEDIUM, 39 LOW (after critic
adjudication and 1 dedupe). All ROUND2 regressions verified FIXED
(H1–H2, M1–M12, L1–L19; M7/L11/L16 by documented alternative).

Severity: HIGH = broken install/startup or fleet-wide outage class;
MEDIUM = wrong behavior with operator/security impact under plausible
conditions; LOW = narrow/cosmetic/defense-in-depth.
Critic verdicts: 4 CONFIRMED, 1 CONFIRMED-mechanism with MEDIUM→LOW
challenge (visibility), 1 REFUTED-as-MEDIUM surviving as LOW (tmp probe).

Dedupes applied (cross-refs kept):
- D1: sync-git N2 ≡ sync-daemon N5 (sync.rs:5651 L2 twin) → R3-L01.
- D2: M5 reported by sync-git (inspect-only) and system (tested) → one
  Checked-clean entry (system test evidence wins).
- D3: L1/L2 reported by both sync-git and sync-daemon → one entry each.

---

## HIGH (1)

### R3-H1 — Nix watchdog script provisioning points at absent flake sources
- Location: `flake.nix:481-492` (`${self}/dracon-sync/scripts/...` ×2,
  `${self}/dracon-system/scripts/...` ×1).
- Severity: HIGH. Critic CONFIRMED (parent `git ls-files`=0,
  `check-ignore` proves git-filtered `self` lacks scripts; checker uses
  git+file eval yet asserts only `.executable`; ci.yml documents the same
  incident class).
- Why: `dracon-sync/|system/|warden/` are gitignored, so flake `self`
  (git-filtered tree, cf. D1 comment flake.nix:66-74 + A7 test) contains
  no utility sources. A Nix/HM install fails at generation build on the
  missing `home.file` source (whole `home-manager switch` fails, not just
  dracon). Invisible to CI: check-flake.sh:127-129 asserts only
  `.executable`, never `.source`; `nix build .#default` never realizes
  home.file.
- Fix-direction: source from pinned inputs
  (`${draconSyncSrc}/scripts/...`, `${draconSystemSrc}/scripts/...`;
  nested repos track the scripts) and assert `.source` existence in
  check-flake.sh. Proof by inspection + A7 test + git absence only;
  `nix eval` / HM build not executed (see Unresolved).

---

## MEDIUM (3)

### R3-M1 — Bootstrap sweep measures worktree, not staged blob (M4 bypass on root commit)
- Location: `dracon-sync/src/sync.rs:4784-4803` →
  `dracon-sync/src/exclude.rs:1538-1548` (`should_stage_entry`).
- Severity: MEDIUM. Critic CONFIRMED (worktree stat; M4 fix only in
  `unstage_oversized_paths` which needs `git reset`, fails unborn; root
  commit pushed with no push-time size block — `detect_large_blobs_ahead`
  is report-side only).
- Why: the bootstrap/root-commit hygiene sweep exists to keep
  operator-pre-staged oversized content out of the initial commit, but
  decides via worktree `metadata`. Routine bypass: `git add` 300 MiB,
  then truncate the worktree copy without re-adding → sweep passes →
  root commit contains the >max blob → pushed to every mirror. M4 impact
  confined to bootstraps.
- Fix-direction: measure staged blobs in the bootstrap sweep (reuse the
  `ls-files -s` + `cat-file --batch-check` primitive from staging.rs;
  keep `git rm --cached` for the unborn-branch unstage).

### R3-M2 — `auto_commit_exclude_patterns` has three inconsistent merge semantics
- Location: worker REPLACE `dracon-sync/src/sync.rs:4309,5199,5453,4705`
  vs dispatch-gate GLOBAL-ONLY `daemon.rs:8025,8088` vs stale-dirty-alert
  UNION `daemon.rs:8160-8163`.
- Severity: MEDIUM. Critic CONFIRMED (all four worker sites `unwrap_or`;
  gate ignores in-scope override; alert explicitly extends; AGENTS.md:200
  "extend" contradicts AGENTS.md:770 `unwrap_or`; global default empty).
- Why: per-repo set silently drops the global list at commit time →
  globally-excluded files auto-committed and pushed (needs both set).
  Gate ignores override entirely → per-repo-only config (the documented
  shape) dispatches every cycle while the worker stages nothing (churn);
  narrower-per-repo can starve commits the worker would make. L3 tripwire
  passes (reference check) — merge-correctness gap.
- Fix-direction: pick extend (matches exclude-list docs + alert site),
  unify all four consumers + docs, add behavioral test.

### R3-M3 — Pre-commit `.gitattributes` check matches commented-out filter lines (primary-gate bypass)
- Location: `dracon-warden/src/main.rs:5068` (MANAGED probe) and `:5152`
  (enforcement probe): `grep -q "filter=dracon"`.
- Severity: MEDIUM. Critic CONFIRMED (both probes substring-grep;
  independent /tmp probe corroborated all 3 arms; no content scan in
  pre-commit; pre-push leaves Tier-2 non-assignment; window = daemon
  process lifetime).
- Why: git ignores `#` comments but the grep matches them, so committing
  `.gitattributes` with filter lines commented out passes (proven exit 0
  live) while disabling the clean filter for all later commits. Pre-push
  still blocks Tier-1 + quoted assignments; residual leak is Tier-2
  non-assignment secrets (Age keys, session strings, unquoted
  secret=/api_key=) committed AND pushed in plaintext. Window: up to a
  daemon-process lifetime on daemon repos (harden `once` per process),
  indefinite on manual-only repos.
- Fix-direction: validate with `git check-attr filter -- <probe>` (or
  grep excluding `^[[:space:]]*#`) at both probes.

---

## LOW (39)

### R3-L01 — L2 twin: detached-ahead fallback `unwrap_or(0)` in push gate [D1]
- Location: `dracon-sync/src/sync.rs:5651` (sync-git N2 ≡ sync-daemon N5).
- Severity: LOW. Narrow: fixed backstop site `:5078-5080` fails the cycle
  first under the same condition; needs a transient failure between calls.
- Why: count error → 0-ahead → `should_push=false` → `Attempted{ok:true}`
  → `NothingToDo` → daemon Success clears the stuck ledger: silent skip
  with zero failure accounting (L2 shape, detached HEAD only).
- Fix-direction: propagate like 5078, or `record_push_attempt_error` +
  explicit skip.

### R3-L02 — Origin path `push_with_retries` exceeds its documented budget
- Location: `dracon-sync/src/git/push.rs:391` vs docstring `:236-237`.
- Severity: LOW.
- Why: after the loop exhausts `attempts`, line 391 runs another SSH push
  plus the full per-forge HTTPS chain (retries=0 → up to 5 spawns vs
  mirror path's pinned 1). Each docstring claims L5 unification; only the
  mirror path was changed.
- Fix-direction: thread remaining budget into the fallback call, or
  correct the docstring and name the extra attempt.

### R3-L03 — `push_with_retries` discards the transport-fallback error
- Location: `dracon-sync/src/git/push.rs:391-394`.
- Severity: LOW.
- Why: `if let Ok(())` throws away the fallback error (post-M2/M3 detail
  + chained SSH cause) and returns stale loop `last_err`; ledger shows an
  earlier SSH error, never the fallback verdicts.
- Fix-direction: return the fallback error (it already chains the SSH
  cause) or join both.

### R3-L04 — `get_remote_url` failure discards the SSH cause
- Location: `dracon-sync/src/git/multi_remote.rs:681-682`.
- Severity: LOW. Handling correct (lands transport-class, retries);
  message impoverished.
- Why: `.ok_or_else(|| anyhow!("remote {} not found"))?` returns without
  chaining `ssh_msg` in the narrow remote-vanishes-between-attempt-and-
  lookup race. Verified not to mis-fire forge eviction/permanent class.
- Fix-direction: chain `ssh_msg` as at `:703-710`.

### R3-L05 — Absent forge tokens silently skip HTTPS fallback legs
- Location: `dracon-sync/src/git/push.rs:77,109`.
- Severity: LOW.
- Why: `if let Some(token)` with no `else`: missing/permission-broken
  token file yields generic "all HTTPS push attempts failed" with zero
  hint the leg was skipped; operator chases transport instead of secrets.
- Fix-direction: push a `"gitlab: no token configured (skipped)"`-style
  entry (no token material).

### R3-L06 — Push-path redactor is https-only; raw journal prints bypass it
- Location: `dracon-sync/src/git/push.rs:12-27`; raw prints `:357-361`.
- Severity: LOW. Persisted ledger NOT exposed (writes use the all-scheme
  `ownership::redact_url_credentials`); journal-only surface.
- Why: `redact_credentials_for_log` matches only literal `https://`, so
  `http://user:pass@...` userinfo leaks in git error echoes; two
  divergent redactors (narrower one in push paths).
- Fix-direction: use the all-scheme redactor in push paths; redact the
  raw journal prints.

### R3-L07 — `strip_url_credentials` strips to bare `@` in path
- Location: `dracon-sync/src/git/urls.rs:24` (approx).
- Severity: LOW.
- Why: over-strips credential-looking `@` segments in the URL path,
  mangling display/log labels.
- Fix-direction: strip userinfo only (before host), not path segments.

### R3-L08 — Askpass pre-remove weakens O_EXCL
- Location: `dracon-sync/src/git/ops.rs:563` (approx).
- Severity: LOW.
- Why: pre-removing the askpass socket path before exclusive create
  reopens a narrow symlink/race window the O_EXCL contract otherwise
  closes.
- Fix-direction: create-exclusive first; remove only on NotFound-safe
  retry paths.

### R3-L09 — `--json` summary counts double-count warn+concern rows
- Location: `dracon-sync/src/report.rs:4764-4769` (header `:4697-4699`).
- Severity: LOW. Rows correct; summary integers lie
  (`ok+active+warn+concern > repos`).
- Why: `warn_count` excludes active but not concern; two production paths
  create warn&&concern (warn forced after concern at `:4270-4276`;
  concern forced after `warn=!concern&&dirty` at `:4165` vs `:4217`).
- Fix-direction: compute warn after all concern forcing, or exclude
  concern from warn_count.

### R3-L10 — JSON shape inconsistencies
- Location: `dracon-sync/src/report.rs:1684` (`PublishState` PascalCase vs
  `StateCause` snake_case `:2480`; Unowned object `:2501` vs string
  others; `"-"`/`"none"` sentinels `:4450,:4541`).
- Severity: LOW.
- Why: consumers must special-case three conventions; mitigated by
  `state_cause_label`.
- Fix-direction: `skip_serializing_if` + nulls (breaking — version or
  document).

### R3-L11 — Stuck-ledger lost-update races (no locking)
- Location: `dracon-sync/src/daemon.rs:5064-5157` (RMW),
  `:840-858` (apply-phase full-map save), reload `:6693`.
- Severity: LOW. Effect bounded: counters off by small N, Exhausted
  delayed not defeated.
- Why: unlocked load-modify-save across workers (up to
  `sem_max_concurrent_sync`); apply-phase cycle-start snapshot can
  clobber a same-cycle worker increment.
- Fix-direction: in-process mutex around ledger RMW +
  reload-before-save in apply phase.

### R3-L12 — `FilterOnly→Success` resets failure accounting but keeps stuck ledger
- Location: `dracon-sync/src/daemon.rs:862-875` vs `:840,857` and enum
  doc `:782-786`.
- Severity: LOW.
- Why: returns Success (resets `failure_count`, clears cooldown, drops
  activity incl. per-remote pause memory) without the stuck-ledger
  remove Synced/NothingToDo do; alternating push-fail/filter-only cycles
  defeat backoff.
- Fix-direction: clear the ledger too, or map FilterOnly to a
  retain-activity outcome.

### R3-L13 — Push cancellation counted as failure despite "unknown, not failed"
- Location: `dracon-sync/src/sync.rs:5784-5802` →
  `Attempted{ok:false}`/`PushFailed`/`failure_count++`.
- Severity: LOW.
- Why: contradicts the code's own comment and
  `record_push_attempt_error`'s correct no-op for cancellations
  (daemon.rs:5085-5088); wedged-task abort storms burn failure budget.
- Fix-direction: map cancellation to PushPaused (unknown, retain
  activity, no count).

### R3-L14 — Push path never revalidates ownership; Owned verdict sticky forever
- Location: `dracon-sync/src/daemon.rs:1367-1372,7113-7164` (only
  negatives re-detect on TTL); push path `handle_ahead_push` unchecked.
- Severity: LOW (defense-in-depth).
- Why: origin retarget/identity drift after Owned classification keeps
  pushing manually-committed ahead work with operator credentials
  (pre-commit guard covers auto-commit only). `sync-now` CLI likewise
  relies on in-pipeline guards (arguably consent).
- Fix-direction: revalidate origin trust on the push path when the
  verdict is older than the redetect TTL.

### R3-L15 — Dead settling knobs still parsed/defaulted (L3 residual, disclosed)
- Location: `dracon-sync/src/policy.rs:2373-2381`
  (`settling_max_delay_secs`/`dirty_max_age_action` quarantined,
  UNIMPLEMENTED in example.toml).
- Severity: LOW. Operator harm limited to silently-ignored config.
- Why: no production consumer outside defaults/tests; quarantined +
  disclosed per L3 fix.
- Fix-direction: implement the settling feature or remove both halves +
  docs per the quarantine comment.

### R3-L16 — Stale gc doc claims `--prune=now`
- Location: `dracon-sync/src/git/mod.rs:4755` vs `:4766-4783,4825`.
- Severity: LOW. Implementation (plain `gc`, 2-week grace) is the safe
  one; doc wrong. Gc itself sound (async, 600s bound, 1h cooldown,
  dry-run gated).
- Why: doc/behavior mismatch invites "fixing" toward the unsafe form.
- Fix-direction: correct the docstring to plain `gc`.

### R3-L17 — Visibility any-public aggregation lacks slug-divergence guard (downgraded)
- Location: `dracon-sync/src/visibility.rs:403-447` (any-public),
  `:766-868` (flip + cache write); display-only guard
  `report.rs:1465,8491`.
- Severity: LOW (downgraded from MEDIUM per critic: `sync_visibility`
  defaults false = opt-in; origin ignored and query name comes from
  local dir so URL slug drift cannot misdirect; stale mirror URL already
  leaks via direct push; only decoupled channel is stale
  repo_name_map/auto_create_account; aggregation documented intended).
- Why: any-owned-forge-public flips GitLab/Codeberg mirrors public and
  writes public to the cache that authorizes Codeberg pushes, with no
  divergence check. Reachable only opt-in + misdirected query.
- Fix-direction: refuse to aggregate (treat as unknown) when same-host
  remotes name different projects; consider account-level guard.

### R3-L18 — Merge driver registers unquoted %O/%A/%B; space paths break merges
- Location: `dracon-warden/src/main.rs:1723`.
- Severity: LOW. Fail-closed (merge stops unmerged/conflicted);
  availability-only, narrow.
- Why: git substitutes temp paths without quoting; space-containing repo
  paths word-split the driver command, clap exits 2.
- Fix-direction: quote placeholders in the registered driver string.

### R3-L19 — `text_merge` conflates merge-file errors with conflicts
- Location: `dracon-warden/src/main.rs:4617`
  (`!output.status.success()`), overwrite `:4586`.
- Severity: LOW. Recoverable (stages 1/2/3 stay in index;
  `git checkout -m` restores); operator-visible, narrow.
- Why: exit 1 (conflicts) and exit >1 (internal error) both take the
  conflict path and overwrite %A with possibly-empty stdout.
- Fix-direction: treat exit 1 as conflict; propagate other statuses as
  hard errors without touching %A.

### R3-L20 — Installer prints success before installing; stale tmp dotfile
- Location: `install.sh:359-366` (echo) vs `:416-419` (cp/mv); no trap.
- Severity: LOW. Exit code honest (`set -euo pipefail` aborts); log line
  + hidden dotfile cosmetic.
- Why: copy failure leaves a `.<binary>.$$` dotfile in `~/.local/bin`
  and a log line already claiming success.
- Fix-direction: move echo after `mv`; trap-remove `$tmp_bin`.

### R3-L21 — Blob-novelty check is O(history) per added file
- Location: `dracon-warden/src/main.rs:5411`
  (`git rev-list --objects --remotes | grep` inside per-file loop).
- Severity: LOW. Fail-closed (stale refs → block); slow-push wart.
- Why: N added files in a large repo pay N full object enumerations.
- Fix-direction: enumerate once per push into a temp file and grep per
  file (or batch).

### R3-L22 — Tmp freshness probe fails open on walk errors (downgraded)
- Location: `dracon-system/src/main.rs:5483-5509`
  (`tree_has_fresh_content` → `continue` → `false` → delete `:5636`).
- Severity: LOW (downgraded from MEDIUM per critic: live /tmp probe
  proved `rm` fails EPERM on unreadable subtrees so fresh content
  survives; unprivileged user service ⇒ read-fail implies remove-fail;
  residual is transient-I/O sliver only).
- Why: code shape fail-open (inconsistent with adjacent fail-closed
  top-level checks `:5596-5603`); cited chmod-000 scenario
  self-neutralizes, but FUSE/NFS ESTALE or `modified()` failure on a
  statable file could still misjudge.
- Fix-direction: return true (fresh/keep) on any walk error, like L6.

### R3-L23 — Checker step-6 false-positives on symlinked roots (live exit 1)
- Location: `dracon-sync/scripts/check-unit-deployment.sh:224-251`
  (guard twin `:178-201` same latent class).
- Severity: LOW. Release wiring advisory-only (no release block); trains
  operators to ignore the M9 detector.
- Why: longest-prefix mount match on the literal root, no
  canonicalization. Live: `~/.ssh → ~/.dracon/secrets/ssh` reports
  read-only + wrong remediation while the daemon writes fine through
  the rw `~/.dracon` bind (mountinfo verified).
- Fix-direction: canonicalize each root before matching.

### R3-L24 — Guard checker lacks the 3b companion check
- Location: `dracon-system/scripts/check-unit-deployment.sh` (steps
  1,2,3,4,5,6, no 3b) vs sync checker `:109-133`.
- Severity: LOW.
- Why: stale/missing `dracon-system-guard-watchdog.service/timer`
  backstop undetected.
- Fix-direction: mirror the 3b byte-for-byte companion loop.

### R3-L25 — check-flake.sh asserts a subset; unasserted props can re-drift
- Location: `scripts/check-flake.sh:77-129` (asserted) vs unasserted
  guard CPUQuota/MemoryMax/TasksMax/RestartSec/ProtectSystem/ProtectHome,
  sync ReadWritePaths/PrivateTmp/MemoryHigh/TasksMax/Nice/ProtectSystem/
  ProtectHome/ExecStartPre/CapabilityBoundingSet, home.file `.source`.
- Severity: LOW. Flake matches shipped units on all of these today
  (side-by-side verified); gap is future drift invisibility (a `.source`
  assertion would have caught R3-H1).
- Why: subset assertions per H1/H2 fix direction never extended to all
  properties.
- Fix-direction: extend assertions to every shipped-unit property.

### R3-L26 — Doctor does not check M8 watchdog timers
- Location: `dracon-system/src/doctor.rs:83-165`.
- Severity: LOW.
- Why: checks guard/sync service active but no
  `dracon-{sync,freeze,system-guard}-watchdog.timer` state → missing
  backstops still report all-green.
- Fix-direction: add timer-active checks.

### R3-L27 — uninstall.sh leaves M8 timers enabled and firing
- Location: `uninstall.sh:67` (only 2 main services) vs
  `install.sh:512` (timers enabled).
- Severity: LOW.
- Why: after uninstall, 3 timers keep firing every 2 min (start attempts
  on removed units = journal errors forever; freeze watchdog keeps
  clearing markers; notify scripts remain).
- Fix-direction: stop/disable/remove the 6 watchdog units + notify
  scripts.

### R3-L28 — Relocate `fits` fails open when df fails
- Location: `dracon-system/src/relocate.rs:263`
  (`avail.map(|a| a >= bytes).unwrap_or(true)`).
- Severity: LOW. Safe (ENOSPC mid-copy bails, source untouched) but
  leaves partial dest that blocks retries ("destination already exists"
  `:254-256`) until manual cleanup.
- Why: unknown space treated as fits.
- Fix-direction: fits=false (not ready) when avail unknown.

### R3-L29 — Freeze watchdog TOCTOU: stat failure reports false auto-clear
- Location: `dracon-sync/scripts/dracon-freeze-watchdog.sh:27-32`.
- Severity: LOW. Cosmetic but noisy; misattributes operator action.
- Why: stat failure → mtime=0 → huge age → prints "auto-clearing",
  notify + logger. Common trigger is normal `resume` racing the 2 min
  tick (marker gone between `-f` and `stat`).
- Fix-direction: skip (continue) when both stat variants fail.

### R3-L30 — Fresh installs leave main services disabled + "not found" misreport
- Location: `install.sh:36-38` (pre-snapshot) + `restart_service`
  `:603-623`.
- Severity: LOW. End state works via M8 backstop (~2.5 min) but
  contradicts the D4 model.
- Why: freshly installed units reported "not found" and never
  enabled/started; operator sees disabled service + enabled timer.
- Fix-direction: on fresh install (absent from pre-snapshot),
  `enable --now` main services; fix the message.

### R3-L31 — Open-file protection silently off when /proc unreadable
- Location: `dracon-system/src/main.rs:5440-5443`
  (`collect_open_paths_under_from` → empty set on /proc failure).
- Severity: LOW.
- Why: `path_has_open_ancestor` never matches → tmp cleanup deletes
  files held open by live processes with no warning (fail-open shape
  adjacent to R3-L22).
- Fix-direction: skip apply (or the pass) when the open-path scan fails.

### R3-L32 — install.sh unit copies fail silently, success still reported
- Location: `install.sh:492-507` (all 8 copies `2>/dev/null || true`),
  `:511` prints success regardless.
- Severity: LOW.
- Why: failed copy (full disk, perms) surfaces only obliquely via the
  timer-enable warning or not at all (main services).
- Fix-direction: track copy failures and fail loudly.

### R3-L33 — `truncate_unicode_width` under-counts VS16 pairs by 1
- Location: `dracon-sync/src/report.rs:2980-3001` (char-sum) vs
  `UnicodeWidthStr` everywhere else.
- Severity: LOW. 1-col cosmetic overflow, no panic/wrong numbers.
  Probed with pinned unicode-width 0.2.2.
- Why: ⚠/⏸=1 + VS16=0 sums to 1 but renders 2: compact/full Gone cells
  overflow (pinned by L18 tests), rich frozen rows with 4+-digit ages
  overflow, any ⚠️/⏸️ near a budget edge overflows by 1.
- Fix-direction: per-grapheme `UnicodeWidthStr::width(g)` (also fixes
  ZWJ 6-vs-2 over-count; update the over-conservative test).

### R3-L34 — Full-tier "⏰ ACTIVITY" header wraps
- Location: `dracon-sync/src/report.rs:5724` (header) vs `:5772`
  (Absolute 11); stale mirror test `:13463`.
- Severity: LOW. Opt-in tier (`--layout full`); readable. Pre-existing
  since F30 trim.
- Why: 11 content + 2 padding = 13 > 11 → comfy-table wraps to two
  lines per the file's own rule; masked by the stale mirror test
  (checks against 17, omits ROLE).
- Fix-direction: widen column or shorten header; refresh the mirror.

### R3-L35 — PUSH-TO green path truncates to 22 while the column holds 30
- Location: `dracon-sync/src/report.rs:719` (green/22) vs `:743`
  (yellow/30); columns Absolute(32) `:5555,:5769`.
- Severity: LOW. Same family as L18.
- Why: 4-remote fleet (29 chars, a topology the file's own test uses)
  renders truncated on the green path only.
- Fix-direction: budget 30 on the green path; drop the stale
  LowerBoundary comment.

### R3-L36 — PENDING / push-stuck / frozen durations use raw `{}m`
- Location: `dracon-sync/src/report.rs:520-522,543-545` vs `shorten_mins`
  elsewhere.
- Severity: LOW. Honest truncation today (… suffix); cosmetic.
- Why: "🟡 waiting 43200m" (17 wide) truncates in rich (budget 14) and
  drops compact activity; "🟡 waiting 30d" (14) would fit rich exactly.
- Fix-direction: one `shorten_mins` call at both sites.

### R3-L37 — Stale tier-threshold / tier-shape docs (6 spots)
- Location: `dracon-sync/src/report.rs:3118` (Rich 6-col+HINT),
  `:3136` ("10-column"), `:5039,:5522,:5677,:5741,:4898` (pre-v0.113.26
  thresholds), `:6785` (TOUCHED 16), `:4886` (layout error omits rich).
- Severity: LOW. Threshold CODE correct (165, test-pinned); docs only.
- Why: L15 fixed only the fn docstring; v0.113.26 auto-pick and v0.113.30
  author-only changes left the rest stale.
- Fix-direction: refresh the six doc spots; no behavior change.

### R3-L38 — Stale width-mirror tests assert pre-F30 values
- Location: `dracon-sync/src/report.rs:13384` (HINT 22 vs 26),
  `:13463` (7 widths stale, ROLE missing — masks R3-L34),
  `:13538` (assumes 35-col ACTIVITY).
- Severity: LOW. Test-quality only; passes but cannot catch column
  drift.
- Why: mirrors not updated with the F30 trim.
- Fix-direction: regenerate mirrors from production widths.

### R3-L39 — Hostile/extreme-count tails
- Location: `dracon-sync/src/report.rs:7030` (A/B budget 7),
  `:7048,:5640,:5829,:5862` (raw `usize`), `#` Absolute(4).
- Severity: LOW. No panic paths found (all slicing boundary-safe,
  math div/min-guarded, sums saturate).
- Why: A/B ≥1M clips ("↑1234567"→"↑12345…", honest …); raw count cells
  need ≥100K/≥1M to wrap (absurd); `#` overflows at 100+ repos (only
  plausible tail, ~35 today; in-repo wrap-vs-clip claims conflict).
- Fix-direction: `format_compact_count` for A/B symmetry; widen `#` or
  document the 99-repo ceiling.

---

## Checked-clean (ROUND2 regressions: all FIXED)

- H1 FIXED — flake guard mirrors shipped unit (flake.nix:341-396;
  `-` paths, ExecReload, CAP_SYS_NICE, EPERM sandbox; asserted
  check-flake.sh:77-104).
- H2 FIXED — flake sync Restart=always, CPUQuota=100%, sandbox + EPERM,
  MDWE deliberately absent (flake.nix:273-328; asserted :108-116).
- M1 FIXED — fail-fast on permanent/pack before HTTPS fallback
  (multi_remote.rs:673-679; test PASS).
- M2 FIXED — tagged per-forge errors retained (push.rs:56+; test PASS).
- M3 FIXED — SSH cause chained on both origin (:217-229) and mirror
  (:699-712) paths (test PASS; narrow residuals R3-L03/L04).
- M4 FIXED (steady-state) — staged-blob sizing via `ls-files -s` +
  `cat-file --batch-check`, fail-closed, exit-0 reset (staging.rs:119+;
  7/7 tests PASS; bootstrap residual R3-M1).
- M5 FIXED — quarantine expiry honors `auto_cleanup_apply`
  (dracon-system/main.rs:6250-6278; dry-run logs; tests.rs:3388) [D2].
- M6 FIXED — both units + flake set SystemCallErrorNumber=EPERM
  (asserted).
- M7 DOCUMENTED (accepted risk) — blocked-helper constraint in unit +
  flake comments; no exemption scoped. Residual: no runbook entry
  (see Unresolved).
- M8 FIXED — 6 watchdog units shipped/installed/enabled/documented
  (install.sh:494-516, flake.nix:401-477, AGENTS.md:473-490).
  Residuals: R3-H1, R3-L26, R3-L27.
- M9 FIXED — sync checker exists with 3b companion check, both suites
  in CI (ci.yml:211-219), advisory release wiring. Residuals: R3-L23,
  R3-L24, verify-spec.sh invariant (see Unresolved).
- M10 FIXED — SECRET_RE renders from `hook_token_shapes_ere`
  (freshness + coverage + ERE + behavioral tests PASS; self-match
  probe negative).
- M11 INTACT — `--locked` builds (install.sh:328-342).
- M12 FIXED — JSON rows carry full hash/msg, render truncates
  (report.rs:4442; reg-test PASS).
- L1 FIXED — bootstrap Err propagates (sync.rs:5041-5049; test PASS;
  daemon maps to Failure, no ledger clear) [D3].
- L2 FIXED at backstop site (sync.rs:5074-5080; garbled-parse
  fail-closed); twin residual R3-L01 [D3].
- L3 FIXED — override-consumption tripwire + UNWIRED quarantine +
  AGENTS/docs (policy.rs:2525; tests PASS; merge-correctness residual
  R3-M2).
- L4 FIXED — loud askpass unlink (ops.rs:641-658; test PASS; caveat
  R3-L08).
- L5 FIXED (mirror path budget; test PASS); origin residual R3-L02.
- L6 FIXED — trash scan fails closed per-entry (main.rs:3674-3693;
  tests PASS).
- L7 FIXED — auto-relocate pauses on df failure (main.rs:6680-6694;
  test PASS).
- L8 FIXED — log truncate writes-first-then-cuts, byte-wise non-UTF8
  (main.rs:4324-4371; tests PASS).
- L9 FIXED — smudge verbatim preservation, lossy arm gone
  (filter.rs:446-459; 8/8 smudge tests PASS).
- L10 FIXED — `-M100%` + `diff-filter=a` + blob-novelty check
  (main.rs:5377,5405-5413; 4/4 tests PASS).
- L11 FIXED via documented-caveat alternative — stale-ref warning, no
  fetch in hook (main.rs:5560-5572; 9/9 tests PASS).
- L12 FIXED — atomic binary install via tmp + rename (install.sh:416;
  `bash -n` PASS).
- L13 FIXED — grapheme-iterating truncator (report.rs:2991; ZWJ/flag
  tests pin whole).
- L14 FIXED — first-grapheme activity split, VS16 stays with icon
  (report.rs:3065; tests PASS).
- L15 FIXED — rich hint link-out under table (report.rs:6617; tests
  PASS; doc residual R3-L37).
- L16 FIXED-BY-DOC — table id-safety limitation documented +
  test-pinned (report.rs:2851).
- L17 FIXED — full REPO explicit truncate + shared branch budget
  (report.rs:2911,5822; test PASS).
- L18 FIXED — per-tier publish/role budgets at all callers
  (report.rs:665,6362; tests PASS; VS16 residual R3-L33).
- L19 FIXED — compact-count abbreviation + const-derived budget
  (report.rs:6604,6948; tests PASS).

Verification observed this session: dracon-system 426 passed / 0
failed (412 unit + 4 events_cli + 10 guard_service, `--locked`);
dracon-sync `report::` 265 passed / 0 failed; push-filtered 101/101;
staging 7/7; warden hook_shapes 3/3, pre_push allows 4/4, pre_rebase
9/9, smudge 8/8; `systemd-analyze --user verify` exit 0 both units;
`bash -n` all scripts OK.

Investigated-and-cleared (no finding): keyless-merge
double-encryption (safe); SECRET_RE drift (none); `scan_and_replace`
attribution/boundary (no leak constructed); `is_hatched` CWD
relativity (fail-closed both ways); hook install/rollback +
foreign-hook preservation; filter-process driver bounds;
`clean_reusing_index` quadruple check; install D4 service handling;
bootstrap ownership gate; `auto_repair_concerns` merge
(global-false wins, single-repo bypass by design); `apply_outcome`
matrix + both callers; visibility cache fail-closed; stuck-ledger
atomic writes + 24h expiry + redaction + outage shield; role.rs
basename fallback (documented compat); run_once arms complete;
trash purge; quarantine move/restore/expire/purge; relocate
plan/apply; links apply; setup guards; df re-read conservative;
watchdog oneshot scripts + timer intervals; C1 (truncate budgets
≥3) re-verified; hostile-data panic sweep (none found).

---

## Unresolved (kept from all areas)

- U1: daemon dispatch cooldowns/starvation and stuck-ledger state
  machine beyond `apply_outcome` (inherited from ROUND2).
- U2: staging `rewrite_ahead_paths` bundle/lease path (out of area).
- U3: live fleet state beyond this single host not inspected
  (deployed units, journals on other machines).
- U4: no fault-injection runs (read-only audit); OOM/ENOSPC/df-failure,
  pressure, clock-skew paths by inspection + existing tests only.
- shellcheck not installed; shell scripts verified with `bash -n` only.
- R3-H1 nix-eval / home-manager-build proof not executed (outside
  allowed ops); rests on inspection + A7 test + git-proven absence +
  the checker's git+file eval.
- comfy-table Absolute-overflow mode (wrap vs clip) not executed;
  in-repo comments conflict (6766 wrap vs 13506 truncate) — determines
  the R3-L39 `#` failure mode at 100+ repos.
- Live 165-col terminal rendering not performed (read-only audit,
  binary not run against hostile data); width claims via pinned
  unicode-width 0.2.2 / unicode-segmentation 1.13.3 probes + 265
  passing report tests.
- M5 dracon-system tests not run by the sync-git researcher (outside
  assigned area; fix verified by inspection) — covered by the system
  researcher's 426-test run above.
- M7 residual: blocked-helper constraint lives only in unit comments;
  no runbook/operator-doc entry — accepted-risk documentation depth is
  an operator call.
- M9 residual: verify-spec.sh has no drift invariant for the checker
  wiring.
- Cross-area note (not counted): `repo_has_warden_filter`
  (dracon-sync/sync.rs:954) tests legacy `filter.dracon.clean`, unset
  by the v0.113.13 migration (warden main.rs:1734) — always-false
  post-migration, so the daemon re-runs harden `once` per repo per
  process start (perf wart; sets the R3-M3 repair cadence).

---

## Omitted-scope (disclosed)

- Fleet state beyond the audited host (other machines' units,
  timers, journals, generations).
- Fault-injection and pressure runs (ENOSPC, OOM, FUSE/NFS ESTALE,
  clock skew, wedged-task storms); exercised paths covered by
  inspection + the passing locked test suites only.
- Nix evaluation / home-manager builds and live terminal rendering
  (see Unresolved for the exact evidentiary limits).
- Networked git pushes, forge API behavior, and live mirror-URL
  account drift; visibility findings reason from resolved config +
  defaults, not live forge state.
- Daemon dispatch cooldown/starvation analysis beyond `apply_outcome`
  and the staging bundle/lease path (U1/U2 carried forward).
- Human-factors review of operator docs/runbooks beyond the cited
  contract contradictions (AGENTS.md exclude-list, gc doc, tier docs).
