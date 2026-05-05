# Project State

## Current Focus
Added a Git command helper for test isolation to prevent PATH resolution races in parallel test runs.

## Context
To improve test reliability, we need consistent Git command execution across parallel test runs. The previous approach relied on PATH resolution which could cause races when multiple tests ran simultaneously.

## Completed
- [x] Added `test_git_cmd()` helper function that respects `DRACON_SYNC_GIT_BIN` environment variable
- [x] Documented the helper function with usage examples
- [x] Ensured the helper maintains the same interface as `std::process::Command`

## In Progress
- [ ] No active work in progress

## Blockers
- None identified

## Next Steps
1. Update existing tests to use the new helper function
2. Verify no test failures occur due to the change
3. Consider adding more test isolation utilities if needed
