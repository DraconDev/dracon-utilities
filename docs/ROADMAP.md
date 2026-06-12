# Documentation Roadmap

This page maps the public Dracon Utilities documentation so users and contributors can find the right guide without relying on private or historical files.

## Public Release Status

The public repository is [`DraconDev/dracon-utilities`](https://github.com/DraconDev/dracon-utilities). The legacy private repository is separate and is not part of this public release history.

Current release docs are tracked on `main`. Release notes live in [`CHANGELOG.md`](../CHANGELOG.md) and GitHub Releases.

## Start Here

| Audience | Document | Purpose |
|----------|----------|---------|
| New users | [README.md](../README.md) | Install, quick start, utility overview, configuration links |
| Operators | [docs/OPERATIONS.md](OPERATIONS.md) | Services, incident response, troubleshooting |
| Contributors | [CONTRIBUTING.md](../CONTRIBUTING.md) | Setup, validation, docs standards, release checklist |
| Security reporters | [SECURITY.md](../SECURITY.md) | Vulnerability reporting policy |
| Maintainers | [CHANGELOG.md](../CHANGELOG.md) | Version history and release notes |

## Per-Utility Docs

| Utility | User README | Blueprint | Example Config |
|---------|-------------|-----------|----------------|
| `dracon-sync` | [dracon-sync/README.md](../dracon-sync/README.md) | [dracon-sync/BLUEPRINT.md](../dracon-sync/BLUEPRINT.md) | [dracon-sync/dracon-sync.example.toml](../dracon-sync/dracon-sync.example.toml) |
| `dracon-system` | [dracon-system/README.md](../dracon-system/README.md) | [dracon-system/BLUEPRINT.md](../dracon-system/BLUEPRINT.md) | [dracon-system/dracon-system.example.toml](../dracon-system/dracon-system.example.toml) |
| `dracon-warden` | [dracon-warden/README.md](../dracon-warden/README.md) | [dracon-warden/BLUEPRINT.md](../dracon-warden/BLUEPRINT.md) | [dracon-warden/dracon-warden.example.toml](../dracon-warden/dracon-warden.example.toml) |

## Architecture and Design Notes

| Document | Purpose |
|----------|---------|
| [docs/ARCHITECTURE.md](ARCHITECTURE.md) | Service architecture, deterministic commit protocol, shared-library boundary |
| [docs/design/cli-print-style.md](design/cli-print-style.md) | Human-facing CLI output conventions and invariants |
| [docs/design/warden-plaintext-sibling.md](design/warden-plaintext-sibling.md) | Warden plaintext escape hatch, threat model, revocation |
| [UTILITY_BOUNDARIES.md](../UTILITY_BOUNDARIES.md) | Canonical ownership boundaries between utilities and `dracon-libs` |

## Historical Notes

Internal audit artifacts, local task state, and private release-prep notes are intentionally not part of the public tree. If a historical detail matters for users, it should be rewritten into a public-safe note in this roadmap, the changelog, or the relevant user guide.
