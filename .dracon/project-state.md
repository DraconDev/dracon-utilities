# Project State

## Current Focus
Refined directory exclusion patterns to improve precision in file synchronization.

## Context
The previous implementation had an edge case where patterns ending with `-` (like `.tmp-`) incorrectly matched names without trailing hyphens (like `.tmpfile`). This could lead to unintended exclusions.

## Completed
- [x] Fixed `.tmp-` pattern to only match names ending with `-` (e.g., `.tmp-` matches `.tmp-foo` but not `.tmpfile`)
- [x] Added explicit comment explaining the `.tmp-` prefix matching behavior
- [x] Added support for glob-style `*` suffix patterns (e.g., `.build*` matches `.build-debug`)

## In Progress
- [ ] No active work in progress

## Blockers
- None identified

## Next Steps
1. Verify the new patterns work as expected in integration tests
2. Document the updated exclusion pattern syntax in user documentation
