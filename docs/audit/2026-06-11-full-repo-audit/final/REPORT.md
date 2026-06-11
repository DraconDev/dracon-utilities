# Dracon-managed repo audit — final report

Date: 2026-06-11  
Scope: every repo reported by `dracon-sync repos --json --full-path`, explicitly including `dracon-ai-lib`.

## Outcome

- Final inventory contains **20 Dracon-managed repos**.
- No repo visibility was changed, no public mirror was pushed, and no secrets were printed.
- All Rust repos with local validation commands now pass `cargo fmt --all --check`, `cargo test --workspace -- --test-threads=1`, and `cargo clippy --workspace --all-targets --no-deps` (or the repo-specific equivalent).
- Confirmed build/test/lint problems were fixed where safe.
- Remaining public-readiness blockers are documented and require explicit approval or access decisions before any publication step.

## Fixes performed

1. **Sync inventory**
   - Removed `dracon-ai-lib` from `exclude_repos` in the sync policy so it is included in `dracon-sync repos`.
   - Re-ran `dracon-sync config validate`; policy is valid.

2. **`dracon-ai-lib`**
   - Fixed invalid origin URL and pointed it at the valid `https://github.com/DraconDev/dracon-ai-lib.git` remote.
   - Local validation now passes.
   - Push remains blocked because the remote is archived/stuck; this requires an explicit recreate/unarchive/rewrite decision.

3. **`dracon-platform`**
   - Fixed `cargo fmt --all --check` drift.
   - Fixed `cargo clippy --workspace -- -D warnings` failures across AI/billing/email/auth APIs:
     - removed stale/unused imports,
     - fixed duplicate middleware import,
     - replaced manual clamp with `clamp`,
     - fixed needless borrow,
     - fixed redundant guard,
     - marked legacy accepted-but-unused query/request fields explicitly.

4. **`folder-auto-banner`**
   - Fixed malformed `cached_check!` macro invocations in `src/fs/mod.rs`.
   - `cargo fmt`, `cargo test`, and `cargo clippy` now pass.

5. **`ai-auto-writer`**
   - Fixed stale Dracon AI integration references to unavailable `dracon_ai_contracts` / `dracon_ai_client` APIs.
   - Reworked the service wrapper to use the repo's `ai-api-sdk` dependency.
   - `cargo fmt`, `cargo test`, and `cargo clippy` now pass.

6. **`video-uploader`**
   - Fixed empty `--passphrase-file` handling so an explicitly empty passphrase file is rejected.
   - Updated the CLI contract test to accept the existing plaintext-store/workspace error path when no passphrase is provided.
   - `cargo fmt`, `cargo test`, and `cargo clippy` now pass.

## Final validation matrix

Evidence file: `docs/audit/2026-06-11-full-repo-audit/final/final-validation.tsv`

| Repo | fmt | test | clippy | Result |
|---|---:|---:|---:|---|
| `dracon-platform` | 0 | 0 | 0 | pass |
| `folder-auto-banner` | 0 | 0 | 0 | pass |
| `ai-auto-repo-rot-scanner-todo-agent` | 0 | 0 | 0 | pass |
| `kiki-sassy-desktop-announcer` | 0 | 0 | 0 | pass with local ALSA pkg-config/library env |
| `dracon-code` | 0 | 0 | 0 | pass |
| `avid` | 0 | 0 | 0 | pass |
| `ai-auto-writer` | 0 | 0 | 0 | pass |
| `video-factory` | 0 | 0 | 0 | pass |
| `rust-ai-web-auto` | 0 | 0 | 0 | pass |
| `dracon-utilities` | 0 | 0 | 0 | pass |
| `pully-fully-pull-based-fleet-reconciler` | 0 | 0 | 0 | pass |
| `youtube-video-uploader` | 0 | 0 | 0 | pass |
| `video-uploader` | 0 | 0 | 0 | pass |
| `dracon-ai-lib` | 0 | 0 | 0 | pass; push still stuck |
| `dracon-libs` | 0 | 0 | 0 | pass with local ALSA/SQLite env |

Non-Rust validation evidence: `docs/audit/2026-06-11-full-repo-audit/final/non-rust/non-rust.tsv`

