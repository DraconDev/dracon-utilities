# Project State

## Current Focus
Added support for automatic force-push when remote is behind local

## Context
This change addresses scenarios where a remote repository has diverged from the local repository, preventing normal push operations. The new functionality enables automatic force-pushes when the remote is determined to be purely behind the local branch.

## Completed
- [x] Added `force_when_behind` parameter to `push_to_named_remote`
- [x] Implemented divergence detection logic
- [x] Added force-push with lease when remote is purely behind
- [x] Maintained retry logic for other failure cases

## In Progress
- [ ] None (feature is complete)

## Blockers
- None (feature is fully implemented)

## Next Steps
1. Verify behavior with integration tests
2. Document the new `force_when_behind` configuration option
3. Consider adding metrics for force-push events
