# Project State

## Current Focus
Strict enforcement of prefix matching in marker parsing to only accept matches starting exactly at the provided offset

## Completed
- [x] Removed test asserting prefix detection at offset 7 (inside the prefix) for "[DRACON_SECRET:abc]"
- [x] Updated test expectation for a marker in the middle of text to return None instead of Some("[DRACON_SECRET:")
