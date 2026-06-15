# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **dracon-sync: durable commit-all policy
  (goal `546d4f9c` / 2026-06-15)**: operator asked
  for the previous goal's policy change to be
  permanent, not a one-time config edit. Code
  defaults in `dracon-sync/src/policy.rs` updated:
  - `default_exclude_file_patterns()`: was
    `["*.log", "nohup.out", "*.sqlite", "*.db", ...]`
    → `Vec::new()` (commit logs and DBs by default)
  - `default_untracked_exclude_patterns()`: removed
    audit/screenshot/media/note patterns
    (`**/audit/**`, `**/evidence/**`,
    `**/screenshots/**`, `*.png`, `*.jpg`, `*.mp4`,
    `**/note.md`, etc.). Now only session-scratch
    patterns remain (`**/scratch/**`, `**/tmp/**`,
    `**/pi-tmp/**`, `.demon/**`, `.sisyphus/**`,
    `.ralph/**`).
  - Test updated: `test_default_exclude_file_patterns`
    now asserts empty.
  - Test added:
    `test_default_untracked_exclude_patterns_is_commit_all_unless_scratch`
    asserts the new defaults (session-scratch in,
    audit/screenshot/media/note out).
  - `dracon-sync/dracon-sync.example.toml` updated
    to reflect new defaults.
  - New `AGENTS.md` created with plain-English
    documentation of the policy, daemon commands,
    and forbidden actions.
  - Result: 850 tests pass (was 849 + 1 new),
    release build clean, cargo deny clean, all
    4 remotes aligned at `34728f4d`. Per-repo
    `auto_commit_exclude_patterns` mechanism still
    works (Junk-Runner-bevy keeps its
    `test-results/` exclusion). Concerns
    investigated: kiki-sassy `github` push-stuck
    (6 failures) is a divergent history issue
    (436 github-only commits, 775 local-only
    commits) — **NEEDS OPERATOR INPUT** to resolve
    (operator-owned repo). Other 2 concerns
    (dracon-platform 5 MOD, Junk-Runner-bevy
    88 MOD) resolved or working as designed.
    Documented in
    `docs/design/commit-all-policy-durable-2026-06-15.md`.
