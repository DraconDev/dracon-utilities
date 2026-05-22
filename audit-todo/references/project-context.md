# Project Context

## Repository layout
```
/home/dracon/Dev/dracon-utilities/   <- workspace root
├── dracon-sync/                     <- crate: git sync daemon
├── dracon-system/                   <- crate: system maintenance daemon
├── dracon-warden/                   <- crate: security hardening daemon
├── dracon-ai/                       <- crate: AI support (not in workspace)
├── .github/workflows/ci.yml         <- CI pipeline
├── TODO.md                          <- current task list
├── CHANGELOG.md
├── deny.toml                        <- cargo-deny config
├── flake.nix                        <- Nix flake
└── install.sh

/home/dracon/Dev/dracon-libs/        <- sibling, required for building
├── tools/sync/dracon-git/           <- git operations library
└── services/ai/                     <- AI adapters
```

## Key binaries
- `target/release/dracon-sync`   — 13.5 MiB
- `target/release/dracon-system`  — 4.2 MiB
- `target/release/dracon-warden`  — 6.5 MiB

## Testing
```bash
DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git cargo test -p dracon-sync --test-threads=1
cargo test -p dracon-system --test-threads=1
cargo test -p dracon-warden --test-threads=1
```

## Service files
- `~/.config/systemd/user/dracon-sync.service`
- `~/.config/systemd/user/dracon-system-guard.service`
- `~/.config/systemd/user/dracon-warden.service`

## Policy files
- `~/.dracon/utilities/sync/dracon-sync.toml`
- `~/.dracon/utilities/system/dracon-system.toml`
- `~/.dracon/utilities/warden/dracon-warden.toml`

## Operational state
- `~/.local/state/dracon/dracon-sync-incidents.jsonl`
- `~/.local/state/dracon/dracon-system-guard.log`

## Dracon-libs git2 dependency
`dracon-libs/tools/sync/dracon-git/Cargo.toml` has `git2 = "0.18"`.
The advisory RUSTSEC-2026-0008 affects git2 0.18.3. Update to latest 0.18.x or 0.19.x.
