# Dracon Utilities Boundaries

Deterministic by default. No AI runtime responsibilities in these tools unless explicitly stated.

Canonical library ownership is defined in `dracon-libs/docs/capability-boundaries.md`.

## Canonical runtime owners

- Core utility set is exactly three: `dracon-sync`, `dracon-warden`, `dracon-system`.
- `dracon-sync`
  - Owns git sync automation (watch roots, pull/commit/push, deterministic commit payloads, freeze toggle).
  - Commit generation is deterministic only; AI commit generation is out-of-scope.
  - Policy path: `/home/dracon/dracon/utilities/sync/dracon-sync.toml`.
  - Operating model doc: `/home/dracon/dracon/utilities/sync/AI_SYNC_MODEL.md`.
  - Required policy controls:
    - `exclude_dir_names` for repo discovery + staging exclusions.
    - `max_stage_file_bytes` (default 104857600 / 100 MiB) for large-file staging guard.
- `dracon-warden`
  - Owns security hardening/watcher behavior (managed `.gitignore`/`.gitattributes`, protected paths).
- `dracon-system`
  - Owns system diagnostics + storage analysis/cleanup + service health checks.
  - Owns setup symlink reconciliation via explicit `[links]` policy in `/home/dracon/dracon/utilities/system/dracon-system.toml` (default: no legacy compatibility links and no `~/.config/dracon` linkage).

## Utility roles (non-overlapping)

- `dracon-security` (legacy, deprecated)
  - Old transitional utility from pre-split architecture.
  - Not part of the canonical runtime model; scheduled for removal.
- `dracon-persistence` (legacy, deprecated)
  - Old transitional utility from pre-split architecture.
  - Not part of the canonical runtime model; scheduled for removal.
- `dracon-ai`
  - Optional AI utility, separate from sync/warden/system deterministic runtime loops.
- `dracon-code` (planned)
  - Optional coding/automation orchestrator utility.
  - May consume `dracon-ai`, but does not own sync/warden/system runtime roles.

## De-dup policy

- Do not introduce another daemon that auto-commits repos outside `dracon-sync`.
- Do not introduce another watcher that enforces protected path policy outside `dracon-warden`.
- Keep system cleanup/health logic in `dracon-system`.
- Keep reusable capability logic in `dracon-libs`; utilities are wrappers/orchestrators.
- Protected branches should use one commit voice (`dracon-sync`) with deterministic JSON payloads.

## Naming + transition policy

- old branding prefixes are legacy and should not be used for new binaries/crates.
- Active runtime binaries are `dracon-sync`, `dracon-warden`, and `dracon-system`.
- `dracon-security` and `dracon-persistence` are deprecated legacy artifacts, not runtime owners.