- **dracon-utilities: removed load-bearing symlink
  `~/Dev/dracon-libs`** (2026-06-15, goal
  `cca2169f`): `~/Dev/dracon-libs` was a symlink
  to `/tmp/lib-edit` and was invisible to the
  daemon (which doesn't follow symlinks). It was
  also load-bearing: the workspace had 6 `path =
  "../dracon-libs/..."` deps. Investigation found
  4 of those deps were dead (no `.rs` file uses
  them) and 2 (`dracon-git`, `dracon-system-lib`)
  were functionally identical to crates.io 94.7.0
  (`diff -rq` confirmed src/ dirs are identical).
  Refactor: dropped the 4 dead deps + dropped the
  `path` for the 2 versioned ones (now use
  crates.io 94.7.0). Refactor committed
  (`b2963348` + `113ff008`), pushed to all 4
  remotes. Archived `/tmp/lib-edit` to
  `/tmp/dracon-libs-snapshot-2026-06-15.tar.gz`
  (866 KB, 215 entries, SHA-256
  `1e8aa374a48d08cd1c9a6ac6ac449cc4d1a0c7c378589be4ae11db9e4146289f`),
  then `rm` the symlink. After deletion: 849
  tests pass, release build clean, cargo deny
  clean. Live daemon report still 13 repos
  (symlink was never watched).

### Fixed
- **Junk-Runner-bevy: applied test-results exclude
  policy to main branch** (2026-06-15, goal
  `c794cf71`): the operator's per-repo policy at
  `.dracon/dracon-sync.toml` (with
  `auto_commit_exclude_patterns = ['**/test-results/**',
  '**/e2e/screenshots/**']`) was committed to
  `tauri2` in `dc8f85fe1` but never made it to
  `main`. The daemon is working on `main` (per
  `.git/HEAD`), so the policy was not being
  applied — test-results/ PNGs were being
  auto-committed (e.g. commit `b71c068db` "3
  file(s) in test-results" at 20:37:50). The
  operator's "junk runner just seems wrong" was
  CORRECT — the policy was on the wrong branch.
  Fix: copied `.dracon/dracon-sync.toml` from
  `tauri2` to `main` in commit `24709b924`, pushed
  to all 4 remotes, now aligned at
  `24709b924db6`. After this fix, future test
  runs that regenerate `test-results/` PNGs will
  NOT be auto-committed. Also pulled the
  `Enable GitHub Sponsors button` commit
  (`6d1e953b6`) from origin (was 1 commit
  behind). Documented in
  `docs/design/junk-runner-investigation-2026-06-15.md`.
- **dracon-platform: manually committed 2 audit
  dirs the daemon missed** (2026-06-15, goal
  `c794cf71`): the daemon was committing other
  files in `dracon-platform` (HOME-AUDIT-...md,
  hegemon source, etc.) but NOT these 2 audit
  dirs (`audit-byteplus-...`, `audit-dp9cqdw-...`).
  Root cause not fully diagnosed (settling
  window? in_flight HashSet? auto-exclude
  pattern?). Manually committed as workaround,
  all 4 remotes aligned at `700d58c28c08`. The
  9 `.pi-tmp/*` scratch dirs and 3 deferred
  source dirs in `dracon-platform` remain
  untracked with documented reasons.
- **dracon-platform: committed 100+ untracked files
  (screenshots, audit dirs, test specs, game assets)
  per operator request** (2026-06-15): the operator
  had 39 untracked entries preserved through goal
  `fa84a5bd` and asked "we woudl love to commit that
  too and push it". This was resolved in goal
  `ca80b0d1`. Created 4 logical commits (top-level
  PNGs, audit dirs, test specs, game JPGs) plus
  6 daemon auto-commits for the operator's in-progress
  source/test/docs work. All 4 remotes (origin,
  github, gitlab, codeberg) aligned at
  `2a7bbd295bdf`. No force-pushes, no `.gitignore`
  changes, no sensitive files committed, no
  `.pi-tmp/*` session scratch committed. Documented
  in
  `docs/design/dracon-platform-untracked-commit-2026-06-15.md`.
  The 3 source dirs (`hegemon/src/lib/`,
  `hegemon/static/assets/`, `slug` route) were
  deferred to a follow-up question for the operator.
- **dracon-sync trailing-drain bug caused permanent skip
  of slow-push repos** (2026-06-15): the daemon's
  `in_flight: HashSet<PathBuf>` is supposed to prevent
  re-dispatching a repo while its `sync_repo` task is
  running. The trailing-drain phase drains leftover
  tasks with a 2s deadline. **On timeout, the
  unfinished tasks were dropped from `in_flight_tasks`
  (which goes out of scope) but their entries in
  `in_flight` were NEVER cleared.** The result: a slow
  sync task (e.g. a 60s push on `dracon-platform`)
  would stay in `in_flight` forever, causing the
  COLLECT phase of every subsequent cycle to skip the
  repo. The repo would never be processed again until
  the daemon restarted. Fix: track dispatched repos
  in a local `dispatched_this_cycle: HashSet<PathBuf>`,
  and on trailing-drain completion or timeout, clear
  any `in_flight` entries that were not drained. The
  daemon now logs `🔄 trailing-drain: clearing N stuck
  in_flight entries: {...}` when this happens. New
  regression test `test_trailing_drain_clears_stuck_in_flight`
  in `daemon.rs`. 1 regression test added. The fix was
  discovered during the `dracon-platform` push
  investigation (goal `fa84a5bd`); after deploying
  the fix, the daemon immediately committed 25 files
  to `dracon-platform` and pushed to all 3 remotes.
  Documented in
  `docs/design/dracon-platform-push-investigation-2026-06-15.md`.
  All 14 watched repos now show `✅ OK` and `healthy`.
- **dracon-sync unit tests polluted the live stuck-push
  ledger** with junk entries (`/tmp/.tmpXXXXX/test-repo`)
  from `tempfile::tempdir()` test fixtures. The push-failure
  tests in `daemon.rs` and `sync.rs` wrote to the real
  ledger at
  `~/.local/state/dracon/dracon-sync-stuck-push-repos.json`
  because they did not redirect `DRACON_SYNC_STATE_DIR` to
  a temp dir. The result: every `cargo test` run added
  fake entries to the operator's live ledger. Fix: 7
  unit tests now use `EnvRestorer::new("DRACON_SYNC_STATE_DIR",
  temp_dir.path())` to redirect ledger writes to a temp
  dir. Affected tests: `test_record_push_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBibDZ2OVhtTWRoYldOZjl4bVllQThiNFZFNTF0aFJMSThkcWZHRTFwSmpZCmY5eWJUZFppMzZlTG1tUnN3cE9ld3RZNU1VcVhZQVZIaGtqUGpJSko5RDAKLT4gWDI1NTE5IDk0Qmk1MUpzUW1lVTJRaXVmb0xKYzdMb09ZZXpMYTVKaU9kbWFpT1RieUkKR1hWc25jTDlqL0xrWmlkL0lOWWsxVGFIbUlPZUd3bm13TVJMdUEwZlpscwotPiBYMjU1MTkgZU9Gc2l2MUtjMU5CUmN2cTNleXJobm9NY1pqc0VYakpvcGJSOVd4UmJDQQpvNVNIRVhBSUlzWG1RMFdqdFBRRWFXZ2YxeDJVK3VuNXhwS0NRbjlseGxnCi0+IFgyNTUxOSBSVi9PWEdFNnE0aDdHUmFHekZ6ZGk2akVkbXp2U2w2U3ZWRzZXWDFHc2hFClhySEFkc29FRi9ROVpLMEY2NnlqUitxeFRiK2laWXNGSFdRRzhGV01qL2cKLT4gWDI1NTE5IGNBeWtXWGkvaTlMKzJ3dkYyWkFqcEErWldHbFlSa1FtNTZaZU5wMGNzbjgKemIrOE81S1hGeVJHb0NuLy9lY3FTcTF0WE14M2NQcDR1VXFZc0VxL3ZUbwotPiBPei1ncmVhc2UKVlhnbWVITDBrYmhnclRUUE1RMGd3aG5xMFdXUFYxWjRaaFpEUzJBVFROWERVVGRNUm5oeWtNSWY3eDJsNDUzQwpWQ3cKLS0tIEVCTjRqdkhjSEN2UExpcncySzlQOENlaXlxZWVOeE1GaFE5UGh5T2c2czQK7ZR0GDNx2GjSag6qE3XqPA9dPXuwE82NTrlJzduN5LgGh4cLCrDb57MH+pjYQIvbINv3WIW7qFkXROB5uoIIL3gI]`,
  `test_record_push_success_clears_entry`, `test_record_push_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBWMXR6cUJBdGRiUU4zSU1jRzFablllYmFjU2NOWFljRzhnbGpGRXhCNFI4CmtyN0lZZXJDWkVrWkE1Zmp2TzQrcnllL3NCQWtycVMrWnlDWUFJWHY0WFEKLT4gWDI1NTE5IEY5SC9SazhTKzBOSmVmQ2loQkFnNU5FNit5NFA1SlBTemE0WURGdXhoVmMKZHIzQWZqQWMrSVVDdzAvSzRjVHlZTEhiQVdhZitwelNQbmtmRVVlMVh0SQotPiBYMjU1MTkgN0FQUEZ5SWIwZDRKTHMyQld3Ri96d1ZBaTJrR3FQdHhNWjRDWWtBTnlTWQo1MkNjcFVRZElWYnRGQlZDcmJhaTA3SndDVFRIb3BMc2Fnb1ZabFRkU3BjCi0+IFgyNTUxOSBSL0ZlN04rZjlDR2lWdm9JZ1F2eml6OExjRnNvU3M5NlBWZjYwNWdRWFVvCksyOENjMGEydG9XSG9TUHJKTXl1a2RnUjFhcWhCVFdnK2wzcysyVTFpWTgKLT4gWDI1NTE5IHc0eUNRMWRpRFlXL1hzWTZCV01rdFJLM2Q2KysvckluTjZqSGxNdHJSUWcKeGNmTHNTSjlnaG84eHVIMHlJY2RFVVNseHV4RjJ2a044SGMyUGJ5a0R3RQotPiAlan1LJVMtZ3JlYXNlCk5hUVhUdndxQmdMdmpPTk84UDVGSzRBdkJxUlVQbWs2M3pUV1ZWYS9PTmNKclVoNHI4WHR4eWwvCi0tLSAvamxmQnhIdlQ2QUFiUXlZdTh2RGNDRVRXNm9NeDU2Zkw3RC96V25ZazNnClHWARpNMNa8FVm4yUzeYzdUy2pUFciTUYpzBF3apEyO7llsUBRO9JsUmjnPIGiAdLJnWL1zBMah25D04xgH]`
  (in `daemon.rs`); `test_sync_repo_mirror_push_failure_returns_false`,
  `test_sync_repo_mirror_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBaUlVWQStKbUNBc0xBVjdJZUs2a3p5UE5JNTNhbnRvbFdURWNKSlJPTFRrCkJsYTZFSURJT3NIdTRoRnhSd1o2RjBxOVNwM0dPbDc4U0JOOTFpc2hINTgKLT4gWDI1NTE5IDhUNnhPMHNLdmsxVVJ2NEpCeU5VQ2ErVEdMQmkvY2FJUW85ME9iWTI3VzgKRzJIczBtU3I0Z0lxNThEeis3TU80WWRDTWVWc3kxQnd0RUJldktCMGxodwotPiBYMjU1MTkgeFVlYm9MSjF6V3ZoWms0RkcxNFNUT0pnbUtqS0ZRVW5QWVl0UUJZRXFodwo4WHU4dkNmYzREcklVM0JEcWZHNG1nZHpmMnNPMm1laWRwNW5oZGFkOXdrCi0+IFgyNTUxOSBkaTh2R3BLb0IxL2FnT0d2Z3JEazMzenV2MjMrVEYyODFnaUtkOTIxZkVFCjNMY25xTjhwRE90a25kWDB1RjBMZC9vdG4yTU1Ba2NRUEthTTV5NmF4bWcKLT4gWDI1NTE5IFNrcTRJalpSU0NHUVZKb1A5OEh6Nko4WnpCZTNNY2xhcHhrU0RJSVQwaTQKT1gxdWpDdDBDTzVjT3Q3Q1QzaUtuQTF1TW0xNklvL2NqUHlMa3hvSHovYwotPiBETGZ9JWYtZ3JlYXNlIH5nJC1ULmAgem5uSTo3UDsKUWs0dDNzZkNWMlJOQW1yRkczT1ZNMWpkVmQ3cHlqZVFjUUFQU3cKLS0tIER2VkNMb3JMYUUxNUdBMHJlandub0UrQ0JOOGUxRFpjQXBTQXFteG9GRFEKmjuKG8YbQXvptpbyaBP08CssAmItLg965y563tBPiXGTCPldA/j6/g71J2Ny4HWl5wXUED7eNDhX]`,
  `test_sync_repo_mirror_push_success_returns_true`,
  `test_sync_repo_auto_github_private_graceful_on_no_gh`
  (in `sync.rs`). 19+ junk entries cleaned from the
  operator's ledger. No new tests added (the existing
  tests still pass; the change is to their setup, not
  their assertions). No regression in
  `cargo test --workspace --locked` (848 passed).
- **dracon-warden was encrypting source code**: the active
  `dracon-warden.toml` config had `protected_patterns` that
  included `*.rs`, `*.ts`, `*.py`, etc. (source code files).
  The SecretScanner's `Mistral API Key` regex
  (`mistral-[A-Za-z0-9_-]{20,}`) matched a public model ID
  `mistralai/[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBkOFV2aU5rM1g4VnRWZDhaUlllUGpqb3hkYmxZWWVWL2c5RjJyN0FrR1QwCkl6cFYyb0dMVGN2WDRVV2VIeXIwbkowTDdVNUdwSTl3RjZ0TVZWbVRrMDgKLT4gWDI1NTE5IHFvanpNaXFMd0NVcVJkbVhiQU9GOEc1WmtWYjNiQ2UvbDgwdXR5Z3B3bXMKSTJUa0N3dWNqdGtlSjl6VXBYN0NuOW9IL0VpTVJYQ2pnc3h0a1JOUmFFMAotPiBYMjU1MTkgN2hLd1R5blUySzNrRUdrVjVyWHZpNGsyNGZYeXEzaXdSRkVGRWp4MjRCUQpHSVJRbGx2QjRvN2lENHc1Y3Y5SE5neGNiZmJFRmhEc3NWQTZmOVcrSmxBCi0+IFgyNTUxOSBzTzFwV0ovdXFMY1UvcGgxR01oWnpLRnRFK2VGNS9WQnZrT05razlUZVgwCkdHWHlKRnJDM1prdCs1aU5uNndoN1Vla2FTd0hwaTNFWUFYWDZ3MlBQR3MKLT4gWDI1NTE5IGViNnJxZU1TM2ZSVnF5a3VaSHJYbGlvVVQ4aExTNEdpVm0vMUhHaWhjVEkKa2haajdlcGN5TVA2SFZlWXZFY2J4S1BydnBaUXlKQUJ6U2cyOXhSR3FlbwotPiBwLWdyZWFzZSA3NCMgKlggQC00CnpoTlJUQ0N6Y0JaKzJFTWVPbmpydlY5RjhIN1llTllqK3lzcHB2aW5oamM1N0xiWGJ3aGRZRm9SOWw0SEdLSDkKRm5JT3BJcHBZYjhUOGNHVkR3Um1pZEV0VG5JCi0tLSBhSWhXM201aHlRam1ObEQ4NDlnTnd1Rm1Wd0JUczB2VGkwSEdZeXZUa1BzCovumk5CzjhOpkcVXbqXrGDWYoHNm1eqcIrGNnxw0hCODblSk/sdYj+zEAw98Jfp2DeLldE+Pdb91FGJksXSpQ==]` in a
  TypeScript test file and encrypted it. The user reported
  this as a no-go via gibuardien: "we are encrypting code".
  Two-part fix:
  1. New `path_is_protected` helper in
     `dracon-warden/src/security/src/modules/filter.rs`
     implements glob-based matching of file paths against
     `protected_patterns`. Files that don't match any
     pattern are passed through unchanged — the scanner
     is NEVER invoked on them. The check is added to
     `smart_clean_with_path` BEFORE the existing
     sensitive-location logic.
  2. Removed all source code patterns from
     `~/.dracon/utilities/warden/dracon-warden.toml`'s
     `protected_patterns`. The list is now scoped to data
     files (`.env`, `*.pem`, `secrets/**`, etc.). A
     comment in the config explains the rationale.
  Restored the corrupted
  `browser-extensions-shared/extensions/vidpro-extension/test/components.test.ts`
  by replacing the encrypted blob with the original
  model ID `mistralai/[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBscnJXMURiclY5dFY3Wi9zcmVlbXNQZ2hpRGw5N2xncm9wNGZCYnduOTNjCmVDLytIWXJkNEhWNEVySGRuc1d6TFArQmY3VFpGcE9PNURaN2tKeWJHR1UKLT4gWDI1NTE5IHdFdzhmMzEyNDhzUU5UOGFBdkRuSFZ4OURXSmllMCtXTWc4SU4zOWNla1kKZ3hZeHdYWldKSkh6bGsyNWNPYnNYMzdsbUNSRU5uTW5kLzVlcG9xbHFQawotPiBYMjU1MTkgUlRLYTRueDk0N1U1WUd1M25nTXpNaEQ0Qm1Uczh3TFd6WnM2Q0dIaGpscwpqcStzL1FMY1I0bjJQNUhJdld0NjVxeG1Ta2pNSEpqUWJlTFZBTWFzb1A4Ci0+IFgyNTUxOSB5akIzS3pSSlhmY2E4bTVBODBNNVJOOWFhenU5MU5JNnU4Nm5OQSsrQVFFCit3R2RlYi9sV3Q5aHJPcldWdHpGNVBsYnVCMkt4RE5JY2JGVkRrc3VuSGsKLT4gWDI1NTE5IC9scGZPSno4S2dXK1lvRThxZnVobGFGc0NxbkZNREZ3bWtyM0d6cW1kQlUKVUFhb2tkekpVc0xKNnZOSGl1Vno2NThCa0xVOXl2NXlDdTZkMVJqQjRGTQotPiAmLWdyZWFzZSAjIUlTSSBIMSw8T3kKeW44M2ZDQ2J2NGUrSGF4NGRFemxGK05oMGd1anBib2FaS3U0NVM4b0pSSDRxbzJvTWxhVS9heXRWQjRZYlFybApzNG8KLS0tIE0zOWpaVmpOY3JWVmluKzlwcCs4U1VDUmdPSXZUbjRPWXYzakt5M29VWjAKbhDcYlgmobj63hglTXpXMWza5+dw5lLC/WmrARbduEGZCArwuv/y9ayEF+henHVAjgdkZO69J6Jm6ufqhCeb]`
  (verified via the sibling
  `utils/byokAdapter.ts` file). 6 new unit tests in
  `dracon-security`: `test_path_is_protected_*` (5) and
  `test_smart_clean_with_path_skips_unprotected_source_code`
  (1). Documented in
  `docs/design/source-encryption-incident-2026-06-15.md`.
  Audit of all 14 watched repos for source-code
  `DRACON_SECRET:YWdl` corruption returned only intentional
  test fixtures (warden's own encryption tests use
  encrypted blobs as test data).

### Added
- **Per-repo `owned` override investigation** (2026-06-15):
  after the operator asked to "default-skip non-owned repos"
  and then said "we def own kiki and others need exploring
  too", we ran `dracon-sync ownership --explain` on all 14
  watched repos and documented the results in
  `docs/design/ownership-investigation-2026-06-15.md`. All
  14 repos are correctly classified — 13 by signal-based
  detection, 1 by per-repo override (`dracon-ai-lib` whose
  upstream HEAD has a historical bad-config author). The
  per-repo `owned = true` override for
  `kiki-sassy-desktop-announcer` was added to bypass the
  earlier "Do not modify user-owned repos" constraint after
  the operator explicitly confirmed ownership. No
  misclassifications were found; no additional per-repo
  overrides are needed. The example config
  (`dracon-sync/dracon-sync.example.toml`) was updated with
  documented `owned = true` and `owned = false` per-repo
  override examples.

- **Ownership detection + default-skip safety guard rail**:
  the daemon now classifies each repo as `Owned`, `Unowned`,
  or `Unknown` based on three configurable signals:
  `git config user.email` ∈ `policy.trusted_emails`,
  HEAD author name/email ∈ `policy.trusted_authors`, and
  `origin` remote URL host ∈ `policy.trusted_remote_hosts`.
  When `auto_skip_unowned = true` (the safety-first default)
  and a repo is classified as `Unowned` or `Unknown`, the
  daemon logs `🚫 /path/to/repo skipping (<reason>): <detail>`
  once per cycle and does NOT issue any `sync_commit` or
  push to it. The `repos` table shows the new `🚫 unowned`
  STATE icon with a `run ownership --explain` HINT pointing
  the operator at the new `dracon-sync ownership` subcommand.
  This protects against repos whose `origin` is someone
  else's account (e.g. `zerostack-reference` →
  `gi-dellav/zerostack.git`) or whose HEAD author is a
  historical bad config (e.g. `dracon-ai-lib` →
  `Dracon <dracon@void>`). 11 new unit tests for the
  classification logic, 4 new policy field tests for
  defaults/serde/per-repo override, and 2 new end-to-end
  tests with `create_test_repo`. Per-repo override
  `RepoPolicyOverride.owned` and `auto_skip_unowned` re-enable
  the daemon for specific repos.

