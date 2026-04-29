# Project State

## Current Focus
Add tests ensuring `check_safe_to_delete` refuses deletion of symlinks targeting root and home directories.

## Completed
- [x] Add test `check_safe_to_delete_rejects_symlink_to_root` that creates a symlink (or copy on non‑Unix) from `/` to a temporary file and asserts the function returns an error containing "refusing to delete".
- [x] Add test `check_safe_to_delete_rejects_symlink_to_home` that creates a symlink (or directory) from `/home` to a temporary file and asserts the function returns an error.
- [x] Ensure temporary directories and files are cleaned up in each test for both Unix and non‑Unix platforms.
