# Project State

## Current Focus
Rename repo-key specific encryption tests to v2 generic tests and update test logic to use new API functions

## Completed
- [x] Rename `test_encrypt_with_repo_key_roundtrip` to `test_encrypt_v2_for_all_roundtrip` and adjust test logic
- [x] Rename `test_encrypt_decrypt_with_repo_key_empty` to `test_encrypt_v2_for_all_empty_data` and adjust test logic
- [x] Replace calls to `encrypt_with_repo_key` with `encrypt_v2_for_all`
- [x] Replace calls to `decrypt_with_repo_key` with `decrypt_v2`
- [x] Update plaintext and assertion messages to reflect v2 behavior
