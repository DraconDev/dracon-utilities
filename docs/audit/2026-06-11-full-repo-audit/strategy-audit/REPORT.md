# Dracon strategy audit

Date: 2026-06-11  
Scope: the full Dracon project strategy as expressed in the authoritative documents of `dracon-utilities`, cross-referenced against the current implementation, tests, daemon behaviour, recent audit reports, and the fresh sync inventory.

## Verdict

**The strategy is sound in intent and mostly consistent with the implementation, but it is drifting in places that matter: the changelog is materially out of date, AGENTS.md is missing a real product roadmap, the workspace-vs-subcrate version model is incoherent, the release-readiness report is already stale (two new STUCK repos), and the strategy docs and the per-utility docs use different naming for the same things.**

The technical contract (auto-commit, deterministic messages, IndexLock, no-kill guard, AGPL auto-copy, fund-yml, never-suffix, scan-on-pre-push) holds up under audit. The drift is in **the strategy-of-the-strategy**: how the strategy itself is authored, versioned, kept current, and reconciled with code.

## What is the Dracon strategy

There is **no single strategy document**. The strategy is expressed across:

| Document | Role | Authoritative for |
|---|---|---|
| `README.md` (root) | User-facing quick start, binary list, install, configuration, environment, testing | Onboarding, install path |
| `AGENTS.md` (root) | **The de-facto strategy doc.** Covers architecture summary, design philosophy ("invisible infrastructure"), operational state, services, systemd, policy files, tokens & secrets, CLI reference, commit-message protocol, dependency hygiene, testing | The most complete reference; mixes policy + operations + reference |
| `docs/ARCHITECTURE.md` | Service overview, core loop, key design decisions, AI-to-AI commit protocol, IndexLock | Architecture |
| `docs/OPERATIONS.md` | Systemd services, incident response, troubleshooting, operational state layout | Operations |
| `docs/ROADMAP.md` | **Documentation roadmap only** (an index of where things live) | Doc navigation, not product direction |
| `docs/public-readiness.md` | Public-readiness assessment | Whether the repo is safe to publish |
| `docs/public-release-plan.md` | Public-release procedure | How to publish safely |
| `per-utility BLUEPRINT.md` (3 files) | Design/improvement notes per binary | Implementation guidance |
| `docs/design/*.md` | Design notes (CLI print style, warden plaintext sibling, etc.) | Specific design decisions |
| `AUDIT.md` (720 lines, 2026-06-09) | The full multi-domain audit (the latest, superseded only by the 2026-06-11 partial audits) | Current security & contract posture |
| `CHANGELOG.md` | Version history | Per-version record (currently drifting) |
| `CONTRIBUTING.md` | PR workflow | Contribution rules |
| `docs/audit/2026-06-11-full-repo-audit/*/REPORT.md` (8 reports) | Post-audit evidence per topic | Specific incidents and fixes |

**Observation:** there is **no product roadmap** anywhere in the repo. `docs/ROADMAP.md` is a documentation index, not a forward-looking plan. The only forward-looking content lives in the per-utility BLUEPRINTs and in scattered AUDIT findings. This is a **structural gap in the strategy layer**: there is no document that says "what is the next 3-6 months of Dracon supposed to be."

## Cross-reference of strategy vs. implementation

Evidence file: `cross-ref.tsv` (23 themes). Highlights:

