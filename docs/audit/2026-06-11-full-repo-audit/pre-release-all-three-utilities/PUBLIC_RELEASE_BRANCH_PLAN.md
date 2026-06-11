# Public-release branch plan: dracon-utilities

Date: 2026-06-11
Scope: `dracon-sync`, `dracon-system`, `dracon-warden`
Status: **plan only — no branch created, no files removed, no push, no tag, no publish.**

## Current baseline

Current evidence:

- Pre-release audit report: `docs/audit/2026-06-11-full-repo-audit/pre-release-all-three-utilities/REPORT.md`
- Current state snapshot: `docs/audit/2026-06-11-full-repo-audit/pre-release-all-three-utilities/evidence/public-release-plan-current-state.txt`
- Push blocker evidence: `docs/audit/2026-06-11-full-repo-audit/pre-release-all-three-utilities/evidence/push-dry-run-origin.log`
- Public-readiness scans: `docs/audit/2026-06-11-full-repo-audit/pre-release-all-three-utilities/evidence/public-readiness-current-summary.txt`

Current release blockers:

1. Existing public-readiness docs say the current tree/history is not safe to publish as-is.
2. Current commits ahead of `origin/main` contain secret-shaped fixture/evidence lines, so the local warden pre-push hook blocks pushing.
3. `dracon-utilities` is ahead of `origin/main`; no push was forced.

## Recommended branch strategy

Create a dedicated public-release branch from the latest validated main state, then sanitize that branch for public release.

Do **not** rewrite or force-push `main`.

Recommended branch name:

```text
public-release-utilities-2026-06-11
```

Branch policy:

- Start from the latest validated `main` commit after the two release-readiness fixes.
- Keep the branch local until all public-readiness checks pass.
- Do not merge back to `main` unless the public-release branch is explicitly approved.
- Do not tag, publish, or push public mirrors until the approval checklist below is complete.

## Sanitization scope

The public-release branch should remove or replace content that is not safe for public history.

Priority 1 — required before public push:

- Audit evidence containing secret-shaped fixture lines.
- Audit evidence containing local-state paths (`.pi/`, `.ralph/`, `.sisyphus/`, `.demon/`, `debug.log`, `autoresearch.jsonl`, `.dracon/`, `docs/audit/`).
- Example tokens or secret-shaped strings in release-plan docs.
- Any local agent/goal/session state that should not be public.

Priority 2 — strongly recommended:

- Move long audit evidence out of the public release branch or into a compact sanitized summary.
- Remove generated validation logs that do not add public value.
- Keep only concise audit reports that document decisions without exposing local-state paths.

Priority 3 — optional polish:

- Add a public README/release note explaining the three utilities.
- Add a concise public changelog summary.
- Keep internal operational docs out of the public branch unless intentionally approved.

## Suggested cleanup procedure

Run all commands on the public-release branch only.

1. Create branch from validated main:

   ```bash
   git switch -c public-release-utilities-2026-06-11
   ```

2. Freeze sync before cleanup:

   ```bash
   dracon-sync pause
   ```

3. Capture a cleanup manifest before deleting anything:

   ```bash
   git status --porcelain=v2 --untracked-files=all > docs/audit/2026-06-11-full-repo-audit/pre-release-all-three-utilities/evidence/public-release-branch-before.tsv
   ```

4. Remove only approved public-release cleanup candidates:
   - local agent/session state
   - generated audit evidence with secret-shaped fixture lines
   - generated audit evidence with local-state paths
   - example-token docs that are not useful publicly
   - debug/autoresearch logs

   Do **not** delete user-owned notes, screenshots, project assets, or intentional public content unless explicitly approved.

5. Run public-readiness scans after cleanup:

   ```bash
   git status --porcelain=v2 --untracked-files=all > evidence/working-porcelain.tsv
   git log --all --name-only --pretty=format: > evidence/history-paths.tsv
   rg -n '^\.pi/|^\.ralph/|^\.sisyphus/|^\.demon/|^debug\.log$|^autoresearch\.jsonl$|^docs/audit/|^audit-todo/|^\.dracon/' evidence/history-paths.tsv > evidence/history-local-state-paths.tsv || true
   rg -n 'AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9_]{36,}|glpat-[A-Za-z0-9_-]{20,}|xox[baprs]-|AGE-SECRET-KEY-' . --hidden -g '!target/**' -g '!**/target/**' > evidence/secret-shaped-current.tsv || true
   ```

6. If secret-shaped matches remain, classify each one:
   - real secret → stop and rotate/revoke
   - test fixture → keep only if intentionally public and add `.plaintext` sibling if using the current warden hook policy
   - example token → replace with clearly fake short tokens like `ghp_EXAMPLE_DO_NOT_USE`
   - scanner regex/source fixture → keep only if public docs/tests require it

7. Run validation:

   ```bash
   cargo fmt --check
   cargo test --workspace -- --test-threads=1
   cargo clippy --all-targets --all-features -- -D warnings
   cargo deny check
   cargo build --release -p dracon-sync -p dracon-system -p dracon-warden
   ./scripts/verify-spec.sh
   dracon-sync config validate
   ./install.sh --dry-run
   ```

8. Run push dry-run:

   ```bash
   git push --dry-run origin HEAD
   git push --dry-run github HEAD
   git push --dry-run gitlab HEAD
   git push --dry-run codeberg HEAD
   ```

9. Only after the above passes, ask for explicit approval to push/tag/publish.

## Approval checklist

Before any public action, require explicit approval for each item:

- [ ] Public-release branch cleanup scope approved.
- [ ] Deletion of audit/local-state evidence approved.
- [ ] Remaining secret-shaped fixture lines approved or sanitized.
- [ ] Push to `origin` approved.
- [ ] Push to public mirrors approved.
- [ ] Tag creation approved.
- [ ] GitHub/GitLab/Codeberg release creation approved.
- [ ] Registry publishing approved, if applicable.

## Rollback plan

If the public-release branch cleanup is not approved or fails scans:

- Delete only the public-release branch.
- Keep `main` unchanged.
- Keep the pre-release audit report as documentation.
- Do not push, tag, or publish.

No force-push, rebase, history rewrite, secret rotation, visibility change, or local/user state removal should happen without explicit approval.
