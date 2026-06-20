# Dracon Utilities

Public release repository for the Dracon system service CLI utilities. These tools install to `~/.local/bin/`, run as user-level system services where appropriate, and keep operational state outside the git tree.

**Current release:** `v0.112.8` — [release notes](https://github.com/DraconDev/dracon-utilities/releases/tag/v0.112.8)

This repository contains the CLI wrappers and release packaging. Shared library code lives in the sibling [`dracon-libs`](https://github.com/DraconDev/dracon-libs) repository.

## Install

The 3 utilities can be installed independently via `cargo install` from [crates.io](https://crates.io):

```bash
cargo install dracon-sync     # Background git sync daemon
cargo install dracon-system   # Disk, process, guard, doctor
cargo install dracon-warden   # Secret, encrypt, age, git-filter
```

Or install all 3 from the monorepo:

```bash
git clone https://github.com/DraconDev/dracon-utilities.git
cd dracon-utilities
cargo build --release
# Binaries at target/release/dracon-{sync,system,warden}
```

Or install any one of the 3 long-name façade repos (independently buildable since v0.112.7):

```bash
git clone https://github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote.git
cd dracon-sync-background-auto-commit-multi-remote
cargo build --release
```

## Utilities

| Utility | crates.io | Purpose | Runtime |
|---------|-----------|---------|---------|
| [`dracon-sync`](dracon-sync/README.md) | [crates.io](https://crates.io/crates/dracon-sync) | Background, auto-commit, multi-remote git sync for developer workspaces | systemd user service |
| [`dracon-system`](dracon-system/README.md) | [crates.io](https://crates.io/crates/dracon-system) | Disk, process, guard, doctor — local machine diagnostics and watchdog | systemd user service |
| [`dracon-warden`](dracon-warden/README.md) | [crates.io](https://crates.io/crates/dracon-warden) | Secret, encrypt, age, git-filter — repository hardening and smudge/clean encryption | git hooks + CLI |

### Façade repos

Each utility has a feature-presentation repo (a "façade") for discoverability on GitHub, GitLab, and Codeberg. As of v0.112.7, each façade repo is **independently buildable**: it contains the actual source code (mirrored from the monorepo), a standalone `Cargo.toml`, tests, examples, and the per-utility README. Cloning a façade repo + its sibling `dracon-libs` (and for warden, `dracon-utilities`) is enough to build that one utility.

- [`DraconDev/dracon-sync-background-auto-commit-multi-remote`](https://github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote) (also on [GitLab](https://gitlab.com/DraconDev/dracon-sync-background-auto-commit-multi-remote) + [Codeberg](https://codeberg.org/dracondev/dracon-sync-background-auto-commit-multi-remote))
- [`DraconDev/dracon-system-disk-process-guard-doctor`](https://github.com/DraconDev/dracon-system-disk-process-guard-doctor) (also on [GitLab](https://gitlab.com/DraconDev/dracon-system-disk-process-guard-doctor) + [Codeberg](https://codeberg.org/dracondev/dracon-system-disk-process-guard-doctor))
- [`DraconDev/dracon-warden-secret-encrypt-age-git-filter`](https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter) (also on [GitLab](https://gitlab.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter) + [Codeberg](https://codeberg.org/dracondev/dracon-warden-secret-encrypt-age-git-filter))

The names are deliberately brutally-descriptive so they are self-explanatory in search results. The 3 façade repos stay in sync with this monorepo via `scripts/regenerate_facade_repos.py` (called from a `post-commit` hook). See `docs/design/github-feature-repos.md` for the full design.

## Repository architecture

This is a 4-repo system with distinct roles. Each repo has one job:

| Repo | Role | Contains | Updated by |
|------|------|----------|------------|
| `DraconDev/dracon-utilities` (this repo) | **Dev workspace + build source** | All 3 utilities' source code + monorepo build + `install.sh` + tests + docs | The operator (manual commits) + `dracon-sync` daemon (auto-commits to all 4 remotes) |
| `DraconDev/dracon-sync-background-auto-commit-multi-remote` | **Canonical install target** for `dracon-sync` | Real source code (mirrored from monorepo) + standalone `Cargo.toml` + tests + README + LICENSE + .github/ | `post-commit` hook → `regenerate_facade_repos.py` → `dracon-sync` daemon (auto-pushes to all 3 remotes) |
| `DraconDev/dracon-system-disk-process-guard-doctor` | **Canonical install target** for `dracon-system` | Same as above | Same |
| `DraconDev/dracon-warden-secret-encrypt-age-git-filter` | **Canonical install target** for `dracon-warden` | Same as above | Same |

**Each repo is a real install target** since v0.112.7. Users can choose any of the 4 paths: `cargo install dracon-{sync,system,warden}` from crates.io, clone the long-name façade repo, or clone the monorepo.

**The flow is one-way**: operator edits code in the monorepo → commits trigger the `post-commit` hook → the hook runs `regenerate_facade_repos.py` for the affected utility → the script writes the new content to the 3 façade repo clones at `/home/dracon/Dev/facade-repos/` → the daemon (`dracon-sync`) sees the local change and auto-pushes to GitHub + GitLab + Codeberg. The crates.io publish is a separate manual step (`cargo publish -p <crate>`).

For a per-utility visitor who lands on a façade repo, the README has standalone build + install instructions. For a developer who wants to build all 3, they clone the monorepo + the sibling `dracon-libs` repo and run `./install.sh` (or `cargo build --release`). For a user who just wants to install one tool, they can `cargo install dracon-{sync,system,warden}` from crates.io. See `docs/design/github-feature-repos.md` for the full design.

## Quick Start

```bash
# Clone the public release repository
git clone https://github.com/DraconDev/dracon-utilities.git
cd dracon-utilities

# Required for building
git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs

# Install all utilities
./install.sh

# Restart services after installation
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
```

`dracon-warden` is installed by `install.sh` and enabled through `dracon-warden setup-hooks --global`; it does not run as a daemon.

## What Each Utility Does

### `dracon-sync`

`dracon-sync` watches configured git repositories, waits for changes to settle, commits deterministic diff-based messages, and pushes to origin and optional mirrors.

Common commands:

```bash
dracon-sync status          # Show policy path, roots, and discovered repos
dracon-sync repos           # One-shot repo report
dracon-sync health          # Daemon health check
dracon-sync daemon          # Run continuous sync loop
dracon-sync sync-now ~/Dev/my-project
dracon-sync sync-now --warns       # handle current WARN rows now
dracon-sync config validate
```

See [`dracon-sync/README.md`](dracon-sync/README.md) for configuration, mirrors, commit messages, repair commands, and release pipeline behavior.

### `dracon-system`

`dracon-system` protects local machines from disk/process pressure and provides storage, link, zram, and service diagnostics.

Common commands:

```bash
dracon-system status        # Show core path and service status
dracon-system doctor        # Run deterministic diagnostics
dracon-system storage ~/Dev # Analyze storage hotspots
dracon-system guard daemon  # Run continuous monitoring
dracon-system link status   # Check configured symlinks
dracon-system zram --status
```

See [`dracon-system/README.md`](dracon-system/README.md) for thresholds, cleanup behavior, process renice policy, and deployment examples.

### `dracon-warden`

`dracon-warden` encrypts secret-shaped content at rest in git while keeping normal plaintext files in the working tree. It uses git clean/smudge filters and global or local hooks as the primary enforcement layer.

Common commands:

```bash
dracon-warden status        # Show resolved policy and repo roots
dracon-warden keygen        # Generate a machine age keypair
dracon-warden setup-hooks --global
dracon-warden once
dracon-warden scrub-markers
dracon-warden resmudge
```

See [`dracon-warden/README.md`](dracon-warden/README.md) for the encryption model, plaintext-sibling escape hatch, recovery tools, and safety notes.

## Configuration

Each utility reads its own TOML policy under `~/.dracon/utilities/`:

| Utility | Policy | Example |
|---------|--------|---------|
| `dracon-sync` | `~/.dracon/utilities/sync/dracon-sync.toml` | [`dracon-sync/dracon-sync.example.toml`](dracon-sync/dracon-sync.example.toml) |
| `dracon-system` | `~/.dracon/utilities/system/dracon-system.toml` | [`dracon-system/dracon-system.example.toml`](dracon-system/dracon-system.example.toml) |
| `dracon-warden` | `~/.dracon/utilities/warden/dracon-warden.toml` | [`dracon-warden/dracon-warden.example.toml`](dracon-warden/dracon-warden.example.toml) |

Operational state lives outside this repository, for example:

```text
~/.local/state/dracon/
├── dracon-sync-incidents.jsonl
├── dracon-sync-stuck-push-repos.json
├── dracon-system-guard.log
└── visibility-sync/
```

## Services

| Service | Binary | Purpose |
|---------|--------|---------|
| `dracon-sync.service` | `dracon-sync daemon` | Git sync automation |
| `dracon-system-guard.service` | `dracon-system guard daemon` | Disk/process protection |

Useful commands:

```bash
systemctl --user status dracon-sync.service dracon-system-guard.service
journalctl --user -u dracon-sync -f
journalctl --user -u dracon-system-guard -f
systemctl --user restart dracon-sync.service dracon-system-guard.service
```

## Development

```bash
# Reliable full test run
export DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git
cargo test --workspace -- --test-threads=1

# Quality gates
cargo fmt -p dracon-sync -p dracon-system -p dracon-warden -- --check
cargo clippy -p dracon-sync -p dracon-system -p dracon-warden --all-targets --no-deps
cargo build --release -p dracon-sync -p dracon-system -p dracon-warden
cargo deny check
./scripts/verify-spec.sh
```

## Documentation

| Document | Purpose |
|----------|---------|
| [docs/ROADMAP.md](docs/ROADMAP.md) | Documentation map and release status |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Service architecture and deterministic commit protocol |
| [docs/OPERATIONS.md](docs/OPERATIONS.md) | Systemd, incident response, troubleshooting |
| [docs/design/cli-print-style.md](docs/design/cli-print-style.md) | Human-facing CLI output conventions |
| [docs/design/warden-plaintext-sibling.md](docs/design/warden-plaintext-sibling.md) | Warden plaintext escape hatch threat model |
| [docs/design/github-feature-repos.md](docs/design/github-feature-repos.md) | GitHub façade repos for feature-focused utility surfaces |
| [dracon-sync/README.md](dracon-sync/README.md) | Sync daemon usage and configuration |
| [dracon-system/README.md](dracon-system/README.md) | System guard usage and configuration |
| [dracon-warden/README.md](dracon-warden/README.md) | Repo encryption usage and configuration |
| [SECURITY.md](SECURITY.md) | Security reporting policy |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution workflow |

## License

AGPL-3.0-only — see [LICENSE](LICENSE) for details.
