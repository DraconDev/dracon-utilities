# Project State

## Current Focus
Update unit tests for `push_large_blob_threshold_bytes` to verify threshold calculation uses the minimum of stage and push limits with custom policy values.

## Completed
- [x] Renamed `test_push_large_blob_threshold_bytes_default` to `test_push_large_blob_threshold_bytes_custom` to reflect custom policy usage.
- [x] Set `max_push_blob_bytes` to `50 * 1024 * 1024` in the test policy.
- [x] Updated assertion to expect `50 * 1024 * 1024` as the threshold.
- [x] Renamed the second test to `test_push_large_blob_threshold_bytes_uses_min_of_all` for clarity.
- [x] Added `max_stage_file_bytes` set to `10 * 1024 * 1024` in the policy.
- [x] Updated assertion to expect `10 * 1024 * 1024`, confirming the threshold uses the smaller of stage and push limits.