- **Settling max-delay + DirtyMaxAgeAction**: the daemon now
  force-commits a dirty repo that has been dirty continuously
  for > `settling_max_delay_secs` (default 60s) regardless of
  fingerprint stability. This prevents the "⏸ stalled Xm"
  pileup the operator was seeing — the user explicitly
  requested that the daemon "should be very actively
  committing". The 5s fingerprint-stability wait is still
  used for actively-edited repos (so the daemon doesn't
  commit on every keystroke). The max-age action is
  configurable via `policy.dirty_max_age_action`
  (`Commit` | `Warn` | `Ignore`); per-repo override via
  `RepoPolicyOverride.settling_max_delay_secs` and
  `dirty_max_age_action`. 5 new unit tests cover the
  defaults, the policy field serde, the per-repo override,
  and the test fixture regression. New `min_commit_interval_secs`
  policy field (default 5) exposes the per-repo commit
  rate-limit gate.

- **Push retry budget + push-stuck state**: the daemon now
  tracks `consecutive_failures` and `last_error` per stuck
  push repo in the on-disk
  `dracon-sync-stuck-push-repos.json` ledger. When
  `consecutive_failures >= push_max_retries` (default 5),
  the ACTIVITY column shows `🛑 push-stuck Xm (N ahead)`
  and the HINT column surfaces the actual `git push` error
  message (e.g. "Permission to gi-dellav/zerostack.git
  denied to DraconDev") with a `repair-concerns --apply`
  hint. The previous behavior was an opaque `🟣 pushing Xm`
  that could stay for days without telling the operator WHY
  the push wasn't landing. 3 new unit tests for the
  record/success/failure paths and 1 new ACTIVITY test
  for the push-stuck label. 9 new field tests for the
  `StuckRepoEntry` serde backward-compat (the new
  `consecutive_failures` / `last_error` / `last_error_at`
  fields default-fill from older JSON files).

- **Tightened in_flight staleness filter**: the on-disk
  `dracon-sync-in-flight.json` is now treated as stale
  after 5 seconds (was 30 seconds). The 30s window was
  leaking false `🔄 now` indicators on OK/idle/synced
  repos because the daemon writes the file every 1s and
  any entry from the past 30s was treated as "in flight".
  Combined with a new state-based suppression
  (`Synced`/`Idle`/`Cold`/`Untracked`/`Healthy` rows
  never show `🔄 now` even if the in_flight file lists
  them), the `🔄 now` indicator now reflects ground truth
  (the repo is currently being processed by a tokio task).
  3 new unit tests cover the 10s-old-stale case, the
  state-suppression case (Synced/Idle/Cold states never
  show `🔄 now`), and the Dirty state still showing
  `🔄 now` when legitimately in flight.

- **Auto-commit backstop for moving-target repos**: when a repo
  has more than `auto_commit_backstop_threshold` (default 20)
  unpushed commits AND the push has been pending
  (`ahead_since`) for more than
  `auto_commit_backstop_min_age_secs` (default 300s = 5 min),
  the daemon stops auto-committing and logs
  `⏸️ daemon backstop: N unpushed commits pending push >Xs,
  skipping auto-commit for <repo>`. This prevents the daemon
  from creating a moving target while a push is failing
  repeatedly (e.g. Junk-Runner-bevy had 2986 unpushed commits
  + 376 tracked test-result PNGs being committed every cycle,
  creating an infinite retry loop). Manual `git add`/`git
  commit` from the operator still works. Set
  `auto_commit_backstop_threshold = 0` in the policy to
  disable the backstop entirely. 5 new unit tests cover
  the below-threshold, above-threshold-but-recent, fully
  active, no-ahead-since, and threshold-zero (disabled)
  cases. Documented in
  `docs/design/dirty-files-investigation.md`.

- **Per-repo `auto_commit_exclude_patterns`**: a new
  per-repo TOML field that lets operators opt specific
  TRACKED file patterns out of auto-commit. Unlike
  `untracked_exclude_patterns` (which only applies to
  newly-added files), this applies to MODIFICATIONS of
  already-tracked files. Use case: a repo has 372
  Playwright screenshots force-tracked by `.gitignore`'s
  `!*.png` allowlist. Every test run regenerates them,
  and the daemon auto-commits each regeneration, creating
  a moving target the push can never resolve. Setting
  `auto_commit_exclude_patterns = ["**/test-results/**"]`
  in the repo's `.dracon/dracon-sync.toml` tells the
  daemon to skip those files. Default is empty (opt-in).
  Wired into `should_stage_entry` and
  `has_sync_relevant_dirty_entries`. 3 new unit tests
  cover the excluded-by-pattern, no-match, and
  empty-patterns cases. Documented in
  `docs/design/dirty-files-investigation.md` and
  `dracon-sync.example.toml`.

- **in_flight file staleness filter**: the on-disk
  `dracon-sync-in-flight.json` file used to make the
  ACTIVITY column show `🔄 now` could persist across
  cycles when a slow push from the previous cycle kept
  a repo in `in_flight` past the trailing-drain deadline.
  The `repos` command would then show `🔄 now` for repos
  whose pushes had completed (or stalled) while the daemon
  had moved on. The fix: `load_in_flight_for_path` now
  checks the file's `written_at` epoch and treats entries
  older than 30s as "stale → treat as empty". 2 new unit
  tests cover the stale and fresh cases. This complements
  the `⏸ stalled` and `🟣 pushing` labels so the ACTIVITY
  column always reflects ground truth.

- **Push stall fixes for `dracon-platform` (and similar)**:
  two related fixes that prevent the daemon from getting stuck
  in a "commit 1 file → push 28 commits → fail at 60s → HTTPS
  fallback → 3 min wait" loop.

  1. **Race condition in `stage_existing_files`**: build tools
     like vite create timestamp-suffixed temp files
     (e.g. `vite.config.ts.timestamp-1781483278562-...mjs`) and
     delete them within milliseconds. If `get_status()` lists
     such a path as untracked but the file is gone by the time
     `git add` runs, the whole `git add` fails with
     `fatal: unable to stat ...`, blocking every other file in
     the commit. The function now re-checks file existence
     right before staging and drops vanished files. Bare
     directory entries in the staging list are also filtered.
     3 new unit tests cover vanished files, directory
     entries, and the all-vanished no-op case.

  2. **Auto-scaled push timeout**: a fixed 60s idle timeout
     for `git push` is too short for a 28-commit push with
     binary test artifacts — git can sit in the negotiate
     phase for >60s before emitting any progress. The push
     timeout is now scaled with the local ahead count:
     `ahead ≤ 5` → base, `≤ 20` → 2x, `≤ 50` → 4x, `> 50`
     → 6x (capped at 600s = 10 min). Scaling is logged so
     operators can see when it kicks in (e.g.
     `⏫ Junk-Runner-bevy scaling push timeout 60s → 360s
     (2986 commits ahead)`). 5 new unit tests cover the
     small/medium/large/huge/zero-base scaling buckets.

  3. **ACTIVITY column now shows ahead count when pushing**:
     the `pushing` label now includes the unpushed-commit
     count (e.g. `🟣 pushing 4m (28 ahead)`) so operators
     can tell at a glance whether a stall is caused by a
     large backlog vs. a transient network blip.

- **ACTIVITY column now distinguishes active from stalled**: the
  `ACTIVITY` column in `dracon-sync repos` was previously a
  duplicate of the `LAST COMMIT` column (just the relative
  time of the last commit), which made it impossible to tell
  whether a row was "actively being processed" or "stalled"
  when many rows had the same timestamp. The new column shows
  one of: `🔄 now` (daemon has an in-flight task for this repo),
  `🟣 pushing Xm` (push in progress), `⏸ stalled Xm` (dirty &
  no daemon action for X minutes), `⏳ settling` (dirty & waiting
  for fingerprint stability), `🟢 synced Xm` (clean & recent),
  `⚪ idle Xh` (clean & waiting), `⚫ cold Xd` (no activity > 24h).
  The daemon now writes its `in_flight: HashSet<PathBuf>` to
  `~/.local/state/dracon/dracon-sync-in-flight.json` on every
  cycle, atomically (write-temp + rename), self-cleaning (file
  removed when set is empty), and removed on daemon shutdown.
  The `repos` command reads this file to render the new column.
  7 new unit tests in `dracon-sync/src/report.rs` for each
  ACTIVITY state. Legend updated to describe the new semantics.

- **No-redispatch invariant for parallel sync**: the daemon now
  tracks an `in_flight: HashSet<PathBuf>` consulted by the COLLECT
  phase's eligibility check. A repo with an active `sync_repo`
  task is not re-dispatched, even if the apply-phase deadline
  breaks out before the task completes. The APPLY phase removes
  repos from `in_flight` when their tasks complete; a bounded
  trailing drain (`pulse_interval_secs * 2`) clears the rest.
  This prevents duplicate `git push` invocations on the same
  `(repo, remote)` pair within a cycle window, which was causing
  a 2-3 minute "traffic jam" when 4 parallel pushes were
  competing for the SSH agent and network bandwidth. Live test:
  3 fresh dirty repos all commit+push in ~38s with no duplicate
  push attempts. New unit test `test_no_redispatch_invariant`
  in `dracon-sync/src/daemon.rs`. Documented in
  `docs/design/dirty-files-investigation.md`.

- **Bounded parallel sync**: `dracon-sync` daemon now dispatches
  `sync_repo` calls in parallel, bounded by the new
  `sem_max_concurrent_sync` policy field (default 4). Previously,
  the main loop was serial: a 60s `push_op_timeout_secs` on one
  slow repo (e.g. kiki-sassy github divergence, one-mil-girls
  gitlab) blocked all other repos from being committed and pushed.
  With 17 watched repos, a fresh dirty state on multiple repos
  now clears in ~35s instead of 10+ min. Live evidence captured
  in `docs/design/dirty-files-investigation.md`. The apply phase
  intentionally simplifies the deeply-nested stuck-ahead/behind
  /mirror notification logic; that gets restored in a follow-up.
  Set `sem_max_concurrent_sync = 1` to restore the original
  serial behavior. The apply phase is also bounded by a
  per-cycle deadline (`pulse_interval_secs * 2`, min 2s) so a
  slow push on one repo cannot block the next cycle indefinitely;
  unfinished tasks remain in the in-flight queue and are drained
  in subsequent cycles.

- **Canonical git identity section in `dracon-sync.example.toml`**: a
  new SECTION 12 documents the canonical `DraconDev <dracsharp@gmail.com>`
  profile, the global/per-repo resolution order, the search command
  to detect drift, and a note that the daemon does not rewrite
  identity at runtime. Operators should keep their profile consistent
  across `~/.gitconfig` and per-repo `.git/config` files.

- **All operator-owned warden pub keys are now tracked**: the
  previous goal tracked only `owner_nixos.pub`. This goal
  audits `~/.dracon/data/keys/`, force-tracks every operator-owned
  `*.pub` (`master.pub`, `micro2_git_key.pub`, `micro2_libs_key.pub`,
  `owner_age15xjl.pub`, `owner_age1f7y5.pub`, `owner_nixos.pub`),
  and pushes to all three public remotes. Private keys (`*.age`,
  `id_age`, `*.key`) remain blocklisted by `.gitignore`. The
  tracking rationale and recovery procedure are documented in
  `docs/design/owner-nixos-pub-tracking.md`.

- **`dracon-ai-lib` profile fix**: the per-repo
  `~/.git/config` had `user.name = Dracon` and
  `user.email = dracon@void`, which was a config drift from the
  canonical `DraconDev <dracsharp@gmail.com>`. Now matches the
  global gitconfig. Future commits from this repo will use the
  canonical profile.

- **`owner_nixos.pub` is now tracked in `dracon-utilities`**: the warden
  public key at `.dracon/data/keys/owner_nixos.pub` is committed and
  pushed to all three public remotes. Operators can recover the
  encryption key from git history if the local `.dracon/data/keys/`
  is ever lost. The `!.dracon/data/keys/*.pub` allowlist in the
  warden-managed `.gitignore` correctly force-tracks pub keys while
  keeping the private key (`*.key`, `id_age`) out of tracking.

- **`auto_stage_untracked` policy field**: a new `auto_stage_untracked`
  boolean (default `true`) and `untracked_exclude_patterns` list
  (default safe patterns for notes, scratch, audit, screenshots, etc.)
  in `dracon-sync.toml`. Together they make the daemon auto-stage
  newly-created untracked working files on the next sync cycle while
  keeping user notes, scratch research, and audit evidence
  permanently untracked. Set `auto_stage_untracked = false` to opt
  out completely. See `docs/design/dirty-files-investigation.md`
  for the per-file classification and the known cases where the
  dirty state persists longer than `inactivity_push_delay_secs`.

- **`dracon-sync repos` `STATE` column**: The `repos` table now includes a
  derived "STATE" column that combines last-commit time, last-push time,
  dirty state, ahead/behind, and push status into a small fixed
  vocabulary the user can scan at a glance. The vocabulary covers
  `working`, `committing`, `pushing`, `synced`, `stalled`, `dirty`,
  `untracked-only`, `intentional`, `failed`, `idle`, `cold`, and
  `healthy`. The `stalled` label specifically surfaces the
  "we changed files but then stopped" case the user asked about:
  dirty tracked/staged work older than `committing_commit_minutes`.
  Recent dirty work is labelled `dirty` so normal sync can pick it up
  after the configured settling delay without a red stalled alarm;
  `sync-now --warns` forces the same dirty-only triage immediately.
- **`dracon-sync repos` shows daemon activity**: added a new `DAEMON`
  column to the live `repos` table that shows the daemon's most recent
  recorded action per repo (e.g. `32s ago sync_commit ok`,
  `5m ago sync_triage ok`, `none`). This is sourced from the incident
  ledger and is wired into `sync_repo` so every auto-commit is recorded.
  The `last_when` / `last_push` columns reset to the moment of the
  daemon's own commit, so the `DAEMON` column closes the gap between
  "is the user still editing" and "is the daemon actively syncing".
  Thresholds
  (`active_commit_minutes`, `committing_commit_minutes`,
  `cold_commit_minutes`) live in the global policy with optional
  per-repo overrides in `RepoPolicyOverride`. The `--json` output
  includes the new `state_cause` and `state_cause_label` fields on
  every row. Documented in `docs/design/repos-state-cause.md`.

### Fixed
- **`dracon-sync` `STATE` semantics**: Recent dirty tracked/staged work
  now classifies as `dirty`, not `stalled`, so normal sync or
  `repair warns --apply` can pick it up without a red alarm. A repo only
  becomes `stalled` when tracked/staged work has sat longer than
  `committing_commit_minutes` without push progress. The `working` label
  means "the daemon is currently working through this repo" (clean, in
  sync, commit and push both within `active_commit_minutes`), not
  "the user is still editing right now". The `synced` label is the
  longer-term clean state (commit/push within `committing_commit_minutes`
  but outside the active window); the `working` vs `synced` split
  means the user can see at a glance which repos the daemon is
  currently working through versus which are merely in a long-term
  clean state.
  Documented in `docs/design/repos-state-cause.md`.

- **`dracon-sync repos` `STATE` docs clarified**: The design docs and
  example config now explain the live table meanings in user-facing
  terms: `idle` is the normal clean quiet state, `cold` is the
  >24h quiet state, `stalled` is dirty tracked/staged work with no
  unpushed commits, and `intentional` is the per-repo no-upstream
  opt-out.

- **`dracon-sync` `PUSHED` column missing for freshly-cloned repos**:
  The `last_push_for_branch` helper used `git reflog show origin/<branch>
  --format=%cr -1`, which returns empty output for repos whose
  remote-tracking reflog has no entries (i.e. freshly cloned and
  never re-fetched). The PUSHED column showed `-` for those repos
  even though the remote-tracking ref was perfectly valid. The helper
  now uses `git log -1 --format=%cr origin/<branch>`, which returns
  the committer date of the remote tip in both the populated-reflog
  and empty-reflog cases. Regression test added: builds a bare repo,
  seeds a commit, clones it, and asserts the helper returns a real
  date.

- **`dracon-sync` per-repo `intentional_no_upstream` opt-out**: A repo
  whose `.dracon/dracon-sync.toml` sets `intentional_no_upstream = true`
  is now recognized as intentionally isolated (e.g., a legacy private
  mirror that the operator no longer wants auto-tracked). The
  `repos` table replaces the `NO_UPSTREAM` flag with the explicit
  `INTENTIONAL_NO_UPSTREAM` flag, the `PUSHED` column shows
  `INTENTIONAL` (rendered green), and the hint says
  `"intentional legacy isolation, no upstream configured"`. The
  `dracon-sync repair concerns` command skips the repo entirely and
  the auto-repair path never runs `git push -u origin HEAD` for it.
  This is a logic defect (the previous "run repair-concerns --apply
  (set upstream)" hint was misleading for repos the operator has
  intentionally left unconnected). Documented as invariant #6 in
  `docs/design/sync-push-classification.md`.

- **`install.sh` dry-run daemon verification**: The running-daemon
  verification block used `local` at top level, which broke
  `./install.sh --dry-run` under shells where `local` is only valid
  inside functions. The service-name variable is now plain shell state.

### Fixed
- **`dracon-sync` system-repo path bug**: The example template's
  `system_repo` default pointed at a non-git legacy directory. The actual git
  repo where the sync daemon's state lives is `~/.dracon`. The example
  template and the installed `dracon-sync.toml` are now both set to the
  correct path.
- **`dracon-sync` `STUCK_PUSH` flag now requires a recorded push failure**:
  The flag used to fire on any `ahead > 0`, including repos the daemon
  had not yet tried to push in the current cycle. It now consults the
  incident ledger for a recent `result: "fail"` entry within the last
  10 minutes. Repos with unpushed commits but no recorded failure show
  as `PENDING` instead. This is a logic defect, not a behavioural
  change to the daemon's actual sync work. Refined in commits
  `1135d6bb8` and `bac8316cc`.
- **`dracon-sync` multi-remote push no longer retries permanent rejections**:
  Pushing to a GitLab/Codeberg protected branch, or any server-side
  `pre-receive hook declined` rejection, used to burn the full
  `push_retries` budget on an outcome that cannot change. The new
  `is_permanent_push_rejection()` classifier detects five canonical
  error patterns (`pre-receive hook declined`, `protected branch`,
  `not allowed to push`, `deny updating`, `hook declined`) and returns
  immediately on match, logging one incident per cycle. This is a
  logic defect, not a change to which remotes are pushed. Added in
  commit `fd93b943f`.
- **`dracon-sync` `repair concerns` aligned with `repos` table**: The
  repair command used the old `ahead > 0 → concern` rule after the
  `repos` table had been refined, so the two surfaces disagreed on
  which repos were concerns. Both now use
  `repo_is_concern_with_push_failure()`: a repo is a concern when it
  has no origin/upstream, or is `behind > 0`, or is `ahead > 0` AND has
  a recent push failure in the incident ledger. The `stuck-push`
  repair filter also uses the same recent-push-failure requirement as
  the table's `STUCK_PUSH` flag. This is a logic defect (inconsistency
  between two views of the same data). Fixed in commit `bac8316cc`.
