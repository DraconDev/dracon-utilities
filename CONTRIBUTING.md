# Contributing to Dracon Utilities

Thank you for your interest in contributing! This project is primarily designed for AI agents working on the dracon system, but human contributions are welcome too.

## License

All contributions are subject to the terms of the [AGPLv3 license](./LICENSE) and the [Contributor License Agreement (CLA)](./CLA.md).

**By submitting a Contribution (including via pull request, issue, comment, or any other method), you agree to be bound by both the AGPLv3 license and the CLA.**

## Before You Submit a Pull Request

1. **Read the CLA** — Make sure you understand and agree to the [Contributor License Agreement](./CLA.md) before submitting any Contribution.
2. **Fork and branch** — Create a feature branch from `main` for your changes.
3. **Write clean, idiomatic code** — Follow the existing style and conventions of the project.
4. **Test your changes** — Ensure all existing and new tests pass before opening a PR.
5. **Describe your changes** — Include a clear PR description explaining *what* changed and *why*.
6. **Keep scope small** — One PR per logical change. Don't bundle unrelated fixes.

## Quick Links

- [README.md](README.md) — User-facing documentation
- [AGENTS.md](AGENTS.md) — AI agent conventions and architecture
- [CHANGELOG.md](CHANGELOG.md) — Version history

## Architecture

This repo contains **CLI binaries** that install to `~/.local/bin/` and run as systemd user services. Shared library code lives in the separate `dracon-libs` repository.

```
dracon-utilities/           <- CLI binaries (this repo)
├── dracon-sync/
├── dracon-system/
├── dracon-warden/
└── dracon-ai/

dracon-libs/                <- Shared libraries (required for building)
├── services/ai/
└── tools/sync/dracon-git/
```

## Prerequisites

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Required sibling directory
git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs

# Verify setup
./doctor.sh
```

## Development Workflow

### Building

```bash
# Build a specific utility
cd dracon-sync && cargo build --release
cd dracon-system && cargo build --release
cd dracon-warden && cargo build --release
cd dracon-ai && cargo build --release

# Or use install.sh for full install
./install.sh
```

### Testing

```bash
# dracon-sync (406 tests, use --test-threads=1 for reliability)
cd dracon-sync
export DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git
cargo test -- --test-threads=1

# dracon-system
cd dracon-system
cargo test -- --test-threads=1

# dracon-warden
cd dracon-warden
cargo test
```

### Code Style

```bash
# Format
cargo fmt

# Lint
cargo clippy
```

## Project Structure

### dracon-sync
- **Purpose**: Git sync automation
- **Key files**: `src/sync.rs`, `src/git.rs`, `src/report.rs`
- **Tests**: 406 tests using `tempfile::TempDir`
- **Config**: `~/.dracon/utilities/sync/dracon-sync.toml`

### dracon-system
- **Purpose**: Disk/process protection
- **Key files**: `src/main.rs` (guard runtime), `src/storage.rs`
- **Tests**: 55 tests
- **Config**: `~/.dracon/utilities/system/dracon-system.toml`

### dracon-warden
- **Purpose**: Security hardening via git filters
- **Key files**: `src/main.rs`, `src/security/`
- **Config**: `~/.dracon/utilities/warden/dracon-warden.toml`

### dracon-ai
- **Purpose**: AI CLI interface
- **Key files**: `src/main.rs`
- **Config**: `~/.dracon/utilities/ai/dracon-ai.toml`

## Design Principles

1. **Deterministic daemons**: Sync, system, and warden must run without AI dependencies
2. **AI utilities**: dracon-ai is the only utility that talks to AI providers
3. **Invisible infrastructure**: Sync should be invisible to the AI coder
4. **Safety first**: All destructive operations require `--apply` flag
5. **Persistent state**: Operational state lives outside the git tree (`~/.local/state/dracon/`)

## Adding New Features

### Process for Adding Features

1. **Update AGENTS.md** if the change affects AI workflows
2. **Update example configs** if new options are added
3. **Add tests** for new functionality
4. **Update README.md** if user-facing behavior changes
5. **Update CHANGELOG.md** with version bump

### Adding New Config Options

When adding config options to a utility:

1. Add to the policy struct in `src/main.rs` with `#[serde(default = "...")]`
2. Add default function
3. Add to the `Default` impl
4. Add validation in `validate_policy()` if needed
5. Add to example config file
6. Document in AGENTS.md

## Commit Messages

Since this repo uses dracon-sync for auto-commit, you don't need to worry about commit messages if sync is running. The AI scribe generates commit messages directly from diffs (with local file-pattern fallback).

For manual commits, use conventional commits:
```
feat(sync): add webhook notifications
fix(system): correct process CPU threshold
docs(readme): update installation instructions
```

## Testing Guidelines

### Writing Tests

- Use `tempfile::TempDir` for file system isolation
- Use `EnvRestorer` for environment variable mutations
- Use `--test-threads=1` for dracon-sync tests (parallel tests have race conditions)

### Test Environment

```bash
# Required for git tests
export DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git

# Optional: increase verbosity
export RUST_LOG=debug
```

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Tag release: `git tag v0.X.Y`
5. Push: `git push origin v0.X.Y`

## Getting Help

- Check [AGENTS.md](AGENTS.md) for architecture details
- Run `./doctor.sh` to diagnose setup issues
- Check `~/.local/state/dracon/dracon-sync-incidents.jsonl` for runtime errors

---

*For details on commercial licensing, see [COMMERCIAL-LICENSE.md](./COMMERCIAL-LICENSE.md).*