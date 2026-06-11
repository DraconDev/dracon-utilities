# Public release notes

This branch is prepared for a public release candidate of the three Dracon utilities:

- `dracon-sync` — deterministic git sync automation for local Dracon workflows
- `dracon-system` — disk/process guard and storage diagnostics
- `dracon-warden` — git filter/hook based secret protection

## Versions

| Utility | Version |
| --- | ---: |
| `dracon-sync` | 0.1.5 |
| `dracon-system` | 0.2.0 |
| `dracon-warden` | 0.3.0 |

All three crates use Rust edition 2021 and AGPL-3.0-only licensing.

## Validation performed

The release candidate was validated with:

```bash
cargo fmt --check
cargo test --workspace -- --test-threads=1
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check
cargo build --release -p dracon-sync -p dracon-system -p dracon-warden
./scripts/verify-spec.sh
dracon-sync config validate
./install.sh --dry-run
```

All checks passed.

## Install

For local installation:

```bash
./install.sh
```

The installer builds the three binaries, installs them to `~/.local/bin/`, installs warden git hooks, installs the sync/system guard systemd user services, and restarts running daemons.

## Warden note

`dracon-warden` is intentionally hook-driven rather than daemon-driven. Its primary enforcement is through git hooks installed by:

```bash
dracon-warden setup-hooks --global
```

## Security note

The utilities include secret scanning and git filter support. Do not commit real credentials, private keys, or environment secrets into public repositories.
