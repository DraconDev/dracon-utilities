# Project State

## Current Focus
Implement verbosity control infrastructure across all Dracon binaries (sync, system, warden) using a global VERBOSITY level and conditional `veprintln!` macro.

## Completed
- [x] Add global `VERBOSITY` static (AtomicU8) to daemon.rs for sync daemon
- [x] Add global `VERBOSITY` static (AtomicU8) to system/main.rs for guard daemon
- [x] Add global `VERBOSITY` static (AtomicU8) to warden/main.rs
- [x] Define `veprintln!` macro in both sync and system modules for conditional error output based on verbosity level
- [x] Wire CLI `-v`/`-vv` flags to VERBOSITY atomic in sync and system entry points
- [x] Convert daemon shutdown messages (SIGTERM, SIGINT, SIGHUP) to use `veprintln!(1, ...)`
- [x] Convert policy reload messages to use `veprintln!(2, ...)` for lower verbosity
- [x] Convert guard daemon startup and shutdown messages to use verbosity-aware output
