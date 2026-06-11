# Pre-release audit: dracon-sync, dracon-system, dracon-warden

Date: 2026-06-11
Scope: `/home/dracon/Dev/dracon-utilities`
Goal: audit all three utilities before any public release and apply only safe, verified fixes.

## Verdict

**Do not publish or tag the current tree as-is.**

The utilities have strong local validation: workspace format, tests, clippy, dependency policy, release builds, spec verification, sync policy validation, scaffold dry-run, and install dry-run all pass after the fixes below. However, public release is still blocked by existing public-readiness evidence and by the warden pre-push hook blocking the current commits because commits being pushed contain secret-shaped fixture/evidence lines. Those are not real leaked credentials in this audit, but they are a release blocker until a public-release branch is sanitized or explicitly approved.

No publish, tag, release, visibility change, force-push, rebase, history rewrite, secret rotation, or local/user state removal was performed.

## Evidence directory

All fresh evidence for this audit is under:

`docs/audit/2026-06-11-full-repo-audit/pre-release-all-three-utilities/evidence/`

Key files:

- `dracon-sync-repos.json` — baseline repo inventory from `dracon-sync repos --json --full-path`
- `final-state-after-fixes.txt` — final repo/state snapshot after fixes
- `git-status.txt`, `git-branches.txt`, `git-remotes.txt`, `git-log-all-20.txt` — branch/remote state
- `utility-cargo-versions.txt`, `version-inventory-current.txt` — version metadata
- `cargo-fmt-check.exit`, `cargo-test.exit`, `cargo-deny-check.log`, `final-validation/combined-final.log` — validation evidence
- `service-install-inventory.txt`, `service-install-check-final.txt` — install/systemd checks
- `public-readiness-current-summary.txt`, `secret-shaped-current.tsv`, `history-local-state-paths.tsv` — public-readiness scans
- `push-dry-run-origin.log`, `final-sync-incidents-tail.jsonl` — push/pre-push blocker evidence
- `CHANGELOG.diff`, `install.diff` were captured as needed, but current diffs are in git history because the sync daemon auto-committed the fixes:
  - `83f04d8b` — `CHANGELOG.md` Unreleased section moved to top
  - `48bd61ba` — `install.sh --dry-run` exit-code fix

## Safe fixes applied

### 1. Changelog ordering

Finding: `CHANGELOG.md` had `[Unreleased]` below released entries, which is release-doc hygiene drift.

Fix: moved `[Unreleased]` to the top of `CHANGELOG.md`, preserving existing entries.

Evidence:

- `git show --stat --oneline --no-textconv 83f04d8b`
- `cargo fmt --check` passed after the edit.

Classification: **fixed**.

### 2. Install dry-run exit code

Finding: `./install.sh --dry-run` printed `✅ Installation complete!` but exited `1`. Root cause: `set -euo pipefail` plus `pgrep -x dracon-warden | head -1` in the final daemon verification loop, when `dracon-warden` is not running.

Fix: changed the daemon verification loop to tolerate missing processes and to build the service-name hint without a failing pipeline.

Evidence:

- Before: `./install.sh --dry-run` exited `1`.
- After: `final-validation/combined-final.log` shows `./install.sh --dry-run` completed and `final-validation/combined-final.exit` is `exit=0`.

Classification: **fixed**.

## Validation status

All required local validation checks passed after the fixes:

| Check | Evidence | Result |
| --- | --- | --- |
| `cargo fmt --check` | `final-validation/combined-final.log` | pass |
| `cargo test --workspace -- --test-threads=1` | `final-validation/combined-final.log` | pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | `final-validation/combined-final.log` | pass |
| `cargo deny check` | `cargo-deny-check.log`, `final-validation/combined-final.log` | pass |
| `cargo build --release -p dracon-sync -p dracon-system -p dracon-warden` | `final-validation/combined-final.log` | pass |
| `./scripts/verify-spec.sh` | `final-validation/combined-final.log` | pass |
| `dracon-sync config validate` | `final-validation/combined-final.log` | pass |
| `dracon-sync scaffold --dry-run` | `final-validation/combined-final.log` | pass |
| `./install.sh --dry-run` | `final-validation/combined-final.log` | pass, exit 0 |

## Utility metadata

| Utility | Crate version | Edition | License |
| --- | ---: | ---: | --- |
| `dracon-sync` | `0.1.5` | 2021 | AGPL-3.0-only |
| `dracon-system` | `0.2.0` | 2021 | AGPL-3.0-only |
| `dracon-warden` | `0.3.0` | 2021 | AGPL-3.0-only |
| workspace package | `0.112.4` | 2021 | metadata only |

The root workspace version differs from per-crate versions. This is existing behavior and was documented in the changelog as a hygiene-only workspace version; it is not treated as a release blocker here because the per-crate package versions are explicit and validation passed.

Classification: **document/keep**.

## Repo and branch state

Baseline `dracon-sync repos --json --full-path` reported:

- repos: 16
- ok: 14
- warn: 2
- concern: 0
- failures: 0

