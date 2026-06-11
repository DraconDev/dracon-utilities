# Public-release branch preparation plan

Date: 2026-06-11  
Branch: `public-release`  
Baseline commit: `43d7505d6e70debdd876295726387bb794c6bf15` (`main`)  
Upstream: none yet — intentionally not pushed until the branch is reviewed.

## Purpose

This branch isolates the work required to make `dracon-utilities` public-release safe without changing `main`, repo visibility, mirrors, or published artifacts.

No destructive cleanup has been performed yet. This plan records the exact approval gates and validation gates before any removal, rewrite, or publication step.

## Current evidence

- Release readiness report: `docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md`
- Public-readiness assessment: `docs/public-readiness.md`
- Public release plan: `docs/public-release-plan.md`
- Final repo audit: `docs/audit/2026-06-11-full-repo-audit/final/REPORT.md`

Known release blockers:

1. `dracon-utilities` is not safe to publish as-is.
2. Current/reachable history contains local agent/task state, audit artifacts, operational logs, and secret-shaped fixture strings.
3. Public release requires an explicit cleanup branch, approval for local-state/history removal, public-safe docs, fresh scans, and validation.

## Approval gates before destructive cleanup

Do not run destructive cleanup until each gate is explicitly approved.

### Gate A — Branch scope

Required approval:

- Confirm this `public-release` branch is the intended public-release isolation branch.
- Confirm `main` remains the normal internal/private development branch.

Evidence:

```bash
git branch --show-current
git rev-parse main
git rev-parse public-release
git diff --name-only main...public-release
```

### Gate B — Local-state removal from current tree

Required approval:

Explicitly approve removal from the public branch for each category:

```text
.pi/
.ralph/
.sisyphus/
.demon/
debug.log
autoresearch.jsonl
docs/audit/
audit-todo/
.dracon/
```

Important constraints:

- Do not remove user-owned notes, screenshots, pasted-image files, project assets, or intentional public content without explicit approval.
- Do not remove `.pi/` on this branch until approval is explicit.
- Do not remove local state from `main`.

Evidence after approved cleanup:

```bash
git status --porcelain=v2 --untracked-files=all
git diff --name-status main...public-release
```

### Gate C — Reachable-history rewrite

Required approval:

- Confirm whether public release must have clean reachable history.
- Confirm approved paths to remove from history.
- Confirm a backup exists before rewrite.

Required backup before rewrite:

```bash
mkdir -p ~/backups/dracon-utilities-public-release
cp -a . ~/backups/dracon-utilities-public-release/dracon-utilities-before-public-cleanup
git log --all --name-only --pretty=format: > ~/backups/dracon-utilities-public-release/history-paths-before-public-cleanup.txt
git status --porcelain=v2 --untracked-files=all > ~/backups/dracon-utilities-public-release/status-before-public-cleanup.txt
```

Evidence after rewrite:

```bash
git log --all --name-only --pretty=format: > history-paths-after-public-cleanup.txt
rg -n '^\.pi/|^\.ralph/|^\.sisyphus/|^\.demon/|^debug\.log$|^autoresearch\.jsonl$|^docs/audit/|^audit-todo/|^\.dracon/' history-paths-after-public-cleanup.txt
```

Expected result: no matches for approved removed paths.

### Gate D — Secret-shaped fixture review

Required approval:

- Review redacted secret-shaped matches.
- Decide whether to remove or replace each fixture.
- Do not paste or expose real secret values.

Evidence:

```bash
rg -n 'AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9_]{36,}|glpat-[A-Za-z0-9_-]{20,}|xox[baprs]-|AGE-SECRET-KEY-' .
```

Expected result: no real-looking token matches, or every remaining match is clearly synthetic and documented.

### Gate E — Public-safe documentation

Required approval:

- Decide whether `AGENTS.md` is public-safe or should be replaced/redacted.
- Add public `SECURITY.md`.
- Review `README.md`, `CONTRIBUTING.md`, `docs/public-readiness.md`, and `docs/public-release-plan.md` for public-facing wording.

Evidence:

```bash
test -f SECURITY.md
rg -n "vulnerability|security|report|private" SECURITY.md
```

### Gate F — Visibility and publishing decision

Required approval:

- Confirm repo visibility change is allowed.
- Confirm whether GitHub Release, crates.io, Nix PR, or other publishing targets are allowed.
- Confirm mirror policy for `one-mil-girls` and any other public mirrors.

Do not run publish/release jobs until all prior gates pass.

## Validation gates after docs/cleanup changes

Run after approved cleanup and public-safe docs changes:

```bash
cargo fmt -p dracon-sync -p dracon-system -p dracon-warden -- --check
cargo clippy -p dracon-sync -p dracon-system -p dracon-warden --all-targets --no-deps
cargo build -p dracon-sync -p dracon-system -p dracon-warden
cargo test --workspace -- --test-threads=1
cargo tree -d
cargo deny check
./scripts/verify-spec.sh
Former `dracon-ai/` CLI wrapper removed from this repo; validate `dracon-libs` AI runtime crates separately when touched.
```

Additional sync/config checks:

```bash
dracon-sync config validate
dracon-sync scaffold --dry-run
```

## Final public-readiness scan

Run after validation:

```bash
git status --porcelain=v2 --untracked-files=all
git log --all --name-only --pretty=format: > history-paths-final.txt
rg -n '^\.pi/|^\.ralph/|^\.sisyphus/|^\.demon/|^debug\.log$|^autoresearch\.jsonl$|^docs/audit/|^audit-todo/|^\.dracon/' history-paths-final.txt
rg -n 'AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9_]{36,}|glpat-[A-Za-z0-9_-]{20,}|xox[baprs]-|AGE-SECRET-KEY-' .
```

Expected result:

- No removed local-state paths in reachable history.
- No real-looking secret-shaped strings.
- No tracked `.env*` files.
- `git status` shows only intended public-release changes.

## Next action

Stop here and wait for explicit approval for Gate B and Gate C before any cleanup or history rewrite.
