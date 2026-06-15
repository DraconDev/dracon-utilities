# AGENTS.md — how we work in dracon-utilities

> **Audience**: AI agents and human operators working in this
> repository. This file documents the durable, ongoing
> behaviors of the `dracon-sync` daemon — what to expect
> and how to interact with it.

## Commit policy (the most important section)

**Default behavior (since 2026-06-15, goal
`9aaf0b08` / `546d4f9c`)**: the daemon **commits all
untracked files by default**. Only files matching
genuine session-scratch patterns are kept untracked.

### What gets committed automatically

Everything else, including:
- User notes (`NOTE.md`, `notes.md`, `scratch.md`)
- Audit evidence (`audit/`, `evidence/`, `screenshots/`)
- Media files (`*.png`, `*.jpg`, `*.mp4`, `*.mov`)
- Logs and database files (`*.log`, `nohup.out`,
  `*.sqlite`, `*.db`)
- Source code, docs, configs, scripts, tests

### What stays untracked (super-good reasons)

These patterns are excluded from auto-staging:

```text
**/scratch/**, **/scratch-*, **/scratch_*
**/tmp/**, **/tmp-*
**/pi-tmp/**, **/.pi-tmp/**
**/research/scratch/**
.demon/**, .sisyphus/**, .ralph/**
```

These are session-scratch directories, agent session
state, and temp directories. They are ephemeral by
design.

### Size limit

Files larger than **100 MiB** (`104857600` bytes) are
NOT auto-staged. This is the hard exclusion threshold.

### Per-repo overrides

The per-repo `.dracon/dracon-sync.toml` can extend the
exclude list with `auto_commit_exclude_patterns`. Example
from `Junk-Runner-bevy`:

```toml
auto_commit_exclude_patterns = [
    "**/test-results/**",
    "**/e2e/screenshots/**",
]
```

This prevents the 2989-commit auto-commit loop that
crashed the daemon originally. The override mechanism
still works under the new global default.

## Investigation-first discipline

When investigating a state anomaly, **read all the
existing design docs first** before forming a hypothesis.
Recent design docs (in `docs/design/`) cover:

- `commit-all-policy-2026-06-15.md` — this policy
- `dracon-libs-deletion-2026-06-15.md` — symlink deletion
- `junk-runner-investigation-2026-06-15.md` — Junk-Runner-bevy policy drift
- `dracon-platform-untracked-commit-2026-06-15.md` — what stays untracked in dracon-platform (and why)
- `sync-push-classification.md` — push rejection classification
- `source-encryption-incident-2026-06-15.md` — encryption incident
- `warden-plaintext-sibling.md` — warden plaintext sibling handling
- `ownership-investigation-2026-06-15.md` — repo ownership analysis

Design docs are durable. Re-read them.

## Daemon commands

- `dracon-sync repos` — live state of all watched repos
- `dracon-sync doctor` — diagnose a specific concern
- `dracon-sync repair-concerns --apply` — apply a fix for a known concern
- `systemctl --user status dracon-sync.service` — daemon health

## Forbidden actions

- **NEVER** use `git add .` — always explicit paths
- **NEVER** force-push to repos with > 5 commits ahead
- **NEVER** rewrite history
- **NEVER** reconnect legacy private remotes
- **NEVER** delete operator-owned repos (kiki-sassy,
  one-mil-girls) without explicit approval
- **NEVER** auto-commit `.env`, `*.pem`, `*.key`,
  `*.age`, `secrets/**`

## Test discipline

- `cargo test --workspace --locked` must pass
- `cargo build --release --locked` must succeed
- `cargo deny check` must be clean
- New code paths require unit tests
- Backwards compatibility with all previously added
  policy fields is required
