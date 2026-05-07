# Project State

## Current Focus
Added automatic git process management to prevent runaway CPU usage

## Context
The system previously only monitored and reniced processes, but could not terminate runaway git operations that consumed excessive CPU resources. This change adds the ability to automatically kill git processes that exceed a configurable threshold.

## Completed
- [x] Added `auto_kill_git` configuration option to GuardPolicy
- [x] Implemented `kill_process` function with graceful TERM followed by forceful KILL
- [x] Added `is_git_process` helper to identify problematic git operations
- [x] Integrated git process killing into the guard monitoring loop

## In Progress
- [x] Implementation of git process monitoring and termination

## Blockers
- None identified

## Next Steps
1. Add configuration documentation for the new git killing parameters
2. Add integration tests for the git process killing functionality
3. Consider adding metrics for git process terminations
