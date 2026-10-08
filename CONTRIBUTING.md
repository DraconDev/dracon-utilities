# Contributing to Dracon Utilities

Thank you for contributing to Dracon Utilities. This repository publishes deterministic local automation tools for git sync, system protection, and secret-at-rest encryption.

## What Belongs Here

This repository is the meta-repo for three CLI utilities and owns their
workspace build, installer, CI, and operational documentation:

- `dracon-sync` — git sync automation (`dracon-sync/`)
- `dracon-system` — disk/process/storage diagnostics and guard behavior (`dracon-system/`)
- `dracon-warden` — git filter encryption and repo hardening (`dracon-warden/`)

Each utility directory is a **nested standalone git repository** (since
2026-09-11): it has its own `.git/`, history, remotes, and tags, and the
parent does NOT track utility source (`git ls-files dracon-sync` returns
nothing — the directories are gitignored here; see AGENTS.md "Repository
architecture"). Clone them next to the parent, edit inside the utility
directory, and commit from there. The published `dracon-git` crate provides
shared library functionality; do not add a local `dracon-libs` path
dependency.

## License

All contributions are licensed under [AGPL-3.0-only](./LICENSE). By submitting a contribution, you agree that it is licensed under the same terms.

## Before You Open a Pull Request

1. **Keep scope small.** One PR should solve one user-visible problem or one cohesive internal refactor.
2. **Update user docs first.** If behavior, configuration, installation, or release process changes, update the root README, the relevant crate README, and examples.
3. **Preserve deterministic behavior.** Daemons and release tooling must not depend on AI, network calls, wall-clock nondeterminism, or hidden local state for core decisions.
4. **Add or update tests.** Use `tempfile::TempDir` for filesystem isolation and scoped environment guards for env mutations.
5. **Run the quality gates.** See [Validation](#validation).
6. **Write a clear PR description.** Explain what changed, why it changed, and how to verify it.

## Setup

Clone the meta-repo, then clone each nested utility next to it (a bare parent
clone does not build — the parent `Cargo.toml` lists the nested crates by
path):

```bash
 # Clone the meta-repo
 git clone https://github.com/DraconDev/dracon-utilities.git
 cd dracon-utilities

 # Clone each nested standalone utility repo next to it
 git clone https://github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote.git dracon-sync
 git clone https://github.com/DraconDev/dracon-system-disk-process-guard-doctor.git dracon-system
 git clone https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter.git dracon-warden

# Optional local diagnostics
./doctor.sh
```

Utility work happens inside the utility directory: `cd dracon-sync`, make the
change, commit there, and push from there. The parent CI checks out all four
repos explicitly.

## Validation

Run these from the repository root:

```bash
# DRACON_SYNC_GIT_BIN is NixOS-only (points at the system git); skip elsewhere.
[ -e /run/current-system/sw/bin/git ] && export DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git

cargo fmt -p dracon-sync -p dracon-system -p dracon-warden -- --check
cargo clippy --workspace --locked --all-targets --no-deps -- -D warnings
cargo test --workspace --locked
cargo build --release --locked -p dracon-sync -p dracon-system -p dracon-warden
cargo deny check
./scripts/verify-spec.sh
./scripts/check-nested-pins.py
./scripts/check-flake.sh
./install.sh --dry-run
```

If you hit flaky races locally (some tests mutate process-wide state such as `PATH` or environment variables), re-run the failing crate serially with `-- --test-threads=1` to confirm.

When a nested utility advances, run `scripts/check-nested-pins.py --check-local`
after updating the CI checkout refs and `flake.lock`. The check also verifies
that the parent `Cargo.lock` carries the nested package versions. The generic
Nix checker emits a harmless warning for the conventional `homeManagerModules`
output; `scripts/check-flake.sh` treats only that known warning as allowed.

## Documentation Standards

- The root [`README.md`](README.md) is the public quick start and must stay accurate.
- Each utility README must explain purpose, install, commands, configuration, safety notes, and links to deeper docs.
- Design notes in `docs/design/` describe decisions and tradeoffs. They are not user guides.
- Global hook ownership behavior is documented in [`docs/design/warden-global-hook-ownership-2026-08-15.md`](docs/design/warden-global-hook-ownership-2026-08-15.md).
- Blueprints in crate directories are implementation notes. Keep them updated when behavior changes.
- Do not link to removed internal audit files, private state, local task directories, or legacy paths that do not exist in the public tree.

## Commit Messages

Manual commits should be concise and searchable. The sync daemon generates deterministic commit messages from diffs; contributors do not need to hand-craft sync commits.

For manual commits, prefer simple subjects such as:

```text
docs(readme): clarify public install steps
fix(sync): repair origin URL detection
test(warden): cover plaintext sibling hatch
```

## Release Checklist

1. Update the utility's crate version in its nested repo (`dracon-<name>/Cargo.toml`); releases are tagged in the NESTED repo (`dracon-sync-vX.Y.Z`, …).
2. Add release notes to that utility's `CHANGELOG.md` (the parent CHANGELOG is a frozen historical record).
3. Run the full validation command set.
4. Create and push the annotated tag in the nested repo, for example `dracon-sync-v0.113.95`.
5. Create the GitHub release from the tag.
6. Verify the release tag, release notes, and public README before announcing.

## Getting Help

- Start with [`README.md`](README.md).
- Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the service model.
- Read [`docs/OPERATIONS.md`](docs/OPERATIONS.md) for runtime troubleshooting.
- Report security issues according to [`SECURITY.md`](SECURITY.md).
