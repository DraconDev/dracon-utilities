# Daemon quiesce policy — never `systemctl stop` the sync daemon

**Date**: 2026-08-07
**Applies to**: dracon-sync daemon (`dracon-sync.service`), all operators
and agent loops working in daemon-owned repos.
**Status**: Policy + tooling (v0.113.44) + watchdog (operational).

## The incident that motivated this

2026-08-06 21:57 — a `pi-fix-identity` agent script rewound
`dracon-platform`'s local `main` from `fe8b2f6db` to `ac48aa74d` to
clean up 5 foreign-identity commits. The divergence + stuck merge state
produced the "Mirror Degraded" alert storm. The remediation procedure
(pause daemon → `git merge --abort` → `git rebase origin/main` → push →
restart) began with:

```
systemctl --user stop dracon-sync.service
```

**The problem**: a manual `systemctl stop` has NO backstop. systemd's
`Restart=always` only covers crashes — a manually stopped unit stays
stopped until someone starts it. The procedure worked this time (the
operator restarted the daemon), but any future agent copying the
procedure could stop the daemon and leave it stopped. Then the fleet
stops syncing, silently.

## The fix: three layers

### Layer 1 — Pause-first discipline (policy)

**`systemctl --user stop dracon-sync.service` is banned for
remediation.** The sanctioned quiesce paths are:

| Command | Effect |
|---|---|
| `dracon-sync pause` | Writes `~/.dracon/dracon-sync.freeze`. Daemon keeps RUNNING, health stays green, skips all sync cycles. |
| `dracon-sync resume` | Removes the freeze marker. |
| `dracon-sync maintenance -- <cmd...>` | **v0.113.44+**: pauses, runs `<cmd>`, ALWAYS resumes (even on failure), exits with the command's exit code. A pause that predates the invocation is left untouched. |

Why pause beats stop:

- The service never goes down — health monitoring and `dracon-sync
  repos`/`status` stay green; there is nothing to "forget to restart".
- Self-healing: freeze markers older than the 24h TTL are auto-cleared
  (`FREEZE_MARKER_TTL_SECS`, `policy.rs:1858`), so even a forgotten
  pause lifts itself.
- Freeze takes effect within one pulse interval (default 1s).
- The maintenance command makes "forgot to resume" impossible by
  construction.

### Layer 2 — `dracon-sync maintenance` (v0.113.44)

```bash
dracon-sync maintenance -- git merge --abort
dracon-sync maintenance -- git rebase origin/main
```

Behavior:

1. If not already paused: writes the freeze marker.
2. Runs the command with inherited stdio.
3. ALWAYS removes the marker afterwards (success or failure), then
   exits with the command's exit code (127 spawn failure, 128
   signal-kill).
4. If sync was ALREADY paused (marker or `DRACON_SYNC_FREEZE` env):
   runs the command and leaves the freeze state untouched — a pause
   that predates the invocation is not ours to lift.

Guaranteed-resume semantics are unit-tested (4 tests in `main.rs`).

### Layer 3 — systemd watchdog (operational)

`dracon-sync-watchdog.{service,timer}` — user-level systemd units at
`~/.config/systemd/user/` (same place as the status/notify timers):

- Fires every 2 minutes (+30s jitter).
- If `dracon-sync.service` is inactive AND no hold marker exists →
  `systemctl --user start dracon-sync.service`, logged to the journal.
- Worst case: a stopped daemon is back within ~2.5 minutes, no human
  or agent action required.

Escape hatch for genuine downtime (release installs, hardware work):

```bash
touch ~/.dracon/dracon-sync.maintenance-hold   # watchdog stops restarting
rm    ~/.dracon/dracon-sync.maintenance-hold   # watchdog resumes
```

`release.sh` does NOT stop the daemon (the operator uses `systemctl
--user restart`, which the watchdog never observes as "down"), so no
hold is needed around releases — but the marker exists for anyone
doing a manual multi-minute stop.

## Live verification (2026-08-07)

1. `systemctl --user stop dracon-sync.service` → `inactive`
2. `~/.dracon/sync-notify/dracon-sync-watchdog.sh` →
   `dracon-sync.service is inactive — restarting`
3. `systemctl --user is-active dracon-sync.service` → `active` ✓

## Runbook — repo surgery (the replacement for the old procedure)

```bash
# OLD (banned): systemctl --user stop dracon-sync.service
# ...surgery...
# OLD (banned): systemctl --user start dracon-sync.service

# NEW (v0.113.44+):
dracon-sync maintenance -- git merge --abort
dracon-sync maintenance -- git rebase origin/main
```

Or for interactive multi-step work:

```bash
dracon-sync pause
# ...surgery, at your own pace ...
dracon-sync resume        # forgotten resume self-heals via 24h TTL
```

## Where the policy lives

- This design doc.
- `AGENTS.md` (meta repo): "Daemon quiesce policy" section.
- `dracon-sync.example.toml`: freeze/pause documentation (existing).
