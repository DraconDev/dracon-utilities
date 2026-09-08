# Contributing to Dracon Utilities

Thank you for contributing to Dracon Utilities. This repository publishes deterministic local automation tools for git sync, system protection, and secret-at-rest encryption.

## What Belongs Here

This repository is the monorepo for three CLI utilities and owns their
workspace build, installer, CI, and operational documentation:

- `dracon-sync` — git sync automation (`dracon-sync/`)
- `dracon-system` — disk/process/storage diagnostics and guard behavior (`dracon-system/`)
- `dracon-warden` — git filter encryption and repo hardening (`dracon-warden/`)

Each utility's implementation lives directly in its tracked directory here
(imported via subtree merges on 2026-08-22, so history stays connected).
The standalone GitHub repos of the same names are frozen mirrors — work in
this repo, not in clones of those. The published `dracon-git` crate provides
shared library functionality; do not add a local `dracon-libs` path dependency.

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

```bash
# Clone the monorepo and work in its tracked utility directories
 git clone https://github.com/DraconDev/dracon-utilities.git
 cd dracon-utilities

# Optional local diagnostics
./doctor.sh
```

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

1. Update crate/workspace versions as needed.
2. Add release notes to [`CHANGELOG.md`](CHANGELOG.md).
3. Run the full validation command set.
4. Create and push an annotated tag, for example `v0.112.5`.
5. Create the GitHub release from the tag.
6. Verify the release tag, release notes, and public README before announcing.

## Getting Help

- Start with [`README.md`](README.md).
- Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the service model.
- Read [`docs/OPERATIONS.md`](docs/OPERATIONS.md) for runtime troubleshooting.
- Report security issues according to [`SECURITY.md`](SECURITY.md).
