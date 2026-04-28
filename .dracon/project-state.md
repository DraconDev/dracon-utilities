# Project State

## Current Focus
Fix Nix garbage collection to only delete old generations when `apply` is true

## Completed
- [x] Modified Nix garbage collection logic to conditionally delete old generations based on `apply` flag
- [x] Added conditional check for `apply` parameter in garbage collection function
