# Project State

## Current Focus
Added `force_push_when_behind` flag to remote configurations to enable automatic force-push when remote is behind local

## Context
This change supports scenarios where users need to overwrite remote branches when they are behind the local branch, which is common in CI/CD pipelines or when working with temporary branches.

## Completed
- [x] Added `force_push_when_behind` flag to remote configuration struct
- [x] Initialized flag to `false` in test cases

## In Progress
- [ ] Implementation of actual force-push logic when flag is enabled

## Blockers
- Need to implement the actual force-push behavior in the sync operation

## Next Steps
1. Implement force-push logic when `force_push_when_behind` is true
2. Add integration tests for the force-push behavior
