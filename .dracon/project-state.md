# Project State

## Current Focus
Add tests ensuring `sync_repo` early‑returns when a rebase, merge, or cherry‑pick is in progress.

## Completed
- [x] Add `test_sync_repo_skips_rebase_in_progress` verifying early return during rebase
- [x] Add `test_sync_repo_skips_merge_in_progress` verifying early return during merge
- [x] Add `test_sync_repo_skips_cherry_pick_in_progress` verifying early return during cherry‑pick
