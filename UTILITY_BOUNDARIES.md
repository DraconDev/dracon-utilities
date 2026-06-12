# Dracon Utilities Boundaries

Deterministic by default. These tools do not depend on AI runtimes for commit messages, release decisions, or safety enforcement.

Canonical library ownership is defined in `dracon-libs/docs/capability-boundaries.md`.

## Canonical runtime owners

The public release owns exactly three utilities:

- `dracon-sync`
- `dracon-warden`
- `dracon-system`

Each utility has a narrow runtime role:

### `dracon-sync`

- Owns git sync automation: watch roots, pull/commit/push, deterministic commit payloads, freeze toggle, and mirror pushes.
- Commit generation is deterministic only; AI commit generation is out of scope.
- Policy path: `~/.dracon/utilities/sync/dracon-sync.toml`.
- Incident ledger: defaults to `~/.local/state/dracon/dracon-sync-incidents.jsonl` and stays out of repos to avoid perpetual dirty state. Override with `DRACON_SYNC_LEDGER`.
- Required policy controls include:
  - `exclude_dir_names` for repo discovery and staging exclusions.
  - `max_stage_file_bytes` for large-file staging guard.
  - `pull_op_timeout_secs` and `push_op_timeout_secs` for remote latency tolerance without false stuck signals.

### `dracon-warden`

- Owns security hardening and git filter enforcement.
- Policy path: `~/.dracon/utilities/warden/dracon-warden.toml`.
- Secret invariants:
  - Files matched by secret patterns are encrypted at rest in git via filter, while the working tree remains plaintext through smudge.
  - Tracked plaintext JSON must never contain `[DRACON_SECRET:...]` markers; those indicate a leak path.
- Recovery commands:
  - `dracon-warden once`
  - `dracon-warden scrub-markers --apply`
  - `dracon-warden resmudge --apply`

### `dracon-system`

- Owns system diagnostics, storage analysis/cleanup, link reconciliation, zram reporting, and guard health checks.
- Policy path: `~/.dracon/utilities/system/dracon-system.toml`.
- Link reconciliation is opt-in through `[links]` policy. The public default does not assume legacy compatibility links or `~/.config/dracon` linkage.

## Utility roles are non-overlapping

- `dracon-security` was a transitional artifact and is not a runtime utility.
- `dracon-persistence` was a transitional artifact and is not a runtime utility.
- `dracon-code` is a separate coding-workflow product. It may consume AI runtime crates from `dracon-libs`, but it does not own sync, warden, or system runtime roles.

## De-dup policy

- Do not introduce another daemon that auto-commits repos outside `dracon-sync`.
- Do not introduce another watcher that enforces protected path policy outside `dracon-warden`.
- Keep system cleanup and health logic in `dracon-system`.
- Keep reusable capability logic in `dracon-libs`; utilities are wrappers and orchestrators.
- Protected branches should use one deterministic commit voice: `dracon-sync`.
- Do not move `dracon-code` concerns into `dracon-utilities`.

## Naming and transition policy

- Old branding prefixes are legacy and should not be used for new binaries or crates.
- Active utility binaries are `dracon-sync`, `dracon-warden`, and `dracon-system`.
- User services are `dracon-sync.service` and `dracon-system-guard.service`.
- `dracon-warden` has no daemon service; git hooks are the primary enforcement layer.
- `dracon-ai` was removed from this repo as an orphaned CLI wrapper; AI runtime crates remain in `dracon-libs`.
