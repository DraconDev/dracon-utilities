# Project State

## Current Focus
Add test‑only constructors for `RepoKey` and `TeamKey` to simplify unit test setup.

## Completed
- [x] feat(security): provide `RepoKey::from_secret_bytes([u8;32])` under `#[cfg(test)]` for creating repo keys directly from byte arrays in tests.
- [x] feat(security): provide `TeamKey::from_identity_string(String)` under `#[cfg(test)]` for creating team keys from identity strings in tests.
