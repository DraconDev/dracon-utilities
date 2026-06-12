# Dracon Utilities

CLI binaries for dracon system services. These install to `~/.local/bin/` and run as systemd user services.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/DraconDev/dracon-utilities.git
cd dracon-utilities

# Install all utilities
./install.sh

# Restart services after installation
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
```

## Utilities

### dracon-sync

**Invisible git sync for AI-powered development.** An auto-commit, multi-mirror daemon that watches your repos, commits every change with deterministic, facts-based messages, and pushes to GitHub, GitLab, and Codeberg simultaneously.

**Key Features:**
- Auto-commit on file changes
- Multi-provider mirror (GitHub, GitLab, Codeberg)
- Deterministic commit messages (routing keys for AI-to-AI communication)
- Automatic remote creation
- Self-healing and repair

**Commit Message Format:**
```
CLOSED: Implement JWT | 3 file(s) in src [auth.py, jwt.py] DELTA:+140/-12 | TEST:45
WIP: Refactor DB | 2 file(s) in src [db.py] DELTA:+50/-10
3 file(s) in src [auth.py] DELTA:+100/-20 | TEST:30 | NEW:src/auth.py DEPS:+reqwest,-hyper
```

Every metric is extracted deterministically from the diff — no AI, no guessing. Messages are optimized for `git log --grep=` queries.

**Quick Commands:**
```bash
dracon-sync status          # Show policy path and sync scope
dracon-sync repos           # Report across all repositories
dracon-sync daemon          # Run continuous sync loop
dracon-sync sync-now ~/Dev/my-project  # Sync specific repo now
```

**Documentation:** [dracon-sync/README.md](dracon-sync/README.md)

### dracon-system

**Proactive disk space monitoring and automatic cleanup.** Prevents "disk full" emergencies on development machines and servers.

**Key Features:**
- Disk space monitoring with configurable thresholds
- Automatic Rust target directory cleanup
- Process monitoring and graduated renice
- Storage hotspot analysis
- Zombie process detection

**Quick Commands:**
```bash
dracon-system status        # Show core path and service status
dracon-system doctor        # Run deterministic diagnostics
dracon-system storage ~/Dev # Analyze storage hotspots
dracon-system guard daemon  # Run continuous monitoring
```

**Documentation:** [dracon-system/README.md](dracon-system/README.md)

### dracon-warden

**Git filter + repo hardening tool.** Encrypts secrets at rest in git while keeping plaintext in your working tree. Uses git hooks (not a daemon) as the primary enforcement layer.

**Key Features:**
- Age-based encryption for secrets
- Secret scanning and detection
- Clean/smudge git filter pipeline
- Repo hardening and key management
- Team key distribution

**Quick Commands:**
```bash
dracon-warden status        # Show resolved policy path and repo roots
dracon-warden once          # Run one hardening pass
dracon-warden keygen        # Generate new age keypair
dracon-warden setup-hooks --global  # Install git hooks (primary enforcement)
```

**Documentation:** [dracon-warden/README.md](dracon-warden/README.md)

## Architecture

```
dracon-utilities/           <- CLI binaries (this repo)
├── dracon-sync/            -> ~/.local/bin/dracon-sync
├── dracon-system/          -> ~/.local/bin/dracon-system
└── dracon-warden/          -> ~/.local/bin/dracon-warden

dracon-libs/                <- Shared libraries (REQUIRED for building)
├── services/ai/            <- AI adapters, router, lanes
└── tools/sync/dracon-git/  <- git operations library
```

**Key point:** `dracon-utilities` contains the CLI wrappers. `dracon-libs` contains shared library code. Only the CLI binaries get installed.

## Prerequisites

**Required sibling directory:** `dracon-libs` must be checked out as a sibling to `dracon-utilities`:

```
~/Dev/
├── dracon-utilities/    <- this repo
└── dracon-libs/         <- required for building
    ├── services/ai/
    └── tools/sync/dracon-git/
```

Clone if needed:
```bash
git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs
```

## Installation

All binaries install to `~/.local/bin/`:

```bash
./install.sh
```

The install script will:
1. Build all release binaries
2. Install to `~/.local/bin/`
3. Set up systemd user services
4. Start/enable services

## Systemd Services

| Service | Binary | Purpose |
|---------|--------|---------|
| dracon-sync.service | dracon-sync daemon | Git sync automation |
| dracon-system-guard.service | dracon-system guard daemon | Disk/process protection |

> **Note:** `dracon-warden` has no systemd service — git hooks (installed via `setup-hooks --global`) are the primary security enforcement layer.

```bash
# Check status
systemctl --user status dracon-sync.service
systemctl --user status dracon-system-guard.service

# View logs
journalctl --user -u dracon-sync -f
journalctl --user -u dracon-system-guard -f

# Restart after config changes
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
```

## Configuration

Each utility has its own configuration file:

| Utility | Config Path | Example |
|---------|-------------|---------|
| dracon-sync | ~/.dracon/utilities/sync/dracon-sync.toml | [dracon-sync.example.toml](dracon-sync/dracon-sync.example.toml) |
| dracon-system | ~/.dracon/utilities/system/dracon-system.toml | [dracon-system.example.toml](dracon-system/dracon-system.example.toml) |
| dracon-warden | ~/.dracon/utilities/warden/dracon-warden.toml | [dracon-warden.example.toml](dracon-warden/dracon-warden.example.toml) |

## Environment Variables

| Variable | Utility | Purpose |
|----------|---------|---------|
| `DRACON_SYNC_GIT_BIN` | dracon-sync | Override path to git binary |
| `DRACON_SYNC_POLICY` | dracon-sync | Custom sync policy file path |
| `DRACON_SYSTEM_POLICY` | dracon-system | Custom system policy file path |

## Testing

```bash
export DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git

# dracon-sync (serial for reliability):
cargo test -p dracon-sync -- --test-threads=1

# dracon-system & dracon-warden:
cargo test -p dracon-system
cargo test -p dracon-warden
```

## Development

```bash
# Build all utilities
cargo build --release

# Build specific utility
cargo build --release -p dracon-sync
cargo build --release -p dracon-system
cargo build --release -p dracon-warden

# Code quality
cargo clippy --all-targets --all-features
cargo fmt --check
cargo deny check
```

## Documentation

| Document | Purpose |
|----------|---------|
| [docs/ROADMAP.md](docs/ROADMAP.md) | Where to find everything |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Service architecture and commit protocol |
| [docs/OPERATIONS.md](docs/OPERATIONS.md) | Systemd, troubleshooting, incident response |
| [dracon-sync/README.md](dracon-sync/README.md) | Sync daemon usage and configuration |
| [dracon-system/README.md](dracon-system/README.md) | System guard usage and configuration |
| [dracon-warden/README.md](dracon-warden/README.md) | Repo encryption usage and configuration |
| [SECURITY.md](SECURITY.md) | Security reporting policy |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Development workflow |

## License

AGPL v3 — See [LICENSE](LICENSE) for details.
