# Operations

Systemd services, incident response, and troubleshooting for dracon-utilities.

## Systemd Services

### Service Files

`install.sh` ships **eight** unit files (two long-running services plus three
watchdog service/timer pairs) and three notify scripts. All three timers are
enabled with `--now` at install time.

| Unit | Runs | Cadence | Purpose |
|------|------|---------|---------|
| `dracon-sync.service` | `dracon-sync daemon` | continuous | Git sync automation |
| `dracon-system-guard.service` | `dracon-system guard daemon` | continuous | Disk/process protection |
| `dracon-sync-watchdog.{service,timer}` | `~/.dracon/sync-notify/dracon-sync-watchdog.sh` | every 2 min | Restarts `dracon-sync.service` if it is ever inactive |
| `dracon-freeze-watchdog.{service,timer}` | `~/.dracon/sync-notify/dracon-freeze-watchdog.sh` | every 2 min | Warns on a forgotten `dracon-sync pause`, auto-clears it |
| `dracon-system-guard-watchdog.{service,timer}` | `~/.dracon/system-notify/dracon-system-guard-watchdog.sh` | every 2 min | Restarts the guard if it is ever inactive |

The notify scripts are installed to `~/.dracon/sync-notify/` and
`~/.dracon/system-notify/`; the timer units call them from there, so a missing
script shows up as a failed oneshot in the journal rather than silently
passing. `doctor.sh` checks all eight units, all three timers, and all three
scripts — run `./doctor.sh` when you suspect an install gap.

> `dracon-warden` has no systemd service. Git hooks (installed via
> `setup-hooks --global`) are the primary enforcement layer.

### Common Commands

```bash
# Status
systemctl --user status dracon-sync.service
systemctl --user status dracon-system-guard.service
systemctl --user list-timers 'dracon-*'      # watchdog backstops

# Logs
journalctl --user -u dracon-sync -f
journalctl --user -u dracon-system-guard -f
journalctl --user -u dracon-sync-watchdog -n 20
journalctl --user -u dracon-freeze-watchdog -n 20

# Restart after config changes
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
```

### Resource Limits

Values below are what `install.sh` ships from the unit files in
`dracon-sync/` and `dracon-system/`. (An earlier revision of this table listed
`CPUQuota=15%` and `Restart=on-failure` for sync — both were changed in
`dracon-sync` `4f22bd7` (2026-09-17) after a measured classifier-starvation
wedge. The shipped units and `flake.nix` are authoritative.)

**dracon-sync.service:**
| Setting | Value | Purpose |
|---------|-------|---------|
| `Nice` | 10 | Lower CPU priority |
| `CPUQuota` | 100% | No throttling — the 15% cap starved classification |
| `MemoryHigh` | 768M | Soft memory limit |
| `MemoryMax` | 2G | Max 2GB RAM |
| `TasksMax` | 96 | Max 96 threads |
| `Restart` | always | Restarts even on clean exit (see below) |

**dracon-system-guard.service:**
| Setting | Value | Purpose |
|---------|-------|---------|
| `MemoryMax` | 250M | Max 250MB RAM |
| `CPUQuota` | 20% | Max 20% CPU usage |
| `TasksMax` | 64 | Max 64 threads |
| `Restart` | on-failure | See the disabled-policy note below |

### Security Hardening (both services)

- `NoNewPrivileges=true`
- `ProtectSystem=strict`
- `ProtectHome=read-only` (with explicit `ReadWritePaths`)
- `dracon-sync.service`: `PrivateTmp=true`
- `dracon-system-guard.service`: `PrivateTmp=false` intentionally, with
  `/tmp` added to `ReadWritePaths`; its `clean_tmp` policy targets the host
  `/tmp`, so a private namespace would make that cleanup ineffective.
- `clean_tmp` validates `tmp_search_paths` as a canonical descendant of the
  explicit `/tmp` root before scanning or deleting; arbitrary home roots such
  as `~` are refused.

