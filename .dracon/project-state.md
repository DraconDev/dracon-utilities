# Project State

## Current Focus
Implement platform‑specific secure file creation with restrictive permissions for encrypted keys and backups.

## Completed
- [x] fix(security): enforce 0o600 permission on newly written encrypted key files on Unix and fallback to default write on non‑Unix platforms.
- [x] fix(security): enforce 0o400 permission on backup files during encryption, using Unix `OpenOptionsExt` for Unix and explicit permission setting for other platforms.
