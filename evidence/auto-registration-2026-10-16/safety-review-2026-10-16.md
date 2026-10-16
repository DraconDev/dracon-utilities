# Safety review — automatic bucket asset discovery and registration

Date (UTC): 2026-10-16
Source range (dracon-sync): `20893c8..HEAD` (HEAD `dd7681b` at review time;
plus task-6 additions `scripts/qualify-live-enrollment.mjs`,
`scripts/verify-live-enrollment.sh`, live-enrollment evidence — same scope)
Reviewer: executor (independent contract review by a fresh reviewer agent
follows this document; see task 6).

## 1. Scoped final diff

22 files, all inside four paths. Zero changes outside them:

- `scripts/bucket-runner/` — 9 files: adapters, capabilities, planner,
  publisher + their tests, e2e harness (new, standalone; nothing imports
  them from `src/`, no daemon wiring).
- `scripts/` root — 4 files: `inventory-bucket-discovery.py` (+ its test),
  `verify-bucket-registration-e2e.sh`, `verify-live-enrollment.sh`,
  `qualify-live-enrollment.mjs`.
- `docs/design/automatic-bucket-registration-2026-10-16.md` — design record.
- `evidence/auto-registration-2026-10-16/` — logs, fleet inventory, live-run
  artifacts.

In particular the diff contains:

- **No `src/` changes.** `git diff 20893c8..HEAD -- src/` is empty. No Rust
  product code, no daemon scheduling, no sync/push/report paths touched.
- **No hook changes.** No `pre-push`, `pre-commit`, `filter.dracon` or
  `core.hooksPath` content touched. The warden hook enforcement stack is
  byte-identical before/after.
- **No credential changes.** No `.age`/`.pem`/`.key`/`secrets/`/credential
  files touched; no `uploaderCredentialFile`/`consumerCredentialFile`
  created. The capability profile references `/operator/*.env` handles that
  do not exist on this machine; `loadProfile`/`verifyInstalledRunner` are
  exercised only against `/tmp` fixtures in tests.
- **No `bucket-assets.json` created or modified anywhere in the fleet.**
  The daemon's explicit opt-in workflow (`src/bucket_assets.rs`) is
  therefore inert: "No configuration => no work."
- **No `.dracon/dracon-sync.toml` or watch-root changes.** No repo gained
  or lost `auto_commit_exclude_patterns`, ownership or scheduling knobs.
- **No history rewrite.** No `filter-repo`/`filter-branch`/`rebase --onto`/
  `--force` push in the range; the warden pre-push hook blocked nothing
  because nothing non-fast-forward was attempted. Daemon backup-branch
  scan shows no new `backup/pre-sync-largeblob-fix-*` branches.

## 2. Forward-only bucket guards preserved and live-firing

The existing guard (`src/bucket_guard.rs`, untouched) continued to fire
during this goal's deployment window. Post-restart journal (task 5,
`quality-gates/post-restart-journal.log`):

- `neonbreak`: guard refused auto-commit of staged
  `static/audio/factions-choose.mp3` + `hover-stinger.mp3`
  (`BUCKET_STRATEGY_GUARD_FORWARD_ONLY`); staged/worktree content retained.
- Unrelated repos (`polis`, `hellhunter`, `browser-extensions-shared`,
  `dracon-platform`) committed normally in the same window.

No guard was weakened, bypassed, or reconfigured to admit the new
component: the bucket-runner never stages, never pushes, never writes
outside its caller-supplied `manifestPath`/`indexPath`/cache dir.

## 3. Operator edits, encryption, tracked assets

- No operator file was modified by any goal command. All mutations are new
  files under `scripts/bucket-runner/`, `docs/design/` and `evidence/`.
- No tracked asset was retired, renamed, or deleted. The publisher's
  rename/delete handling was qualified in-process (legacy entries preserved
  with `bucketed:true`); no fleet manifest exists to apply it to.
- Warden encryption flow untouched: no smudge/clean filter, index-batch or
  hook-installer changes. `cargo test --workspace` (2430 passed) includes
  the warden storage-stream suites.
- The live-enrollment demo ran in `/tmp/live-qual-*` scratch repos only,
  removed afterward (`rm -rf` in the script). No watched worktree received
  a manifest, index, or asset from this goal.

## 4. Credential privilege boundary

Explicitly NOT expanded. The goal never created IAM credentials, never
touched `~/.aws`, `~/.config/gh`, `~/.git-credentials`, KWallet, or any
OVH credential handle. Live OVH writes are impossible from the delivered
code path without an operator-installed profile + credential files, which
do not exist. The live-enrollment demo labels its transport
`in-process-stub (no live bucket write)` in both the log and the summary
artifact.

## 5. What is deliberately NOT claimed

- Daemon-integrated autonomous triggering is **not implemented**: the
  bucket-runner is a qualified standalone component; scheduling it from
  `src/daemon.rs` is a separate lane gated on the operator rollout
  decision (design doc "Component ownership", lane 2).
- No live bucket object was created, listed, or read. "Live" in the
  task-6 demo means: real timestamps, live filesystem, real git repo,
  operational (non-test-runner) execution, concurrent live daemon doing
  unrelated fleet work — with a stub transport, honestly labeled.
- Fleet enrollment state is unchanged: all 34 watched repos remain
  `not_qualified` per `fleet-initial.json`; no repo was enrolled.

## 6. Rollback

- Binary rollback: `~/.dracon/backups/dracon-sync-pre-bucket-e2e-20261016T230817Z/dracon-sync`
  (pre-install SHA `8f7f45c9…`), outside all watched trees, with manifest.
- Source rollback: every goal commit is forward-only on `main`; reverting
  the 22 files restores the pre-goal tree (`git revert` or checkout of
  `20893c8` for those paths). No migration, no schema change, no state
  file format change accompanies the goal.
