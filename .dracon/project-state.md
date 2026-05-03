# Project State

## Current Focus
Added `force_push_when_behind` flag to remote configurations to enable automatic force-push when remote is behind local.

## Context
This change supports scenarios where users need to overwrite remote branches when they are behind local changes, which is useful for development workflows where force-pushing is acceptable.

## Completed
- [x] Added `force_push_when_behind` boolean flag to remote configuration struct
- [x] Initialized flag to `false` by default in test configurations

## In Progress
- [x] Implementation of actual force-push logic (not yet implemented in this commit)

## Blockers
- Need to implement the actual force-push behavior when the flag is true

## Next Steps
1. Implement force-push logic when `force_push_when_behind` is true
2. Add unit tests for the new behavior
