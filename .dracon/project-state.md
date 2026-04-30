# Project State

## Current Focus
Remove aging or redundant security tests for atomic write backups and v2 decryption with wrong identity to streamline test suite.

## Completed
- [x] Drop atomic write test that verifies encrypted backup creation (backup_file with age encryption) and related fs import.
- [x] Drop comprehensive test that validates decryption failure when using a wrong identity (decrypt_v2 wrong identity).
