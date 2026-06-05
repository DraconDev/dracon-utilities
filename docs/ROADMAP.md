# Documentation Roadmap

Where to find everything in the dracon-utilities repository.

## User-Facing Docs

| Document | Location | Purpose |
|----------|----------|---------|
| **README** | [README.md](../README.md) | Quick start, install, overview of all utilities |
| **Changelog** | [CHANGELOG.md](../CHANGELOG.md) | Version history and release notes |
| **Contributing** | [CONTRIBUTING.md](../CONTRIBUTING.md) | Development workflow and contribution guidelines |

## Per-Utility Docs

| Utility | README | Blueprint | Example Config |
|---------|--------|-----------|----------------|
| dracon-sync | [dracon-sync/README.md](../dracon-sync/README.md) | [dracon-sync/BLUEPRINT.md](../dracon-sync/BLUEPRINT.md) | [dracon-sync/dracon-sync.example.toml](../dracon-sync/dracon-sync.example.toml) |
| dracon-system | [dracon-system/README.md](../dracon-system/README.md) | [dracon-system/BLUEPRINT.md](../dracon-system/BLUEPRINT.md) | [dracon-system/dracon-system.example.toml](../dracon-system/dracon-system.example.toml) |
| dracon-warden | [dracon-warden/README.md](../dracon-warden/README.md) | [dracon-warden/BLUEPRINT.md](../dracon-warden/BLUEPRINT.md) | [dracon-warden/dracon-warden.example.toml](../dracon-warden/dracon-warden.example.toml) |

## Architecture & Operations

| Document | Location | Purpose |
|----------|----------|---------|
| **Architecture** | [ARCHITECTURE.md](ARCHITECTURE.md) | Sync architecture, AI-to-AI commit protocol |
| **Operations** | [OPERATIONS.md](OPERATIONS.md) | Systemd services, incident response, troubleshooting |
| **AI Agent Guide** | [AGENTS.md](../AGENTS.md) | Guidelines for AI agents working in this repo |

## Superseded Documents

The following root-level documents have been superseded by the docs above:

| Former Document | Superseded By |
|-----------------|---------------|
| `dracon-sync-architecture.md` | [docs/ARCHITECTURE.md](ARCHITECTURE.md) |
| `MASTER_ROADMAP_2026-06-01.md` | Completed — archived at [archive/](archive/) |
| `REPOS_CLEANUP_PLAN_2026-06-01.md` | Completed — archived at [archive/](archive/) |
| `STUCK_PUSH_TRIAGE_2026-06-02.md` | Completed — archived at [archive/](archive/) |
| `REFACTORING_BLOCKER_ANALYSIS.md` | Completed — archived at [archive/](archive/) |
| `SPEC.md` | Superseded by per-utility BLUEPRINTs |
| `tasks.md` / `TODO.md` / `todo.md` | Resolved — deleted |
| `audit.md` / `AUDIT.md` / `AUDIT_2026-05-29.md` / `AUDIT_CHECKLIST.md` | Completed — deleted |
