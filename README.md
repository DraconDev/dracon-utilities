# Dracon Utilities

CLI binaries for dracon system services. These install to `~/.local/bin/` and run as systemd user services.

## Binaries

| Binary | Purpose |
|--------|---------|
| **dracon-sync** | Git sync automation daemon — auto-commit, pull, push with AI-powered commit messages |
| **dracon-system** | Disk/process protection daemon — monitors space, auto-cleans Rust targets |
| **dracon-warden** | Security hardening daemon — encrypted-at-rest secrets via git filters |

## Features

- **dracon-sync**: Auto-commits changes, manages freeze markers, self-heals stuck repos, creates GitHub remotes automatically
- **dracon-system**: Proactive disk monitoring (70/80/90/95% thresholds), protects active builds
- **dracon-warden**: Git filter encryption for secrets, scrub-markers recovery tool

## Quick Start

```bash
# Install
./install.sh

# Restart services
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
systemctl --user restart dracon-warden.service
```

See [AGENTS.md](AGENTS.md) for full documentation.