- **`dracon-system` doctor "dracon-libs" check is now correctly labeled
  dev-only**: The check used to say "dracon-libs (sibling)" with no
  hint that it is optional for installed binaries. It is now labeled
  `dracon-libs (dev sibling)` and the remediation explains that the
  sibling is required only for `cargo build` from source. This is a
  logic defect (mislabeling an optional check as required). Fixed in
  commit `fd93b943f`.
- **`dracon-sync` stage cooldowns are now enforced**: The daemon previously
  inserted a `stage_cooldowns` entry after `git add` timeout but never
  consulted it on later cycles. It now skips repos with active cooldowns
  and removes expired entries, preventing repeated timeout attempts while
  the cooldown is active.
- **`dracon-sync` multi-remote push retries after HTTPS fallback failure**:
  A failed HTTPS fallback used to return immediately and skip the SSH
  retry loop. The retry loop now runs after fallback failure, so transient
  SSH failures can still recover.
- **`dracon-sync` origin push stops immediately on permanent rejections**:
  `push_with_retries()` and the lower-level transport fallback path now
  check `is_permanent_push_rejection()` before auto-pull/retry/fallback,
  matching the multi-remote push path and avoiding retry-budget burn on
  protected branches.
- **`dracon-sync` config validation now warns on unsafe timing/ledger
  values**: `stage_cooldown_secs`, `pull_op_timeout_secs`,
  `push_op_timeout_secs`, `repo_sync_timeout_secs`,
  `inactivity_push_delay_secs`, `repair_cooldown_secs`, and ledger
  retention values now warn before they cause incident-ledger spam or
  misleading push-failure windows.
