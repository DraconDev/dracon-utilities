{
  "version": 3,
  "id": "mq0pvqhu-uqok7l",
  "objective": "Clean up deprecated/relic commands across all three dracon utilities and add broken symlink detection to dracon-system.\n\nThis goal thoroughly audits and modernizes the CLI surface area:\n\n1. **Remove deprecated `daemon` command from dracon-warden** — it has been deprecated since hooks became the primary security layer. Currently the installed systemd service still runs `dracon-warden daemon` which immediately exits with a deprecation warning, causing an infinite restart loop and making the service functionally dead. Fix the service file to use a no-op or remove the service entry from install.sh entirely.\n\n2. **Audit all utilities for relic commands/flags** — review every subcommand and flag across dracon-sync, dracon-system, and dracon-warden to identify anything that:\n   - Is no longer needed (e.g. `force` flag on `sync-now` kept \"for CLI compatibility\" with removed mass-deletion guard)\n   - Is broken/non-functional (e.g. `mass_deletion_guard_blocked_total` metric that is always 0)\n   - Is misleading or vestigial\n   - Has a newer/better replacement\n\n3. **Add broken symlink detection to dracon-system** — implement a `dracon-system symlinks` (or `links doctor --broken`) command that scans key locations (`~/Dev`, `~/.dracon`, `~/.local/bin`, `~/.config`) for broken symlinks and reports them. Should be a report-only command (no auto-fix) that the AI can use to surface when things break.\n\n4. **Clean up obsolete metric/field references** — the `dracon_sync_mass_deletion_guard_blocked_total` metric should be removed entirely rather than kept as an \"obsolete, always 0\" stub. AGENTS.md and CHANGELOG.md references to removed features should be reviewed for accuracy.\n\n**Scope:** All three utilities (dracon-warden, dracon-system, dracon-sync) and the install.sh + service files. Documentation in AGENTS.md/CHANGELOG.md updated to reflect removals.",
  "status": "active",
  "autoContinue": true,
  "usage": {
    "tokensUsed": 3710668,
    "activeSeconds": 356
  },
  "sisyphus": false,
  "createdAt": "2026-06-05T09:23:41.922Z",
  "updatedAt": "2026-06-05T09:30:02.985Z",
  "activePath": ".pi/goals/active_goal_2026060510234192_mq0pvqhu-uqok7l.md",
  "taskList": {
    "tasks": [
      {
        "id": "audit-clis",
        "title": "Audit all CLI subcommands and flags for relic/deprecated status",
        "status": "complete",
        "completedAt": "2026-06-05T09:26:30.289Z",
        "evidence": "Wrote comprehensive audit to .dracon/audit-cli.md covering all 3 utilities. Found: 1 deprecated subcommand (warden daemon), 1 dead service file, 1 hidden no-op flag (sync-now --force), 1 obsolete metr",
        "verificationContract": "For each utility (dracon-warden, dracon-system, dracon-sync): list every subcommand + flag, mark as KEEP/REMOVE/REPLACE with justification. Findings written to .dracon/audit-cli.md."
      }
    ],
    "blockCompletion": false,
    "proposedAt": "2026-06-05T09:23:41.924Z"
  }
}

# Goal Prompt

Clean up deprecated/relic commands across all three dracon utilities and add broken symlink detection to dracon-system.

This goal thoroughly audits and modernizes the CLI surface area:

1. **Remove deprecated `daemon` command from dracon-warden** — it has been deprecated since hooks became the primary security layer. Currently the installed systemd service still runs `dracon-warden daemon` which immediately exits with a deprecation warning, causing an infinite restart loop and making the service functionally dead. Fix the service file to use a no-op or remove the service entry from install.sh entirely.

2. **Audit all utilities for relic commands/flags** — review every subcommand and flag across dracon-sync, dracon-system, and dracon-warden to identify anything that:
   - Is no longer needed (e.g. `force` flag on `sync-now` kept "for CLI compatibility" with removed mass-deletion guard)
   - Is broken/non-functional (e.g. `mass_deletion_guard_blocked_total` metric that is always 0)
   - Is misleading or vestigial
   - Has a newer/better replacement

3. **Add broken symlink detection to dracon-system** — implement a `dracon-system symlinks` (or `links doctor --broken`) command that scans key locations (`~/Dev`, `~/.dracon`, `~/.local/bin`, `~/.config`) for broken symlinks and reports them. Should be a report-only command (no auto-fix) that the AI can use to surface when things break.

4. **Clean up obsolete metric/field references** — the `dracon_sync_mass_deletion_guard_blocked_total` metric should be removed entirely rather than kept as an "obsolete, always 0" stub. AGENTS.md and CHANGELOG.md references to removed features should be reviewed for accuracy.

**Scope:** All three utilities (dracon-warden, dracon-system, dracon-sync) and the install.sh + service files. Documentation in AGENTS.md/CHANGELOG.md updated to reflect removals.

## Progress

- Status: running
- Auto-continue: on
- Sisyphus mode: no
- Time spent: 5m56s
- Tokens used: 3.7M (3,710,668) tokens
## Tasks

<!-- blockCompletion: false -->
- [x] audit-clis: Audit all CLI subcommands and flags for relic/deprecated status — evidence: Wrote comprehensive audit to .dracon/audit-cli.md covering all 3 utilities. Found: 1 deprecated subcommand (warden daemon), 1 dead service file, 1 hidden no-op flag (sync-now --force), 1 obsolete metr