| # | Theme | Strategy doc | Implementation | Status |
|---|---|---|---|---|
| 1 | Orphan `dracon-warden.service` | AUDIT.md 1.1 P1 | File deleted; `install.sh` never installed it; `rg "Command::Daemon"` = 0 | **Resolved** |
| 2 | `simple_ai.rs` / `scribe.rs` still compiled | AUDIT.md 1.2 P1 | Both files removed; `rg "scribe_update\|SimpleAiService"` = 0; Cargo features block is now `default = []` with no `scribe`/`ai-bumper` | **Resolved** |
| 3 | Tracked 13 MB `pi-session-*.html` | AUDIT.md 1.3 P2 | `git ls-files \| rg "pi-session-"` = 0 | **Resolved** |
| 4 | Tracked `rust_out` ELF | AUDIT.md 1.4 P2 | `git ls-files \| rg "^rust_out$"` = 0 | **Resolved** |
| 5 | `unsafe { std::env::set_var(...) }` on edition 2021 | AUDIT.md 1.5 P3 | Still present in `report.rs:2745-2763`, `print.rs:158-164` | **Open P3** (cosmetic) |
| 6 | AGENTS.md test counts | AGENTS.md lines 846-850 say 431 / 692 | Latest run: 705 passed, 9 ignored (22 suites) | **Doc drift P2** |
| 7 | README.md `cargo fmt --check` | README.md line 212 | Invalid for a workspace root (no root crate source); CI uses per-package form | **Doc drift P3** |
| 8 | CHANGELOG mentions `scribe` / `ai-bumper` Cargo features | CHANGELOG.md "Unreleased" / 0.112.0 | `dracon-sync/Cargo.toml [features]` has `default = []` only; 0 `#[cfg(feature = "scribe")]` references in source | **CHANGELOG drift P2** |
| 9 | `AGPL` auto-copy | AGENTS.md "Standard Files" | `standard_files.rs:ensure_standard_files`; per-repo `skip_standard_files`; `LICENSE` in 4 manifests | **Consistent** (extended to `FUNDING.yml` per post-funding report) |
| 10 | `IndexLock` coordination | ARCHITECTURE.md / AGENTS.md | `dracon-warden/src/main.rs:946-998`; `dracon-sync/src/sync.rs:2121-2124` | **Consistent** |
| 11 | No-kill guard | AGENTS.md "CRITICAL INVARIANT" | `dracon-system/src/main.rs:586-587` explicit comment; `rg "SIGKILL\|SIGTERM"` = 0 in `dracon-system/src` | **Consistent** |
| 12 | Merge strategy | ARCHITECTURE.md / AGENTS.md | CHANGELOG 0.112.0 documents the switch; `dracon-git` honours it | **Consistent** |
| 13 | Release pipeline toggles | AGENTS.md "Release Pipeline" | `sync.rs` implements `auto_tag`/`auto_release`/`auto_publish`/`nix_auto_update`; live policy has `sync_visibility=true` and **no** `[[publish_targets]]` | **Consistent (safe default)** |
| 14 | `auto_github_private` suffix-loop ban | AGENTS.md "NEVER create suffixed repos" | Enforced in code; 61 historical orphan repos cleaned up by `cleanup-except-pi` | **Consistent with cleanup performed** |
| 15 | Secrets layout | AGENTS.md "Tokens & Secrets" | `secrets.rs:sync_secrets_dir()` returns `~/.dracon/utilities/sync/secrets`; `load_secret` scans `*.env` | **Mostly consistent** (post-2026-06-11 git-auth-prompt fix, `~/.dracon/secrets/pat/` is canonical via symlinks; but `secrets/registry/crates-io-token` has **no `.env` ext** so the code would miss it if the symlink were removed) |
| 16 | Git credential / PAT | User question 2026-06-11 | `git-credential-github.sh` wired as first helper; `env -u GH_TOKEN git ls-remote` returns SHA without prompt | **Resolved** |
| 17 | Mirror visibility sync | AGENTS.md "Mirror Visibility & Metadata Sync" | `visibility.rs`; live policy has `sync_visibility=true`; cache in `~/.local/state/dracon/visibility-sync/` | **Consistent** |
| 18 | Fresh sync inventory | `docs/audit/2026-06-11/release-readiness/REPORT.md` said "no STUCK_PUSH" | Fresh run now shows **2 new STUCK_PUSH** repos: `browser-extensions-shared` (`AHEAD:1,STUCK_PUSH`) and `folder-auto-banner` (`AHEAD:1,STUCK_PUSH`) | **Stale report** (the release-readiness report is from earlier today and does not cover the current state) |
| 19 | Workspace version vs subcrate version | CHANGELOG root `0.112.4` (2026-06-07) | Subcrate `Cargo.toml` versions: `dracon-sync 0.1.5`, `dracon-system 0.2.0`, `dracon-warden 0.3.0` — all unchanged since **2024-05-03** | **Semantic drift** |
| 20 | `tarpaulin.toml` tracked | `.gitignore` rule `**/tarpaulin-report.*` (added in 0.112.4) | `git ls-files \| rg tarpaulin` returns `tarpaulin.toml` (config file, not a coverage report) | **Not a problem** (the rule correctly targets reports) |

