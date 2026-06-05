# CLI Audit — dracon-utilities

**Date:** 2026-06-05
**Scope:** All three utilities (dracon-warden, dracon-system, dracon-sync) and supporting files
**Goal:** Identify deprecated, broken, vestigial, or relic commands/flags. Clean up.

---

## Summary of Findings

| # | Utility | Item | Action | Justification |
|---|---------|------|--------|---------------|
| 1 | dracon-warden | `daemon` subcommand | **REMOVE** | Deprecated since hooks became primary security layer. Service file uses this dead command → infinite restart loop → service is functionally dead. |
| 2 | dracon-warden | `dracon-warden.service` `ExecStart=daemon` | **REPLACE** with no-op or remove service entirely | Service runs a command that immediately exits 0 with deprecation warning. Restarts every 3s forever. |
| 3 | dracon-warden | `install.sh` warden service handling | **REMOVE** warden service start/restart logic | Hooks (installed via `setup-hooks --global`) are the primary enforcement. No reason to have a systemd service for the daemon that does nothing. |
| 4 | dracon-sync | `--force` flag on `sync-now` | **REMOVE** | Mass-deletion guard was removed (per AGENTS.md). Flag is `hide = true` and a no-op. |
| 5 | dracon-sync | `dracon_sync_mass_deletion_guard_blocked_total` metric | **REMOVE** | Always 0, marked "Obsolete (guard removed)". No value in keeping a stub. |
| 6 | dracon-system | `link doctor` and `link status` are identical | **MERGE** or **DEFER** | Both call `build_link_report()`. Slight functional redundancy, but distinct intent (status=read, doctor=diag). Not a clear bug. |
| 7 | All | `filter-clean` / `filter-smudge` on warden | **KEEP** | Called by git, required for smudge filter. Not for direct use but must remain. |
| 8 | All | `setup-hooks` on warden | **KEEP** | Primary security layer. Required. |

---

## Detailed Audit

### 1. dracon-warden

**Location:** `dracon-warden/src/main.rs`

| Subcommand | Flags | Status | Justification |
|------------|-------|--------|---------------|
| `status` | — | KEEP | Reports policy path and watch roots. Standard status command. |
| `once [repo]` | `repo: Option<PathBuf>` | KEEP | One-shot hardening pass. Primary entrypoint for manual hardening. |
| `daemon` | — | **REMOVE** | Deprecated (line 206). Implementation (line 1348) only prints warning and exits 0. The systemd service runs this command, so the service is dead. |
| `scrub-markers` | `--apply`, `repo` | KEEP | Scans for DRACON_SECRET markers. Used by `repair`. |
| `resmudge` | `--apply`, `repo` | KEEP | Fixes stuck ciphertext in working tree. Useful recovery tool. |
| `repair` | `--dry-run`, `--strict`, `repo` | KEEP | Combined harden + resmudge + scrub pass. Primary recovery command. |
| `filter-clean` | `path: Option<String>` | KEEP | Git filter clean. Required for smudge mechanism to work. Not for direct use. |
| `filter-smudge` | `path: Option<String>` | KEEP | Git filter smudge. Required. Not for direct use. |
| `keygen` | — | KEEP | Generates age keypairs. Required for setup. |
| `setup-hooks` | `--global` / `--local`, `repo` | KEEP | Installs pre-commit + pre-push hooks. Primary security layer. |

**Service file issue:** `dracon-warden/dracon-warden.service` has `ExecStart=%h/.local/bin/dracon-warden daemon`. The `daemon` subcommand immediately exits 0 with a deprecation message, but the service has `Restart=always` with `RestartSec=3`, causing an infinite restart loop. The service is essentially dead code.

**Resolution:** 
- Remove `Command::Daemon` enum variant from `main.rs`
- Replace service `ExecStart` with a no-op (e.g., `sleep infinity` is too heavy; better to remove service entirely from install.sh)
- Update install.sh to skip warden service start/restart

### 2. dracon-system

**Location:** `dracon-system/src/main.rs`

| Subcommand | Sub-subcommand | Flags | Status | Justification |
|------------|----------------|-------|--------|---------------|
| `status` | — | `--json` | KEEP | Reports core path and service status. |
| `doctor` | — | `--json`, `--strict` | KEEP | Diagnostics with strict mode. |
| `events` | — | `-t/--tail`, `--source`, `-s/--severity`, `--dedup`, `--json` | KEEP | Cross-utility event viewer. |
| `storage [root]` | — | `--json`, `--cleanup`, `--apply`, `--allow-tracked`, `--min-size-mb`, `--kinds` | KEEP | Storage analyzer with safe cleanup. |
| `link` | `status` | `--json` | KEEP | Read-only link reconciliation report. |
| `link` | `doctor` | `--json` | MERGE (defer) | Identical implementation to `status` (both call `build_link_report`). Could be merged but distinct intent. |
| `link` | `apply` | `--json`, `--force-replace` | KEEP | Applies link policy. |
| `zram` | — | `--status`, `--gen-config`, `--memory-percent`, `--algorithm` | KEEP | NixOS-specific zram tuning. |
| `guard` | `once` | `--json` | KEEP | One-shot guard evaluation. |
| `guard` | `daemon` | — | KEEP | Continuous guard loop. Active service. |
| `guard` | `prune` | `--json`, `--docker`, `--docker-volumes`, `--package-caches`, `--apply` | KEEP | Manual prune of caches/docker. |
| `guard` | `clean` | `--json`, `--apply`, `--rust`, `--trash`, `--nix`, `--caches`, `--node-modules`, `--docker`, `--all`, `--min-size-mb` | KEEP | Bulk cleanup with explicit kinds. |

