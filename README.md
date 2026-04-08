# Dracon Utilities

CLI binaries for dracon system services.

## Binaries

- **dracon-sync** - Git sync automation daemon
- **dracon-system** - Disk/process protection daemon  
- **dracon-warden** - Security hardening daemon

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
