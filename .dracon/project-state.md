# Project State

## Current Focus
Added automatic git process management to prevent runaway CPU usage

## Context
The system previously had CPU management features but lacked specific handling for git processes. This change adds configuration options to automatically detect and terminate git processes that sustain high CPU usage for extended periods.

## Completed
- [x] Added `auto_kill_git` configuration option to enable/disable git process monitoring
- [x] Added `git_kill_threshold_secs` configuration to set the CPU usage duration threshold
- [x] Implemented default values (60 seconds) for the new configuration options

## In Progress
- [ ] Implementation of the actual process monitoring and termination logic

## Blockers
- Need to implement the process monitoring and termination logic
- Requires testing with various git operations to ensure proper detection

## Next Steps
1. Implement the process monitoring and termination logic
2. Add comprehensive tests for the new git process management features