- **`dracon-sync` recent-push-failure lookup now reads the ledger tail**:
  The `STUCK_PUSH` classification no longer scans the entire append-only
  incident ledger on every `repos` call. It reads a bounded tail window
  (500 lines) and still uses the same 10-minute `recent_push_failure`
  semantics.
- **`dracon-sync repair-warns` no longer uses a coarse sync timeout**:
  Large but healthy repos can exceed the legacy `repo_sync_timeout_secs`
  wrapper while individual git operations are still making progress. Warn
  repair now delegates to `sync_repo`'s per-operation timeouts instead of
  aborting the whole triage pass with a synthetic timeout.
- **`scripts/scaffold_feature_repos.py` `--init-git` flag for self-contained
  workflow**: Generates the façade files, initializes a local git repo,
  commits them with `--no-verify`, and adds `DraconDev/<name>` as the
  `origin` remote. The operator only has to `git push -u origin main` after
  the GitHub repository exists.
- **`scripts/scaffold_feature_repos.py` `--monorepo-root` defaults to the
  script's own directory**: The previous default of `Path.cwd()` only
  worked when the operator ran the script from the monorepo root. The new
  default is `--monorepo-root` resolves to the directory that contains
  `scripts/`, so the script behaves the same regardless of cwd. CLI flags
  still take precedence.
