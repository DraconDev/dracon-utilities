# Public Release Plan: dracon-utilities

Generated: 2026-06-10

## Decision

`dracon-utilities` is **not safe to publish as-is**. Suggested publication path: **create a public-release branch, clean current tree/history, add public-safe docs, re-scan, validate, then publish**.

## Evidence used

- Git metadata: `/home/dracon/.local/state/dracon/dracon-utilities-public-readiness-evidence/git-metadata.txt`
- Working-tree risk paths: `/home/dracon/.local/state/dracon/dracon-utilities-public-readiness-evidence/working-risk-paths.tsv`
- Reachable-history risk paths: `/home/dracon/.local/state/dracon/dracon-utilities-public-readiness-evidence/history-risk-paths.tsv`
- Secret-shaped content scan: `/home/dracon/.local/state/dracon/dracon-utilities-public-readiness-evidence/secret-shaped-content.tsv`
- Env classification: `/home/dracon/.local/state/dracon/dracon-utilities-public-readiness-evidence/env-classification.tsv`
- Warden evidence: `/home/dracon/.local/state/dracon/dracon-utilities-public-readiness-evidence/warden/warden-evidence.log`
- Validation logs: `/home/dracon/.local/state/dracon/dracon-utilities-public-readiness-evidence/validation/`
- Current public-readiness assessment: `docs/public-readiness.md`

## Step 0 — Freeze publication

**Do now.**

- Do not change repo visibility.
- Do not push public mirrors.
- Do not run release/publish jobs.
- Do not rewrite history or remove local state until explicit approval.

Evidence gate:
- `git status --porcelain=v2 --untracked-files=all`
- `git remote -v`
- `git branch --verbose --verbose`

## Step 1 — Create an isolated public-release branch

**Do after Step 0.**

Commands:

```bash
git switch main
git switch -c public-release
```

Purpose:
- Keep cleanup separate from normal work.
- Avoid changing `main` until the public branch is verified.

Evidence gate:
- `git branch --show-current` should be `public-release`.
- `git status --porcelain=v2 --untracked-files=all` should show only intended docs.

## Step 2 — Backup and record current state

**Do before any destructive cleanup.**

Commands:

```bash
mkdir -p ~/backups/dracon-utilities-public-release
cp -a . ~/backups/dracon-utilities-public-release/dracon-utilities-before-public-cleanup
git log --all --name-only --pretty=format: > ~/backups/dracon-utilities-public-release/history-paths.txt
git status --porcelain=v2 --untracked-files=all > ~/backups/dracon-utilities-public-release/status-before-public-cleanup.txt
```

Purpose:
- Preserve current tree and history before rewrite.
- Make cleanup reversible.

Evidence gate:
- Backup directory exists.
- `history-paths.txt` and `status-before-public-cleanup.txt` are non-empty.

## Step 3 — Decide and approve local-state cleanup

**Requires explicit approval.**

Current blockers:
- Working tree: 63 high-risk tracked paths.
- Reachable history: 131 high-risk paths.
- Main categories:
  - `.pi/goals/`
  - `.ralph/`
  - `.sisyphus/`
  - `.demon/`
  - `debug.log`
  - `autoresearch.jsonl`
  - internal audit/task notes

Suggested cleanup targets for the public branch:

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

Commands after approval:

```bash
git rm -r .pi .ralph .sisyphus .demon docs/audit audit-todo .dracon debug.log autoresearch.jsonl
git commit -m 'remove local state from public-release branch'
```

Stop condition:
- Do not remove user-owned notes/screenshots/local task state unless explicitly approved.
- If any path ownership is uncertain, stop and ask.

## Step 4 — Rewrite history if public history must be clean

**Requires explicit approval and backup from Step 2.**

Why:
- Public publishing exposes reachable history by default.
- Current history contains local-state paths even if the current tree is cleaned.

Recommended tool:
- `git filter-repo` or BFG Repo-Cleaner.
- Do not use `git filter-branch` unless there is no alternative.

Example command on the `public-release` branch after backup:

```bash
git filter-repo \
  --path .pi \
  --path .ralph \
  --path .sisyphus \
  --path .demon \
  --path debug.log \
  --path autoresearch.jsonl \
  --path docs/audit \
  --path audit-todo \
  --path .dracon \
  --invert-paths \
  --force
```

If actual secrets are found during history content review:
1. Rotate them first.
2. Then rewrite history.
3. Then verify the rewritten history.

Evidence gate after rewrite:

```bash
git log --all --name-only --pretty=format: > history-paths-after-public-cleanup.txt
rg -n '^\.pi/|^\.ralph/|^\.sisyphus/|^\.demon/|^debug\.log$|^autoresearch\.jsonl$|^docs/audit/|^audit-todo/|^\.dracon/' history-paths-after-public-cleanup.txt
```

Expected result:
- No matches for the removed local-state paths.

## Step 5 — Add public-safe documentation

**Can be done before or after cleanup.**

Suggested docs:

- Add `SECURITY.md`
- Keep `README.md`
- Keep `CONTRIBUTING.md`
- Keep `LICENSE`
- Keep `docs/public-readiness.md`
- Keep or replace `docs/public-release-plan.md`
- Review `AGENTS.md` before publishing; it is internal workflow guidance

Suggested `SECURITY.md` contents:
- Supported versions or branch policy
- How to report vulnerabilities privately
- What not to post publicly
- Maintainer contact or issue template link

