# Project State

## Current Focus
Refactor and enhance unit tests for marker parsing, JSON salvage, and git repository detection using TestDir utilities.

## Completed
- [x] Removed obsolete assertions in `marker_prefix_at` test cases.
- [x] Updated `salvage_invalid_json_marker_at_end_of_string` test to verify proper handling of JSON containing a marker.
- [x] Refactored `find_git_repo` tests to use `TestDir` fixtures instead of raw `tempfile`.
- [x] Updated test setup and assertions to employ the new `TestDir` utilities.
