# Project State

## Current Focus
Add test utility to isolate HOME environment per test via temporary directory and mutex-guarded setup/teardown.

## Completed
- [x] Introduce HomeGuard that creates a temp HOME, locks concurrent env changes, and cleans HOME on drop.
