# GitHub Feature Repositories — superseded design

Status: historical. The façade model described here was replaced on
2026-06-21 by three standalone utility repositories. The current architecture
is documented in the root [`README.md`](../../README.md) and
[`AGENTS.md`](../../AGENTS.md).

## Current architecture

`dracon-utilities` is a meta-only workspace. The canonical install targets
are the nested standalone repositories:

- `dracon-sync-background-auto-commit-multi-remote`
- `dracon-system-disk-process-guard-doctor`
- `dracon-warden-secret-encrypt-age-git-filter`

Each contains its own source, tests, Cargo manifest, release scripts, history,
and remotes. The parent workspace lists the nested paths so a local checkout
can run the shared test and build gates. The daemon watches and commits each
nested repository independently.

No `regenerate_facade_repos.py`, façade post-commit hook, or `dracon-libs`
path-dependency checkout is required by the current layout. `dracon-git` and
`dracon-system-lib` are consumed from crates.io; the warden security crate is
kept inside the warden repository.

## Historical context

The former façade proposal kept small navigation-only repositories in front of
the monorepo. It was superseded when the utility repositories became the
actual standalone install targets. This file remains so old release notes and
links have a clear explanation rather than silently pointing at removed
scripts.
