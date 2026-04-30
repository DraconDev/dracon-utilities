# Project State

## Current Focus
Updated test input and assertion for `salvage_invalid_json_markers` to verify that a valid DRACON_SECRET marker within a JSON value is properly salvaged and yields "null" or "__scrubbed__".

## Completed
- [x] Changed test input from `r#"{"key": "value", "secret": [DRACON_SECRET:abc}"# to `r#"{"key": "value", "secret": "[DRACON_SECRET:abc]"}"#`
- [x] Modified the test to unwrap the salvage result with `expect` and assert that the output contains "null" or "__scrubbed__"