Evidence gate:
- `test -f SECURITY.md`
- `rg -n "vulnerability|security|report|private" SECURITY.md`

## Step 6 — Harden secret-shaped fixtures

**Requires review.**

Current evidence:
- `secret-shaped-content.tsv` lists 25 redacted matches.
- Matches are in README/tests/scanner fixture contexts, not raw secrets in this report.

Suggested actions:
- Replace token-like examples with obviously synthetic placeholders.
- Add comments naming them as fixtures.
- Prefer patterns that do not match public scanners.

Examples:

```text
[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAwaFZ3amNyV2RyL0tSN1AvTERPYWo4Ny9OWC9UYlU5YlNCWDc5dDZBY1hJCnY4M0JxK2VIdlV6ZkJOYy96QVgycENBTHZZUy9qTnVLTjdrckQrT3BaRm8KLT4gWDI1NTE5IDdUenp6RHlzb3Y3WDRSUGRPam9lYlFGd3Jzam9hdHU3RjVZcnU0T2hhMkUKKzdzSUJCMkhTdGVzakhTNmFaVXc2VGtZbGV1SVpvT1dSTDF4SUNvL1I1OAotPiBYMjU1MTkgNTBYcElxcTZseXkyV0FCYkE5ZUZCd0t5eTYwTVRNUXJxbTIrdjlpdXlsOAp6N3JUTVJ4UlpBWEZlajgvTkxLaklvenAxWWpZcnYyMVc1VldxSDdvU2dRCi0+IFgyNTUxOSA3WWVCUlBQZzh1QVMwUFhHakdDeEZ2cXZpS3gxMmtNM3N4bHI1MTJCOUJ3CkpCU0hSRkRHdUR4ZHlGcXBSOE05SDVBT3dlb3dTNy9FOGZIdTRjejF2VjAKLT4gWDI1NTE5IFRpZUwvdk1LQ2FtRlBmeTdla0hGalV2T0Zsc2wxSUFVKzhEOUh4WXkreG8KRUhlbklVMEljOUE1T25VcW1ZYVl2RXhpd1hQTnA2R01BTjBCK1QyMUJ6MAotPiBYMjU1MTkgWUZvU3pxbjlBb3p6ejlvZzVHQThNYThMa0N4UXlvejBmWWRnV0daL3p3TQpZaWhtT014U1VxMzlKTENLRUlMTFFwemMrRXRxQno0OWw4RkJMdTdMWllzCi0+IFgyNTUxOSBqNkI4czZEZlRKMmFIVGFYZnluWEFUWVlaR0czeml5MEJ2RUYvbkIzNlJrCmh6Uk5qVjZ6OFlaWFBQWGM0STBoRnNwN1phSi9ZWjQ2NFE0Q3pHR25kRUkKLT4gdVBALDk8LWdyZWFzZSBoL0wlSmZ4IHgsWUQKS3NRamdPRE5UNXBHM1pvcVNrUDdrRVRXUFBXM2J3b29TRjBpS2gwWktDdEJxUHRkSTlOS1hBU21GeEJwWndZMwp1R1BVcTgvckVQUWVIYS8wOFFaeFJDN0RGUTBPaTlyTW1PVFR5aExyTSsvcXZxaVRrUW1sOHkzRHBDa2RjcXhRClNTcUsKLS0tIFk2amV5TnN1MjJyd080WEtCNExieUtTWEZ2a2JieU9laWJhNXoxRm5WM2MKa5DJNtQY7skRdB+NJiIdxb2rZk/Mj+kfpy6pYcG53gEY820eun6bCoc9hWYNDr8cjPIj5Flm9Go41MMmEFsQMLqtcqvJlSrRsNjpOw==]000000
AKIA_PUBLIC_EXAMPLE_DO_NOT_USE_000000000000
xoxb-PUBLIC_EXAMPLE_DO_NOT_USE
```

Evidence gate:

```bash
rg -n 'AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9_]{36,}|glpat-[A-Za-z0-9_-]{20,}|xox[baprs]-|AGE-SECRET-KEY-' .
```

Expected result:
- No real-looking token matches, or every remaining match is clearly synthetic and documented.

## Step 7 — Run validation gates

**Run after cleanup and docs changes.**

Required commands:

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

Expected result:
- All commands pass.

Notes:
- `cargo fmt --workspace --check` is not a valid `cargo fmt` invocation here.
- `cargo fmt --all --check` is not a suitable repo-equivalent because the sibling `dracon-libs` checkout can contain paths rustfmt tries to resolve; CI uses the package-specific command above.

## Step 8 — Run final public-readiness scan

**Run after validation.**

Commands:

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

## Step 9 — Final publish decision

**Do not publish until all prior gates pass.**

Publish only if:

- Steps 0–8 are complete.
- All validation commands pass.
- Final scan is clean.
- `SECURITY.md` exists.
- `AGENTS.md` has been reviewed or removed/replaced.
- You explicitly approve changing visibility.

Final publish action is intentionally separate:

```bash
# Do not run until explicitly approved.
# Example only:
# gh repo edit DraconDev/dracon-utilities --visibility public
```

## Stop conditions

Stop and ask before continuing if any of these occur:

- A path ownership is uncertain.
- A real secret is found in current tree or history.
- History rewrite would remove files you may want to preserve.
- Validation fails and the cause is not understood.
- Public scanners still flag fixture strings after hardening.
- You are unsure whether internal docs such as `AGENTS.md` should be public.
