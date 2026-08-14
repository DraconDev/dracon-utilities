# Release process — 2026-06-21

Status: historical design (2026-06-21). The coordinated monorepo flow below
is retained as an audit record, but is not the current release entry point.

Current status (2026-08-14): releases are cut per nested standalone repo.
For `dracon-sync`, use
[`dracon-sync/scripts/release.sh`](../../dracon-sync/scripts/release.sh);
it runs the required gates, packages the crate, and checks the packaged
artifact with the isolated gitignore fixture before tagging. The old
coordinated cut remains subject to the operator decisions documented below.

## Why this exists

The Dracon utilities workspace has 4 crates that all need coordinated
releases:

- `dracon-sync` (crates.io: `dracon-sync`)
- `dracon-warden` (crates.io: `dracon-warden`, has a path-dep on
  `dracon-security`)
- `dracon-system` (crates.io: `dracon-system`)
- `dracon-security` (crates.io: `dracon-security`, internal path-dep
  under `dracon-warden/src/security/`; not advertised as a coordinated
  release target — bumps on its own cadence)

Plus 3 per-utility façade repos that mirror the parent monorepo:

- `DraconDev/dracon-sync-background-auto-commit-multi-remote`
- `DraconDev/dracon-warden-secret-encrypt-age-git-filter`
- `DraconDev/dracon-system-disk-process-guard-doctor`

And the monorepo's own GitHub release + CHANGELOG entry.

Cutting a release used to mean ~10 manual steps, all easy to forget or
to do out of order. This script does them in one command, with the
order fixed by design.

## The hard order

1. **Bump versions** in `Cargo.toml` (workspace), `dracon-sync/Cargo.toml`,
   `dracon-warden/Cargo.toml`, `dracon-system/Cargo.toml`. Skip
   `dracon-warden/src/security/Cargo.toml` (internal path-dep; its own
   cadence). The script will not bump a crate that is already at the
   target version.
2. **Close `CHANGELOG.md [Unreleased]`** → `## [<new>] - <date>` and
   leave a fresh empty `## [Unreleased]` on top.
3. **Create `release-notes-v<X>.<Y>.<Z>.md`** at the workspace root,
   sourced from the just-closed `[Unreleased]` block.
4. **`cargo publish --workspace --dry-run`** as a gate check.
5. **`cargo update -w`** to regenerate `Cargo.lock` to match the new
   toml versions (the bump mutates the lockfile; cargo refuses
   `--locked` + version-bump in the same step).
6. **`cargo publish -p <each-bumped-crate>`** for real, in dependency
   order (path-deps first).
7. **Commit + push** the bumped toml/CHANGELOG/notes files.
8. **`git tag v<X>.<Y>.<Z>`** + **`git push <remote> <tag>`**.
   Tag is the contract that "this version is on crates.io", so the tag
   is created **only after** the publish step succeeds.
9. **`gh release create v<X>.<Y>.<Z> --notes-file <notes>`** for the
   GitHub release.
10. **(Optional) regenerate the 3 façade repos** by running
    `scripts/regenerate_facade_repos.py --all`. The daemon auto-pushes
    the façade updates to github/gitlab/codeberg.
11. **(Optional) install the post-commit hook** at
    `.git/hooks/post-commit` to keep the façades in sync on every
    future monorepo commit.

The order is fixed by the script. The script aborts loudly on the
first failure, so a half-done release is recoverable.

## Usage

```bash
# safe preview (no remote state changes; no file changes either)
scripts/release.sh 0.112.13 --dry-run

# undo a local-file-mutating dry-run (after the dry-run produced files
# but you changed your mind)
scripts/release.sh 0.112.13 --abort

# real cut (will fail loudly if anything goes wrong)
scripts/release.sh 0.112.13 --yes

# real cut + install the post-commit hook for ongoing façade syncing
scripts/release.sh 0.112.13 --yes --install-hook
```

Flags:

| Flag | Effect |
| --- | --- |
| `--dry-run` | Run every step; mutate no remote state; do not push; do not publish for real; do not commit/tag/release. The bump step, CHANGELOG close, release-notes creation, and `cargo publish --dry-run` are all run. After completion, the operator can review the local changes and either commit + push themselves, or run `--abort` to revert. |
| `--abort` | Revert any local modifications from a previous `--dry-run`. Scoped to release-flow files only (`*.toml`, `CHANGELOG.md`, `release-notes-v*.md`); does not touch other uncommitted work. |
| `--skip-facade` | Skip step 10 (façade regeneration). Use when the release only changes workspace-level metadata. |
| `--install-hook` | After the release, install the monorepo post-commit hook so future commits auto-regenerate the façades. Off by default. |
| `--remote <name>` | Push to this git remote (default: `github`). |
| `--yes` | Skip the "are you sure" prompt. Required for non-interactive runs. |

## Preconditions (checked by the script)

- Working tree is clean (no uncommitted changes).
- `gh` is authenticated (`gh auth status`).
- `~/.cargo/credentials.toml` exists (cargo is logged in to crates.io).
- The version string is semver (`N.N.N` or `N.N.N-pre`).

If any precondition fails, the script exits with code 2 and a clear
message. It will not half-mutate state.

## Recovery from a partial run

