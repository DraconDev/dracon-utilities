# Project State

## Current Focus
ONE LINE: Serialize modifications to the `HOME` environment variable in tests to prevent race conditions.

## Completed
- [x] feat(tests): introduced a global `HOME_MUTEX` and acquire its lock in tests that temporarily modify `HOME`, ensuring safe concurrent execution.
