# Release v0.112.7 — 2026-06-16

## Summary

This release **flips the architecture** of the 3 long-name façade repos. They
were navigation shells (7 files of metadata that pointed to the monorepo); they
are now **real install targets** with the actual source code, a standalone
`Cargo.toml`, tests, examples, and the per-utility README. This is in direct
response to the operator's feedback: "are they mains? we are not pushing to
them they are still shells".

## Standalone install (the new flow)

The 3 long-name façade repos are now the **canonical install targets** for
each utility. A user can clone any one of them and build it without cloning
the monorepo first.

### `dracon-sync-background-auto-commit-multi-remote`

```bash
git clone https://github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote.git
cd dracon-sync-background-auto-commit-multi-remote
git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs
cargo build --release
# target/release/dracon-sync is ready to install
```

### `dracon-system-disk-process-guard-doctor`

```bash
git clone https://github.com/DraconDev/dracon-system-disk-process-guard-doctor.git
cd dracon-system-disk-process-guard-doctor
git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs
cargo build --release
# target/release/dracon-system is ready to install
```

### `dracon-warden-secret-encrypt-age-git-filter`

```bash
git clone https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter.git
cd dracon-warden-secret-encrypt-age-git-filter
git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs
git clone https://github.com/DraconDev/dracon-utilities.git ../dracon-utilities
cargo build --release
# target/release/dracon-warden is ready to install
# (the monorepo is needed for the `dracon-security` kit in src/security)
```

## What changed (architecture inversion)

| Before (v0.112.6) | After (v0.112.7) |
|--------------------|------------------|
| Façade repos had 7 metadata files only | Façade repos have 25-50 files including source |
| Source code lived only in the monorepo | Source code lives in both (mirrored one-way) |
| `Cargo.toml` did not exist in the façade | `Cargo.toml` exists, with path-dep siblings |
| README pointed to the monorepo for source | README has sibling-clone instructions + standalone build |
| The façade was a "navigation surface" | The façade is a "real install target" |
| "The monorepo is the only source of truth" | "The monorepo is the source of truth; the façade is a mirror" |

## Auto-sync mechanism (unchanged from v0.112.5)

The monorepo's `post-commit` hook calls `scripts/regenerate_facade_repos.py`
which detects which utility's source files changed and re-mirrors the content
to the corresponding façade repo. The `dracon-sync` daemon picks up the local
change in `/home/dracon/Dev/facade-repos/<name>` and auto-pushes to the 3
remotes (github, gitlab, codeberg). The flow is one-way: monorepo → façade.

## File counts (post-conversion)

| Repo | Before | After |
|------|--------|-------|
| `dracon-sync-background-auto-commit-multi-remote` | 7 files | 47 files |
| `dracon-system-disk-process-guard-doctor` | 7 files | 25 files |
| `dracon-warden-secret-encrypt-age-git-filter` | 7 files | 50 files |

## Verified standalone builds

| Repo | Build | Tests (sequential) |
|------|-------|---------------------|
| `dracon-sync-background-auto-commit-multi-remote` | ✓ | 575 passed, 0 failed, 3 ignored |
| `dracon-system-disk-process-guard-doctor` | ✓ | 86 passed, 0 failed, 0 ignored |
| `dracon-warden-secret-encrypt-age-git-filter` | ✓ | 86 passed, 0 failed, 0 ignored |

## Version bumps

- Root workspace: `0.112.6` → `0.112.7` (patch-level, doc/infra only)
- `dracon-sync`: `0.1.7` → `0.1.8`
- `dracon-system`: `0.2.2` → `0.2.3`
- `dracon-warden`: `0.3.2` → `0.3.3`

## What's in the box (since v0.112.6)

The full `[0.112.7]` CHANGELOG section includes:

- **Architecture inversion**: 3 long-name façade repos are now real install
  targets with mirrored source code (not navigation shells)
- **Standalone build support**: each façade repo has a `Cargo.toml` with
  path-dep siblings; `cargo build --release` works from each repo
- **Updated `scripts/scaffold_feature_repos.py`**: new
  `_copy_utility_source()` + `_standalone_cargo_toml()` functions that mirror
  per-utility source code from the monorepo and generate standalone manifests
- **Updated design doc** (`docs/design/github-feature-repos.md`): invariants
  section flipped; "Why this is not a hack" updated to describe the
  one-way mirror mechanism
- **Verified**: all 3 repos build + test standalone (test counts above)

## Migration notes

- If you previously cloned only the monorepo, you can now also clone any of
  the 3 long-name façade repos and build it standalone (with the sibling
  clones as documented above).
- The monorepo is still the source of truth for development. The 3 façade
  repos are downstream mirrors. Changes to per-utility source files in the
  monorepo flow to the corresponding façade repo automatically via the
  `post-commit` hook + `regenerate_facade_repos.py`.

## What's next

- The 3 façade repos will now stay in sync with the monorepo via the
  post-commit hook + daemon auto-push (unchanged from v0.112.5)
- Operator to decide on the 3 GitLab Set A repos (still pending from goal
  `83e42c15`; default `A` leave-as-is is in effect)
