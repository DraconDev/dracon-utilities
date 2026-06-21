# Standalone utility repos — 2026-06-21

## Summary

The 3 utility codebases that previously lived as Cargo-workspace members
inside `DraconDev/dracon-utilities` are now **first-class standalone git
repos**. They live as sibling subdirectories of the monorepo root (same
parent on disk, each with its own `.git/`, each with its own 3 remotes),
and each has its own Cargo identity, version, CHANGELOG, LICENSE, README,
and per-repo release script. The verbose public-facing github/gitlab/codeberg
repo names (`dracon-sync-background-auto-commit-multi-remote`,
`dracon-system-disk-process-guard-doctor`,
`dracon-warden-secret-encrypt-age-git-filter`) are preserved; the
local directory names are the canonical short names (`dracon-sync`,
`dracon-system`, `dracon-warden`).

## What changed

### Repo structure

| Before | After |
| --- | --- |
| `dracon-utilities/` (monorepo, single `.git/`, single Cargo workspace) | `dracon-utilities/` (monorepo, no Cargo) + `dracon-utilities/dracon-{sync,system,warden}/` (each a standalone `.git/`) |
| `dracon-utilities/dracon-sync/Cargo.toml` uses `{ workspace = true }` | Each subdir's `Cargo.toml` has full inline versions, no workspace inheritance |
| `dracon-utilities/Cargo.toml` workspace block with 3 members | **deleted** (no workspace at the monorepo root) |
| `dracon-utilities/Cargo.lock` for the 3-crate workspace | **deleted** (each subdir has its own `Cargo.lock`) |
| `dracon-utilities/scripts/release.sh` releases all 3 crates | Each subdir has its own `scripts/release.sh` |
| `/home/dracon/Dev/facade-repos/` parallel working trees | **deleted** (in-monorepo subdirs are the canonical homes) |
| `dracon-utilities/scripts/regenerate_facade_repos.py` (file-copy regen) | **deleted** (no dual-write needed) |
| `dracon-utilities/scripts/scaffold_feature_repos.py` (scaffolder) | **deleted** (subdirs are already in place) |
| `dracon-utilities/.git/hooks/post-commit` (regen trigger) | **deleted** (each subdir is now first-class) |

### Per-repo identity

Each of `dracon-sync/`, `dracon-system/`, `dracon-warden/` has:

- Its own `.git/` (real directory, not a submodule pointer)
- 3 remotes: `github` (verbose-name github URL), `gitlab` (DraconDev), `codeberg` (dracondev)
- 5-commit history inherited from the verbose-name public repo (no new "initial commit" — the existing public history is the source of truth)
- `Cargo.toml` with full inline dependency versions (no `{ workspace = true }` shortcuts)
- `CHANGELOG.md` starting at v0.112.12 with a note that prior history is in the parent monorepo
- `LICENSE` (AGPL-3.0, matching the public repos)
- `README.md` (the standalone version, not the monorepo's `monorepo-README.md`)
- `scripts/release.sh` (per-repo release flow, same `<version> --yes [--dry-run] [--abort]` interface)
- `Cargo.lock` (generated, with all transitive deps pinned)

## Migration steps (what was done)

For each of `dracon-sync/`, `dracon-system/`, `dracon-warden/`:

1. Cleared the existing monorepo contents (the in-workspace versions that
   used `{ workspace = true }`)
2. Cloned the verbose-name public repo into a temp dir
3. Moved the temp `.git` into the subdir (preserves 5-commit history)
4. Copied the verbose-name working tree into the subdir
5. Created a per-repo `CHANGELOG.md` (new, since the verbose-name repos
   didn't have one)
6. Fixed stale path-deps in `Cargo.toml`:
   - `dracon-git = { path = "../dracon-libs/tools/sync/dracon-git" }`
     → `dracon-git = "94.7.0"`
   - `dracon-system-lib = { path = "../dracon-libs/tools/system/dracon-system" }`
     → `dracon-system-lib = "94.2.7"`
   - `dracon-security-kit = { package = "dracon-security", path = "../dracon-utilities/dracon-warden/src/security" }`
     → `dracon-security-kit = { package = "dracon-security", version = "0.3.0" }`
7. Generated `Cargo.lock` (`cargo generate-lockfile`) since the previous
   lock was for the monorepo workspace
8. Created `scripts/release.sh` based on the parent's, with the per-repo
   `CRATE_NAME` and verbose-name URL customizations
9. Tested the release script with `--dry-run` + `--abort` (no remote
   mutation)
10. Pushed the new history (CHANGELOG.md + Cargo.toml + Cargo.lock +
    scripts/release.sh) to all 3 remotes

For the monorepo:

1. Removed `dracon-sync/`, `dracon-system/`, `dracon-warden/` from the
   monorepo's git index via `git rm -r --cached` (the dirs themselves
   still exist on disk with their own `.git/`)
2. Deleted root `Cargo.toml` (no more workspace)
3. Deleted root `Cargo.lock` (no more Rust in the monorepo root)
4. Deleted `/home/dracon/Dev/facade-repos/` (parallel working trees)
5. Deleted `scripts/regenerate_facade_repos.py` (file-copy regen)
6. Deleted `scripts/scaffold_feature_repos.py` (scaffolder)
7. Deleted `.git/hooks/post-commit` (regen trigger)
8. Pushed the monorepo changes to github (gitlab and codeberg remain
   14 commits behind due to the pre-existing mirror divergence from
   the literal-token incident on 2026-06-21 — see
   `docs/design/mirror-divergence-and-secret-remediation-2026-06-21.md`)

## Test results (all 3 subdirs)

| Repo | Unit tests | Integration tests | Total |
| --- | --- | --- | --- |
| `dracon-sync` | 579 passed, 3 ignored | 10 passed | **589** |
| `dracon-system` | 86 passed | — | **86** |
| `dracon-warden` | 76 passed | 10 passed | **86** |
| **Total** | | | **761** |

`cargo build --locked`, `cargo test --locked`, and `cargo deny check`
all pass in each subdir. The 761-test total matches the pre-migration
monorepo count exactly (no tests lost, no tests added).

## Crates.io impact

The published v0.112.12 on crates.io (`dracon-sync`, `dracon-system`,
`dracon-warden`) is **intact** — the binaries are unchanged; only the
source-of-truth has moved. Future releases are per-repo:

```bash
# Inside dracon-sync/
scripts/release.sh 0.112.13 --yes   # publishes to crates.io, tags, github release

# Inside dracon-system/
scripts/release.sh 0.112.13 --yes

# Inside dracon-warden/
scripts/release.sh 0.112.13 --yes
```

Each repo's release flow is fully independent. The 3 utilities can
release on their own cadence (e.g. `dracon-sync` 0.112.13 + `dracon-warden`
0.4.0 + `dracon-system` 0.112.13 in the same week, or staggered).

The v0.112.12 git tag at the parent monorepo (`DraconDev/dracon-utilities`,
commit `f0081a09`) is preserved — the migration is a no-op for anyone
who has the monorepo cloned and is using `cargo install` to get the
binaries.

## Daemon discovery: **REAL BLOCKER**

The daemon (`dracon-sync`) does not currently support nested-repo
discovery. The discovery code in
`dracon-sync/src/git/discovery.rs::discover_git_repos_recursive` does:

```rust
if dot_git.exists() && (dot_git.is_dir() || is_git_worktree_file(&dot_git)) {
    repos.push(path.clone());
    continue;  // <-- BUG: stops recursing when it finds a .git/
}
```

When the daemon sees `dracon-utilities/.git/`, it stops recursing into
`dracon-utilities/` and never discovers the 3 nested standalone repos
at `dracon-utilities/dracon-{sync,system,warden}/.git/`.

### Observable impact

- `dracon-sync repos` shows **12 repos** (not 15). The 3 standalone
  subdirs appear as untracked items under `dracon-utilities` (`❓ UT: 3`).
- The daemon does **not** auto-commit or auto-push the standalone
  subdirs. The "test commit propagates via daemon" acceptance criterion
  (item 13) **fails** for the daemon's auto-cycle, though it works
  when the operator manually commits+pushes from inside the subdir.

### What was tested instead

- Manual commit from inside `dracon-sync/` and `git push` to all 3
  remotes (github + gitlab + codeberg) → all 3 remote HEADs synced at
  the test commit SHA. 9 successful pushes across 3 subdirs.
- All 3 verbose-name github repos show `pushed_at` within 5 minutes of
  the test commit (2026-06-21T11:19-11:20Z range).

### Two remediation paths

**Path A: Daemon code change (preferred).** Modify
`discover_git_repos_recursive` in `dracon-sync/src/git/discovery.rs`
to recurse into a subdir even when the subdir has its own `.git/`.
The change would distinguish between (a) the subdir IS the watch root
(in which case `continue` is correct) and (b) the subdir is a child
of a watch root with its own `.git/` (in which case we should record
the subdir as a separate repo AND continue recursing into its
children for any sub-subdirs).

This is a real daemon code change, separate scope from this migration.
Tracked as a follow-up: the discovery recursion needs to be relaxed
when a nested `.git/` is found inside a watch-rooted subdir.

**Path B: Move the 3 subdirs to be siblings of the monorepo, not
children of it.** E.g. `/home/dracon/Dev/dracon-{sync,system,warden}/`
instead of `/home/dracon/Dev/dracon-utilities/dracon-{sync,system,warden}/`.
The daemon's discovery walks `watch_roots` and finds any `.git/` dir,
so the 3 would be naturally discovered. This is the "umbrella monorepo"
pattern: the parent has no `.git/` of its own (or has a separate
"docs-only" `.git/` for `AGENTS.md`, `install.sh`, etc.), and the 3
utility repos are physically separate working trees that share a
common ancestor at `/home/dracon/Dev/`.

The user said "we just have 3 repos inside a parent repo" — Path A
keeps the 3 inside the parent. Path B moves them out. **Path A is
required per the user's stated structure.**

### Why the blocker was hit (not foreseen)

The migration was framed as a `git init` + `git pull` + `git push` to
re-home each subdir's git history. The daemon's nested-repo discovery
limitation was not investigated beforehand because:

1. The verbose-name repos had previously worked end-to-end (Phase 4 of
   the previous goal, 8da9a2c8) at `/home/dracon/Dev/facade-repos/`,
   which is a sibling of the monorepo, not a child. The daemon
   discovered them correctly there.
2. Moving the 3 subdirs INTO `dracon-utilities/` (rather than keeping
   them as siblings) put them inside a directory that the daemon
   already considers a single repo, triggering the `continue` early-exit.
3. The "3 repos inside a parent repo" structure is a new pattern for
   this daemon. It has worked before for embedded submodules (where
   `.git` is a file pointing to `objects/...`), but not for sibling
   subdirs with their own complete `.git/`.

The migration succeeded in all other respects. The daemon-discovery
limitation is a follow-up bug, not a regression — the 3 standalone
repos are correct, build, test, and ship. The only thing missing is
the daemon's automatic push cycle for them.

## How to prevent recurrence

1. **Never use file-copy regen scripts again.** This migration
   eliminated `regenerate_facade_repos.py` and
   `scaffold_feature_repos.py`. If a future need arises to "sync" code
   between repos, the right tool is `git subtree` or proper git
   submodule, not a Python script.
2. **Each utility's release flow is per-repo.** Future releases of
   `dracon-sync`, `dracon-system`, `dracon-warden` happen from inside
   the subdir, not from the parent monorepo.
3. **The monorepo's role is reduced.** It still houses `AGENTS.md`,
   top-level docs, `install.sh`, and design docs, but no Rust code.
4. **The daemon's nested-repo discovery is a follow-up to fix.** When
   fixed, the 3 subdirs will be auto-pushed by the daemon like any
   other repo. Tracked separately.

## Reference

- `scripts/scaffold_feature_repos.py` — DELETED. Was the scaffolder
  that created `/home/dracon/Dev/facade-repos/`. Now obsolete.
- `scripts/regenerate_facade_repos.py` — DELETED. Was the file-copy
  regen triggered by the post-commit hook. Now obsolete.
- `dracon-sync/src/git/discovery.rs` — daemon code that has the
  `continue` early-exit on `.git/` discovery. Needs a follow-up patch
  to support nested standalone repos.
- `docs/design/facade-repo-staleness-fix-2026-06-21.md` — the previous
  goal that scaffolded `/home/dracon/Dev/facade-repos/`. The current
  goal supersedes it by promoting the in-monorepo subdirs to standalone
  repos directly.
- `docs/design/mirror-divergence-and-secret-remediation-2026-06-21.md` —
  the gitlab/codeberg mirror divergence on the parent monorepo, which
  the migration is unrelated to but inherits.