- **`dracon-sync repos` last-push query skips unsafe branch names**:
  `last_push_for_branch()` now short-circuits when the current branch is
  empty (detached HEAD) or contains shell-special characters that would
  break the `git reflog show origin/{branch}` argument. Previously the
  command was run unconditionally and the column silently showed "-".
- **`dracon-sync repos` `git log` subject parser preserves unit separators**:
  `parse_git_log_meta_line()` rejoins any extra unit-separated fields back
  into the subject, so a commit subject that itself contains `\x1f` is
  reconstructed verbatim instead of being truncated at the first extra
  field.
- **`dracon-sync repos` hint text now matches WARN vs CONCERN semantics**:
  A dirty repo with unpushed commits but no recent push failure is still
  `WARN`, so its hint now says the daemon will push after changes settle
  instead of suggesting `repair-concerns`.
- **`dracon-sync repos --json` keeps stdout machine-readable on repo failures**:
  Repo init/status failures are still counted and reported, but in JSON
  mode their human failure lines are sent to stderr so stdout remains valid
  JSON.
- **`dracon-sync` broken-tracking repair log now shows the real old
  tracking ref**: The startup repair used to print a fake
  `branch/branch -> origin/branch` message. It now parses the actual
  `[origin/master: gone]` ref and prints the real old/new mapping.
- **`dracon-warden` plaintext-sibling hatch checks now use the repo path**:
  `scrub-markers` and `resmudge` previously checked for `<file>.plaintext`
  relative to the current working directory. They now check under the repo
  being scanned, so the hatch works when the command is run from outside
  the repo.
- **`dracon-system` renice state now updates only after `renice` succeeds**:
  The guard used to record a PID as reniced even if the external `renice`
  command failed. It now treats renice failure as a failed action and does
  not update in-memory state.
- **Cargo.lock refreshed for the current dependency graph**: `cargo build
  --locked` failed because the committed lockfile was stale. Refreshing it
  keeps CI/build validation reproducible without requiring lockfile writes
  during validation.

### Added
- **GitHub utility feature façade scaffolding**: Added
  `scripts/scaffold_feature_repos.py` and
  `docs/design/github-feature-repos.md` so `dracon-sync`,
  `dracon-system`, and `dracon-warden` can be presented as separate GitHub
  feature surfaces without duplicating or moving implementation code out of
  the monorepo.
- **`dracon-sync` `stage_op_timeout_secs` policy field**: Configurable
  idle timeout (default 60s, min 10s) for `git add -A` and other
  staging operations on a single repo. The previous hardcoded 30s
  timeout was too tight for large repos (e.g.,
  `browser-extensions-shared` with 2500+ dirty paths took 88s) and
  caused the daemon to log a "staging timeout" incident on every
  cycle. The default of 60s gives headroom for typical work without
  making the daemon feel stuck. Added in commit `1135d6bb8`.
