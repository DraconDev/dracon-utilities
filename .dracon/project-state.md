# Project State

## Current Focus
Refactor daemon and report modules to use XDG-compliant state directory paths

## Completed
- [x] Update `stuck_repos_path()` in daemon.rs to use `~/.local/state/dracon/dracon-sync-stuck-push-repos.json` instead of `~/.dracon/dracon-sync-stuck-push-repos.json`
- [x] Update `incident_ledger_path()` in report.rs to use `~/.local/state/dracon/dracon-sync-incidents.jsonl` instead of `~/.dracon/dracon-sync-incidents.jsonl`