Baseline WARN rows:

- `Junk-Runner-bevy` — `tauri2`, modified tracked files, push OK. Preserved as user work.
- `one-mil-girls` — `main`, modified/untracked files, push OK. Preserved as user/audit work.

Final `dracon-sync repos` after this audit showed more non-OK rows because other repositories accumulated user work during the audit and because `dracon-utilities` itself became ahead of origin after the sync daemon auto-committed the local fixes:

- `dracon-utilities` — `main`, `AHEAD:3`, `STUCK_PUSH`, concern true. Push was blocked by the local warden pre-push hook; no push was forced.
- `browser-extensions-shared`, `ai-auto-repo-rot-scanner-todo-agent`, `rust-ai-web-auto`, `one-mil-girls` — dirty WARN rows from user work. Preserved.

Evidence:

- `dracon-sync-repos.json`
- `final-state-after-fixes.txt`
- `git-status.txt`
- `git-branches.txt`
- `git-remotes.txt`

Classification: **document/preserve user work; no destructive cleanup**.

## Push/public-release blockers

### Blocker 1: existing public-readiness docs say not safe to publish as-is

`docs/public-readiness.md` and `docs/public-release-plan.md` state that the current tree/history is not safe to publish as-is. The current scan refreshed this concern:

- `public-readiness-current-summary.txt`
- `history-local-state-paths.tsv`
- `secret-shaped-current.tsv`

The scan found many history paths under audit/local-state directories and secret-shaped matches in documentation, README examples, scanner tests, and source fixtures. These matches are not treated as proof of real credential leakage, but they are a public-release blocker because public readiness requires a sanitized public-release branch and approval.

Classification: **release blocker; require public-release branch cleanup and explicit approval**.

### Blocker 2: warden pre-push blocks current commits

`git push --dry-run origin HEAD` was attempted only as a diagnostic dry run. It exited 0 at the shell wrapper but printed the warden pre-push block and did not push:

```text
⚠️ Possible plaintext secrets detected in push.
   The warden filter may have been bypassed.
error: failed to push some refs to 'https://github.com/DraconDev/dracon-utilities.git'
```

The scan of commits ahead of `origin/main` shows secret-shaped fixture/evidence lines from warden tests/scanner fixtures and audit evidence. This is expected security behavior, not a bug. It means the current branch cannot be pushed to public remotes without either a sanitized public-release branch or explicit operator approval to bypass/adjust the hook policy.

Classification: **release blocker; do not bypass without approval**.

## Install/systemd/service behavior

Findings:

- `dracon-sync.service` is tracked, installed, enabled, and active.
- `dracon-system-guard.service` is tracked, installed, disabled in `list-unit-files` output but active in the final active-units check.
- `dracon-warden.service` is intentionally absent. `dracon-warden` has no daemon service; enforcement is via git hooks (`pre-commit` and `pre-push`) installed by `dracon-warden setup-hooks --global`.

Evidence:

- `service-install-inventory.txt`
- `service-install-check-final.txt`
- `install.sh --dry-run` in `final-validation/combined-final.log`

Classification: **documented/expected**.

## Dependency policy

`cargo deny check` passed:

```text
advisories ok, bans ok, licenses ok, sources ok
```

Classification: **pass**.

## Release-readiness findings matrix

| ID | Finding | Classification | Status |
| --- | --- | --- | --- |
| F1 | Changelog `[Unreleased]` ordering drift | fix | fixed |
| F2 | `install.sh --dry-run` exit 1 despite success output | fix | fixed |
| F3 | Existing public-readiness docs/evidence say not safe to publish as-is | approval-required | not fixed |
| F4 | Current commits blocked by warden pre-push due secret-shaped fixture/evidence lines | approval-required | not fixed |
| F5 | `dracon-utilities` ahead 3 / `STUCK_PUSH` after local fixes | document | current state, no push performed |
| F6 | Other repos dirty from user work | preserve | no action |
| F7 | `dracon-warden` has no systemd service | document | expected behavior |
| F8 | Root workspace version differs from per-crate versions | document | existing metadata scheme |
| F9 | `cargo deny check` | pass | dependency policy ok |
| F10 | Format/tests/clippy/build/spec/config/scaffold/install validation | pass | release validation ok |

## Recommendation

Proceed with **local release validation only**. Do **not** publish, tag, or push public mirrors from the current branch yet.

Before public release, create or use a dedicated public-release branch and require explicit approval for:

1. Sanitizing or excluding audit/local-state evidence and secret-shaped fixture lines from public history.
2. Resolving or intentionally documenting the warden pre-push block.
3. Pushing public mirrors / creating tags / publishing releases.

The three utilities themselves are locally healthy after the two low-risk fixes above, but the current repository state is not yet public-release-ready.

## Required next input

The next concrete step is **approval to execute the public-release branch cleanup plan**:

`PUBLIC_RELEASE_BRANCH_PLAN.md`

No further release work should happen until approval is explicit for branch cleanup, sanitization scope, and public push/tag/publish actions.
