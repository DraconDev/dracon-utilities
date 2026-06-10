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
[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA0VVFUc2tYVU9Ca0QzOUNGMG9kbGc3OW1kUDUvZEtaTVY5VGxyTXQvcWd3CnNEZXJZN2UxZDBzSVEyNmt5V3ZvS2w2UFRheTNkaG1TcmZPT25YZWxQNTQKLT4gWDI1NTE5IG1ISlBXWjJMeVlVWHRCQ0xEYmhnVHZ2VFU2ZzFpV1NORS8zcGVPcE9MMjgKeTM2LyttZlN6VnA3bUJkMEE3RVVFMU93U2Y3YnNJS2trRkZ4a29QbjNwdwotPiBYMjU1MTkgdXduVUVuRTVKSzJlNks1WDFXTncwTEFWQ1Z3SmJwejZyVnZaWjNzN0pucwpIWjhJakpFclpwUTQzUDFOeG1UWWY2MWozc2FJTkdqTUhIbmJIZGpmZ2NBCi0+IFgyNTUxOSBNWDJtK1k3OGtNSTJuZWRWdGxIemMxMHZoMEJ0ZzdLZ3NUcjZnWW1FZWlNCjNnR0E4eWxxSDg5elNYMG1MYmRWZDJ0S2dRY3FVNy9VdEF5aEE0eUpqVGsKLT4gWDI1NTE5IEVZZ0lVTVFOQXE5NXZtRjU2SlBwc3dWSEU4SjhrOTB5Y29iN2FabFFEaDQKVnJ0eE1uU2gvc05CditpSXB5cGhON2hVdG5LUi9BendNaHhaRW5jdWhTMAotPiBYMjU1MTkgbE1RWkk5eWVyVlJybkluUVhwYUZFajVmcm4xeldVYXhqRWJ4SnNqSUozUQpMYU9IYk43WGsra0pGYm05NDl1MStOYjdQbjZKa2pnQmNJN3A5aFFzYlRJCi0+IFgyNTUxOSBuUWpleTZYc3UrZ3B0RGdJZDZxZStVdFhwT0hXWTV5OFY1bGx0YWt0KzFRCk0xbFhZNi85UDgzVUdiUEhYQ0FHREkwZkNETnBkUkV6UUNZTjAzWDhxS3MKLT4gXy1ncmVhc2UKOWdQMXZ1cU5PUDh5NmlnUU80Nk9UMWxQVFVTVDVob3NrYkMranBPZWMzSjNwK0cvOUNXYThjeDZ0eEdqMGRpVgpZNFpFV0xVdmZraEpkNmFwCi0tLSBKcm5STE5ob0RWbVFJSFVRKysrTUUrMUJHVGQvdHRwOGF4VEZxakd5anJZCpXDP5MPiYY38TIfOdr84WgLwrdd9Y+50yhXQHti6ltbH8xAv6FMcX8a3Co/wzeVNakgP7IBbd9bE25vXss9YjgsP6ttuD+UqkCpCUI=]000000
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
cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1
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
