# Full Audit 2026-10-02 — Round 2 (merged ranked report)

Date: 2026-10-02. Method: read-only source audit; merges prior results
1 (push/staging/outcome/policy/askpass), 2 (warden/installer/smudge/hooks),
3 (units/guard/watchdog), 5 (report.rs display/HINT truncation).
Prior result 4 (critic) produced no usable verdict — see Unresolved.
Regression baseline was AUDIT_FIXES_2026-10-02 + current source only;
AUDIT_FULL_2026-07-26.md was not re-read (see U4). No fault-injection
runs, no live fleet inspection, no hook/filter installs or executions.

Dedupe notes: the three push-fallback error findings (M1/M2/M3) share a
theme but are distinct defects (fail-fast policy, per-forge HTTPS error
loss, original SSH error loss) and are kept separate with cross-refs.
M7 (hooks inherit sandbox) and L10 (per-commit scans now binding on
daemon pushes) both stem from the post-A6 hooks-honored change and
cross-reference each other. No other overlaps found; nothing dropped.

Severity scale: HIGH = broken install/startup or fleet-wide outage
class; MEDIUM = wrong behavior with operator/security impact under
plausible conditions; LOW = narrow, cosmetic, or defense-in-depth.

---

## HIGH (2)

### H1 — Nix guard unit stale vs shipped unit; fresh Nix installs may not start or cannot write
- Location: `flake.nix:302-330` vs shipped `dracon-system-guard.service`
- Severity: HIGH [units]
- Why it matters: `ReadWritePaths` lacks `-` prefixes, so systemd
  refuses to start the guard when `%h/Dev` etc. are absent (the shipped
  unit documents the live 226/NOPERM lesson); it also lacks
  `%h/.local/share`, nix-profile, quarantine/cold roots, `ExecReload`,
  `CAP_SYS_NICE`, and the seccomp/sandbox hardening. Nix-installed
  guards get a unit that may fail to start or cannot write where the
  daemon must. `check-flake.sh:59-72` asserts only 5 guard properties,
  so this drift is invisible to CI.
- Fix direction: sync the Nix unit with the shipped unit (or generate
  one from the other) and extend `check-flake.sh` to assert the
  currently-drifting properties.

### H2 — Nix sync unit stale: 15% CPUQuota starves the classifier, weaker restart, no sandbox parity
- Location: `flake.nix:266-299` vs shipped `dracon-sync.service`
- Severity: HIGH [units]
- Why it matters: `CPUQuota=15%` (shipped: 100%) reproduces the
  measured 49.2s-vs-13s classifier starvation; `Restart=on-failure`
  (shipped: always) leaves clean-exit-down states unrecovered; missing
  sandbox + `ExecReload` and a bare-pkill `ExecStartPre` diverge from
  the hardened shipped unit. Same invisibility as H1: no checker
  asserts these properties.
- Fix direction: same as H1 — single-source the unit contents and add
  checker coverage for quota/restart/sandbox/ExecReload.

---

## MEDIUM (12)

### M1 — Initial SSH failure has no permanent/pack fail-fast; every cycle burns a doomed HTTPS attempt
- Location: `dracon-sync/src/git/multi_remote.rs:665-680` (contrast `push.rs:144`)
- Severity: MEDIUM [push]
- Why it matters: unlike the `push.rs` path, the multi-remote initial
  SSH failure falls through to HTTPS fallback unconditionally. A policy
  rejection therefore costs a full `timeout_secs` doomed HTTPS attempt
  every cycle and doubles stuck-ledger noise. See also M2/M3.
- Fix direction: apply the same permanent/pack fail-fast before
  falling through to HTTPS.

### M2 — HTTPS fallback discards each forge error; classifier then mislabels policy rejections
- Location: `dracon-sync/src/git/push.rs:18-86` (`push_https_fallback`)
- Severity: MEDIUM [accounting]
- Why it matters: per-forge errors are discarded and replaced with
  generic "all HTTPS push attempts failed", so `classify_push_failure`
  mislabels e.g. HTTPS policy rejections as transport/auth in
  stuck-ledger HINTs/alerts — the operator chases the wrong cause.
- Fix direction: retain first/last forge error detail in the returned
  error for classification.

### M3 — Fallback path discards the original SSH error; root cause lost to operator and ledger
- Location: `dracon-sync/src/git/push.rs:139-157`, `multi_remote.rs:669-670`
- Severity: MEDIUM [accounting]
- Why it matters: when the fallback fails, the generic fallback error
  (or "remote not found") wins and the original SSH push error is
  dropped. The ledger records a symptom, not the cause.
