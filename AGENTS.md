# Dracon Utilities

CLI binaries for dracon system services. These install to `~/.local/bin/` and run as systemd user services.

## Architecture

```
dracon-utilities/           <- CLI binaries (this repo)
├── dracon-sync/            -> ~/.local/bin/dracon-sync
├── dracon-system/          -> ~/.local/bin/dracon-system
├── dracon-warden/          -> ~/.local/bin/dracon-warden
└── dracon-ai/              -> ~/.local/bin/dracon-ai

dracon-libs/tools/          <- Shared libraries (not installed)
├── sync/dracon-git/        <- git operations library
├── system/dracon-system/   <- system diagnostics library
└── config/dracon-config/   <- config parsing library
```

**Key point:** `dracon-utilities` contains the CLI wrappers. `dracon-libs` contains shared library code. Only the CLI binaries get installed.

## Installation

All binaries install to `~/.local/bin/`:

```bash
# Install all utilities
./install.sh

# Or individually:
cargo install --path dracon-sync --root ~/.local --force
cargo install --path dracon-system --root ~/.local --force
cargo install --path dracon-warden --root ~/.local --force
cargo install --path dracon-ai --root ~/.local --force
```

## Services

Services are in `~/.config/systemd/user/`:

| Service | Binary | Purpose |
|---------|--------|---------|
| dracon-sync.service | dracon-sync daemon | Git sync automation |
| dracon-system-guard.service | dracon-system guard daemon | Disk/process protection |
| dracon-warden.service | dracon-warden daemon | Security hardening |

```bash
# Restart after install
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
systemctl --user restart dracon-warden.service
```

## Policy Files

| Utility | Policy Path |
|---------|-------------|
| dracon-sync | ~/dracon/utilities/sync/dracon-sync.toml |
| dracon-system | ~/dracon/utilities/system/dracon-system.toml |
| dracon-warden | ~/dracon/utilities/warden/dracon-warden.toml |
