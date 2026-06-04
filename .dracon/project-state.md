# Project State

## Current Focus
TODO sprint — iteration 3: events + links modules extracted from system/main.rs

## Context
Working through todo.md items. System/main.rs modularization in progress.

## Completed This Sprint
- [x] Item 1: unwrap audit — 0 production unwraps
- [x] Item 2: sync.rs tests — 39 tests, good coverage
- [x] Item 5: CI/CD pipeline — `.github/workflows/ci.yml`
- [x] Item 6: missing-doc warnings — 0 in own code
- [x] Item 7: dracon-ai docs — activation path noted
- [x] Item 8: `#![warn(missing_docs)]` lint gates on all 4 crate roots
- [x] Item 12: incident ledger rotation — already implemented
- [x] Events module extraction — `dracon-system/src/events.rs` (260 lines)
- [x] Links module extraction — `dracon-system/src/links.rs` (233 lines)
- [x] All 706 tests passing after both extractions

## In Progress
- Item 4: system/main.rs split — 3,926 → 3,484 lines. Remaining: guard, storage, zram, doctor, safety
- Item 3: git.rs split — planned (complex cross-module dependencies)

## Blockers
- None

## Next Steps
1. Continue system/main.rs split: extract zram, doctor, safety modules (smaller targets)
2. Begin git.rs split: extract multi_remote module

## Operational Notes

### Freeze marker origin (2026-06-04 15:30)

**Incident**: Daemon paused at 15:30:16 (freeze marker created at `~/.dracon/dracon-sync.freeze`),
3 repos accumulated CONCERN status (dracon-platform, folder-auto-banner, dracon-utilities — all
`AHEAD: 1`). Discovered 1h23m later when user reported concern repos.

**Root cause**: External `dracon-sync pause` command was run (likely by a pi session or user
during a delicate operation on cli-file-manager), followed by SIGKILL of the daemon 1 second later.
The marker was never removed via `dracon-sync resume`. The restarted daemon honored the marker
and stayed paused indefinitely.

**Fix applied**: `dracon-sync resume` removed the marker. Daemon resumed syncing within 30s.
All 3 CONCERN repos recovered to OK. 0 CONCERN remaining.

**Prevention**: `dracon-sync pause` should always be paired with `dracon-sync resume`. Consider
adding an auto-expiry to the freeze marker (e.g., 24h TTL) so paused sync doesn't accumulate
stale state indefinitely.
# final test 1780477840
