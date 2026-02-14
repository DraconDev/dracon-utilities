# Dracon Utilities Boundaries

Deterministic by default. No AI runtime responsibilities in these tools unless explicitly stated.

Canonical library ownership is defined in `dracon-libs/docs/capability-boundaries.md`.

## Canonical runtime owners

- Core utility set is exactly three: `dracon-sync`, `dracon-warden`, `dracon-system`.
- `dracon-sync`
  - Owns git sync automation (watch roots, pull/commit/push, deterministic commit payloads, freeze toggle).
  - Commit generation is deterministic only; AI commit generation is out-of-scope.
  - Policy path: `/home/dracon/dracon/utilities/sync/dracon-sync.toml`.
  - Incident ledger: defaults to `~/.dracon/dracon-sync-incidents.jsonl` (kept out of repos to avoid perpetual DIRTY state). Override with `DRACON_SYNC_LEDGER`.
  - Operating model doc: `/home/dracon/dracon/utilities/sync/AI_SYNC_MODEL.md`.
  - Required policy controls:
    - `exclude_dir_names` for repo discovery + staging exclusions.
    - `max_stage_file_bytes` (default 52428800 / 50 MiB) for large-file staging guard.
    - `pull_op_timeout_secs`, `push_op_timeout_secs`, `repo_sync_timeout_secs` for remote latency tolerance without false "stuck" signals.
- `dracon-warden`
  - Owns security hardening/watcher behavior (managed `.gitignore`/`.gitattributes`, protected paths).
  - Policy path: `/home/dracon/dracon/utilities/warden/dracon-warden.toml`.
  - Secret invariants:
    - Files on `protected_patterns` are encrypted-at-rest in git (via filter), but plaintext on disk via smudge.
    - Tracked plaintext JSON must never contain `[DEMON_SECRET:...]` / `[DRACON_SECRET:...]` markers (they indicate a secret leak path).
  - Auto-repair:
    - `dracon-warden once` and `dracon-warden daemon` automatically run the marker scrub pass before hardening.
    - Manual command: `dracon-warden scrub-markers --apply`.
- `dracon-system`
  - Owns system diagnostics + storage analysis/cleanup + service health checks.
  - Owns setup symlink reconciliation via explicit `[links]` policy in `/home/dracon/dracon/utilities/system/dracon-system.toml` (default: no legacy compatibility links and no `~/.config/dracon` linkage).

## Utility roles (non-overlapping)

- `dracon-security` (removed runtime utility)
  - Legacy transitional utility removed from canonical runtime.
- `dracon-persistence` (removed runtime utility)
  - Legacy transitional utility removed from canonical runtime.
- `dracon-ai`
  - Optional AI utility, separate from sync/warden/system deterministic runtime loops.
- `dracon-code` (planned)
  - Optional coding workflow utility (repo scaffolding + context persistence).
  - Owns `do.md` + `plan/` conventions for "git as AI version control".
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
- `dracon-security` and `dracon-persistence` are removed runtime artifacts, not runtime owners.
