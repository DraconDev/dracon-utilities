# Project State

## Current Focus
Hardening failure reporting in git restore/sync and tightening leak-prevention policy for sensitive history/credential files.

## Completed
- [x] Make restore_paths fail explicitly when both restore and reset fallback fail, replacing a silent warning with a descriptive error.
- [x] Promote reset-HEAD failures after filter-only commits to fatal errors in sync_repo to prevent proceeding in an ambiguous state.
- [x] Refine leak-prevention tests to treat sensitive-path text files as readable unless they are high-risk histories or credential files (credentials, .env*, *history, vault.yml).