The script's contract: if it fails after step 1 (bump), nothing has
been pushed and you can re-run with `--abort` to revert local files
and start over. If it fails after step 6 (publish), the crates are on
crates.io but no tag was created; you can re-run with the same version
— the script will skip the bump (already at version) and re-run the
publish (idempotent if version matches, or skip with a message) and
the tag + release.

Worst case: the script's push step fails after the commit. The commit
is local; push manually with `git push github main` (or `--remote
<other>`), then run the release script again — it will detect the
existing commit and proceed to tagging.

## Secrets handling

The script never prints or logs any credential material. It does
require the operator's `~/.cargo/credentials.toml` to be present
(cargo's standard mechanism) and `gh` to be authenticated.

The script does **not** read or echo `cat ~/.cargo/credentials.toml`.
If a release-flow note or design doc needs to mention that
"credentials exist", the text should say "a `token = "cio2…"`
entry is present" — never the literal token. (This is a lesson
learned from goal `3db1a52a`: a previous design note inlined the
full token, which then leaked into local git history when the file
was auto-committed. Treat the token as a secret even in markdown.)

## dracon-git / dracon-system-lib external deps

The workspace depends on `dracon-git = "94.7.2"` and
`dracon-system-lib = "94.2.7"`. These are external crates.io deps
and are not published by this script. `dracon-git` 94.7.2 is already
published; the release flow assumes both dependencies are stable. If
a new version of either is needed, the operator must publish it
separately first and then update the workspace's `Cargo.toml`
requirement before running `scripts/release.sh`.

## Façade repos — auto-sync design

The 3 per-utility façade repos are separate GitHub repos that mirror
the monorepo's per-utility subdirs (`dracon-sync/`, `dracon-warden/`,
`dracon-system/`). They are watched by `dracon-sync` and auto-pushed
to their 3 mirrors on local change.

The sync is one-way: monorepo → façade. The script triggers the sync
by running `scripts/regenerate_facade_repos.py --all`, which copies
the monorepo's per-utility subdirs into the façade clones and commits
the result. The daemon then picks up the local commit and pushes it
to github/gitlab/codeberg.

For ongoing syncing (every monorepo commit, not just releases), install
the post-commit hook: `scripts/release.sh --install-hook`. The hook
lives at `.git/hooks/post-commit` and runs the regenerator on every
monorepo commit that touches a per-utility subdir.

## Validation evidence (2026-06-21)

Goal: prove the pipeline works end-to-end.

- `scripts/release.sh 0.112.12-test --dry-run --skip-facade` — ran
  clean; all 6 steps display; no file changes; no remote state changes.
- `scripts/release.sh 0.112.12-test --skip-facade --yes` (real run) —
  bumped all 4 toml files; closed `[Unreleased]`; created
  `release-notes-v0.112.12-test.md`; `cargo publish --workspace --dry-run`
  packaged and verified all 4 crates successfully; script halted at
  step 5 because `dracon-security@0.3.0` already exists on crates.io
  (correct: dry-run is the gate, real publish would have advanced).
- `--abort` — reverted the bumped toml files; script reported
  "local modifications reverted".
- `cargo test --workspace --locked` — 589 passed, 3 ignored
  (the existing baseline from goal `0b174456`).
- `cargo build --release --locked` — 0 errors.
- `cargo deny check` — clean.

## Open questions for the operator

These are the same 5 from the original goal question set
(goal `3db1a52a-7359-4547-bfd9-35bb3d90bf67`):

1. **Target version.** Next patch on all four (e.g. `dracon-sync
   0.1.13`, `dracon-warden 0.3.8`, `dracon-system 0.2.8`,
   `dracon-security 0.3.1`, workspace `v0.112.12`) or different?

2. **Tag signing.** The goal said "sign the tag"; existing tags
   (`v0.112.10`, `v0.112.11`, `dracon-sync-v0.1.12`, …) are unsigned
   lightweight. Match the existing convention (unsigned lightweight)
   or install GPG and sign going forward?

3. **Now or flow-first.** This design doc + the script is the
   flow-first answer. The script exists, is tested, and one
   `scripts/release.sh <version> --yes` will cut a real release.
   Confirm whether to actually cut a release now.

4. **`dracon-security` bump.** It's a path-dep crate published on
   its own cadence. The script defaults to skipping it. Confirm
   that the default is correct for this release.

5. **Changelog framing.** Anything specific to call out in the
   release headline besides the publish-upstream + concern-investigation
   fixes from goals `1107ae07` and `0b174456`?

## Blocked on the operator

Historical status note (2026-08-14): the release flow described here has
since been exercised and extended. `dracon-sync/scripts/release.sh` now runs
the full workspace gates and verifies the packaged artifact with
`dracon-sync/scripts/verify-install.sh` before tagging. The remaining items
below are the original operator prerequisites for a real release cut, not
missing implementation in the script.

The goal was not complete at the time this document was written. The script
was in place and the dry-run validated the flow, but the actual release cut
required:

- Target version (Q1)
- Tag-signing decision (Q2) — script defaults to unsigned
- The "go" signal (Q3)
- The `dracon-security` decision (Q4) — script defaults to skip
- A rotated crates.io API token. The previous token (a `cio2…`
  entry in `~/.cargo/credentials.toml`) was exposed in a local
  git history that was not pushed to a public remote. It must
  still be rotated as a precaution. The operator must go to
  https://crates.io/settings/tokens, revoke the old token, generate
  a new one, and run `cargo login <new-token>` on this machine.
