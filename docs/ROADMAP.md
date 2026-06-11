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

## Historical / Superseded Documents

The following documents are historical references, archived documents, or older drafts. Use the current docs above for implementation and operation details.

| Former Document | Current Status |
|-----------------|----------------|
| `dracon-sync-architecture.md` | Archived in [archive/](archive/); use [docs/ARCHITECTURE.md](ARCHITECTURE.md) for current architecture. |
| `MASTER_ROADMAP_2026-06-01.md` | Archived in [archive/](archive/); use this roadmap for current documentation navigation. |
| `REPOS_CLEANUP_PLAN_2026-06-01.md` | Archived in [archive/](archive/); cleanup work is complete. |
| `STUCK_PUSH_TRIAGE_2026-06-02.md` | Archived in [archive/](archive/); use `dracon-sync repair stuck-list` for current stuck-push triage. |
| `REFACTORING_BLOCKER_ANALYSIS.md` | Archived in [archive/](archive/); use per-utility blueprints for current design work. |
| `SPEC.md` | Superseded by per-utility BLUEPRINTs. |
| `tasks.md` / `TODO.md` / `todo.md` | Removed; pi goals and current task workflow are canonical. |
| `audit.md` / `AUDIT_2026-05-29.md` / `AUDIT_CHECKLIST.md` | Superseded by [AUDIT.md](../AUDIT.md) and `docs/audit/`. |
| `docs/audit/audit-2026-06-06*.md` and `docs/audit/audit-2026-06-07*.md` | Historical audit records; use [AUDIT.md](../AUDIT.md) for the current full audit and closure evidence. |