### Pre-start Cleanup

The sync service kills stale `dracon-git pulse` processes before starting to prevent lockups.

### Restart Behavior

The two services deliberately differ:

| Service | `Restart` | `RestartSec` | `RestartPreventExitStatus` |
|---------|-----------|--------------|---------------------------|
| `dracon-sync.service` | `always` | 5 | `2 78` |
| `dracon-system-guard.service` | `on-failure` | 10 | `2 78` |

- `Restart=on-failure` (guard) restarts crashes, abnormal signal termination,
  and other nonzero failures, but **not** a clean exit from a valid disabled
  policy.
- `Restart=always` (sync) restarts the daemon even after a clean exit, so a
  daemon that exits 0 in a bad state still comes back — the watchdog's job is
  to cover the case where systemd itself is not managing it.
- `RestartPreventExitStatus=2 78` on both — no restart on CLI usage errors (2)
  or the guard's startup policy status 78 (`EX_CONFIG`).

The guard emits `guard disabled in policy` and exits 0 when
`[guard].enabled = false`; systemd therefore leaves that intentionally
disabled service stopped. A missing, malformed, or unreadable explicit
startup policy is reported as status 78, avoiding a retry storm while still
allowing crashes, abnormal
signal termination, and runtime failures to restart. Deliberate SIGTERM/SIGINT
shutdown is handled cleanly. Fix the policy, then run
`systemctl --user reset-failed dracon-system-guard.service` (if needed) and
restart the service.

### Watchdogs and the quiesce policy

Three timer-driven oneshots (every 2 minutes) are the fleet's backstop against
silent downtime:

- **`dracon-sync-watchdog`** — restarts `dracon-sync.service` whenever
  `systemctl --user is-active` says it is not running.
- **`dracon-freeze-watchdog`** — watches the `dracon-sync pause` freeze marker:
  warns (journal + `notify-send`) at **10 minutes**, auto-clears it at
  **30 minutes**, while the daemon hard-clears any survivor at **1 hour**.
- **`dracon-system-guard-watchdog`** — restarts the guard if it is ever
  inactive, including after a manual `stop` or a disabled unit.

**Do not use `systemctl --user stop` to quiesce sync for remediation.** A
manual stop has no backstop (`Restart=always` covers crashes, not a forgotten
restart), and the watchdog would simply restart the daemon mid-surgery. Use
one of the sanctioned paths instead:

```bash
dracon-sync maintenance -- <cmd...>   # pause → run → ALWAYS resume
dracon-sync pause                     # interactive multi-step work
```

A forgotten `pause` self-heals (10m warn, 30m auto-clear, 1h hard TTL). For
genuine multi-minute downtime, touch the hold marker first and remove it
afterwards:

```bash
touch ~/.dracon/dracon-sync.maintenance-hold    # suppresses the sync watchdog
```

Guard-side equivalent for its own maintenance:

```bash
touch ~/.dracon/dracon-system.maintenance-hold  # suppresses the guard watchdog
```

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
4. For an intentional destructive operation, `git add` the exact paths you
   mean (e.g. `git add -- old-assets/`), commit, and let the daemon push it.
   Never use a bare `git add -A` or `git add .`: enumerate the paths so a
   stray secret or credential file cannot ride along — see AGENTS.md
   "Forbidden actions".

### Removing Large Numbers of Files

Enumerate the paths explicitly rather than staging everything:

```bash
git ls-files 'old-assets/*' > /tmp/paths.txt
git add --pathspec-from-file=/tmp/paths.txt
git commit -m 'retire old-assets'
```

A bare `git add -A` sweeps anything currently in the worktree, including files
added since the incident started.

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

A repo shows up as `STUCK_PUSH` only when the daemon has actually recorded
a recent push failure for it (within the last 10 minutes, checked against
the incident ledger). AHEAD repos with no recorded failure show as
`PENDING` instead — they're "has unpushed commits" without an error, and
the daemon is working through the queue. See
`docs/design/sync-push-classification.md` for the full classification rules.