## Strategy strengths

- **The contract is honest.** The "invisible infrastructure" promise (auto-commit, deterministic messages, no AI at the commit boundary) is implemented and tested. The fingerprint-based scheduling, inactivity delay, IndexLock, mass-deletion guard, and per-URL credential fallback are all real, not aspirational.
- **Security-by-default.** AGPL-only, hardened systemd units, `SystemCallFilter=@system-service`, no capabilities, `ProtectHome=read-only`, `MemoryDenyWriteExecute`, no-kill guard, `IndexLock` coordination, `permissions 600` on secrets with a world-writable check, suffix-loop ban — all in place and audited.
- **Scribe/AI-at-the-commit-boundary is gone.** AUDIT 1.2's P1 was a real risk (LLM-scribed messages and a non-feature-gated LLM client shipped by default). The fix is the right fix: delete the code, align the docs. The only residual is **the CHANGELOG still describing scribe features** (see drift below).
- **Deterministic over clever.** Routing keys are grep-searchable; that is the right call and it is consistent across README, AGENTS, ARCHITECTURE, and code.
- **Reversibility is a first-class concern.** The release pipeline is per-toggle, per-repo opt-in, dry-run-by-default, and the publish master toggle is off by default. `auto_publish = false` is the right default.
- **The audit chain is real.** AUDIT.md (720 lines, 2026-06-09) → 8 post-audit reports (2026-06-11) → fresh per-repo evidence on disk → durable. The pattern of writing evidence files alongside the report is the right one and should be the template for any future audits.

## Gaps and contradictions

### P1 (must fix before public release)

1. **No product roadmap.** `docs/ROADMAP.md` is a doc index. There is no document that answers "what is the project trying to become in the next 3-6 months." For a public release, that is a real gap — a reader of the repo deserves a forward-looking statement. Recommendation: create `docs/ROADMAP.md` as an actual roadmap (rename the current index to `docs/DOCS_INDEX.md` to preserve the navigation role).

2. **CHANGELOG is materially wrong.** The "Unreleased" / 0.112.0 section describes:
   - `scribe` and `ai-bumper` Cargo features that **do not exist** (`dracon-sync/Cargo.toml [features]` is `default = []`; zero `#[cfg(feature = ...)]` references in source)
   - `generate_commit_message()` and `local_fallback_message()` — code does not exist
   - `parse_conventional_commit()` — code does not exist
   - `parse_ai_bump_response` and the major-bump cap — code does not exist
   - `discover_git_repos_recursive` optimization — code path may still exist, but the surrounding "scribe / ai-bumper" context is fictional
   
   This is a real P1 because the CHANGELOG is a contractual document: it is what downstream users (and the AI) trust to know what changed. Reading it now, a maintainer would think there is a `scribe` feature flag to test. There is not. The CHANGELOG needs a full rewrite pass to match the actual `0.112.4` release and to keep `[Unreleased]` honest or remove it.

3. **Stale release-readiness report.** The 2026-06-11 release-readiness report claims "no unexplained CONCERN/STUCK_PUSH remains." The fresh inventory right now shows two new `STUCK_PUSH` repos (`browser-extensions-shared`, `folder-auto-banner`). This is a one-day-stale report, but it is the document the operator is most likely to read first. Recommendation: either (a) re-run and refresh the report, or (b) add a freshness timestamp and a "valid for inventory snapshot YYYY-MM-DDTHH:MM" header so readers know it is point-in-time.

