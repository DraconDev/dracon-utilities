# Project State

## Current Focus
Added `force_push_when_behind` flag to remote configurations to enable automatic force-push when remote is behind local.

## Context
This change supports scenarios where local changes need to overwrite remote history, such as during emergency recovery or when the remote repository is known to be stale. The flag provides explicit control over this behavior.

## Completed
- [x] Added `force_push_when_behind` field to remote configuration struct
- [x] Set default value to `false` for backward compatibility
- [x] Maintained existing test case for URL resolution

## In Progress
- [ ] No active work in progress

## Blockers
- None identified

## Next Steps
1. Add documentation for the new flag
2. Create integration tests for force-push scenarios
