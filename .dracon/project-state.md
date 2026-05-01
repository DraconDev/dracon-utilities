# Project State

## Current Focus
Configure operational state files to reside outside the `.dracon` repository root to avoid versioning runtime data

## Completed
- [x] datadir: Relocate dracon-sync-incidents.jsonl, dracon-sync-stuck-push-repos.json, and fleet.db to `~/.local/state/dracon/` to prevent self-referential commits
- [x] deps: Update dracon-system dependency resolver with latest upstream platform requirements
