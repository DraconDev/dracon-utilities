# Project State

## Current Focus
Refactor team key handling and simplify the `decrypt_repo_key_with_team_key` method

## Completed
- [x] Remove Unix-specific permission handling from `DemonSecurity`
- [x] Refactor team identity derivation to use `expose_secret()` and `FromStr` for clearer error messaging
- [x] Wrap encrypted input in `Cursor` when initializing `age::Decryptor`
