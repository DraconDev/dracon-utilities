# Dracon Utilities

Monorepo for the three Dracon system CLI utilities — `dracon-sync`,
`dracon-system`, and `dracon-warden` — plus the shared Cargo workspace,
CI/Nix wiring, installer, and operational documentation.

All three source trees live in this repository (since 2026-08-22,
imported via subtree merges so commit history stays connected).
Binaries install to `~/.local/bin/`, run as user-level systemd services
where appropriate, and keep operational state outside the git tree
under `~/.dracon/`.

## Latest versions (2026-08-22)

| Utility | Version | Notes |
|---------|---------|-------|
| `dracon-sync` | **0.113.53** | Watched-repo-vanished CONCERN; cold-render parallelism fix; probe-timeout false-BROKEN fix |
| `dracon-system` | **0.112.38** | Active-build detection + storage-cleanup protected-path fixes |
| `dracon-warden` | **0.113.5** | Release candidate built & installed; registry publication remains an operator step |

Per-crate details live in each directory's `CHANGELOG.md` /
`release-notes-*.md`. Releases are tagged on this monorepo going
forward; the historical standalone repositories
(`DraconDev/dracon-sync-background-auto-commit-multi-remote`,
`dracon-system-disk-process-guard-doctor`,
`dracon-warden-secret-encrypt-age-git-filter`) remain as **frozen
mirrors** of pre-merge history and are no longer updated.

## Repository layout

```
dracon-utilities/
├── dracon-sync/      # background auto-commit multi-remote git sync daemon
├── dracon-system/    # disk/process guard, storage & diagnostics
├── dracon-warden/    # age-based git secret encryption (hooks + CLI)
│   └── src/security/ #   embedded dracon-security crate
├── .github/workflows/ci.yml  # lint / test / release-build / deny / nix
├── flake.nix         # Nix build (single-checkout; no external src inputs)
├── scripts/          # release, checks, audit tooling
└── AGENTS.md         # how agents/operators work in this repo
```

## Build & test

```bash
git clone https://github.com/DraconDev/dracon-utilities.git
cd dracon-utilities

cargo build --release --locked          # all three binaries -> target/release/
cargo test --workspace --locked         # full suite (~1000 tests)
cargo clippy --workspace --locked -- -D warnings
cargo deny check                        # advisories / licenses / bans
nix flake check                         # optional Nix build path
```

Install:

```bash
./install.sh
systemctl --user restart dracon-sync.service dracon-system-guard.service
```

Each utility is also independently buildable:
`cargo build --release --locked -p dracon-{sync,system,warden}` from the
repo root.

## Governance & daily checks

- [`AGENTS.md`](AGENTS.md) — repo architecture history, daemon policies,
  commit discipline, and forbidden actions.
- Daily systemd timer (`dracon-nested-pins-check.timer`, 09:00) runs
  `scripts/check-repo-identities.py`: every repo's effective git identity
  must be canonical (`DraconDev <dracsharp@gmail.com>`) or its deliberate
  `<name>-dev` loop identity.
- Audit baseline: [`AUDIT_FULL_2026-08-21.md`](AUDIT_FULL_2026-08-21.md)
  (0 HIGH; remediation status tracked inside).

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
| [docs/design/github-feature-repos.md](docs/design/github-feature-repos.md) | Historical façade-repo design and its replacement |
| [dracon-sync/README.md](dracon-sync/README.md) | Sync daemon usage and configuration |
| [dracon-system/README.md](dracon-system/README.md) | System guard usage and configuration |
| [dracon-warden/README.md](dracon-warden/README.md) | Repo encryption usage and configuration |
| [SECURITY.md](SECURITY.md) | Security reporting policy |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution workflow |

## License

AGPL-3.0-only — see [LICENSE](LICENSE) for details.
