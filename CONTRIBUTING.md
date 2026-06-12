# Contributing to Dracon Utilities

Thank you for contributing to Dracon Utilities. This repository publishes deterministic local automation tools for git sync, system protection, and secret-at-rest encryption.

## What Belongs Here

This repository owns three CLI binaries and their release packaging:

- `dracon-sync` — git sync automation
- `dracon-system` — disk/process/storage diagnostics and guard behavior
- `dracon-warden` — git filter encryption and repo hardening

Shared library code lives in the sibling [`dracon-libs`](https://github.com/DraconDev/dracon-libs) repository. Keep reusable capabilities in `dracon-libs`; keep these crates focused on CLI, policy, packaging, and orchestration.

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
# Required sibling dependency
git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs

# Optional local diagnostics
./doctor.sh
```

## Validation

Run these from the repository root:

```bash
export DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git

cargo fmt -p dracon-sync -p dracon-system -p dracon-warden -- --check
cargo clippy -p dracon-sync -p dracon-system -p dracon-warden --all-targets --no-deps
cargo test --workspace -- --test-threads=1
cargo build --release -p dracon-sync -p dracon-system -p dracon-warden
cargo deny check
./scripts/verify-spec.sh
./install.sh --dry-run
```

Use `--test-threads=1` for the full workspace test run. Some tests mutate process-wide state such as `PATH` or environment variables, and serial execution avoids flaky races.

## Documentation Standards

- The root [`README.md`](README.md) is the public quick start and must stay accurate.
- Each utility README must explain purpose, install, commands, configuration, safety notes, and links to deeper docs.
- Design notes in `docs/design/` describe decisions and tradeoffs. They are not user guides.
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
