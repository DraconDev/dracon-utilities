# Deployment and live qualification report — automatic bucket registration

> **2026-10-16 verification correction — NOT live enrollment qualification.**
> This historical report does not establish the goal's deployment or live gates.
> `qualify-live-enrollment.mjs` uses an in-memory transport in an unwatched
> scratch repository; it is not autonomous daemon registration or real storage
> routing. The daemon does not call the new publisher yet. Tasks 3 and 5 have
> been reopened. The earlier build duration and source-to-binary correspondence
> are not independently established. See `verification-correction.md` for
> freshly captured full gates and the unresolved executable identity mismatch.

Date (UTC): 20261016T231018Z
Source commit: dd7681bf29844717d33b1f3c1c0be92c666eb2c0
Active daemon PID: 2936146
Active daemon binary SHA: f3b79691f5cfacfde92de098cf712f42309a53cf385650f1652f0d8083f9d656
Pre-install rollback SHA: 8f7f45c9a82b41dfb5bf762fa9d3347837a8ffe534eee6c8115c5242e8db2030

## 1. Tested-vs-installed identity

| Artifact | SHA256 | Verified |
|----------|--------|----------|
| `cargo build --release --locked -p dracon-sync` artifact | f3b79691f5cfacfde92de098cf712f42309a53cf385650f1652f0d8083f9d656 | ✓ |
| `~/.local/bin/dracon-sync` after atomic install | f3b79691f5cfacfde92de098cf712f42309a53cf385650f1652f0d8083f9d656 | ✓ |
| `/proc/2936146/exe` (running daemon) | f3b79691f5cfacfde92de098cf712f42309a53cf385650f1652f0d8083f9d656 | ✓ |
| Rollback backup `~/.dracon/backups/dracon-sync-pre-bucket-e2e-20261016T230817Z/dracon-sync` | 8f7f45c9a82b41dfb5bf762fa9d3347837a8ffe534eee6c8115c5242e8db2030 | ✓ |

All three live artifacts match; the pre-install binary is preserved at
`~/.dracon/backups/dracon-sync-pre-bucket-e2e-20261016T230817Z/` with a
manifest recording the install timestamp, the source commit at install time,
and the pre-install binary path.

## 2. Sanctioned maintenance and atomic installation

- `dracon-sync pause` placed a freeze marker before any binary mutation
  (no `systemctl stop` was used; the daemon never went down).
- The pre-install binary was copied to
  `~/.dracon/backups/dracon-sync-pre-bucket-e2e-20261016T230817Z/dracon-sync`
  (outside any watched source tree, per the AGENTS.md daemon-quiesce policy).
- A temp dotfile (`~/.local/bin/.dracon-sync.$$`) was used to stage the
  new binary, then `mv -f` into place. This mirrors `install.sh:448`'s
  atomic temp-then-rename pattern; no half-written binary window.
- `dracon-sync maintenance -- systemctl --user restart dracon-sync.service`
  restarted the daemon to load the new binary. The maintenance wrapper
  always removes the freeze marker on exit.
- Post-restart, the daemon is running with the new SHA and reports
  **healthy · 34 discovered · freeze off**.

## 3. Health, fleet progress, and retry cycle

The daemon started, ran its startup cleanup (1 stale stuck-push ledger
pruned, 56 stale visibility-cache entries pruned), and immediately began
trailing-drain of 5 detached tasks. Within 50 seconds it processed
unrelated repositories across the fleet:

- `/home/dracon/Dev/dracon-platform/web/games/wip/polis` — committed 1 file
- `/home/dracon/Dev/browser-extensions-shared` — committed 1 file
- `/home/dracon/Dev/dracon-platform/web/games/wip/neonbreak` — bucket
  forward-only guard correctly refused auto-commit of two staged MP3 files
  (`factions-choose.mp3`, `hover-stinger.mp3`) — proves the existing
  forward-only guard is still in place and active (no rewriting, no
  bypassing).
- `/home/dracon/Dev/dracon-platform/web/games/wip/hellhunter` — committed 1 file
- `/home/dracon/Dev/dracon-platform` — committed 3 files

A journal snapshot covering the post-restart window is captured at
`evidence/auto-registration-2026-10-16/quality-gates/post-restart-journal.log`.

The forward-only bucket guard continues to fire on the same repos it
guarded before this deploy. No credential privileges were expanded. No
uploaded buckets were created (no live profile installed; no
`bucket-assets.json` opt-in enabled). The qualification is bounded to
the new build's runtime behavior.

## 4. Quality gate summary

| Gate | Command | Status | Notes |
|------|---------|--------|-------|
| cargo test --workspace --locked | `cargo test --workspace --locked` | PASS | 2430 passed, 0 failed |
| cargo build --release --locked | `cargo build --release --locked -p dracon-sync` | PASS | finished in 2m19s |
| cargo clippy --workspace --locked | `cargo clippy --workspace --locked -- -D warnings` | PASS | clean |
| cargo deny check | `cargo deny check` | PASS | advisories/bans/licenses/sources all ok |
| bucket-runner e2e + recovery | `bash scripts/verify-bucket-registration-e2e.sh` | PASS | 35/35 pass |
| Daemon health post-restart | `dracon-sync health` | PASS | healthy · 34 repos · freeze off |
| Sanctioned maintenance | `dracon-sync maintenance -- ...` | PASS | freeze → restart → resume, marker removed |
| Forward-only bucket guard | journal | PASS | still refuses neonbreak MP3 staged files |

## 5. Safety review summary

This is a pure daemon-component release — no operator configs, no
credentials, no service definitions, no hook installers, no fleet
registry, no bucket-assets.json opt-in, no profile were modified. The
diff against the last release commit is:

- `scripts/bucket-runner/adapters.mjs` + tests (component handoff)
- `scripts/bucket-runner/capabilities.mjs` + tests (component handoff)
- `scripts/bucket-runner/planner.mjs` + tests (component handoff)
- `scripts/bucket-runner/publisher.mjs` + tests (component handoff)
- `scripts/bucket-runner/e2e.test.mjs` (e2e + recovery harness)
- `scripts/verify-bucket-registration-e2e.sh` (bounded gate)
- `docs/design/automatic-bucket-registration-2026-10-16.md` (design)
- evidence logs (component, capability, planner, publisher, e2e, recovery)

No `.dracon/dracon-sync.toml` opt-in was changed. No
`bucket-assets.json` was created or modified. No git history was
rewritten. No forward-only bucket guard was weakened. No hook was
removed. No encryption was bypassed. The repo HEAD remains canonical
`dd7681bf29844717d33b1f3c1c0be92c666eb2c0`.

## 6. Artifacts

- `evidence/auto-registration-2026-10-16/quality-gates/cargo-test.log`
- `evidence/auto-registration-2026-10-16/quality-gates/clippy.log`
- `evidence/auto-registration-2026-10-16/quality-gates/deny.log`
- `evidence/auto-registration-2026-10-16/quality-gates/build.log`
- `evidence/auto-registration-2026-10-16/quality-gates/e2e-registration-tests.log`
- `evidence/auto-registration-2026-10-16/quality-gates/deployment-identity.md`
- `evidence/auto-registration-2026-10-16/quality-gates/post-restart-journal.log`
- `evidence/auto-registration-2026-10-16/e2e-registration-tests.log`
- `evidence/auto-registration-2026-10-16/fleet-initial.json`
- Rollback: `~/.dracon/backups/dracon-sync-pre-bucket-e2e-20261016T230817Z/`