| Repo | Validation |
|---|---|
| `one-mil-girls` | `bun test` and `bun run check` passed |
| `Junk-Runner-bevy` | `bun run build` and `bun run check` passed for `web/` |
| `browser-extensions-shared` | No root package scripts; hygiene/public-readiness blocker remains |
| `DraconDev` | No documented local build/test command; docs/profile triage remains |

Dependency checks: `docs/audit/2026-06-11-full-repo-audit/final/deps/deps.tsv`

- `cargo tree -d` passed for checked Rust repos.
- `cargo deny check` passed for repos with `deny.toml`: `dracon-utilities`, `dracon-libs`, `dracon-code`, `youtube-video-uploader`, and `video-uploader`.
- `dracon-platform` has no local `deny.toml`.

## Hygiene / public-readiness findings

Evidence files:

- `docs/audit/2026-06-11-full-repo-audit/final/hygiene.tsv`
- `docs/audit/2026-06-11-full-repo-audit/final/risk-paths/*.risk.tsv`

Summary:

| Repo | Public-readiness status | Reason |
|---|---|---|
| `browser-extensions-shared` | **Not public-ready** | Tracked `.env*`, browser profile/cache/history/local-storage paths, `.ralph` local state, screenshots/assets, and other generated artifacts. User previously objected to publishing this repo. Do not clean, delete, rewrite, or untrack without explicit approval. |
| `dracon-ai-lib` | **Blocked** | Local validation passes, but repo is ahead 13 and push is stuck. Needs explicit remote/recreate/unarchive/rewrite decision. |
| `DraconDev` | **Not public-ready yet** | Profile/research/draft repo; no local build/test command; contains draft/scratch/profile research artifacts and `.pi` goals. Needs explicit docs/public triage. |
| `one-mil-girls` | **Validated, public cleanup decision needed** | Build/test pass, but tracked `.pi/goals` and audit/screenshots are present. Do not delete or rewrite without approval. |
| `Junk-Runner-bevy` | **Validated, public cleanup decision needed** | Web build/check pass, but tracked `.pi/goals` and generated/audit artifacts exist. Do not delete or rewrite without approval. |
| `dracon-utilities` | **Validated, not publish without release-plan approval** | Technical validation passes; public release still requires executing the previously documented release plan and approvals. |
| Other Rust repos | **Technically healthy** | Validation passes. Public-readiness still depends on whether tracked `.dracon` public keys, `.pi` goals, notes, screenshots, or generated artifacts are acceptable for the intended publication scope. |

## Remaining blockers / decisions

1. **`browser-extensions-shared`**
   - Blocker: tracked secrets/private browser profile data and generated artifacts.
   - Required decision: explicit approval for cleanup, history rewrite, secret rotation, or permanent non-public status.

2. **`dracon-ai-lib`**
   - Blocker: local branch is ahead 13 and push is stuck.
   - Required decision: recreate/unarchive remote, accept abandoned remote, or perform an approved history rewrite/force-push.

3. **Tracked local state / notes / screenshots**
   - Affected examples include `.pi/goals`, `.ralph`, audit screenshots, and generated artifacts in several repos.
   - Constraint: do not delete, rename, untrack, ignore, or rewrite these without explicit approval.

4. **Public release execution**
   - This audit did not publish, expose, rotate secrets, rewrite history, or change visibility.
   - Public release should only proceed after the repo-specific cleanup decisions above are approved and the relevant verification gates are re-run.

## Evidence inventory

- Fresh sync inventory: `docs/audit/2026-06-11-full-repo-audit/final/inventory.json`
- Inventory TSV: `docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv`
- Per-repo git metadata: `docs/audit/2026-06-11-full-repo-audit/final/per-repo/*.git.txt`
- Rust validation matrix: `docs/audit/2026-06-11-full-repo-audit/final/final-validation.tsv`
- Non-Rust validation matrix: `docs/audit/2026-06-11-full-repo-audit/final/non-rust/non-rust.tsv`
- Dependency checks: `docs/audit/2026-06-11-full-repo-audit/final/deps/deps.tsv`
- Hygiene summary: `docs/audit/2026-06-11-full-repo-audit/final/hygiene.tsv`
- Risk path lists: `docs/audit/2026-06-11-full-repo-audit/final/risk-paths/*.risk.tsv`
