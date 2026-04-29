# Project State

## Current Focus
Add optional daemon scan interval override and new subcommands for managing stuck and dual‑branch repositories with verbosity support.

## Completed
- [x] Modified `run_daemon` in `daemon.rs` to accept an `override_interval_secs: Option<u64>` and compute `scan_interval` from it.
- [x] Updated `run_daemon` invocation in `main.rs` to pass the new interval argument.
- [x] Added a global `--verbose` flag (countable up to 2) via `clap::ArgAction::Count`.
- [x] Extended the `Daemon` subcommand with an `interval_secs` option to override the policy scan interval at runtime.
- [x] Added `Stuck` and `DualBranch` enum variants with nested `List`, `Unstuck`, and `Repair` subcommands.
- [x] Updated `main` logic to handle the new `Daemon` variant and to call `run_daemon` with the supplied interval.
- [x] Corrected the stuck‑repo error message to reference the proper command (`stuck unstuck`).
- [x] Inserted `SimpleAiService::reset_health()` before testing AI providers and restructured the testing loop to track provider names and the first working provider.
- [x] Made minor adjustments in `dracon-system/src/main.rs` to align CLI parsing with the new subcommands.
