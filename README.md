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
- [`DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBIZk1XN3I3bjZHSDBhRXZoWjhQWkpnY0ZpMGpHQnFEZkpsTzF5TmVqVkdBCm1oYzFMbURENnhYa2hXOEMzM0JvbU9lNVpLdUVPR212cmtneUx3MEVxaDgKLT4gWDI1NTE5IDVtR0toUUEwaFhjOUZKQ0h6bkhFb0xOak94amoxNnI5bzN1eCs2ajZOUzgKNkJSZmc0VDk1WWZ3bVIrcjlrTGREU1Bsd01nT2ZPbWFXK3h2dDRieGxLawotPiBYMjU1MTkgOFR6TkRPaDBBN1A0RlBlRG85N09OUHhqUHY5cnVIdGh1bktCa3RDc3FTawplSG53c05qZU9NK3pZL0hTVS80OGNLRmZOQ0FpWVV0eWN3cCtpblpicUk4Ci0+IFgyNTUxOSBoeGpZZUJHaDNqUUdBZWgvalhEcWpoZDZRKy9pdnlRNTU4L1gwVXdHTDJnCll4cTI0d05ickRGbTJzMS9vbWhlRnh0QzVJU1UxQzB2Q2RSVFNxd1VOZnMKLT4gWDI1NTE5IHRkNmVERldQN09USmJnK2xQeGhVYUVzYmxvMXZ0QVBRV3dPLzhzZ2N0UWcKbEM5YzBkM1krb08vdWdUbUdaczdwZ1pURDB5dGMwU1ZOeXBUK0FWK0daRQotPiBKZy1ncmVhc2UgMSUpIHFOVFx+PCBkent0bCAxCmMwYlphV2dueDFISk5BRkhBOGJpSEFBCi0tLSBhSnQ2MDl4TWVJaWRwb3BiTzlsQXRqek56RjA5NUFlQ2wwa1hwWmJsM3JVCnzagnCMJi5uC41IkSJ/Gvphw0aDpYolqVI3eSz5udXKsSpRQps5Ue9QE3q0W4YZeqL0uTYb9Mc=]`](https://github.com/DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB3SU01VG94UHpjQWdnczVkVDZhUncrZVBCUDV4RVo0VmxQcjFqYzM4ZFY4Cm5VNHNuSTRTVVFad0xaODlMbnVlYjd2anJodXREVUhLRzcvRXdOaGgwSUkKLT4gWDI1NTE5IFpUQnFVQ0UwWnZRWVVvbjhZeTdqemFSaVgvUHBRbGswVmR1YUc5TFZrMDgKR1dRUlZBdjFFYVQwZVM0L1NhWXBKRkhPQUR6MUVSTzk0YllURVVYeis0NAotPiBYMjU1MTkgZE5kM0l4cWJYSGFsZmVuQ0NJZGZ0MzdZbVFJNW9vYmpOTVFWMFhkQS9VYwpoRkRQdlFYWjZaZkUrVG1pVUFScEFWaEs5VnlhQ2QwS3hoeWs3djcxMlYwCi0+IFgyNTUxOSB5cmNTN0JhQ0J5WXUxbDVRVEdCOFZZNTFCK0pqWnJBanpiNnpRaFBGamkwClBxMnlwZ2FWT3FwTlFEK1ZVYVZxLzVxWTVWM1NQTE5QdlRlcUZsVWxhT28KLT4gWDI1NTE5IE93VjViNGxYa0kxNmVySkRMK1R1Yi9EdVNpbGRKaWxMeVY1TlcwdFpUeHMKL0xFYVBOSEpQdWdJSEZBSFFVbmxlRXVvQ25JY1NSM0IvTDdSSjVxZkx6NAotPiBURTc3Oi1ncmVhc2UgbzxqP2YgXV1acD1hI1Agby0gJwpDbFFuUnphcm1YblY3UQotLS0gL2RHNzEveWdnMWJDZ0tQbEJuMDhMaHpEZEdyUllpQTA2ZlRMalRwSUwwSQqqJJ+zkHOxsSeOSjaLaYwSUsvJGWfTTknkRAEx0aKyDWGQa2WlpNl/TdSCie1G/84LFvnl58Ni]) (also on [GitLab](https://gitlab.com/DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB1aE4xemlGVktmaU41S1dHN0EwbDAxcjdrbzUyL1FCaGxwRGpaNEJYVmo0CjZGc2hRQ2NGS05XM2xEc1o2ZkVNUFdxWlNUTHhwVjRQT3pGR28rdTlnMVUKLT4gWDI1NTE5IDlRdkJob29wY2xtVG0vc0hZK0w4Wk1lWVphT0lLNEVPTTJPNTFqN0R3QncKRUhVNmxPMzdnMmEvQmFYd3pRTFUzdDBOcmNreXhWQ0tNUWJFQjQzMlRQVQotPiBYMjU1MTkgU0FWU3FVRGZ3VHcvRjE2M2NyUHdXZHpBbGErSUZtVDZ6SXNkK0w2Ym0zWQo0Q2QzQksva0NGMzMxdjA3SHlmMUlLTDFOVHoxMHZNK0VRTUJPK2tiVE5FCi0+IFgyNTUxOSBFNGY4QmUyekxNTXE2TUtNcFRxRmFrRFljbGRFUkZxcEVyTk5wUDNKLzBJCnpXWVZHQkxleEZUK1ZzVjBxZ1BrSDAwL1hmdEJpYW1mYUVrZ3kwVzBqdTQKLT4gWDI1NTE5IFpzY09RNlZaUE02VzBISUxyUndtcEFtdWJnK1NhQmwrRzhNOU5NeEhKbTgKVjllQmliMUR6NGxKRmwvY29nOEdBRGlhdUJiVmFzVVVXQ0FSdDRXY3FISQotPiBeRS9QLkpNRi1ncmVhc2UgSXNtS0QgPzlbIDwqdGc7IHMKK0REbHJFeHpVWkY1VWExQUVBCi0tLSBCRUFFQmsxanVTYzYzbG54MDhabHMzZ08zVGd3ZlN0ZGhXVkRSOC9rV3ZjCkO6bOA1aXGqHFdi7szBVFw0inm8J4IycK3qIwh8k8IqMAA+U944F0B4nd7eOoiHsrOsmCgsIPI=]) + [Codeberg](https://codeberg.org/dracondev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA5dDNYeHlMMi8yalFTUVQ0cUpldk54NFNnYnF1MHZFeUtBd1UxVGlkR1NVCitMcHpURWNvVlFBc1hiNlNVc0dWcDc0SDA4alNNTHV1eHc0cEo3QmtINDgKLT4gWDI1NTE5IHh6c1dFVnFXUWkxdnFEeXlpdWs0QXZCWmtVY2tvWjVJSFBhcExxZzU0RGcKbzFEUzZkUC95ekVlWmR1VWhuOXFUSU14amZrdGFrS0ZpaXZhUG5HQ1p6QQotPiBYMjU1MTkgRjNGSlhQM3RDeXFZVW9NVzRPc2Q4bVgrcHFWTGs4eUxSSmxCVWVOWDFGZwpmemlpaXpDcHY2TzBMUXA2dlBGNW8vRnA5NFhMUGs5ZEJ1R3QwQzh3R0x3Ci0+IFgyNTUxOSBZOHhsbWFOMFNOK3hUOTE5c1A0aUVTSDdkUWVhbW1qeHN6bTYyVzVHd0VBCmJnSG1VVVJJcm42SkVHeEVjQUNkU1JOUUh2OTRRS0h6RlI4aXdrWkM3bXcKLT4gWDI1NTE5IEdjemNqMlg0cXBDU3dOaW5Iamg4Mk9OcGF0N25MeVlnY0hYdC9NSVU3bW8KemtlbU8reEJKeWt0cjN4YXlacFJLL3p0VSs4TnU5cnZGNU1sZ3BwSDNmWQotPiBIPGYtZ3JlYXNlCnBaVGxmVTNDcDNHdXJOWUV0NGx4bG5SSC9yUklkVjJ4SUNkNG5wdFZCalE3MCtKaUVZR010UDNHc2hRVHh6elgKRko1THhONlByTjBXRVkxREpDR0dxa0pNZGRIZkZxYXRIRisrWDkwCi0tLSA3L21HNEdxUmdvWXEzWUlabS9qOWdzdUF4b3RRemV6NXR1SUR2c2xTVXRVChxA2/+MJ8JRRh4pNdPA9v9gV8/nA22C9WM14SJOmfGFi4ir6g2fwQgGBrUkK1/yuJ9e6TucHZg=]))
- [`DraconDev/dracon-warden-secret-encrypt-age-git-filter`](https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter) (also on [GitLab](https://gitlab.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter) + [Codeberg](https://codeberg.org/dracondev/dracon-warden-secret-encrypt-age-git-filter))

The names are deliberately brutally-descriptive so they are self-explanatory in search results. The 3 façade repos stay in sync with this monorepo via `scripts/regenerate_facade_repos.py` (called from a `post-commit` hook). See `docs/design/github-feature-repos.md` for the full design.

## Repository architecture

This is a 4-repo system with distinct roles. Each repo has one job:

| Repo | Role | Contains | Updated by |
|------|------|----------|------------|
| `DraconDev/dracon-utilities` (this repo) | **Dev workspace** | All 3 utilities' source code + monorepo build + `install.sh` + tests + docs | The operator (manual commits) + `dracon-sync` daemon (auto-commits to all 4 remotes) |
| `DraconDev/dracon-sync-background-auto-commit-multi-remote` | **Façade main** for `dracon-sync` | README + LICENSE + SECURITY + .gitignore + .github/ + docs/SOURCE_OF_TRUTH.md | `post-commit` hook → `regenerate_facade_repos.py` → `dracon-sync` daemon (auto-pushes to all 3 remotes) |
| `DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA0Z1RLL1hybFlIUVl3dnFvakxLbENCUERaaHozVjB4MHRWYzNOa0tHSW5ZCjlidVNRU3E4NERzQWNKYURLdi93cllHR0dJYWIvVU12cmRGcjEzaEVQRGsKLT4gWDI1NTE5IGVGM0RLY1FFcEpHd3VPK2NMSHo3eEVNQWtUMWNwSkF2ODNjS1RMUW9lMzAKdDNlMFgrWHFkV2E0d3lmaFg2NGpOa0d0bDMwVHFsZWNoTXlaMTZMOFJKZwotPiBYMjU1MTkgQk41MjhtRFExSVhuNU1HTWtjU1gzMXVDOUlBQ2JRaHNIcjhpMUh6ZVRnawpUbkhCQWtMMkozMGI5cU5MeGpiSTRKYXcxOExCK1pwdzkyWitXSkRPcFJrCi0+IFgyNTUxOSAwcXBjbmNoSG1OWVUrbkNJT205RFMzcHlHeXkvb0NYQTdjL25CM1ZWa2tvCjFwaEI2ZEtPbjh3T2dHVWROcGF3akRVTG55OFFjREsrS3lHbWpRNldxSmMKLT4gWDI1NTE5IGt3ZDJrdTVjS3Uyd3l3ZGpQQnc0clZvejUrZ1RxMmNJVWJlOFNoVTZ1VGMKZTRJSlhCcEFJTnF1VTAzeld1VkRjUmRJRi9MbEtaY0JZRU5oWURuRzUxbwotPiBYeWFkSTAnVS1ncmVhc2UgNk1IRmQgYmI3fkF4QgpwUHd2VWhMZU5VaTRqV2J1SHZLT1ZWakdUYTRXcWxXbmZES2RYcE91d1VRd3phZ0ZSK0p3ZmZGK3lCRFVVZjRrClBWcDMwSGxNSVBxV0toQzR1UFJnZ3FTU1ovS0NDT3BZcGJ5S1p6dExVSEFXL2cxL1FpU1BwWXRWYldYS0FOcHAKWjRrCi0tLSBKNkxaVTF1MEZHcjcvWDdibk1naWFxRC9kU3pFVVdUUFZ4dDREOElLOFBrCn8q4nJWUwRDjwHJlvEs0onxr894YoIc0qJfLdbDjGpLvKcUpi7/NFuLzpPN9MJRYDBaHoOSJWQ=]` | **Façade main** for `dracon-system` | Same 7 files as above | Same |
| `DraconDev/dracon-warden-secret-encrypt-age-git-filter` | **Façade main** for `dracon-warden` | Same 7 files as above | Same |

**The 3 façade repos are the canonical "mains" for users**: when someone searches for `dracon-sync` on GitHub, GitLab, or Codeberg, the brutal-descriptive name leads them to the right place. Each façade repo is also cross-linked from the other 2 on all 3 remotes.

**The monorepo is the canonical "main" for builds**: `./install.sh` clones the monorepo, not the 3 façade repos. The façade repos contain only presentation content; cloning them would not give you a buildable source tree. The source code lives in the monorepo.

**The flow is one-way**: operator edits code in the monorepo → commits trigger the `post-commit` hook → the hook runs `regenerate_facade_repos.py` for the affected utility → the script writes the new README + metadata to the 3 façade repo clones at `/home/dracon/Dev/facade-repos/` → the daemon (`dracon-sync`) sees the local change and auto-pushes to GitHub + GitLab + Codeberg.

For a per-utility visitor who lands on a façade repo, the README points them to the monorepo for the source, and to the install instructions in this repo. For a developer who wants to build, they clone the monorepo + the sibling `dracon-libs` repo and run `./install.sh`. The façade repos are not a workaround for sync state — they are a presentation surface. See `docs/design/github-feature-repos.md` for the full design.

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
