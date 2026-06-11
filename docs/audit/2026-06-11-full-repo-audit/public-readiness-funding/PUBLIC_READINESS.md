# Public-readiness note after FUNDING.yml scope clarification

Date: 2026-06-11  
Goal: Make `FUNDING.yml` personal-to-Dracon and document the public-release gap.

## Decision

`FUNDING.yml` is now explicitly scoped as **Dracon-specific**, not a generic `dracon-sync` default for external users.

- The code default remains unchanged: `SyncPolicy::default()` has no `standard_files`, so external users are not forced to receive `FUNDING.yml`.
- The generic example policy documents the external starter as `standard_files = ["LICENSE"]`.
- The live Dracon policy explicitly opts in to `.github/FUNDING.yml`.
- The template comments say external users only receive it if they explicitly add the long-form entry.
- `AGENTS.md` and `dracon-sync/README.md` now state the same scope.

This means the FUNDING.yml change is **not a public-readiness blocker** by itself. It is safe, public, secret-free, and documented.

## Public-release gap

We are **not ready to publish the whole Dracon-managed repo set yet**.

The closest state is:

- The `dracon-utilities` workspace is technically healthy.
- The FUNDING.yml behavior is correct and documented.
- Most Rust repos pass fmt/test/clippy.
- Non-Rust repos `one-mil-girls` and `Junk-Runner-bevy/web` pass their local checks.
- The remaining blockers are mostly public-readiness/hygiene, push/remote, and pre-existing user-owned changes — not FUNDING.yml.

## What must happen before public release

Minimum approvals/decisions needed:

1. **Scope decision**: publish `dracon-utilities` only, or publish a broader Dracon-managed repo set.
2. **`browser-extensions-shared`**: either exclude it from public release or approve a cleanup/rewrite/secret-rotation plan for tracked `.env*`, browser profile/cache/history/local-storage data, and `.ralph` state.
3. **`dracon-ai-lib`**: decide remote strategy. It is still AHEAD:21 and push is blocked by the archived remote.
4. **Tracked local state**: approve whether `.pi/goals`, `.ralph`, audit screenshots, and generated artifacts in repos like `one-mil-girls` and `Junk-Runner-bevy` may remain public. If not, they need explicit cleanup/rewrite approval.
5. **Pre-existing clippy warnings**: decide whether public CI will require `-D warnings` for all repos. Several repos still fail clippy under `-D warnings` for pre-existing issues.
6. **User-owned changes**: `dracon-code` currently has user-owned fmt/clippy drift; preserve it and do not rewrite without approval.
7. **Final public release plan**: run a final public-readiness check after the above decisions, then execute the release plan.

## Current repo status

### Healthy / validated

- `dracon-utilities`: workspace passes; public docs/release plan still need approval.
- `dracon-platform`: passes fmt/test/clippy, but current inventory shows user-owned changes in AI API tests and hosted web assets; public readiness depends on reviewing/committing those changes and whether tracked local state/artifacts are acceptable.
- `folder-auto-banner`: passes fmt/test/clippy.
- `ai-auto-repo-rot-scanner-todo-agent`: passes fmt/test/clippy.
- `kiki-sassy-desktop-announcer`: passes fmt/test/clippy with ALSA env.
- `avid`: passes fmt/test/clippy.
- `rust-ai-web-auto`: passes fmt/test/clippy.
- `pully-fully-pull-based-fleet-reconciler`: passes fmt/test/clippy.
- `one-mil-girls`: non-Rust checks pass; tracked `.pi/goals`/audit artifacts remain.
- `Junk-Runner-bevy/web`: non-Rust checks pass; tracked `.pi/goals`/audit artifacts remain.

### Blocked / not public-ready yet

- `browser-extensions-shared`: not public-ready due tracked secrets, browser profile data, and `.ralph` state.
- `dracon-ai-lib`: local validation passes, but push is blocked (AHEAD:21, archived remote).
- `dracon-code`: user-owned changes currently cause fmt/clippy failures; preserve them unless the user approves cleanup.
- `ai-auto-writer`, `video-factory`, `youtube-video-uploader`, `video-uploader`, `dracon-ai-lib`, `dracon-libs`: tests pass, but pre-existing clippy warnings remain under `-D warnings`.
- `DraconDev`: no documented local build/test command; profile/research artifacts need public triage.

## Evidence

- Inventory: `inventory.json` / `inventory.tsv`
- Per-repo git metadata: `per-repo/*.git.txt`
- Rust validation matrix: `validation-logs/final-validation.tsv`
- Workspace validation log: `workspace-validation.log`
- Non-Rust validation matrix: `non-rust/non-rust.tsv`
- Dependency checks: `deps/deps.tsv`
- Hygiene summary: `hygiene.tsv`

## Bottom line

We are **close on the technical side**, but not ready to publish the whole set yet.

The FUNDING.yml issue is resolved: it is Dracon-specific, external users are not forced to receive it, and the behavior is documented. The remaining public-release blockers are hygiene, remote/push, and explicit approval decisions.
