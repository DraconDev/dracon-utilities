# Project State

## Current Focus
Refactor security constructors and remove unused test helper

## Completed
- [x] Convert `RepoKey::from_secret_bytes` to use `.to_vec()` for Vec<u8> conversion
- [x] Remove redundant `TeamKey::from_identity_string` test constructor
- [x] Adjust `RepoKey::from_vec` to explicitly validate 32‑byte length and return `Option<Self>`
