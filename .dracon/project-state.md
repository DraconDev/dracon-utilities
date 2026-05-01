# Project State

## Current Focus
Adjust the daemon’s shared state path to conform with XDG base directories by moving from the legacy “.dracon” folder to “$HOME/.local/state/dracon”.

## Completed
- [x] Update `stuck_repos_path` base directory to `$HOME/.local/state/dracon` for XDG compliance.
