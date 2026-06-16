# Dracon Utilities

Public release repository for the Dracon system service CLI utilities. These tools install to `~/.local/bin/`, run as user-level system services where appropriate, and keep operational state outside the git tree.

**Current release:** `v0.112.5` — [release notes](https://github.com/DraconDev/dracon-utilities/releases/tag/v0.112.5)

This repository contains the CLI wrappers and release packaging. Shared library code lives in the sibling [`dracon-libs`](https://github.com/DraconDev/dracon-libs) repository.

## Utilities

| Utility | Purpose | Runtime |
|---------|---------|---------|
| [`dracon-sync`](dracon-sync/README.md) | Background, auto-commit, multi-remote git sync for developer workspaces | systemd user service |
| [`dracon-system`](dracon-system/README.md) | Disk, process, guard, doctor — local machine diagnostics and watchdog | systemd user service |
| [`dracon-warden`](dracon-warden/README.md) | Secret, encrypt, age, git-filter — repository hardening and smudge/clean encryption | git hooks + CLI |

### Façade repos

Each utility has a feature-presentation repo (a "façade") for discoverability on GitHub, GitLab, and Codeberg. The façade repos contain only navigation + metadata; the implementation code lives here in the monorepo.

- [`DraconDev/dracon-sync-background-auto-commit-multi-remote`](https://github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote) (also on [GitLab](https://gitlab.com/DraconDev/dracon-sync-background-auto-commit-multi-remote) + [Codeberg](https://codeberg.org/dracondev/dracon-sync-background-auto-commit-multi-remote))
- [`DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBNT0tKTloyK3BtckQvekRVRDBXKzUyV0xwVmljb2M0NEdSSGo4OW81cjFFCk5zRUpOcHBoeXpvZmJqTldQWDkrUnJmb0dVSEZFN1Z3bThVUjlpekd0dGcKLT4gWDI1NTE5IEErS3N2SVI1WitUNkJ6YVdoaDNMRTZPbkVUNzlQcFQxWlRVd2s5enlveGcKYzYrLzV5TnJadHJJa2RobWt0Q1RUbEV4WnBIMnpGaUdzb3doQlYvclN6RQotPiBYMjU1MTkgOGlWM01QZ29NQndIUytqaUhqdkgyV2ZKT3ZSZVp1TThEczR2L3duOEhRQQpoeWNqSGk3MHBsYUVNQW5oenVmN1dibEtDNzJlSjhvQzhraVFpU1AzeDFJCi0+IFgyNTUxOSBTbjBQSUtERGx0UGRzUDFxL2M3SW1tSGY4TkFjWmpndnhxYlNzbngvMmxRCkNmL2E0WEdDbDlhYUlpOTI3bjBKak9FTUZLa0xndzdSaXZ3NHJXYzduY2sKLT4gWDI1NTE5IGZuRFZwYXhncDlSUXpFQk9BOGUyRStsblc2K3Z1NjBSWi9aTGZhNEw5VFkKSTNEb2lZTDBTQW1VVTJlQmtyN2NlbWIxa3g4QmV2OHRxRjNuMFZnZkQrMAotPiBrLWdyZWFzZQprV0J4WExwODNjdndHeFE2VE9TRjVtQy8xbXlFSldwWXJjb0loT2xXOW9CRlRXd2JGMFc0SjExRXpnCi0tLSBMTVUxaDRLcXkvaWMwWVhTOXdIamMxTmJobDdBSTZQNDBXYm41d01MVjQwCjNwskuCDl0O6GRL7CEYS+pOgpP4d5kjlIDDdV1tPGGuk82ZQUCCBFpqWvMpYZ5eGwn2cLhsClA=]`](https://github.com/DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBRRE95UWFhMTZMejVTV1hCQlhTV2hXbXJhL0NTdUwxNVB4eTNxcGJFZmxFCkJBTHl0a29tcmVubndyL2RqQ3M4S2I1bVVic3FHMTRNU0Z2TFd3TVZEK0EKLT4gWDI1NTE5IEcvdXhnMExtY0J3RDlkTVJ2cGFvK0lnRElXTVNWTGkyNW5jZGdkQmpmMlEKa2ttUG0rczdxazJmK095MWUxUktVakFHdExFUGxLZkFLYUdSSGFFWTRHWQotPiBYMjU1MTkgTmFXVDI5cVhlcnQ5cjVOVm1CTHFoWWhpMWE2VEhxNFlKSzVUUlB4S1VWbwpJdEZ3VklRVkMxeEovbEZDR2Y2VGlNaS9kSzZLYmtBRG1MSWE1TlNRaFJZCi0+IFgyNTUxOSBKSGt4UXlHR1ZYK21NcFBDek9LcnlGanQ3dHR6cXBjUDlVOE9mYVI4eFFzCnBHWSszbzJqNE9jTk4xT01wMXE4MTZUTXQweHJUd3FuSUNSWFFqM3pmUmsKLT4gWDI1NTE5IFhlWXBxMTY5WU0vRy9LcnU4dTk4OFpnU25XWE8zV2R6Ykg1WTZXVjVtU0EKTmRMcURyRFFVVXpFR1BaVnYzZHBWakZvMDFzckUzSUJqSWRYLy9aMkFIOAotPiBobTstZ3JlYXNlIGIKZ2pxRlpnY1prV3NRclBvCi0tLSBZNEhLSFcydHlUQlgrb2tqSm5henZTWDNyRFl6R1h2emcwdjFzd0xzVmZBCt/Xc902xD8sfQLAaYScHeWXBzAe4Kule4sNTlIGXgzt3hIm1qF1ylzSYbg/HiebRdOZDHhGOmg=]) (also on [GitLab](https://gitlab.com/DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA3VlBrdHk3T0lveUVUZkNHR1U0L1JRUjg0RjlDOEZ4UjJYQVFwa1AzVURjCnA2M0cvbXZkaC85M2RPZFNKSEFYUHhNRVliSVZlM0g4eW1LcnNwNTdMWFEKLT4gWDI1NTE5IDkyWWR2d3o1UDA1WGpnaTRucmwzNGttN0M2WkY3ZEpUMExnLytmVjhwMTgKVkp6RHp0ZU9HZVlMbDRYdnU4Z1ZzZGJKaGsyMUtlb01qMW5PRTR6bUdLQQotPiBYMjU1MTkgRy9qakdUamEraG1JR2FDMTZteUNPZnJDbWlneEkrSktidVZVWXhwbFJYZwpNQzRVQmIxTys0ako1N2ZVSVVaYkdaTk9QYTdNQm5UaXNYK1Mwclh5VVg0Ci0+IFgyNTUxOSA0RWFJRlAyNEhCanJLVFEvbDJRZ1JLZDFtOGZranhkK1JYRjRpb2p6alVnCnNrZnd5S3R2MGNqVTU2Zk1JR2MxVW5TeVAxZlNEL2xDR290ZHFMZENsNkUKLT4gWDI1NTE5IExmWitLU09aUEh2SEhqeUJTemxpZFYwcjlHNVlnWlZUTmlDdFVIamc3U2cKUXJabzB5N0RxL1A5dlBUTDhoUkI3WUJlcG9WeklDSnM0UndBa0NjY1cxUQotPiBGKVc4KS1ncmVhc2UKd3IxOGF1UkRrbyttY0JlWWtiMXQ4WTR1c0l3TEp6RGZMQXdyemcKLS0tIFVMUmlDR1dQZGlUcTRWMmdQQkEwWEN0cUxwM014Q25zdlFPbFUydjhNejgKKsmBnPnYo478tLCYyY5gF5NmcHdqke5jkaJXlv7fRxdyAOeuQFh0KNnWf44PU6OyatwS5u/dgw==]) + [Codeberg](https://codeberg.org/dracondev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBJTG5UU2t2c0ErUzZSWG44NzVuOERmYS93S3UrY3VISGN1NERyeVRvL2g4CnNzTk5SaGxsR2lsSExwMGVkN1ltM3pGRktLblBqSVpkRFlwajVOVUlNM2sKLT4gWDI1NTE5IEExa0E2ZTQ4R0JacXVKakVma1JtakpsU3hJNmNmMTJNNWFieldXeVNSRFEKVWdYMkxQOGkvamhMdWczc0RXVzVsTWxCdDhYc3o5Y3E3eEt1aXI1MUZ3ZwotPiBYMjU1MTkgOUN4V3ZqbEVpMmJHQ0tyelIrdkxiWjdaaCt0dk9xbGdoRmFqUmVVL0dIYwpPV3pJTkx5MmM4TkE0VnNvUWxDRTBTVWV5Q0tnRzFqaEt4b0NMKzJ4dmY0Ci0+IFgyNTUxOSBkS0hDMkIrS2lGMmRyY1Y5blBBei92YUJOT2xSSzc3S1lMbEl0UWFXS2pFCnlROWg1amp1ZGlEODY0cm1qc3RYbW9pejJwbFpHRHJwRjJyTWZJazJrakUKLT4gWDI1NTE5IFdGdjc3QUhGY1ZKVWpEc2JuUnR4VGpYcGpIRVdZT0gxKzg4cDUvSDdhUUUKUGxFTEhBcFpCNnpFMFl2UTJ0ZVBWUThsNkpqckRMNVpqMWM1VjM5TXhvNAotPiBlP3VgXGJpYi1ncmVhc2UgWWJcCkJSQ2ZhOE5nN0RPd2RaWHo1MWg4aWgwd1E0QVg5dWF4TUZqazJ3S3BHWGNLcHlEUWtNTVh5MzRaUTJNL2FBCi0tLSBLbzloS1ZBNU0wckRFekIrT09uMzMwNHN5Z09lWVR5YjRzVFNpMDhoeVpnCq4erM8Yc4Hx1WtNabW/xExs+5BEHM+hK8q59jkYvXgl4GA9zAWG6tUtf4gbasi98BNhM4aQFuc=]))
- [`DraconDev/dracon-warden-secret-encrypt-age-git-filter`](https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter) (also on [GitLab](https://gitlab.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter) + [Codeberg](https://codeberg.org/dracondev/dracon-warden-secret-encrypt-age-git-filter))

The names are deliberately brutally-descriptive so they are self-explanatory in search results. The 3 façade repos stay in sync with this monorepo via `scripts/regenerate_facade_repos.py` (called from a `post-commit` hook). See `docs/design/github-feature-repos.md` for the full design.

## Quick Start

```bash
# Clone the public release repository
git clone https://github.com/DraconDev/dracon-utilities.git
cd dracon-utilities

# Required for building
git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs

# Install all utilities
./install.sh

# Restart services after installation
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
```

`dracon-warden` is installed by `install.sh` and enabled through `dracon-warden setup-hooks --global`; it does not run as a daemon.

## What Each Utility Does

### `dracon-sync`

`dracon-sync` watches configured git repositories, waits for changes to settle, commits deterministic diff-based messages, and pushes to origin and optional mirrors.

Common commands:

```bash
dracon-sync status          # Show policy path, roots, and discovered repos
dracon-sync repos           # One-shot repo report
dracon-sync health          # Daemon health check
dracon-sync daemon          # Run continuous sync loop
dracon-sync sync-now ~/Dev/my-project
dracon-sync sync-now --warns       # handle current WARN rows now
dracon-sync config validate
```

See [`dracon-sync/README.md`](dracon-sync/README.md) for configuration, mirrors, commit messages, repair commands, and release pipeline behavior.

### `dracon-system`

`dracon-system` protects local machines from disk/process pressure and provides storage, link, zram, and service diagnostics.

Common commands:

```bash
dracon-system status        # Show core path and service status
dracon-system doctor        # Run deterministic diagnostics
dracon-system storage ~/Dev # Analyze storage hotspots
dracon-system guard daemon  # Run continuous monitoring
dracon-system link status   # Check configured symlinks
dracon-system zram --status
```

See [`dracon-system/README.md`](dracon-system/README.md) for thresholds, cleanup behavior, process renice policy, and deployment examples.

### `dracon-warden`

`dracon-warden` encrypts secret-shaped content at rest in git while keeping normal plaintext files in the working tree. It uses git clean/smudge filters and global or local hooks as the primary enforcement layer.

Common commands:

```bash
dracon-warden status        # Show resolved policy and repo roots
dracon-warden keygen        # Generate a machine age keypair
dracon-warden setup-hooks --global
dracon-warden once
dracon-warden scrub-markers
dracon-warden resmudge
```

See [`dracon-warden/README.md`](dracon-warden/README.md) for the encryption model, plaintext-sibling escape hatch, recovery tools, and safety notes.

## Configuration

Each utility reads its own TOML policy under `~/.dracon/utilities/`:

| Utility | Policy | Example |
|---------|--------|---------|
| `dracon-sync` | `~/.dracon/utilities/sync/dracon-sync.toml` | [`dracon-sync/dracon-sync.example.toml`](dracon-sync/dracon-sync.example.toml) |
| `dracon-system` | `~/.dracon/utilities/system/dracon-system.toml` | [`dracon-system/dracon-system.example.toml`](dracon-system/dracon-system.example.toml) |
| `dracon-warden` | `~/.dracon/utilities/warden/dracon-warden.toml` | [`dracon-warden/dracon-warden.example.toml`](dracon-warden/dracon-warden.example.toml) |

Operational state lives outside this repository, for example:

```text
~/.local/state/dracon/
├── dracon-sync-incidents.jsonl
├── dracon-sync-stuck-push-repos.json
├── dracon-system-guard.log
└── visibility-sync/
```

## Services

| Service | Binary | Purpose |
|---------|--------|---------|
| `dracon-sync.service` | `dracon-sync daemon` | Git sync automation |
| `dracon-system-guard.service` | `dracon-system guard daemon` | Disk/process protection |

Useful commands:

```bash
systemctl --user status dracon-sync.service dracon-system-guard.service
journalctl --user -u dracon-sync -f
journalctl --user -u dracon-system-guard -f
systemctl --user restart dracon-sync.service dracon-system-guard.service
```

## Development

```bash
# Reliable full test run
export DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git
cargo test --workspace -- --test-threads=1

# Quality gates
cargo fmt -p dracon-sync -p dracon-system -p dracon-warden -- --check
cargo clippy -p dracon-sync -p dracon-system -p dracon-warden --all-targets --no-deps
cargo build --release -p dracon-sync -p dracon-system -p dracon-warden
cargo deny check
./scripts/verify-spec.sh
```

## Documentation

| Document | Purpose |
|----------|---------|
| [docs/ROADMAP.md](docs/ROADMAP.md) | Documentation map and release status |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Service architecture and deterministic commit protocol |
| [docs/OPERATIONS.md](docs/OPERATIONS.md) | Systemd, incident response, troubleshooting |
| [docs/design/cli-print-style.md](docs/design/cli-print-style.md) | Human-facing CLI output conventions |
| [docs/design/warden-plaintext-sibling.md](docs/design/warden-plaintext-sibling.md) | Warden plaintext escape hatch threat model |
| [docs/design/github-feature-repos.md](docs/design/github-feature-repos.md) | GitHub façade repos for feature-focused utility surfaces |
| [dracon-sync/README.md](dracon-sync/README.md) | Sync daemon usage and configuration |
| [dracon-system/README.md](dracon-system/README.md) | System guard usage and configuration |
| [dracon-warden/README.md](dracon-warden/README.md) | Repo encryption usage and configuration |
| [SECURITY.md](SECURITY.md) | Security reporting policy |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution workflow |

## License

AGPL-3.0-only — see [LICENSE](LICENSE) for details.
