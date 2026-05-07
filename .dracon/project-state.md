# Project State

## Current Focus
Added process monitoring and auto-kill features for runaway Git processes in `dracon-system`.

## Context
To prevent CPU resource exhaustion from misbehaving Git operations, the system now monitors processes and can automatically terminate Git commands that sustain high CPU usage for extended periods.

## Completed
- [x] Added persistent JSONL logging for heavy process events
- [x] Implemented auto-kill for Git processes exceeding CPU thresholds
- [x] Configured log rotation and file location settings

## In Progress
- [ ] Testing edge cases for process detection and termination

## Blockers
- None identified; feature is complete pending validation

## Next Steps
1. Verify log rotation behavior with large log files
2. Document the auto-kill configuration options in user guides
