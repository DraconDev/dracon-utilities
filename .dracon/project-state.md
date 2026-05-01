# Project State

## Current Focus
docs(AGENTS.md): updated operational state documentation to clarify file locations and sync behavior

## Context
The change clarifies that runtime operational files (like the incident ledger) are stored in `~/.local/state/dracon/` rather than inside the project directory, preventing the sync daemon from auto-committing its own operational data.

## Completed
- [x] removed `fleet.db` from AGENTS.md since it's no longer used
- [x] clarified operational state file locations in documentation

## In Progress
- [ ] none

## Blockers
- none

## Next Steps
1. Verify documentation aligns with current implementation
2. Update related documentation if operational state paths change further
