# Cleanup audit — full checklist except `.pi`

Date: 2026-06-11  
Goal: Run the full public-readiness cleanup checklist except `.pi` local task state.

## Summary

The cleanup checklist was executed for all locally available Dracon-managed repos with one hard exclusion: **`.pi/` was not cleaned or modified**.

Result:

- **Cleaned:** 390 candidate paths across 4 repos.
- **Preserved:** 1,112 candidate paths that are user-owned notes, screenshots, project assets, or otherwise intentional content.
- **Blocked by `.pi` exclusion:** 317 `.pi/` paths; before/after diffs are empty.
- **Blocked-needs-approval:** 418 paths, mainly `.env*`, `.ralph/*.md`, `.ralph/*.state.json`, ambiguous TODO/checklist docs, and source files whose names include "secret" but are actual code/tests.

## Cleaned items

| Repo | Action | Scope | Reason |
|---|---|---|---|
| `browser-extensions-shared` | Removed tracked browser profile data | `auto-form-filler/.audit-ui/{aria-check2,aria-test2,dd-debug,popup-check}/Default/` | Browser profile/cache/history/local-storage data outside `.pi`; public-readiness blocker. |
| `browser-extensions-shared` | Removed tracked generated coverage | `SamAI/coverage/` | Generated coverage output, not source or user-owned project asset. |
| `ai-auto-repo-rot-scanner-todo-agent` | Removed stale local runner event file | `.ralph/audit-remediation/.ralph-runner/events.jsonl` | Stale `.ralph-runner` generated event log outside `.pi`. |
| `dracon-utilities` | Removed stale non-Warden local public key | `.demon/data/keys/owner_age1wz5p.pub` | Stale local state outside `.pi`; not the Warden master/team key system. |
| `one-mil-girls` | Removed generated audit JSON files | `docs/audit/2026-06-11-full-audit-v2/script-audit.json`, `docs/audit/visual-qa/convo-redesign-after/inspect/inspect.json` | Generated audit artifacts outside `.pi`; project screenshots/assets were preserved. |

## Preserved items

Preserved because they are user-owned notes, screenshots, project assets, or intentional content:

- All `.pi/` paths.
- Screenshots and pasted-image files.
- Project assets such as icons, images, audio/video assets, and source files.
- `.ralph/*.md` and `.ralph/*.state.json` where they appear to be local task/session notes.
- Ambiguous TODO/checklist docs.
- `.env*` and secret-like paths, because removing them can require rotation/approval.
- Source files whose names include "secret" but are actual code/tests.

## `.pi` unchanged proof

Evidence directory: `pi-proof/`

- Captured `.pi` tracked and untracked paths before cleanup.
- Captured `.pi` tracked and untracked paths after cleanup.
- Diffed before/after `.pi` files for every repo.
- Result: `pi_diff_count 0`.

No `.pi/` path was cleaned, deleted, renamed, untracked, ignored, rewritten, sanitized, or moved.

## Remaining public-readiness blockers

Cleanup reduced the hygiene burden, but the repo set is still **not public-ready as a whole**.

Remaining blockers:

1. **`browser-extensions-shared`**
   - Browser profile data and generated coverage were cleaned.
   - Still has tracked `.env*` / secret-like files and user-owned screenshots.
   - `.env*` cleanup is blocked-needs-approval because secret removal requires rotation/approval.
   - Screenshots are preserved unless explicit approval is given.

2. **`dracon-ai-lib`**
   - Still AHEAD:24 and push is blocked by the archived remote.

3. **Secret-like files across repos**
   - `.env*`, `.envrc`, example secret files, and secret-like fixtures remain classified as `blocked-needs-approval`.

4. **`.ralph` local task/session notes**
   - Preserved unless clearly generated; many are Markdown notes/state files.

5. **Ambiguous TODO/checklist docs**
   - Preserved because they may be intentional project docs.

6. **User-owned changes**
   - `dracon-code`, `browser-extensions-shared`, and `dracon-ai-lib` have user-owned changes that were preserved.

## Validation

### `dracon-utilities` workspace

Evidence: `validation-logs/workspace-validation.log`

- `cargo fmt --all --check` → pass
- `cargo clippy --workspace -- -D warnings` → pass
- `cargo test --workspace -- --test-threads=1` → **709 passed, 9 ignored**
- `cargo deny check` → pass
- `scripts/verify-spec.sh` → pass
- `dracon-sync config validate` → pass
- `dracon-sync scaffold --dry-run` → pass (`No standard files to scaffold (all repos already have them).`)
- funding-specific tests → **6 passed**

### Affected repos

Evidence: `validation-logs/*.cleanup-validation.log`

- `ai-auto-repo-rot-scanner-todo-agent`: `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace -- --test-threads=1` passed.
- `one-mil-girls`: `bun test` and `bun run check` passed.
- `browser-extensions-shared`: no root package scripts were present; cleanup was verified by git/hygiene evidence instead.
- `dracon-utilities`: workspace validation passed.

## Evidence inventory

- Before inventory: `before/inventory.json`, `before/inventory.tsv`
- After inventory: `after/inventory.json`, `after/inventory.tsv`
- Per-repo before/after git metadata: `per-repo/before.*.git.txt`, `per-repo/after.*.git.txt`
- Cleanup candidate scan: `candidates/cleanup-candidates.tsv`
- Cleanup manifest: `CLEANUP_MANIFEST.md`
- `.pi` unchanged proof: `pi-proof/*.diff` (all empty)
- Hygiene summary after cleanup: `hygiene.tsv`
- Validation logs: `validation-logs/`

## Bottom line

The full cleanup checklist was run except `.pi`, exactly as requested.

The highest-risk browser profile/cache data and generated coverage were removed. `.pi` was proven unchanged. Remaining public-release blockers are now limited to secret rotation/approval decisions, preserved user-owned content, user-owned changes, and the pre-existing `dracon-ai-lib` push blocker.
