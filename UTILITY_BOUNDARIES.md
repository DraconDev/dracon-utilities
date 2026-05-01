# Dracon Utilities Boundaries

Deterministic by default. No AI runtime responsibilities in these tools unless explicitly stated.

Canonical library ownership is defined in `dracon-libs/docs/capability-boundaries.md`.

## Canonical runtime owners

- Core utilities in this repository are exactly four:
  - `dracon-sync`
  - `dracon-warden`
  - `dracon-system`
  - `dracon-ai`
- Utility classes:
  - Always-on service utilities: `dracon-sync`, `dracon-warden`, `dracon-system`
  - Interactive utility: `dracon-ai`
- `dracon-sync`
  - Owns git sync automation (watch roots, pull/commit/push, deterministic commit payloads, freeze toggle).
  - Commit generation is deterministic only; AI commit generation is out-of-scope.
  - Policy path: `/home/dracon/dracon/utilities/sync/dracon-sync.toml`.
  - Incident ledger: defaults to `~/.local/state/dracon/dracon-sync-incidents.jsonl` (kept out of repos to avoid perpetual DIRTY state). Override with `DRACON_SYNC_LEDGER`.
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
    - Tracked plaintext JSON must never contain `[DRACON_SECRET:...]` markers (they indicate a secret leak path).
  - Auto-repair:
    - `dracon-warden once` and `dracon-warden daemon` automatically run the marker scrub pass before hardening.
    - Manual command: `dracon-warden scrub-markers --apply`.
- `dracon-system`
  - Owns system diagnostics + storage analysis/cleanup + service health checks.
  - Owns setup symlink reconciliation via explicit `[links]` policy in `/home/dracon/dracon/utilities/system/dracon-system.toml` (default: no legacy compatibility links and no `~/.config/dracon` linkage).
- `dracon-ai`
  - Owns interactive machine-task assistance (planning + command execution loop and AI query UX).
  - Is not a deterministic background daemon.
  - Consumes AI routing/runtime/secrets from `dracon-libs`; does not own provider wiring.
  - Must not own sync/warden/system daemon responsibilities.

## Utility roles (non-overlapping)

- `dracon-security` (removed runtime utility)
  - Legacy transitional utility removed from canonical runtime.
- `dracon-persistence` (removed runtime utility)
  - Legacy transitional utility removed from canonical runtime.
- `dracon-ai`
  - Interactive utility, separate from sync/warden/system deterministic runtime loops.
- `dracon-code`
  - Optional coding workflow utility (repo scaffolding + context persistence).
  - Owns `do.md` + `plan/` conventions for "git as AI version control".
  - May consume `dracon-ai`, but does not own sync/warden/system runtime roles.
  - Is not part of `dracon-utilities` ownership/runtime.

## De-dup policy

- Do not introduce another daemon that auto-commits repos outside `dracon-sync`.
- Do not introduce another watcher that enforces protected path policy outside `dracon-warden`.
- Keep system cleanup/health logic in `dracon-system`.
- Keep reusable capability logic in `dracon-libs`; utilities are wrappers/orchestrators.
- Protected branches should use one commit voice (`dracon-sync`) with deterministic JSON payloads.
- Do not move `dracon-code` concerns into `dracon-utilities`; `dracon-code` remains a separate product.

## Naming + transition policy

- old branding prefixes are legacy and should not be used for new binaries/crates.
- Active utility binaries are `dracon-sync`, `dracon-warden`, `dracon-system`, and `dracon-ai`.
- Always-on services are `dracon-sync`, `dracon-warden`, and `dracon-system`.
- `dracon-security` and `dracon-persistence` are removed runtime artifacts, not runtime owners.
