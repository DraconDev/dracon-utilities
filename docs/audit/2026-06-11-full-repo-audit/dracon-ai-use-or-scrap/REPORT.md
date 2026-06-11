# Dracon AI use-or-scrap report

Goal: determine whether the Dracon AI components in the Dracon workspace still have a valid use case or should be removed, then implement the chosen outcome end-to-end.
Date: 2026-06-11

## Decision

**Scrap the orphaned `dracon-ai` CLI wrapper from `dracon-utilities`. Preserve the shared AI runtime crates in `dracon-libs`.**

Rationale:
- The local `dracon-utilities/dracon-ai/` CLI wrapper was not installed (`~/.local/bin/dracon-ai` missing).
- It was not built by the main workspace (`Cargo.toml` workspace members are only `dracon-sync`, `dracon-system`, `dracon-warden`).
- It was not referenced by the sync policy, install path, or active utility runtime.
- It had its own stale config at `~/.dracon/utilities/ai/dracon-ai.toml` pointing at `/home/dracon/dracon`, not this `~/Dev/dracon-utilities` tree.
- It had real functionality, but also stale integration debt: `cargo test` and fmt passed, while clippy failed under `-D warnings` with 12 existing lint errors.
- The shared AI runtime crates in `dracon-libs` still validate cleanly in focused validation and are useful independently, so they were not removed.

## Actions taken

1. Inventoried the Dracon AI surface:
   - `evidence/initial-inventory.md`
   - `evidence/dracon-ai-crate-inventory.md`
   - `evidence/dracon-ai-source.md`
   - `evidence/consumer-reference-search.md`
   - `evidence/ai-runtime-inventory.md`

2. Ran standalone validation on the former CLI wrapper:
   - `cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1` → passed: 7 tests.
   - `cargo fmt --manifest-path dracon-ai/Cargo.toml --check` → passed.
   - `cargo clippy --manifest-path dracon-ai/Cargo.toml --workspace -- -D warnings` → failed with 12 existing lint errors.
   - Evidence: `evidence/dracon-ai-cargo-test.log`, `evidence/dracon-ai-cargo-fmt-check.log`, `evidence/dracon-ai-cargo-clippy.log`.

3. Got explicit user approval to **scrap the CLI wrapper** while preserving shared runtime crates.

4. Removed only `dracon-utilities/dracon-ai/`:
   - Deleted files captured in `evidence/pre-removal-dracon-ai-cli-snapshot.md`.
   - Removal commit: `abe683f9`.

5. Updated current docs/policy references:
   - `CONTRIBUTING.md` no longer lists `dracon-ai/` as an active utility directory.
   - `UTILITY_BOUNDARIES.md` now says the core utilities are three, not four, and records that `dracon-ai` was removed as an orphaned CLI wrapper.
   - `AGENTS.md` now says the former `dracon-ai/` CLI wrapper was removed and AI runtime crates live in `dracon-libs`.
   - Public-readiness / public-release docs now say the former CLI wrapper was removed and runtime crates should be validated separately when touched.
   - `Cargo.toml` removed the stale `exclude = ["dracon-ai"]` block; workspace remains `exclude = []`.

6. Preserved shared AI runtime crates in `dracon-libs`:
   - `ai-routing-runtime`
   - `ai-runtime-adapters`
   - `ai-runtime-config`
   - `dracon-ai-contracts`
   - `dracon-ai-runtime-contracts`
   - Focused validation: `cargo test -p dracon-system-lib -p ai-routing-runtime -p ai-runtime-adapters -p ai-runtime-config -p dracon-ai-contracts -p dracon-ai-runtime-contracts -- --test-threads=1` → passed.
   - Evidence: `evidence/final-focused-dracon-libs-cargo-test.log`.

7. Fixed an incidental compile error discovered during validation in `dracon-libs/tools/system/dracon-system/src/lib.rs`:
   - The helper used `tokio::process::Command` in a synchronous context and had an async/sync ordering issue.
   - `run_command_checked` is now synchronous and uses `std::process::Command`; `run_command` remains `pub async unsafe fn`.
   - Commit: `e62742d`.

## Final verification

### Removed target

- `test ! -e dracon-ai` → `dracon-ai directory absent`.

### Current references

Remaining references to `dracon-ai` outside historical audit/goal docs are only intentional notes that the CLI wrapper was removed, or shared runtime dependency names:

- `UTILITY_BOUNDARIES.md` — documents removal.
- `AGENTS.md` — documents removal and standalone runtime validation.
- `docs/public-readiness.md`, `docs/public-release-branch/PUBLIC_RELEASE_PREP.md`, `docs/public-release-plan.md` — document removal.
- `Cargo.toml` — still references `dracon-ai-runtime-contracts` from `dracon-libs`, which is correct and unrelated to the removed CLI wrapper.

Historical audit/goal documents still mention `dracon-ai` or `dracon-ai-lib`; these were preserved as historical evidence.

### Workspace validation

- `cargo fmt --check` in `dracon-utilities` → passed.
- `cargo test --workspace -- --test-threads=1` in `dracon-utilities` → passed.
- `cargo fmt --check` in `dracon-libs` → passed.
- Focused `dracon-libs` validation for affected crates → passed.
- Full `dracon-libs` workspace test remains blocked by a system dependency: linker cannot find `-lsqlite3` while compiling `dracon-memory-runtime`. This is unrelated to the Dracon AI scrap and is documented in `evidence/final-dracon-memory-runtime-blocker.log`.

### Final evidence

- `evidence/final-evidence.md`
- `evidence/pre-removal-dracon-ai-cli-snapshot.md`
- `evidence/dracon-ai-cargo-test.log`
- `evidence/dracon-ai-cargo-fmt-check.log`
- `evidence/dracon-ai-cargo-clippy.log`
- `evidence/ai-runtime-cargo-test.log`
- `evidence/final-focused-dracon-libs-cargo-test.log`
- `evidence/final-dracon-memory-runtime-blocker.log`

## Constraints respected

- No force-push, rebase, history rewrite, visibility change, publish, secret rotation, or destructive cleanup outside the approved `dracon-ai/` CLI wrapper.
- Shared `dracon-libs` AI runtime crates were preserved.
- Historical audit/goal evidence was not deleted.
- User-owned local state (`.pi/`, `.ralph/`, `.sisyphus/`, `.demon/`) was not touched.
- No unapproved shortcuts, TODO placeholders, hidden assumptions, or undocumented behavior changes remain.

## Result

The orphaned `dracon-ai` CLI wrapper is gone from `dracon-utilities`. The Dracon AI runtime remains available as shared `dracon-libs` crates and validates independently in focused tests. The workspace now accurately documents the three active utilities only: `dracon-sync`, `dracon-warden`, and `dracon-system`.