- **`dracon-sync` `stage_cooldown_secs` policy field**: When
  `git add` exceeds `stage_op_timeout_secs`, the daemon pauses
  further staging attempts on that repo for the configured duration
  (default 3600s = 1 hour). The point is to stop incident-ledger
  spam: a single repo that consistently times out will otherwise log
  a new incident every cycle. After the cooldown elapses, the daemon
  tries `git add` again; if it times out once more, the cooldown
  resets. The cooldown is per-repo; other repos are unaffected. Added
  in commit `00bba440d`.
- **`dracon-sync` push-rejection classification design note**:
  `docs/design/sync-push-classification.md` documents the
  `STUCK_PUSH` vs `PENDING` semantics, the 10-minute
  `recent_push_failure` window derived from the incident ledger, the
  `is_permanent_push_rejection` regex set, the retry policy, and the
  `repos` ↔ `repair concerns` invariant. Added in this release.


### Changed
- **CLI print style**: All three binaries (`dracon-sync`, `dracon-warden`,
  `dracon-system`) now use a consistent visual language for human-facing
  output. The `status` tables include a summary one-liner and grouped
  sections; byte counts and timeouts are formatted as human-readable
  (e.g. `50.0 MiB`, `1m 30s`); freeze/doctor indicators are coloured
  (suppressed when `NO_COLOR` is set). `dracon-system doctor` now emits
  per-check remediation hints. Design note:
  `docs/design/cli-print-style.md`.
- **CLI print polish (round 2)**: Four specific surfaces that were still
  weak have been upgraded. `dracon-sync repos` now has a legend line,
  multi-line icon+label headers, ✅/⚠️/❌ status cells, and a color-aware
  summary (no raw ANSI when piped). `dracon-sync health` now uses a single
  table with a summary one-liner; warnings are grouped into their own
  block with a count. `dracon-warden scrub-markers`/`resmudge`/`repair`/
  `keygen`/`setup-hooks` each print a 2-3 line informative summary, even
  when nothing was changed. `dracon-system events` shows a severity-counts
  footer and a one-line summary before the table. See the
  `docs/design/cli-print-style.md` design note for the full set of
  conventions.

### Added
- **Warden plaintext-sibling escape hatch**: `dracon-warden` now supports an
  opt-in escape hatch for files that should be stored verbatim (not encrypted).
  Touch a `<file>.plaintext` sibling next to any tracked file to opt it in.
  The clean filter returns the file unchanged, the pre-push hook silently
  skips it, and `scrub-markers` / `resmudge` leave it alone. Threat model,
  revocation story, and what the hatch does NOT protect against are in
  `docs/design/warden-plaintext-sibling.md`. Default install behaviour is
  unchanged: no hatch, no plaintext.
- **CI/CD pipeline**: `.github/workflows/ci.yml` — fmt check, clippy, build, serial tests
- **Lint gates**: `#![warn(missing_docs)]` on all 4 crate roots
- **dracon-libs docs**: Fixed all 95 missing-doc warnings in dracon-git
- **Module extraction** (dracon-system): `events.rs`, `links.rs`, `zram.rs`, `doctor.rs`, `safety.rs` — 850 lines, 20% main.rs reduction
- **Module extraction** (dracon-sync): `branch.rs`, `config.rs`, `diff.rs`, `discovery.rs`, `misc.rs`, `multi_remote.rs`, `ops.rs`, `push.rs`, `staging.rs`, `status.rs`, `urls.rs` — 1,846 lines, 45% git/mod.rs reduction
- **Startup cleanup**: Sync daemon prunes stale state on every start/restart — stuck repos, incident ledger retention, visibility cache orphans, guard log rotation
- **Broken tracking repair**: `repair_broken_tracking()` detects `origin/master: gone` refs and re-points to `origin/{branch}` — runs at daemon startup
- **GitHub orphan cleanup script**: `scripts/cleanup-github-orphans.sh` — lists and deletes 83 suffixed orphan + test repos (needs `delete_repo` scope)
- **dracon-libs get_diff fallback**: `get_diff()` now falls back to CLI on libgit2 errors (binary blobs, nul bytes)

### Changed
- **Dead code cleanup**: Removed `git_list_paths` (zero callers), `Level::as_str`/`Event`/`timestamp_secs` from log.rs (unused after JSON→human refactor), gated `fallback_status_rank`/`acquire_path_lock` with `#[cfg(test)]`, fixed all clippy unused-import/never-constructed warnings across all 3 crates
- **Scratch file cleanup**: Removed local task/scratch files and stale task directories from git tracking; added matching `.gitignore` rules.
- **Service restart policy**: All 3 services changed from `Restart=on-failure` to `Restart=always` — daemons now restart even after clean exits, preventing 5+ hour outages
- **CLI output style**: All status commands now use Title Case keys (`Policy:` not `POLICY:`) for consistency with JSON output and health check format
- **Daemon log noise**: Silent when healthy — concern/warn summaries only print when `found > 0`
- **Structured logging**: `log.rs` now prints human-readable `⚠️ message` to stderr instead of raw JSON — JSON incident records stay in the ledger file only
- **Link status**: Prints "No configured links" instead of empty table when 0 links exist
- **dracon-sync**: Scribe refactor — commit messages from diffs, not `project-state.md`
  - `generate_commit_message()`: AI receives current diff (main) + 10 previous diffs (background) + recent subjects → returns subject line
  - `local_fallback_message()`: file-pattern fallback (e.g., "update auth, jwt and 2 files") when AI unavailable
  - Removed `scribe_update()` and `stage_project_state()` — replaced by direct commit message generation
  - Removed `read_project_focus`, `extract_category_scope_from_focus`, `extract_scope_from_focus`, `git_log_recent_subjects`
  - `project-state.md` is now manual-only: sync no longer auto-generates, stages, or commits it
  - `parse_conventional_commit()` extracts (category, scope, description) from AI subject to prevent double-prefix
- **dracon-warden**: Secret scanner pattern fixes
  - Added "Hex Secret (Quoted)" pattern: catches 32+ char mixed-case hex strings in quotes
  - Added "High-Entropy Secret (Quoted)" pattern: catches 24+ char alphanumeric strings in quotes
  - Added "Slack Bot Token (Compact)" pattern: catches `Slack token prefixes without numeric ID segments
  - GitHub token patterns (`GitHub token prefixes): accept 30-40 chars (was exactly 36)
  - Mailgun API Key pattern: accept 28-34 chars (was exactly 32)

### Added
- **dracon-sync**: Mirror visibility sync (`sync_visibility` config)
  - Mirrors on Codeberg/GitLab automatically match GitHub's public/private status
  - Cache-gated: at most one API check per repo per `sync_visibility_interval_hours` (default: 24h)
  - `gh api` for GitHub reads, `curl` for GitLab/Codeberg writes
  - `strip_ansi()` helper for `gh api` JSON output (GitHub CLI injects color codes)
- **dracon-sync**: Mirror metadata sync (`sync_metadata` config)
  - Mirrors get GitHub's description and topics/tags synced automatically
  - Shares the same cache-gate as visibility sync
- **dracon-sync**: Three-toggle release pipeline (`auto_tag`, `auto_release`, `auto_publish`)
  - `auto_tag = true` (default on): Git tag `v{version}` on every version bump
  - `auto_release = false` (default off): GitHub Release on major bumps via `gh release create`
  - `auto_publish = []` (default empty): Publish to crates.io/npm/PyPI (per-registry opt-in)
  - All three require per-repo opt-in via `.dracon/dracon-sync.toml`
  - Dry-run safety: `cargo publish --dry-run` / `npm publish --dry-run` before real publish
  - Idempotent: skips if version already exists on registry
  - Non-fatal: publish failures log incidents but don't break the sync cycle
- **dracon-sync**: `publish` and `publish-status` CLI subcommands
  - `publish <repo>`: Manually publish to configured registries
  - `publish-status <repo>`: Check current version and registry status
- **dracon-sync**: `SyncOutcome` enum (`Synced`/`NothingToDo`/`Blocked`)
  - Replaces `Result<bool>` — daemon only increments failure count on actual errors
  - Clean repos no longer accumulate false failure counts
