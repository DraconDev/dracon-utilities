# Operations

Systemd services, incident response, and troubleshooting for dracon-utilities.

## Systemd Services

### Service Files

| Service | Binary | Purpose |
|---------|--------|---------|
| `dracon-sync.service` | `dracon-sync daemon` | Git sync automation |
| `dracon-system-guard.service` | `dracon-system guard daemon` | Disk/process protection |

> `dracon-warden` has no systemd service. Git hooks (installed via `setup-hooks --global`) are the primary enforcement layer.

### Common Commands

```bash
# Status
systemctl --user status dracon-sync.service
systemctl --user status dracon-system-guard.service

# Logs
journalctl --user -u dracon-sync -f
journalctl --user -u dracon-system-guard -f

# Restart after config changes
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
```

### Resource Limits

**dracon-sync.service:**
| Setting | Value | Purpose |
|---------|-------|---------|
| `Nice` | 10 | Lower CPU priority |
| `CPUQuota` | 15% | Max 15% CPU usage |
| `MemoryMax` | 2G | Max 2GB RAM |
| `MemoryHigh` | 768M | Soft memory limit |
| `TasksMax` | 96 | Max 96 threads |

**dracon-system-guard.service:**
| Setting | Value | Purpose |
|---------|-------|---------|
| `MemoryMax` | 250M | Max 250MB RAM |
| `CPUQuota` | 20% | Max 20% CPU usage |
| `TasksMax` | 64 | Max 64 threads |

### Security Hardening (both services)

- `NoNewPrivileges=true`
- `ProtectSystem=strict`
- `ProtectHome=read-only` (with explicit `ReadWritePaths`)
- `PrivateTmp=true`

### Pre-start Cleanup

The sync service kills stale `dracon-git pulse` processes before starting to prevent lockups.

### Restart Behavior

- `Restart=always` — restarts on any exit (clean or crash)
- `RestartSec=5` (sync) / `10` (guard)
- `RestartPreventExitStatus=2 78` — don't restart on config/argument errors

## Incident Response

### Viewing Incidents

```bash
cat ~/.local/state/dracon/dracon-sync-incidents.jsonl | tail -20
```

Each line is a JSON object:
```json
{"ts_unix":1714896000,"scope":"safety","repo":"/path/to/repo","reason":"description","action":"action_taken","backup_branch":null,"result":"result","details":"additional details"}
```

Common `scope` values: `safety` (safety guard triggers), `repair` (auto-repair), `sync` (sync operations), `mirror` (mirror push failures).

### After an Incident

1. Read the incident ledger to understand what happened
2. Check the repo status: `git status` and `git log --oneline -5`
3. Take appropriate action based on the incident type
4. For intentional destructive operations: `git add -A && git commit -m 'delete files'` directly

### Removing Large Numbers of Files

Use `git add -A && git commit -m 'delete files'` directly — no daemon involvement needed.

## Troubleshooting

### Daemon Health

```bash
dracon-sync health [--json]
dracon-sync metrics
```

### Repo Report

```bash
dracon-sync repos
```

Shows real dirty file counts, OK/WARN/CONCERN status, mirror status.

### Stuck Pushes

```bash
dracon-sync repair stuck-list
dracon-sync repair stuck-unstuck <repo>
```

### Dual Branches

```bash
dracon-sync repair dual-branch-list
dracon-sync repair dual-branch-repair <repo>
```

### Origin Repair

```bash
dracon-sync repair-origins [--apply]
```

### Freezing Sync

```bash
dracon-sync pause    # Creates freeze marker
dracon-sync resume   # Removes freeze marker
```

### Guard Pruning

```bash
dracon-system guard prune
dracon-system guard clean
```

### Process Renice

The guard never kills processes — only renices. To undo:

```bash
dracon-system guard clean  # Releases all renices
```

## Operational State

Mutable runtime files live outside the `.dracon` git tree:

```
~/.local/state/dracon/
├── dracon-sync-incidents.jsonl        # Append-only incident ledger
├── dracon-sync-stuck-push-repos.json  # Stuck push tracking
├── dracon-system-guard.log            # Guard log (auto-rotated)
└── visibility-sync/                   # Per-repo metadata sync timestamps
```