- Fix direction: chain/context the original SSH error into the
  fallback-failure error.

### M4 — Oversize check stats the worktree file, not the staged blob (TOCTOU); stat errors fail open
- Location: `dracon-sync/src/git/staging.rs:59-77` (`unstage_oversized_paths`)
- Severity: MEDIUM [staging-TOCTOU]
- Why it matters: stage-large-then-truncate commits a >100MiB blob
  past `max_stage_file_bytes`; `if let Ok` on stat errors silently
  keeps the path staged. No test covers worktree/index divergence, so
  the size gate the commit-all policy depends on is bypassable.
- Fix direction: size the staged blob (index), fail closed on stat
  error, add a divergence test.

### M5 — Quarantine expiry hardcodes apply=true; unattended deletes armed in dry-run mode
- Location: `dracon-system/src/main.rs:6161-6184` (`maybe_expire_quarantine`)
- Severity: MEDIUM [guard]
- Why it matters: gated only on interval + `clean_quarantine_first`,
  ignoring `auto_cleanup_apply`, while `example toml:170` claims apply
  must be true for daemon deletes. Interval defaults to daily
  (`policy.rs:910`) and is absent from the example, so the "gentler
  posture" arms unattended deletes for operators who believe they are
  in dry-run mode.
- Fix direction: honor `auto_cleanup_apply`, and document the interval
  default in the example config.

### M6 — "Returns EPERM instead of SIGSYS" comment is false; sandbox violations kill and misdiagnose
- Location: `dracon-system-guard.service:115-116`, `dracon-sync.service:59-60`
- Severity: MEDIUM [units]
- Why it matters: neither unit sets `SystemCallErrorNumber`, and the
  installed systemd-258 man page states the default is SIGSYS
  termination. Sandbox violations therefore kill processes and present
  as crashes, sending diagnosis down the wrong path.
- Fix direction: set `SystemCallErrorNumber=EPERM` or correct the
  comment and runbooks to SIGSYS.

### M7 — Post-A6 honored hooks inherit the full sync sandbox; helper syscalls die SIGSYS, opaquely
- Location: `dracon-sync.service:53-60`
  (`SystemCallFilter=@system-service`, `RestrictNamespaces`,
  `RemoveIPC`, `PrivateDevices`)
- Severity: MEDIUM [units]
- Why it matters: hook helpers needing blocked calls (userns/
  bubblewrap, BPF, perf) die with SIGSYS — opaque per M6. MDWE was
  removed for exactly this class; the rest of the sandbox remains.
  See also L10 (scans now binding on daemon pushes since A6).
- Fix direction: scope a sandbox exemption/allowlist for the hook
  execution path, or document the blocked-helper constraint.

### M8 — Guard/freeze watchdog timers ship nowhere; fresh installs lack restart and freeze auto-clear
- Location: repo-wide — no `.timer` files; `install.sh:478-479` copies
  only the two `.service` files; `flake.nix` has no timers; zero
  repo-wide references
- Severity: MEDIUM [watchdog]
- Why it matters: fresh installs silently lack restart-if-stopped and
  the 30m freeze auto-clear (degraded to the 1h daemon hard-clear),
  contradicting AGENTS.md's quiesce policy backstops.
- Fix direction: ship, install, and reference both timers (services +
  Nix + docs).

### M9 — No sync equivalent of check-unit-deployment.sh; sync-unit drift has no detector
- Location: (missing) vs `dracon-system/scripts/check-unit-deployment.sh`
  (run only in `release.sh:567`, not CI/verify-spec)
- Severity: MEDIUM [units]
- Why it matters: sync-unit drift like the 2026-10-02 MDWE removal has
  no detector — the exact repo-vs-deployed class the guard script
  exists to catch.
- Fix direction: add a sync unit-deployment checker and wire both
  checkers into CI/verify-spec, not just release.

### M10 — Pre-push SECRET_RE omits Tier-1 provider-token shapes the clean filter encrypts everywhere
- Location: `dracon-warden/src/main.rs:5204` vs
  `security/src/modules/scanner.rs:50` (`tier1_patterns`)
- Severity: MEDIUM [pre-push]
- Why it matters: only AKIA/PEM/password/secret/api_key assignments
  trip the hook; `ghp_/gho_/ghu_/ghs_/ghr_`, `github_pat_`, `glpat-`,
  `sk_live_/rk_live_/sk_test_/rk_test_`, `whsec_`, `xox*`, `SG.`,
  `SK/AC` hex push clean when the filter is bypassed — the same
  advertised-defense gap class as fixed A1.
