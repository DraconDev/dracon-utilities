# Project State

## Current Focus
Refactor and extend security tests to improve backup handling, idempotence, and edge‑case coverage.

## Completed
- [x] Introduce `init_with_temp_home()` helper to streamline security initialization across tests.
- [x] Remove redundant HOME setup in individual tests, reducing duplication.
- [x] Replace old accept‑team‑invite tests with checks for non‑invite paths and nonexistent files.
- [x] Add tests for backup recursion guard to reject paths inside `.demon/backups` or `arcane/backups`.
- [x] Implement round‑trip backup and restore test ensuring data integrity.
- [x] Add idempotence test for `ensure_current_user_key` confirming single key file creation.
- [x] Add error case test for restoring a file when no backups exist.
- [x] Remove stale test logic and simplify test structure for clarity and maintainability.
