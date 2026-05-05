# Project State

## Current Focus
Update dracon-sync configuration paths and add PATH warning for dracon utilities

## Context
The changes standardize configuration paths for dracon-sync and add a warning when ~/.local/bin isn't in PATH, ensuring users can access dracon utilities.

## Completed
- [x] Changed dracon-sync policy path from absolute to user-relative (`~/.dracon/utilities/sync/dracon-sync.toml`)
- [x] Added PATH warning for ~/.local/bin in install.sh
- [x] Removed obsolete note.md

## In Progress
- [ ] None (changes are complete)

## Blockers
- None (changes are complete)

## Next Steps
1. Verify PATH warning works across different shells
2. Update documentation to reflect new paths