- Fix direction: share one token-shape source between clean and the
  pre-push hook.

### M11 — Fleet deployment builds with floating deps (no --locked); installed binary drifts from tested
- Location: `install.sh:329-333` (`cargo build --release`, zero
  `--locked` occurrences in file)
- Severity: MEDIUM [installer]
- Why it matters: the deployment path builds live nested checkouts
  with floating semver deps, bypassing the Cargo.lock/deny pin chain
  CI enforces — supply-chain drift between tested and installed
  binaries (echoes the installed-binary-drops-patch incident class).
- Fix direction: build with `--locked` (and fail loudly when the lock
  is stale).

### M12 — JSON output leaks display truncation: last_hash is an invalid rev, last_msg pre-clipped
- Location: `dracon-sync/src/report.rs:4407` (`last_hash=truncate(&h,12)`
  = 11 hex + `…`), `:4411` (msg pre-truncated to 150);
  `RepoReportRow:1481` derives `Serialize`, `--json` emits rows
  directly (`4739-4752`). HINT is full in JSON (good).
- Severity: MEDIUM [report/json]
- Why it matters: script consumers get a `last_hash` unusable as a rev
  and a clipped message from what should be the machine-readable
  interface. Highest-severity report.rs finding.
- Fix direction: emit full hash/message in JSON rows; truncate only at
  display render.

---

## LOW (19)

### L1 — Bootstrap failure maps to NothingToDo, which the daemon counts as Success
- Location: `dracon-sync/src/sync.rs:5029-5032` → `daemon.rs:845-860`
- Severity: LOW [outcome]
- Why it matters: a repo failing bootstrap every cycle clears the
  stuck ledger and resets `failure_count` — reads healthy with no
  failure accounting.
- Fix direction: map bootstrap `Err` to a failure outcome, not
  `NothingToDo`.

### L2 — Ahead-count status error reads as 0-ahead, disarming the backstop gate
- Location: `dracon-sync/src/sync.rs:5054` (`count_ahead_commits().unwrap_or(0)`)
- Severity: LOW [outcome]
- Why it matters: a status error silently presents as "0 ahead",
  disarming the backstop/ahead gate instead of surfacing the error.
- Fix direction: propagate/surface the error; fail closed on the gate.

### L3 — Override-coverage tripwire compares field-name sets only, not merge-at-use
- Location: `dracon-sync/src/policy.rs:2287-2335`
- Severity: LOW [policy]
- Why it matters: message + AGENTS.md promise "merge resolution at the
  point of use", but a both-halves-present yet never-merged knob
  passes. Structural only — spot-check `active_commit_minutes` IS
  merged (`report.rs:2576`).
- Fix direction: strengthen the tripwire (e.g. per-field merge
  assertion) or soften the promise.

### L4 — Askpass Drop ignores unlink errors silently; token script can linger in /tmp
- Location: `dracon-sync/src/git/ops.rs:641-648`
- Severity: LOW [askpass]
- Why it matters: a failed unlink leaves the token script in /tmp with
  no log; no test covers unlink failure.
- Fix direction: log unlink failures; test the failure path.

### L5 — retries=0 still performs 3 attempts; "no retry" semantics inconsistent and hammers sick forges
- Location: `dracon-sync/src/git/multi_remote.rs:682` (`retries.max(1)`
  AFTER initial SSH + HTTPS) vs `push.rs:175` (total attempts)
- Severity: LOW [push-retries]
- Why it matters: inconsistent "no retry" semantics; sick forges get
  hammered despite retries=0.
- Fix direction: unify on total-attempts counting.

### L6 — Trash credential scan fail-opens twice: depth cap + swallowed walk errors
- Location: `dracon-system/src/main.rs:3626-3648` (`max_depth(8)`,
  `'_ => {}'` on walk errors)
- Severity: LOW [guard]
- Why it matters: secrets deeper than 8 levels, or in unreadable
  subtrees, purge despite `trash_credential_guard` — depth-evasion
  bypass of the 2026-08-10 credential discipline.
- Fix direction: fail closed (skip purge) on walk errors; document or
  raise the depth bound.

### L7 — Auto-relocate maps df failure to max pressure and keeps moving trees with no signal
- Location: `dracon-system/src/main.rs:6576-6579`
- Severity: LOW [guard]
- Why it matters: uncommented fail-open — an unreadable df
  (namespace/EROFS) triggers moves instead of pausing them.
- Fix direction: pause moves on df failure (or comment the rationale).

