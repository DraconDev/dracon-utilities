# Project State

## Current Focus
Idling — all planned fixes deployed

## Context
Fixed commit spam loop (7 bugs across dracon-sync and dracon-system) and disk cleanup failures (5 bugs). All binaries rebuilt, installed, and services restarted. Dedup guard and improved stale check now active.

## Completed
- [x] Fix stale focus check in report.rs (symmetric comparison with prefix stripping)
- [x] Add "fix" to action_words preventing fix(fix ...) scope
- [x] Add commit dedup guard (blocks after 2 identical subjects)
- [x] Fix nix-store collect-garbage → nix-collect-garbage
- [x] Fix auto_cleanup_apply defaults to false → set true in config
- [x] Fix notify_command to absolute path with auto-detection
- [x] Fix sync_freeze_marker to writable location
- [x] Fix pkill to absolute path in sync service
- [x] Add resolve_bin() for NixOS binary path resolution
- [x] Add better error messages for ps command failures

## In Progress
- None

## Blockers
- None

## Next Steps
1. Monitor dedup guard in production
2. Consider squashing 354 historical spam commits
