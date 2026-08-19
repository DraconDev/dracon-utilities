# Warden hygiene defaults and v0.113.5 handoff

**Date:** 2026-08-18/19  
**Status:** implemented in the standalone source; local release candidate
built and installed; registry/tag/forge publication intentionally pending.

## Decision

Warden's omitted `hygiene_patterns` value now has a narrow, machine-local
default. The policy is designed to keep harness state, browser traces, and
regeneratable frontend caches out of repository content without hiding normal
source or audit evidence.

| Default pattern | Reason |
|---|---|
| `**/.pi*` | Pi harness/runtime state is machine-local. Durable evidence belongs in normal documentation/source paths. |
| `**/chrometrace.log` | Browser trace output is regeneratable diagnostic output. |
| `**/.svelte-kit/` | Regeneratable SvelteKit build cache. |
| `**/.vite/` | Regeneratable Vite cache. |
| `**/.turbo/` | Regeneratable Turborepo cache. |
| `**/.cache/` | Regeneratable tool cache. |

A configured `hygiene_patterns = []` remains an explicit operator override.
Warden does **not** add a broad `*.log` default. Repository-local ignore rules
outside the Warden-managed block remain local policy and are not silently
removed by this change.

## Implementation

- `dracon-warden/src/main.rs` supplies the defaults through serde omission
  handling.
- `dracon-warden/dracon-warden.example.toml` documents the same baseline.
- Warden hardening propagates the managed patterns into repository
  `.gitignore` files while preserving content outside the managed block.
- The fleet Pi migration removed previously tracked `.pi*` paths from indexes
  while retaining local files on disk.
- The managed broad `*.log` entry was removed from affected generated blocks;
  no broad-log entry remains in a Warden-managed block.

## v0.113.5 local release candidate

The standalone Warden package metadata, lockfile package entry, changelog, and
release notes are aligned at `0.113.5`. The locked release artifact is
installed at `~/.local/bin/dracon-warden` and verified with
`scripts/verify-install.sh`. Warden has no systemd service; its installed
binary is used by the global Git hooks and CLI commands.

The source candidate is committed and mirrored on GitHub and GitLab. Registry
publication, tag creation, and forge-release creation remain separate,
operator-approved operations so a clone can be deleted without implying that
an external release was made.

## Verification checklist

- `cargo fmt -- --check`
- `cargo test --workspace --locked -- --test-threads=1`
- `cargo clippy --workspace --locked --all-targets -- -D warnings`
- `cargo build --release --locked`
- `scripts/verify-install.sh ~/.local/bin/dracon-warden`
- `dracon-warden --version` reports `0.113.5`
- `git diff --check`
- local/GitHub/GitLab `main` SHAs agree after daemon synchronization