Permanent push rejections (GitLab/Codeberg protected branch, pre-receive
hook declined, etc.) are detected up front and are NOT retried. One
incident is logged per cycle and the repo is flagged `STUCK_PUSH` until
the server-side policy is resolved. A dirty repo that is merely ahead
without a recent push failure remains `WARN`; its hint says the daemon will
push after changes settle rather than suggesting concern repair.

### Large-Repo Staging Cooldown

`git add -A` on a repo with thousands of dirty files can take 60–90s,
longer than the per-operation idle timeout. To prevent the daemon from
logging a "staging timeout" incident on every cycle, the policy supports:

```toml
# Idle timeout (seconds) for `git add` and other staging operations
# on a single repo. Default: 60. Minimum accepted: 10.
stage_op_timeout_secs = 60

# When `git add` exceeds stage_op_timeout_secs, the daemon pauses
# further attempts on that repo for this many seconds. Default: 3600
# (1 hour). The point is to stop incident-ledger spam.
stage_cooldown_secs = 3600
```

After the cooldown elapses, the daemon tries `git add` again; if it
times out once more, the cooldown resets. This is a per-repo gate — one
large repo on cooldown does not affect any other repo's sync. The daemon
also enforces the cooldown in the main loop, so a repo on cooldown is
skipped until the timer expires.

`repair-warns` does not wrap the whole sync triage pass in the legacy
`repo_sync_timeout_secs` value. Large repos can still spend longer than
that on otherwise healthy staging, commit, push, or mirror operations; the
individual git operations keep their own progress-aware or idle timeouts
instead.

### Repair Concerns vs `repos` Table

`dracon-sync repair concerns` and `dracon-sync repos` use the same
concern-classification logic: a repo is a CONCERN when it has no
origin/upstream, or is `behind > 0`, or is `ahead > 0` AND has a recent
push failure recorded in the incident ledger. The `dracon-sync repos`
table is the user-visible view; the `repair concerns` command is the
operator's repair action against that same set. Their counts must agree
at all times.

### Dual Branches

```bash
dracon-sync repair dual-branch-list
dracon-sync repair dual-branch-repair <repo>
```

### Origin Repair

```bash
dracon-sync repair origins [--apply]
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

### Process Mitigation

The guard never directly kills processes. During CPU, memory, or swap
pressure, its optional reversible mitigations are:

- graduated `renice` to lower the priority of heavy processes;
- `oom_score_adj` biasing during critical pressure, which only influences
  the kernel's last-resort OOM victim choice if the kernel kills anyway; and
- optional `CPUQuota` throttling for a stuck busy-loop, without killing or
  moving the process permanently.

Tracked adjustments are restored when pressure recovers. The guard's
notifications identify the top memory offenders; operators remain in control
of any deliberate process termination.

`guard clean` is **disk-space cleanup**, not process-mitigation rollback. It
cleans reclaimable Rust targets, Trash, Nix generations, caches,
`node_modules`, and Docker resources. Package-cache apply deletion is
intentionally refused because external package managers provide no lock shared
with the guard; this is the only way to guarantee that an operation cannot
start between detection and recursive deletion. Dry-run cache estimates still
detect cargo/rustc, npm, pip, and go operations (including common wrappers) and
skip the corresponding active cache; unavailable process metadata or a failed
process listing fails that inspection closed. A bare invocation selects all six
cleanup targets and previews by default; add `--apply` to execute, or select
a subset with `--rust`, `--trash`, `--nix`, `--caches`, `--node-modules`, and
`--docker`. `--all` selects every target and additionally enables Docker's
all-unused-images mode.

```bash
dracon-system guard clean                 # Preview all cleanup targets
dracon-system guard clean --trash --apply  # Apply one selected cleanup
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
