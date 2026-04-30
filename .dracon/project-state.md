# Project State

## Current Focus
Remove keygen‑related tests (overwrite protection and hostname validation) from `dracon-warden/src/main.rs`

## Completed
- [x] Removed test `run_keygen_refuses_to_overwrite_existing_secret_key`
- [x] Removed test `run_keygen_rejects_empty_hostname`
- [x] Removed test `run_keygen_refuses_to_overwrite_existing_pubkey`
- [x] Cleaned up associated environment variable restores and assertions
