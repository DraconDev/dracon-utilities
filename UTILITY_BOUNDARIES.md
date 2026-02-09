# Dracon Utilities Boundaries

Deterministic by default. No AI runtime responsibilities in these tools unless explicitly stated.

Canonical library ownership is defined in `dracon-libs/docs/capability-boundaries.md`.

## Canonical runtime owners

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

## Utility roles (non-overlapping)

- `dracon-security`
  - Filter utility (`clean`/`smudge`) for protected file content transformation.
  - Not a daemon; not a repo sync orchestrator.
- `dracon-persistence` (legacy)
  - Retained only for state relocation/symlink repair flows.
  - Sync/daemon behavior is deprecated in favor of `dracon-sync`.
- `dracon-ai`
  - Optional AI utility, separate from sync/warden/system deterministic runtime loops.

## De-dup policy

- Do not introduce another daemon that auto-commits repos outside `dracon-sync`.
- Do not introduce another watcher that enforces protected path policy outside `dracon-warden`.
- Keep system cleanup/health logic in `dracon-system`.
- Keep reusable capability logic in `dracon-libs`; utilities are wrappers/orchestrators.
- Protected branches should use one commit voice (`dracon-sync`) with deterministic JSON payloads.

## Naming + transition policy

- `demon-*` naming is legacy and should not be used for new binaries/crates.
- Active runtime binaries are `dracon-sync`, `dracon-warden`, and `dracon-system`.
- `dracon-security` is a filter utility and not an overlapping daemon.
