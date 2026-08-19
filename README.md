# Dracon Utilities

Public release repository for the Dracon system service CLI utilities. These tools install to `~/.local/bin/`, run as user-level system services where appropriate, and keep operational state outside the git tree.

This is the meta workspace and release-documentation repository. The three
utilities are standalone git repositories nested below this directory; their
published `dracon-git` and `dracon-system-lib` dependencies come from
crates.io, so no `dracon-libs` checkout is required.

## Latest versions (2026-08-19; Warden candidate prepared locally)

| Utility | Version | What shipped | Release notes |
|---------|---------|--------------|---------------|
| `dracon-sync` | **v0.113.50** | Standalone daemon repository; see its changelog for the latest release notes | [dracon-sync/CHANGELOG.md](dracon-sync/CHANGELOG.md) |
| `dracon-warden` | **v0.113.5** | Standalone encryption and repository-hardening repository; local release candidate built and installed | [dracon-warden/CHANGELOG.md](dracon-warden/CHANGELOG.md) |
| `dracon-system` | **v0.112.37** | Standalone disk/process guard repository | [dracon-system/CHANGELOG.md](dracon-system/CHANGELOG.md) |

The Warden v0.113.5 candidate is source-complete and built locally from the
locked checkout; registry publication, tag creation, and a forge release are
still separate operator-approved steps. The workspace gate is `cargo test
--workspace --locked`, with the additional format, build, deny, and clippy
checks documented in `AGENTS.md`.

## Install

The 3 utilities can be installed independently via `cargo install` from [crates.io](https://crates.io):

```bash
cargo install dracon-sync     # Background git sync daemon
cargo install dracon-system   # Disk, process, guard, doctor
cargo install dracon-warden   # Secret, encrypt, age, git-filter
```

Or build all 3 from a checkout of this meta workspace:

```bash
git clone https://github.com/DraconDev/dracon-utilities.git
cd dracon-utilities
# The parent tracks the standalone repositories by path; clone them first.
git clone https://github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote.git dracon-sync
git clone https://github.com/DraconDev/dracon-system-disk-process-guard-doctor.git dracon-system
git clone https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter.git dracon-warden
cargo build --release --locked
# Binaries at target/release/dracon-{sync,system,warden}
```

Each long-name repository is also independently buildable:

```bash
git clone https://github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote.git
cd dracon-sync-background-auto-commit-multi-remote
cargo build --release --locked
```

## Utilities

| Utility | crates.io | Purpose | Runtime |
|---------|-----------|---------|---------|
| [`dracon-sync`](dracon-sync/README.md) | [crates.io](https://crates.io/crates/dracon-sync) | Background, auto-commit, multi-remote git sync for developer workspaces | systemd user service |
| [`dracon-system`](dracon-system/README.md) | [crates.io](https://crates.io/crates/dracon-system) | Disk, process, guard, doctor — local machine diagnostics and watchdog | systemd user service |
| [`dracon-warden`](dracon-warden/README.md) | [crates.io](https://crates.io/crates/dracon-warden) | Secret, encrypt, age, git-filter — repository hardening and smudge/clean encryption | git hooks + CLI |

### Standalone utility repositories

Each utility has its own standalone repository on GitHub and GitLab. Codeberg
is retired from the active mirror set. The standalone repositories contain the
implementation, tests, examples, and release metadata; this parent repository
provides the shared workspace, installer, CI, and operational documentation.

- [`DraconDev/dracon-sync-background-auto-commit-multi-remote`](https://github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote) (also on [GitLab](https://gitlab.com/DraconDev/dracon-sync-background-auto-commit-multi-remote))
- [`DraconDev/dracon-system-disk-process-guard-doctor`](https://github.com/DraconDev/dracon-system-disk-process-guard-doctor) (also on [GitLab](https://gitlab.com/DraconDev/dracon-system-disk-process-guard-doctor))
- [`DraconDev/dracon-warden-secret-encrypt-age-git-filter`](https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter) (also on [GitLab](https://gitlab.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter))

The names are deliberately descriptive so they are self-explanatory in search
results. The daemon commits each nested repository independently; there is no
façade-generation script or parent post-commit mirror step.

## Repository architecture

This is a 4-repo system with distinct roles. Each repo has one job:

| Repo | Role | Contains | Updated by |
|------|------|----------|------------|
| `DraconDev/dracon-utilities` (this repo) | **Meta workspace** | Workspace manifest, installer, CI, policy docs, and audit records | Operator + daemon for this repo |
| `DraconDev/dracon-sync-background-auto-commit-multi-remote` | **Standalone source/install target** | `dracon-sync` source, tests, config, and release metadata | Daemon watches and pushes this repo |
| `DraconDev/dracon-system-disk-process-guard-doctor` | **Standalone source/install target** | `dracon-system` source, tests, config, and release metadata | Daemon watches and pushes this repo |
| `DraconDev/dracon-warden-secret-encrypt-age-git-filter` | **Standalone source/install target** | `dracon-warden` plus its embedded security crate | Daemon watches and pushes this repo |

Each nested utility repository is a real install target. Users can choose
`cargo install dracon-{sync,system,warden}` from crates.io, clone one of the
standalone repositories, or clone this meta workspace plus its three nested
members.

Releases are cut from the relevant nested repository with its local
`scripts/release.sh`. The parent workspace is for coordinated build and audit
checks; it is not a source-mirroring layer.

## Quick Start

```bash
# Clone the public release repository
git clone https://github.com/DraconDev/dracon-utilities.git
cd dracon-utilities

# Restore the three nested standalone repositories
git clone https://github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote.git dracon-sync
git clone https://github.com/DraconDev/dracon-system-disk-process-guard-doctor.git dracon-system
git clone https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter.git dracon-warden

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
| [docs/design/github-feature-repos.md](docs/design/github-feature-repos.md) | Historical façade-repo design and its replacement |
| [dracon-sync/README.md](dracon-sync/README.md) | Sync daemon usage and configuration |
| [dracon-system/README.md](dracon-system/README.md) | System guard usage and configuration |
| [dracon-warden/README.md](dracon-warden/README.md) | Repo encryption usage and configuration |
| [SECURITY.md](SECURITY.md) | Security reporting policy |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution workflow |

## License

AGPL-3.0-only — see [LICENSE](LICENSE) for details.