### L8 — Log-truncate crash window empties the log; non-UTF8 lines silently dropped
- Location: `dracon-system/src/main.rs:4279-4280` (`set_len(0)` then
  `write_all`), `:4249-4250,4259` (`map_while(Result::ok)`)
- Severity: LOW [guard]
- Why it matters: a crash between truncate and write leaves the log
  empty; non-UTF8 lines vanish in a reclaim path — data-loss windows.
- Fix direction: write-temp-and-rename (or append-safe) truncate;
  preserve or count non-UTF8 lines.

### L9 — Non-canonical whole-file binary tags corrupt via lossy smudge, then re-encrypt the corruption
- Location: `dracon-warden/security/src/modules/filter.rs:449`
  (`smart_smudge` + `from_utf8_lossy`) vs exact-match
  `decrypt_whole_file_tag` (`security/src/lib.rs:1140-1160`)
- Severity: LOW [smudge]
- Why it matters: a whole-file binary tag with leading bytes/BOM or
  embedded in text misses exact-match, corrupts via the lossy path,
  and the next clean re-encrypts the corruption — residual of fixed H1
  outside canonical framing.
- Fix direction: scan-for-tag framing instead of exact-match, or fail
  closed on lossy binary.

### L10 — Per-commit scans lack blob-novelty check; pure renames/republishes block pushes
- Location: `dracon-warden/src/main.rs:5318-5351`
- Severity: LOW [pre-push]
- Why it matters: pure rename (D+A pair) or republish of grandfathered
  hook-matching content trips added-lines/added-blob scans and blocks
  the push; binding on daemon pushes since A6 made them honor hooks
  (see M7). Availability/precision gap.
- Fix direction: skip blobs already present in history (novelty check).

### L11 — Pre-rebase published check uses local remote-tracking refs; unfetched commits escape
- Location: `dracon-warden/src/main.rs:5486`
  (`git branch -r --contains`)
- Severity: LOW [pre-rebase]
- Why it matters: published-but-unfetched commits escape the rewrite
  guard when refs are stale — hook bypass.
- Fix direction: fetch or consult `ls-remote` for the boundary check
  (or document the staleness caveat).

### L12 — Live binaries replaced non-atomically; concurrent filter spawn exec-fails mid-install
- Location: `install.sh:404-405` (`rm -f` then `cp`) vs warden's atomic
  rename (`main.rs:4807`)
- Severity: LOW [installer]
- Why it matters: a concurrent git filter spawn in the window
  exec-fails — fail-closed, but wedges add/checkout mid-install.
- Fix direction: install-temp-then-rename like the hook installer.

### L13 — truncate_unicode_width doc claims grapheme safety; implementation iterates chars
- Location: `dracon-sync/src/report.rs:2917` (doc) vs `:2927-2962` (impl)
- Severity: LOW [report/docs]
- Why it matters: ZWJ sequences/flag pairs/skin-tone modifiers can
  split (e.g. cut after ZWJ leaves a trailing joiner + …); tests cover
  only single emoji/CJK. Display-only.
- Fix direction: fix the doc, or iterate graphemes.

### L14 — split_activity splits multi-codepoint emoji; state_plus_act_cell budget slightly off
- Location: `dracon-sync/src/report.rs:3022-3037` (first-char icon:
  `⏸️`/`⚠️` leak VS16 into text), `:2981-3005` (budget)
- Severity: LOW [report/cosmetic]
- Why it matters: cosmetic corruption of status icons in activity cells.
- Fix direction: split on grapheme clusters; reconcile the budget.

### L15 — Rich table (default ≥165 cols) has NO HINT column; docstring still describes 6-col-with-HINT
- Location: `dracon-sync/src/report.rs:6570-7007` (16-col header
  `6757-6775`), tier choice `:3114-3121`, stale docstring `:6555-6566`
- Severity: LOW [report/visibility]
- Why it matters: the default wide view hides HINT (detail only via
  `repos <name>`/vertical/JSON) while the docstring still promises it
  — visibility gap plus stale doc. HINT pipeline otherwise traced
  end-to-end (builder `4279-4319`, vertical width-2 `:5035`, compact 24
  `:5584`, full 13 `:5841`, summary embeds-then-truncates `:6386-6428`).
- Fix direction: add a HINT affordance to rich (or link-out), refresh
  the docstring.

### L16 — Goal-id preservation defeated at table budgets; effective only for vertical/JSON
- Location: `dracon-sync/src/report.rs:2848-2879`
  (`format_commit_subject_for_display`, budget 150) vs compact/full
  re-truncate to 16/15 (`:5554`, `:5776`)
