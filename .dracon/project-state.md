# Project State

## Current Focus
Added support for custom repository name mapping in remote configurations.

## Context
The change enables mapping local repository names to different remote project names when needed, particularly for repositories that require sanitization (e.g., names starting with dots).

## Completed
- [x] Added `repo_name_map` field to `RemoteConfig` for name mapping
- [x] Implemented `resolve_push_url` to use mapped names when available
- [x] Added `resolve_repo_name` helper method
- [x] Updated default configurations to include empty maps

## In Progress
- [ ] None

## Blockers
- None identified

## Next Steps
1. Add tests for the new name mapping functionality
2. Document the new configuration option in user documentation