**Broken symlink detection:** The `link doctor` command already detects broken links (target_missing) — but only for symlinks declared in the policy. The user wants a `symlinks` command that scans the filesystem for any broken symlink, not just policy-declared ones. This is a new feature, not a relic.

**Resolution:** 
- Add new `symlinks` subcommand that scans `~/Dev`, `~/.dracon`, `~/.local/bin`, `~/.config` (or `~/`) for broken symlinks
- Report-only (no auto-fix)
- JSON output support for AI consumption

### 3. dracon-sync

**Location:** `dracon-sync/src/main.rs`

| Subcommand | Flags | Status | Justification |
|------------|-------|--------|---------------|
| `status` | `--json` | KEEP | Policy + scope report. |
| `repos` | `--only-concern`, `--only-warn`, `--json`, `--sort`, `--filter`, `--full-path` | KEEP | Repo status report. Core feature. |
| `validate-config` | — | KEEP | Validates policy. |
| `repair-concerns` | `--apply`, `--repo`, `--push-timeout-secs`, `--push-retries`, `--rewrite-large-any`, `--only-stuck-push`, `--only-stuck-pull`, `--json` | KEEP | Auto-repair concern repos. |
| `repair-warns` | `--apply`, `--repo`, `--json` | KEEP | Triage warn repos. |
| `once` | — | KEEP | One-shot sync. |
| `daemon` | `--interval-secs` | KEEP | Active service. Not deprecated (unlike warden). |
| `sync-now` | `--dry-run`, `--force` (hidden) | **REMOVE `--force`** | Mass-deletion guard removed (per AGENTS.md). Flag is a no-op. |
| `pause` | — | KEEP | Freeze sync. |
| `resume` | — | KEEP | Unfreeze sync. |
| `edit-config` | — | KEEP | Open policy in editor. |
| `health` | `--json` | KEEP | Daemon health check. |
| `metrics` | — | **REMOVE obsolete metric** | `dracon_sync_mass_deletion_guard_blocked_total` is always 0. |
| `stuck` | (subcommand) | KEEP | Manage stuck repos. |
| `dual-branch` | (subcommand) | KEEP | Manage dual-branch repos. |
| `repair-origins` | `--apply` | KEEP | Orphan origin detection. |
| `publish` | `run`, `status` | KEEP | Registry publishing. |
| `publish-status` | — | KEEP | Publish status. |
| `scaffold` | `--repo`, `--files`, `--overwrite`, `--dry-run` | KEEP | Standard file scaffolding. |
| `repair` | `concerns`, `warns`, `origins`, `stuck-list`, `stuck-unstuck`, `dual-branch-list`, `dual-branch-repair` | KEEP | All sub-subcommands are valid. |

**Note:** The `repair-concerns`, `repair-warns`, `stuck`, `dual-branch`, `repair-origins` top-level commands appear redundant with `repair` subcommand structure but are kept for backward compatibility.

**Resolution:**
- Remove `--force` flag from `SyncNow` in `main.rs`
- Remove `dracon_sync_mass_deletion_guard_blocked_total` metric from `metrics` command

---

## Files to Modify

1. `dracon-warden/src/main.rs` — Remove `Command::Daemon` variant and its match arm
2. `dracon-warden/dracon-warden.service` — Replace `ExecStart` with a no-op OR remove service
3. `install.sh` — Remove warden service install/start/restart logic
4. `dracon-sync/src/main.rs` — Remove `--force` flag, remove obsolete metric
5. `dracon-system/src/main.rs` — Add `Symlinks` subcommand (new feature)
6. `dracon-system/src/links.rs` — Add `cmd_symlinks` implementation
7. `AGENTS.md` — Update to reflect removals

---

## Verification Plan

1. Build all three utilities (`cargo build --release` for each)
2. Run unit tests (`cargo test -- --test-threads=1` for each)
3. Run `install.sh --dry-run` to verify install script still works
4. Run `install.sh` to install and verify warden service no longer restarts
5. Verify `dracon-warden --help` no longer shows `daemon` command
6. Verify `dracon-sync metrics` no longer shows `mass_deletion_guard_blocked_total`
7. Verify `dracon-system symlinks` command works and reports broken symlinks
8. Confirm all 23 repos still OK after daemon restart
