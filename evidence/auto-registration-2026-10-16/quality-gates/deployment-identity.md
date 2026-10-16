# Deployment identity record

Date (UTC): 20261016T230929Z
Repository HEAD: dd7681bf29844717d33b1f3c1c0be92c666eb2c0
Source commit: dd7681bf29844717d33b1f3c1c0be92c666eb2c0

## Tested-vs-installed identity (matched)

| Artifact | Path | SHA256 |
|----------|------|--------|
| Test artifact | target/release/dracon-sync | f3b79691f5cfacfde92de098cf712f42309a53cf385650f1652f0d8083f9d656 |
| Installed binary | ~/.local/bin/dracon-sync | f3b79691f5cfacfde92de098cf712f42309a53cf385650f1652f0d8083f9d656 |
| Running daemon | /proc/2936146/exe | f3b79691f5cfacfde92de098cf712f42309a53cf385650f1652f0d8083f9d656 |

Tested == installed == running. The daemon was restarted via sanctioned
maintenance `dracon-sync maintenance -- systemctl --user restart …` to load
the new binary; the freeze marker was removed automatically on exit.

## Retained rollback copy (outside watched source trees)

| Field | Value |
|-------|-------|
| Path | ~/.dracon/backups/dracon-sync-pre-bucket-e2e-20261016T230817Z/dracon-sync |
| Pre-install SHA | 8f7f45c9a82b41dfb5bf762fa9d3347837a8ffe534eee6c8115c5242e8db2030 |
| Manifest | ~/.dracon/backups/dracon-sync-pre-bucket-e2e-20261016T230817Z/manifest.txt |
| Head at backup | dd7681bf29844717d33b1f3c1c0be92c666eb2c0 |

## Quality gate summary

| Gate | Command | Status |
|------|---------|--------|
| cargo test --workspace --locked | `cargo test --workspace --locked` | 2430 passed; 0 failed |
| cargo build --release --locked | `cargo build --release --locked -p dracon-sync` | ok (2m19s) |
| cargo clippy --workspace --locked | `cargo clippy --workspace --locked -- -D warnings` | clean |
| cargo deny check | `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok |
| bucket-runner e2e + recovery | `bash scripts/verify-bucket-registration-e2e.sh` | 35/35 pass |
| Sanctioned maintenance install | `dracon-sync maintenance -- ...` | daemon restart clean |
| Daemon health post-restart | `dracon-sync health` | healthy · 34 repos · freeze off |

## Daemon health (post-install)

- status: healthy
- daemon: running (pid 2936146)
- freeze: off
- policy: valid
- discovered repos: 34 across 2 roots
