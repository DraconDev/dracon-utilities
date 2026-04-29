# Project State

## Current Focus
Add comprehensive CLI reference documentation and new subcommands, plus optional root path and JSON flag for storage analysis.

## Completed
- [x] Added detailed CLI reference documentation for dracon-sync, dracon-system, and dracon-warden, listing all commands, flags, and subcommands.
- [x] Introduced new dracon-sync subcommands: `once`, `daemon`, `sync-now`, `edit-config`, `test-ai`, `stuck`, `dual-branch`, and related list/unstuck variants.
- [x] Added global flags `-v`, `-vv` for verbosity and `-V` for version.
- [x] Implemented optional `--root` argument for the `storage` command, allowing specification of a custom analysis root path.
- [x] Added `--json` flag to the `storage` command to output analysis results in JSON format.