- **dracon-sync**: `GIT_ASKPASS` for GitLab/Codeberg HTTPS PAT push
  - Replaces URL-embedded `oauth2:TOKEN@` and `git:TOKEN@` patterns
  - Tokens no longer visible in process listings or logs
- **dracon-sync**: `effective_auth_type()` and `resolve_account()` on `RemoteConfig`
  - Auto-detects GitLab/Codeberg from push URL when `auth_type` not explicitly set
  - Extracts account name from push URL pattern for API calls
- **dracon-sync**: Permission checks on secrets directory
  - `load_secret` rejects world-writable directories and warns on world-readable files
- **dracon-sync**: AI major-bump cap — `parse_ai_bump_response` downgrades `Major` → `Minor`
  - Major version bumps require manual intervention
- **dracon-sync**: HTTPS+PAT fallback for GitLab and Codeberg pushes
  - `gitlab_https_url()` and `codeberg_https_url()` functions convert SSH URLs to HTTPS
  - `GITLAB_TOKEN` and `CODEBERG_TOKEN` used for authentication over HTTPS
  - Applied to both `push_to_named_remote` and `push_with_transport_fallbacks`
- **dracon-sync**: `GIT_TERMINAL_PROMPT=0` set on all git push commands
  - Prevents interactive SSH login prompts in daemon, CLI, and tests
- **dracon-sync**: Repo discovery optimization
  - `discover_git_repos_recursive` now skips descending into subdirectories of already-discovered repos
- **dracon-sync**: `HashSet`-based filter-only path matching
  - `git_diff_head_files` returns `HashSet<PathBuf>` for exact path matching
  - Prevents substring collision (e.g., `main.rs` matching `src/main.rs`)
- **dracon-sync**: Visibility cache uses repo path hash as key
  - Prevents same-name repo collisions across different watch roots
- **dracon-system**: `is_protected_ancestor` replaces exact-match path protection
  - `/home` now protects `/home/dracon/Dev`, `/etc` protects `/etc/nginx`, etc.
  - Root path `/` is exact-match only (prevents protecting everything)
- **dracon-system**: `auto_cleanup_apply` guard config (default: `false`)
  - Daemon runs cleanup in dry-run mode by default
  - `auto_truncate_logs` also gated behind `auto_cleanup_apply`
- **dracon-system**: Docker prune respects `apply` gate
  - `docker_prune(false, ...)` returns 0 without invoking docker
- **dracon-system**: PID verification before SIGKILL
  - Reads `/proc/{pid}/cmdline` after SIGTERM wait to confirm PID still belongs to same git process
- **dracon-system**: Strict git process command matching
  - Replaced substring `contains("git")` with `starts_with("git ")` + exact subcmd whitelist
- **dracon-system**: `expand_tilde` fallback changed from `/home` to `.` with logged warning
- **dracon-system**: `process_cpu_percent` default changed from `180.0` to `50.0`
- **dracon-warden**: Binary file passthrough in smudge filter
  - `is_binary_content()` detects null bytes; binary files pass through unchanged
- **dracon-warden**: Individual regex patterns with memory limits
  - `RegexBuilder` with `dfa_size_limit(1_000_000)` and `size_limit(10_000_000)`
  - Prevents ReDoS via catastrophic backtracking
- **dracon-warden**: Path-component matching for sensitive directories
  - `path_components_match` uses `.windows()` for multi-component dirs like `.config/gcloud`
  - `smart_clean_with_path` uses `path.components()` instead of substring `contains`
- **dracon-warden**: Exact filename matching (fixes `coreutils` false positive)
  - `starts_with("core")` replaced with exact match or `"{name}."` prefix

## [0.112.4] - 2026-06-07

### Fixed
- `dracon-sync/README.md` and `docs/OPERATIONS.md`: replaced flat CLI paths
  (`repair-concerns`, `repair-warns`, `stuck list`, `dual-branch list`,
  `publish-status`, `repair-origins`) with the correct nested subcommand
  syntax (`repair concerns`, `repair stuck-list`, `publish run`, etc.).
  Resolves audit-2026-06-07 P1-2.
- `dracon-warden status` help text and README "Quick Commands" sections
  now say "repo roots" (matching the v0.3.0 `watch_roots` → `repo_roots`
  field rename). Resolves audit-2026-06-07 N-4.
- `dracon-system/README.md` server-deployment systemd snippet: corrected
  resource limits from `MemoryMax=100M CPUQuota=10%` to `MemoryMax=250M
  CPUQuota=20%` (matching `dracon-system-guard.service`).
- Removed `dracon-sync/note.md` (leftover investigation note from a May
  incident). Added gitignore rule so future `note.md` files are not
  tracked. Resolves audit-2026-06-07 P2-5.
- Untracked 4 stale tarpaulin coverage reports (~1.6 MB) across all 3
  binaries. Added `**/tarpaulin-report.*` to `.gitignore`. Resolves
  audit-2026-06-07 P2-4.
- Removed dead `let discover = effective_discovery_roots(&policy);`
  binding in `dracon-warden/src/main.rs:1356` (the result was never
  used; `explicit_discover` was built directly from `policy.discover_roots`).

### Changed
- Workspace version bumped 0.112.3 → 0.112.4 (hygiene-only release, no
  per-crate version changes).
- `dracon-system/src/print.rs` and `dracon-warden/src/print.rs`: added
  module-level `#![allow(dead_code)]` with a doc comment explaining the
  public-API intent (helpers for shared output formatting, awaiting
  callers). Resolves audit-2026-06-07 N-1.

### Audit
- **Audit hygiene**: internal audit artifacts were reviewed during release prep and are not included in the public tree. User-facing release notes and operational docs now carry the public guidance.

## [0.3.0] - 2026-06-07

### Breaking
- **`dracon-warden` `watch_roots` field renamed to `repo_roots`**: The old
  name was misleading (warden has no daemon mode and does not watch
  filesystems; the field is a list of directories to scan for git repos
  on demand). The canonical field is now `repo_roots`. The example toml,
  user guide, and BLUEPRINT all use the new name.

### Deprecated
- **`watch_roots` is still accepted** for backwards compatibility. When
  the old key is set (alone or alongside `repo_roots`), the policy still
  loads, but:
  - A deprecation warning is logged to stderr:
    `warning: 'watch_roots' is deprecated, use 'repo_roots' instead`
  - A yellow ⚠ row appears in `dracon-warden status`
  - When both keys are set, `repo_roots` wins and a different message
    indicates the conflict
  This alias will be removed in a future major release.

### Fixed
- **`dracon-warden` status no longer shows two identical root rows**:
  Previously the status table showed `🛡️ Watch roots` and
  `🧭 Discovery roots` as separate rows that were identical when
  `discover_roots` was unset. The status is now consolidated to a single
  `🔍 Repo roots` row, with an explicit `🧭 Discovery roots (additional)`
  row only when the user has set a non-empty `discover_roots` that
  extends the `repo_roots` set.

### Changed
- **`dracon-warden` legacy path removed from default config**: The
  example toml and the installed user config no longer include a legacy
  non-git directory. The directory itself is not deleted; the user can
  decide what to do with its contents.

## [0.2.0] - 2024-05-03

### Added
- **dracon-system**: Guard daemon for disk/process monitoring
  - Disk usage thresholds (70/80/90/95%)
  - Auto-freeze dracon-sync at 90% disk usage
  - Auto-cleanup Rust target directories
  - Process CPU monitoring with notifications
  - Zombie process detection
  - Inode usage monitoring
- **dracon-warden**: Security hardening daemon
  - Git filter encryption for secrets
  - `DRACON_SECRET` marker support
  - `scrub-markers` recovery tool
  - `resmudge` working tree repair

### Changed
- Restructured as cargo workspace with separate crates

## [0.1.0] - 2024-04-28

### Added
- **dracon-sync**: Initial release
  - Auto-commit, auto-pull, auto-push
  - AI-powered commit messages (scribe)
  - Version bumping (ai-bumper)
  - Incident ledger for debugging
  - Stuck repo management
  - Dual-branch repair (main/master)
  - Orphan origin URL repair
  - GitHub private repo auto-creation
  - Multi-remote push support
  - Webhook notifications

---

## Versioning Notes

- **MAJOR**: Breaking changes to config format or CLI interface
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, documentation updates