### P2 (should fix in the next release cycle)

4. **AGENTS.md test counts are stale** (line 846-850 say 431 / 692, reality is 705 / 9 / 22 suites). The CI workflow does not enforce AGENTS.md accuracy. Recommendation: either drop the hard counts from AGENTS.md (replace with "see latest CI run") or wire a small CI check that fails when AGENTS.md counts and the test binary's own `--list` disagree.

5. **Subcrate version skew.** Root workspace version is `0.112.4` (2026-06-07); subcrate `Cargo.toml` versions are `dracon-sync 0.1.5`, `dracon-system 0.2.0`, `dracon-warden 0.3.0` — all 2024-05-03. This is not consistent with the CHANGELOG's own SemVer policy ("MAJOR: breaking config/CLI"). Recommendation: choose one. Either (a) bump subcrate versions on every release (dracon-sync → 0.112.4 to match), or (b) treat the root as the only version and drop subcrate `version` from the `[package]` sections, or (c) adopt a CalVer / git-sha scheme and stop pretending. The current state is "we say semver, but the numbers don't track" — and the audit count cannot agree because the version-suffixed binary names don't.

6. **Secret layout has a partial gap.** The git-auth-prompt fix made `~/.dracon/secrets/pat/` canonical via symlinks in `~/.dracon/utilities/sync/secrets/`. But `~/.dracon/secrets/registry/crates-io-token` has **no `.env` extension**, so `load_secret` would skip it if the symlink were ever removed. This is a fragile state: the new layout is partially broken. Recommendation: either rename `crates-io-token` to `cratesio.env` (or add a `cratesio.env` file inside `registry/` that `load_secret` can find), or update `secrets.rs` to also look in `~/.dracon/secrets/registry/`.

7. **AGENTS.md is overloaded.** At 891 lines (43.5 KB), it is the de-facto strategy doc but it mixes design philosophy, operational state, systemd tables, policy paths, tokens & secrets, CLI reference, commit-message protocol, dependency hygiene, and testing. That makes it hard to keep accurate and hard to search. Recommendation: split into `docs/STRATEGY.md` (design philosophy, contract), `docs/OPERATIONS.md` (already exists; move systemd/operational state there), `docs/REFERENCE.md` (CLI, env vars, commit-message format, dependency policy), and leave AGENTS.md as the agent-handbook section only.

### P3 (opportunistic)

8. **README.md line 212** says `cargo fmt --check` (invalid for a workspace root). One-line fix.

9. **AUDIT.md 1.5** P3 (`unsafe { std::env::set_var(...) }` under edition 2021) is still present. Cosmetic; can be removed in a single PR.

10. **Per-utility BLUEPRINTs** (`dracon-sync/BLUEPRINT.md`, `dracon-system/BLUEPRINT.md`, `dracon-warden/BLUEPRINT.md`) are not linked from the new doc strategy. They are the implementation guidance, and they live in the per-utility dirs which is fine, but the strategy layer (AGENTS / ARCHITECTURE) doesn't link to them.

11. **`docs/design/`** has 8 design notes (cli-print-style, warden-plaintext-sibling, etc.) but `docs/ROADMAP.md` doesn't link to them. Some are historical (the ones that `ROADMAP.md` says are "archived in archive/") but the surviving design notes have no central index.

12. **AGENTS.md says the per-utility config file paths are under `~/.dracon/utilities/sync/`** but the user's recent `.dracon` reorganization moved secrets to `~/.dracon/secrets/pat/`. AGENTS.md should document the new layout (or at least link to the git-auth-prompt report).