- Severity: LOW [report]
- Why it matters: table tiers always slice mid-id anyway; the stated
  "never splits inside ] when data permits" is unachievable at table
  budgets. Double-ellipsis fixed (single … at render);
  `find_delta_segment` (`2886-2912`) conservative and byte-safe.
- Fix direction: document the limitation or reserve id-safe widths.

### L17 — Full-tier REPO passed untruncated despite truncate-to-17 comment; relies on comfy-table
- Location: `dracon-sync/src/report.rs:5782` vs comment `:5721`
  (compact truncates at `:5542`)
- Severity: LOW [report]
- Why it matters: comment-code mismatch; relies on comfy-table's
  non-unicode-aware truncation (mid-emoji split possible). Branch cells
  untruncated in compact/full (Absolute 11); rich folds branch into
  REPO with ⚡ then truncates (`6876-6895`, good).
- Fix direction: truncate explicitly like compact, fix the comment.

### L18 — Shared-helper budgets mismatch full-tier columns by ±1
- Location: `dracon-sync/src/report.rs:6675` (`publish_cell_label`
  budget 16 overflows full-tier 17-2=15; comment `:5724` claims 15),
  `role_cell` budget 12 under-uses full ROLE 18
- Severity: LOW [report/cosmetic]
- Why it matters: 1-char overflow/under-use in full-tier columns.
- Fix direction: per-tier budgets or widen/narrow helpers.

### L19 — Rich CHG columns clip counts ≥1000 to wrong numbers (e.g. "10…")
- Location: `dracon-sync/src/report.rs:6859-6916` (`chg_budget=3`);
  pulse cols were widened to 7 (`6680-6685`), CHG was not
- Severity: LOW [report]
- Why it matters: same class as the A/B L7 bug fixed in v0.113.18 —
  renders a clipped wrong number. Needs ≥1000 dirty files in one class.
- Fix direction: widen CHG like pulse, or abbreviate (1.2k).

---

## Checked clean (no finding)

- C1: `report.rs` truncators `truncate` (`2811-2820`, char-counted/
  byte-safe via `chars()`) and `truncate_unicode_width` (`2927-2962`,
  byte-safe via `char_indices`+`len_utf8`, saturating budgets,
  max0→`""`); emoji/CJK width tests (`12642-12690`) pass by inspection.
  No panic path. Edges (`truncate(x,0)`→`"…"`, max_width=1 mid-string
  pick) unreachable — no caller uses budget ≤1 (min budget 3).
- C2: PUSH markers budget-checked (`6078-6098`); `push_cell_with_age`
  via `shorten_mins` (`6060-6068`, `1208-1217`); legend wrap
  word-boundary width-aware (`3334-3358`), footer gated <120 (`3407`),
  `--legend` clamped 120..1000 (`3292-3294`); status_pair widths fit;
  `render_summary_notification` caps 5 lines + tail (`192-209`,
  per-line unbounded — minor); ellipsis `…` vs push-stuck `...`
  (`4304`) cosmetic; PENDING `{}m` raw minutes (`521,544`) vs
  `shorten_mins` elsewhere — cosmetic.

---

## Unresolved (preserved, 12)

1. daemon.rs dispatch cooldowns/starvation and stuck-ledger state
   machine beyond apply_outcome
2. sync.rs commit_allowed_by_ownership and auto_repair_concerns merge
   call-site
3. staging.rs rewrite_ahead_paths SYNC-H6 bundle/lease path not
   re-verified
4. AUDIT_FULL_2026-07-26.md not read; regression check used
   AUDIT_FIXES_2026-10-02 + current source only
5. Live fleet state not inspected: deployed unit contents, journal
   evidence of renice/seccomp/hook behavior, watchdog timer presence
   on host.
6. Full policy.rs clamp/coverage review and links.rs tail
   (broken-symlink scan) only skimmed; doctor.sh not covered.
7. No fault-injection runs (read-only audit): OOM-pressure, ENOSPC,
   and df/ps-failure paths verified by code inspection only.
8. Full Tier-2 scanner pattern inventory review (heads + construction
   verified only)
9. dracon-warden/src/tests.rs regression bodies (relied on source +
   remediation test counts)
10. run_merge_impl body below main.rs:4552 (head + F49 closure verified
    only)
11. Live hook/filter E2E (read-only: no installs, no hook execution on
    fleet repos)
12. critic: no usable verdict

## Omitted scope

- Prior result 4 carried no usable items (see U12); there was no
  additional omitted-scope list beyond the Unresolved items above.
  Live-state, fault-injection, and E2E angles (U5/U7/U11) remain the
  known coverage gaps for a future round.
