# Project State

## Current Focus
Refactor security constructors: add file‑based RepoKey loader and consolidate test helpers, moving Vec constructions into impl blocks.

## Completed
- [x] Added `RepoKey::from_file` to load a 32‑byte repository key from a file and expose it via `get_key()`
- [x] Moved `TeamKey::from_identity_string` into `impl TeamKey` with `#[cfg(test)]` and removed an unnecessary closing brace
- [x] Removed duplicated test constructors from `RepoKey` and placed them inside `impl RepoKey` under `#[cfg(test)]` ( `from_vec`, `from_secret_bytes` )
- [x] Cleaned up the test module structure for security types