13. **AGENTS.md is in the workspace** and tracked; **AUDIT.md** is also in the workspace and tracked. Both are useful but they age. Recommendation: move them under `docs/audit/2026-06-09-full-multi-domain-audit/AUDIT.md` and `docs/strategy/AGENTS.md` (or similar) so the live root is just `README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, and a thin `docs/STRATEGY.md` that links into dated subdocs. This is the same pattern that already works for the 2026-06-11 audit chain.

## What is missing from the strategy (intentionally)

- **No enterprise features.** There is no SSO, no audit log aggregation, no multi-tenant mode, no GUI. This is consistent with the "invisible infrastructure for an AI coder" philosophy and is the right call.
- **No push-to-deploy.** There is no `webhook`-driven deployment pipeline. The daemon's webhook is one-way (failure notifications). This is consistent with the philosophy.
- **No versioned Nix flake release pipeline that auto-PRs.** It exists (`nix_auto_update`) but is per-repo opt-in and default-off. This is consistent with the "safe default" pattern.

These are **omissions by design**, not gaps. The strategy is clear that Dracon is a single-operator / small-team daemon, not a SaaS.

## Cross-cutting recommendations (ranked)

| # | Recommendation | Effort | Risk | Priority |
|---|---|---|---|---|
| 1 | Rewrite `CHANGELOG.md` so that `0.112.4` and `[Unreleased]` match the actual source. Remove `scribe`/`ai-bumper`/`generate_commit_message`/`parse_ai_bump_response` references that no longer exist. | S | none | **P1** |
| 2 | Refresh the release-readiness report (or add a freshness header). | S | none | **P1** |
| 3 | Create an actual product `docs/ROADMAP.md` (rename current index to `docs/DOCS_INDEX.md`). | M | low | **P1** |
| 4 | Resolve subcrate-vs-workspace version skew. Pick one model. | S–M | low | **P2** |
| 5 | Update AGENTS.md test counts (or replace hard counts with "see CI"). | XS | none | **P2** |
| 6 | Fix the secret-layout extension gap (`registry/crates-io-token` no `.env` ext). | S | low | **P2** |
| 7 | Split AGENTS.md into `docs/STRATEGY.md` + `docs/REFERENCE.md` + keep `AGENTS.md` as agent handbook. | M | low | **P2** |
| 8 | Update README.md line 212 `cargo fmt --check` to the per-package form. | XS | none | **P3** |
| 9 | Remove the `unsafe { std::env::set_var(...) }` wrappers (AUDIT 1.5 P3). | XS | none | **P3** |
| 10 | Add cross-links from `docs/ARCHITECTURE.md` to the per-utility BLUEPRINTs and surviving `docs/design/*.md`. | XS | none | **P3** |
| 11 | Document the new `~/.dracon/secrets/{pat,registry,ai,...}` layout in AGENTS.md (or a linked "Secret layout" section). | S | none | **P3** |
| 12 | Move `AUDIT.md` and `AGENTS.md` under dated subfolders (`docs/audit/.../AUDIT.md`, `docs/strategy/AGENTS.md`) so root is thin. | S | low | **P3** |

## Constraints respected

- No strategy doc, code, or remote state was changed.
- No publishing, no visibility change, no force-push, no rebase, no history rewrite, no secret rotation.
- Recommendations are listed as proposals only; each requires explicit operator approval before any change.
- The cross-reference is built from fresh, file/line-cited evidence, not from assumptions.

## Evidence inventory

- Doc inventory: `doc-inventory.tsv`
- Strategy ↔ implementation cross-reference (23 themes): `cross-ref.tsv`
- Recommendations: `recommendations.tsv`
- Fresh sync inventory snapshot used for this audit: `dracon-sync repos --json --full-path` (output stored at `/tmp/strategy-inv.json`)
- Cited prior reports (not re-derived, just referenced):
  - `AUDIT.md` (root, 720 lines, 2026-06-09)
  - `docs/audit/2026-06-11-full-repo-audit/final/REPORT.md`
  - `docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md`
  - `docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md`
  - `docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md`
  - `docs/audit/2026-06-11-full-repo-audit/git-auth-prompt/REPORT.md`
  - `docs/public-readiness.md`, `docs/public-release-plan.md`
