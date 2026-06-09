# Dracon Utilities

CLI binaries for dracon system services. These install to `~/.local/bin/` and run as systemd user services.

## Table of Contents
1. [Architecture](#architecture)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [Design Philosophy](#design-philosophy-sync-is-invisible-infrastructure)
5. [Operational State](#operational-state)
6. [Services](#services)
7. [Systemd Service Files](#systemd-service-files)
8. [Policy Files](#policy-files)
9. [Tokens & Secrets](#tokens--secrets)
10. [CLI Reference](#cli-reference)
11. [Environment Variables](#environment-variables)
12. [Commit Messages](#commit-messages)
13. [Dependency Hygiene](#dependency-hygiene)
14. [Testing](#testing)

## Architecture

```
dracon-utilities/           <- CLI binaries (this repo)
├── dracon-sync/            -> ~/.local/bin/dracon-sync
├── dracon-system/          -> ~/.local/bin/dracon-system
└── dracon-warden/          -> ~/.local/bin/dracon-warden

dracon-libs/                <- Shared libraries (REQUIRED for building)
└── tools/sync/dracon-git/  <- git operations library
```

**Key point:** `dracon-utilities` contains the CLI wrappers. `dracon-libs` contains shared library code. Only the CLI binaries get installed.

**Workspace policy:** the root Cargo workspace intentionally includes `dracon-sync`, `dracon-system`, and `dracon-warden` only. `dracon-ai/` is a standalone subcrate and must be validated separately when touched; do not fold it into the main workspace without a separate compatibility review.

Standalone validation for `dracon-ai/`:

```bash
cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1
```

**Warden ↔ Sync are completely independent.** Sync never calls warden. Warden never calls sync. Encryption is enforced by git hooks installed by warden, not by sync.

## Prerequisites

**Required sibling directory:** `dracon-libs` must be checked out as a sibling to `dracon-utilities`:

```
~/Dev/
├── dracon-utilities/    <- this repo
└── dracon-libs/         <- required for building
    ├── services/ai/
    └── tools/sync/dracon-git/
```

Clone if needed:
```bash
git clone https://github.com/your-org/dracon-libs.git ../dracon-libs
```

## Installation

All binaries install to `~/.local/bin/`:

```bash
./install.sh
```

## Design Philosophy: Sync is Invisible Infrastructure

dracon-sync is designed to be **invisible infrastructure** for an AI coder. The AI works on one repo at a time, makes changes, and sync handles the rest — the AI never needs to think about commits, pushes, or cross-repo coordination.

**The AI workflow:**
1. User says "work on dracon-utilities"
2. AI reads `dracon-utilities/.dracon/project-state.md` (if present, for manual context)
3. AI makes changes
4. Sync daemon auto-commits and pushes
5. Done

**What sync provides:**
- Auto-commit on every change (AI doesn't need to think about git)
- Deterministic commit messages (routing keys for AI-to-AI communication)
- Incident ledger for debugging (AI can read what went wrong)
- Freezing for pause (AI can pause sync during delicate operations)
- Deterministic telemetry (task state, blast radius, metrics — all from diff)

**What sync doesn't need:**
- Global workspace state (AI works on one repo at a time)
- Session logging (AI doesn't "resume" — each session is fresh)
- Interactive features (AI runs non-interactively)

## Services

Services are in `~/.config/systemd/user/`:

| Service | Binary | Purpose |
|---------|--------|---------|
| dracon-sync.service | dracon-sync daemon | Git sync automation |
| dracon-system-guard.service | dracon-system guard daemon | Disk/process protection |

> **Note:** `dracon-warden` has no systemd service — git hooks (installed via `setup-hooks --global`) are the primary security enforcement layer.

```bash
# Restart after install (install.sh does this automatically)
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
```

## Systemd Service Files

Service files are installed to `~/.config/systemd/user/` by `install.sh`.

### dracon-sync.service

| Setting | Value | Purpose |
|---------|-------|---------|
| `ExecStartPre` | `-/run/current-system/sw/bin/pkill -x -f "dracon-git pulse"` | Clears stale sync helper processes before startup |
| `ExecStart` | `dracon-sync daemon` | Runs sync daemon |
| `Restart` | `always` | Restarts on any exit (clean or crash) |
| `RestartSec` | `5` | Wait 5s before restart |
| `Nice` | `10` | Lower CPU priority |
| `CPUQuota` | `15%` | Max 15% CPU usage |
| `MemoryMax` | `2G` | Max 2GB RAM |
| `MemoryHigh` | `768M` | Soft memory limit |
| `TasksMax` | `96` | Max 96 threads |
| `NoNewPrivileges` | `true` | Security hardening |
| `ProtectSystem` | `strict` | Read-only system fs |
| `ProtectHome` | `read-only` | Read-only home (except allowed paths) |
| `ReadWritePaths` | `~/.dracon, ~/Dev, ~/.local/state/dracon, ~/.ssh` | Writable directories |
| `PrivateTmp` | `true` | Isolated /tmp |
| `PrivateDevices` | `true` | No device access |
| `ProtectKernelTunables` | `true` | Read-only kernel tunables |
| `ProtectKernelLogs` | `true` | No kernel log access |
| `ProtectClock` | `true` | No clock changes |
| `ProtectHostname` | `true` | No hostname changes |
| `ProtectControlGroups` | `true` | No cgroup writes |
| `LockPersonality` | `true` | Lock execution personality |
| `MemoryDenyWriteExecute` | `true` | No writable executable mappings |
| `RestrictRealtime` | `true` | No realtime scheduling |
| `RestrictSUIDSGID` | `true` | No setuid/setgid |
| `RemoveIPC` | `true` | Remove IPC on service stop |
| `CapabilityBoundingSet` | empty | No Linux capabilities |
| `RestrictNamespaces` | `true` | No namespace creation |
| `SystemCallFilter` | `@system-service` | Syscall allowlist for service work |
| `RestartPreventExitStatus` | `2 78` | Don't restart on config/argument errors |
| `Environment` | `DRACON_SYNC_POLICY` | Points to config file |
| `Environment` | `GIT_TERMINAL_PROMPT=0` | Disables interactive git prompts |
| `PassEnvironment` | `SSH_AUTH_SOCK` | Forward SSH agent socket for git over SSH |

**Pre-start cleanup:** Kills stale `dracon-git pulse` processes to prevent lockups.

### dracon-system-guard.service

| Setting | Value | Purpose |
|---------|-------|---------|
| `ExecStart` | `dracon-system guard daemon` | Runs guard daemon |
| `Restart` | `always` | Restarts on any exit (clean or crash) |
| `RestartSec` | `10` | Wait 10s before restart |
| `MemoryMax` | `250M` | Max 250MB RAM |
| `CPUQuota` | `20%` | Max 20% CPU usage |
| `TasksMax` | `64` | Max 64 threads |
| `NoNewPrivileges` | `true` | Security hardening |
| `ProtectSystem` | `strict` | Read-only system fs |
| `ProtectHome` | `read-only` | Read-only home (except allowed paths) |
| `ReadWritePaths` | `~/.dracon, ~/Dev, ~/.local/state/dracon, ~/.local/share/Trash, ~/.cargo, ~/.cache, ~/.npm` | Writable directories |
| `PrivateTmp` | `true` | Isolated /tmp |
| `PrivateDevices` | `true` | No device access |
| `ProtectKernelTunables` | `true` | Read-only kernel tunables |
| `ProtectKernelLogs` | `true` | No kernel log access |
| `ProtectClock` | `true` | No clock changes |
| `ProtectHostname` | `true` | No hostname changes |
| `ProtectControlGroups` | `true` | No cgroup writes |
| `LockPersonality` | `true` | Lock execution personality |
| `MemoryDenyWriteExecute` | `true` | No writable executable mappings |
| `RestrictRealtime` | `true` | No realtime scheduling |
| `RestrictSUIDSGID` | `true` | No setuid/setgid |
| `RemoveIPC` | `true` | Remove IPC on service stop |
| `CapabilityBoundingSet` | empty | No Linux capabilities |
| `RestrictNamespaces` | `true` | No namespace creation |
| `SystemCallFilter` | `@system-service` | Syscall allowlist for service work |
| `RestartPreventExitStatus` | `2 78` | Don't restart on config/argument errors |



## Policy Files

| Utility | Policy Path | Example Config |
|---------|-------------|----------------|
| dracon-sync | ~/.dracon/utilities/sync/dracon-sync.toml | [dracon-sync.example.toml](dracon-sync/dracon-sync.example.toml) |
| dracon-system | ~/.dracon/utilities/system/dracon-system.toml | [dracon-system.example.toml](dracon-system/dracon-system.example.toml) |
| dracon-warden | ~/.dracon/utilities/warden/dracon-warden.toml | [dracon-warden.example.toml](dracon-warden/dracon-warden.example.toml) |

 ### Standard Files

`dracon-sync` auto-copies AGPL v3 LICENSE to every synced repository. This ensures all Dracon repos carry the same copyleft license. You own the copyright → you're the only one who can sell commercial licenses.

**AGPL v3 LICENSE is auto-copied during every sync cycle.** New repos always get it. Existing files are never overwritten.

```toml
standard_files = ["LICENSE"]
standard_files_auto = true
```

Per-repo opt-out via `.dracon/dracon-sync.toml`:
```toml
skip_standard_files = ["LICENSE"]
```

Templates live in `~/.dracon/utilities/sync/templates/`. Source path resolution: absolute paths used as-is, `~/` expanded to home directory, relative paths resolved from `~/.dracon/utilities/sync/`.

**Important:** In TOML, top-level fields like `standard_files` must appear BEFORE any section headers (`[...]` or `[[...]]`). If placed after a section header, they will be silently parsed as belonging to that section and ignored by the policy loader.

## Operational State

Operational state (mutable files written at runtime) lives **outside the `.dracon` git tree** to prevent self-referential churn:

```
~/.local/state/dracon/
├── dracon-sync-incidents.jsonl   # Append-only incident ledger
├── dracon-sync-stuck-push-repos.json  # Stuck push tracking
└── visibility-sync/              # Cache for visibility/metadata sync (per-repo timestamps)

The incident ledger is appended every sync cycle. Keeping it at `~/.local/state/dracon/` instead of inside `.dracon` prevents the sync daemon from auto-committing its own operational data.

### Startup Cleanup

On every daemon start/restart, the sync daemon prunes stale state:
- **Stuck repos**: Removes entries for repos no longer on disk, saves pruned result to JSON
- **Incident ledger**: Enforces retention (max age + max lines) immediately at start
- **Visibility cache**: Removes orphan `.last` files for deleted repos
- **Broken tracking**: Repairs `origin/master: gone` refs → `origin/{branch}` (also runs every ~5 min in the loop)
- **Stale index.lock**: Removes `.git/index.lock` files with no holding process (left by crashed git operations). Without this, a stale lock blocks all git operations in that repo and the daemon can never commit changes there.
- **Clone race guard (IndexLock)**: The true root cause was the **warden** — `publish_repo_pubkey()` writes `.pub` files to `.dracon/data/keys/` during `harden_repo()`. When triggered by filesystem events during `git clone`, these files appear before git's checkout phase, causing "Untracked working tree file would be overwritten by merge." The definitive fix uses git's own coordination protocol: **`IndexLock`** acquires `.git/index.lock` (same file git uses during checkout) before any working-tree writes. Uses `O_EXCL` (atomic create-new) — no TOCTOU race. If git holds the lock → warden/sync skip. If warden/sync hold it → git's checkout waits. This is exactly how git commands coordinate with each other. The old heuristics (grace period, HEAD check) are kept as defense-in-depth but the `IndexLock` is the primary coordination mechanism. Applied in both warden (`harden_repo` → `apply_overwrite_file` + `publish_repo_pubkey`) and sync (`ensure_standard_files`). The `once`/`repair` commands use `IndexLock::bypass()` since the user explicitly requested the operation.

### Daemon Reliability

**Push timeouts:** Default `push_op_timeout_secs=60` (was 300). A hanging mirror push (e.g. GitLab unreachable) blocks the entire daemon — no other repos get synced until it times out. With 3 mirrors at 300s each, a single repo could block the daemon for 15 minutes. 60s per push / 120s per repo keeps the daemon responsive.

**Filter-only cooldown:** Repos with clean/smudge filter changes (e.g. dracon-warden encryption) show as dirty in `git status` but have no diff after staging. The daemon detects this, resets the staging area, and applies a cooldown to prevent tight re-check loops.

**Fingerprint-based scheduling:** The daemon uses a fingerprint (branch + effective_dirty + staged + ahead + behind) to determine if a repo needs syncing. Only after the fingerprint stays stable for `inactivity_push_delay_secs` (default 5s) does the daemon attempt a sync. This prevents partial-change commits.

### Report Accuracy (repos command)

The `repos` command shows **real dirty file counts** from libgit2's `get_status()`, not filtered counts. The OK/WARN/CONCERN status uses `has_sync_relevant_dirty_entries()` (which excludes target/, node_modules/, oversized files, etc.), but the MOD/STG columns always show the actual number of modified/staged files. Previously, when `effective_dirty` was false (all changes excluded by policy), the report showed 0 — making repos with 30+ uncommitted files appear clean.

The guard daemon rotates its log file if oversized at startup.

### Untracked vs Modified Distinction

**Critical concept:** `git status` groups files into "Changes not staged" (modified tracked files) and "Untracked files" (files git doesn't know about). The sync daemon treats these very differently:

- **Modified tracked files** (`M` in git status): Real code changes. The daemon commits and pushes these.
- **Untracked files** (`??` in git status): Build artifacts (`target/`, `node_modules/`), caches, generated data. The daemon ignores these for sync purposes.

The `repos` command reflects this split:
- **MOD column** shows modified tracked files (real changes)
- **STG column** shows staged files
- **Untracked files** are counted separately and do NOT trigger WARN status
- The OK/WARN/CONCERN status only considers **sync-relevant** dirty entries (tracked modifications), not untracked build artifacts

### !target/ Policy

`.gitignore` is managed by `dracon-warden` (marked with `# --- BEGIN DRACON MANAGED BLOCK ---`). The warden manages a blocklist/allowlist pattern:

1. **Broad excludes**: `target/`, `node_modules/`, `build/`, `dist/`, `__pycache__/`, `*.log`, `*.db`, etc. are all excluded from tracking.
2. **Allowlist overrides**: `!*.rs`, `!*.py`, `!*.toml`, `!*.md`, `!Cargo.lock`, etc. force-track specific file types through the excludes.

**`target/` is NOT in the allowlist.** This means:
- `target/` directories are always untracked (never committed)
- They appear as `??` in `git status` but do NOT affect sync behavior
- `git clean -fdx` can safely delete them (they're build artifacts, not source)

**⚠️ Do NOT add `!target/` to .gitignore.** This would force-track build artifacts, bloating the repo and causing sync conflicts.

### Daemon-Managed Files Warning

**Do NOT edit the following files directly — they are managed by daemon processes:**

| File | Managed by | Risk of editing |
|------|-----------|-----------------|
| `.gitignore` (DRACON MANAGED BLOCK) | dracon-warden | Warden overwrites on next harden pass |
| `.gitattributes` (DRACON MANAGED BLOCK) | dracon-warden | Warden overwrites on next harden pass |
| `.dracon/data/keys/*.pub` | dracon-warden | Warden publishes on harden |
| `.pi/goals/*.md` | pi (auto-sync) | Sync daemon auto-commits active goal files |

Local task/session state directories (`.demon/`, `.ralph/`, `.sisyphus/`, and `.pi/goals/archived/`) may already be tracked from earlier sessions. Do not mass-untrack or delete them without user approval; if the repo should stop tracking future local state, first get explicit approval and back up any existing files.

If you need to modify a daemon-managed file, either:
1. Edit the **source template** in `~/.dracon/utilities/sync/templates/` or `~/.dracon/utilities/warden/`
2. Use the CLI command (`dracon-warden setup-hooks`, `dracon-sync scaffold`)
3. Edit directly but accept that the daemon may overwrite your changes

### GitHub Orphan Cleanup

The old suffix loop bug created 61 orphan repos (e.g., `dracon-code-1` through `dracon-code-11`). A cleanup script is provided:

```bash
# Dry run (list only)
./scripts/cleanup-github-orphans.sh

# Actually delete (requires delete_repo scope)
gh auth refresh -h github.com -s delete_repo
./scripts/cleanup-github-orphans.sh --apply
```

### Incident Response

When incidents or warnings occur, entries are written to the incident ledger:

```bash
# View recent incidents
cat ~/.local/state/dracon/dracon-sync-incidents.jsonl | tail -20
```

Each line is a JSON object:
```json
{"ts_unix":1714896000,"scope":"safety","repo":"/path/to/repo","reason":"description of what happened","action":"action_taken","backup_branch":null,"result":"result","details":"additional details"}
```

Common `scope` values: `safety` (safety guard triggers), `repair` (auto-repair), `sync` (sync operations), `mirror` (mirror push failures).

**After an incident:**
1. Read the incident ledger to understand what happened
2. Check the repo status: `git status` and `git log --oneline -5`
3. Take appropriate action based on the incident type
4. For intentional destructive operations: use `git add -A && git commit -m 'delete files'` directly

### dracon-system Protected Paths

`dracon-system` protects critical system directories from accidental deletion. The following are always protected (exact match):

`/`, `/home`, `/etc`, `/usr`, `/var`, `/boot`, `/nix`, `/run`, `/sys`, `/dev`, `/proc`

Protection uses ancestor matching: `/home` protects `/home/dracon`, `/home/dracon/Dev`, etc. Only `/` requires an exact match (since everything is a descendant of `/`).

You can add custom protected paths in `dracon-system.toml`:

```toml
[guard]
# Additional directories to protect from cleanup operations (storage --cleanup, empty_trash, etc.)
# Use absolute paths. Paths are canonicalized before comparison.
# protected_paths = ["/mnt/data", "/opt/important"]
```

Safety: most `remove_dir_all` call sites in `dracon-system` check the path against both system and user-protected paths before executing. The guard-specific `check_safe_to_delete_guard` skips SYSTEM_PROTECTED (only checks user-protected) because the guard only deletes known artifact/cache directories (target/, node_modules/, ~/.cache/, Trash) which are legitimately under /home. The `--apply` flag is required for destructive operations.

### dracon-system Process Monitoring & Logging

The guard monitors processes using >`process_cpu_percent`% CPU for >`process_sustain_secs` seconds. All heavy processes are logged to a persistent JSONL file regardless of duration.

**Persistent log file:** `~/.local/state/dracon/dracon-system-guard.log`
- Logs both `heavy-brief` (any spike) and `heavy-sustained` (after sustain threshold) events
- Auto-rotates when it exceeds `guard_log_max_mb`
- JSONL format: `{"ts":1234567890,"event":"heavy-brief","details":"pid=123 ppid=1 cmd=git args=git init cpu=61.7% ..."}`

**Graduated auto-renice:**
When `auto_renice = true`, heavy processes are reniced with a graduated nice value based on severity. Higher CPU/memory usage = higher nice value (lower priority). The process still gets full CPU when nothing else needs it — it just yields to the DE and other interactive processes.

**⚠️ CRITICAL INVARIANT: The guard NEVER kills processes — it only renices.** Killing is explicitly banned. If a process needs to be stopped, that must be done manually or by its own service manager. The guard's only process management action is `renice`.

| CPU usage | Nice value | Effect |
|-----------|-----------|--------|
| >= 180% | 5 | Gentle deprio |
| >= 300% | 10 | Moderate deprio |
| >= 500% | 15 | Strong deprio |
| RSS >= 4GB | 5 | Memory hog deprio |
| RSS >= 8GB | 10 | Heavy memory deprio |

When a process is no longer heavy for `release_after_secs` (default 120s), it is un-reniced back to nice 0.

```toml
[guard]
auto_renice = true
renice_value = 5                    # Base nice value (used if no tier matches higher)
release_after_secs = 120             # Un-renice after 2 min of being non-heavy
```

**Log configuration:**
```toml
[guard]
guard_log_file = "~/.local/state/dracon/dracon-system-guard.log"
guard_log_max_mb = 1            # Rotate at 1 MiB
```

### dracon-system Proactive Cleanup

The guard can clean stale Rust target directories **before** disk reaches action/critical levels. This prevents disk pressure from building up in the first place.

**How it works:**
- When disk usage is above `proactive_cleanup_percent` (default 50%) but below `disk_action_percent`, the guard runs a lightweight cleanup every N cycles
- Only target dirs older than `rust_target_max_age_days` (default 14) are removed — recently-used build artifacts are preserved
- Active builds (running cargo/rustc) are always protected
- Full aggressive cleanup (all targets regardless of age) still only triggers at `disk_action_percent`/`disk_critical_percent`

```toml
[guard]
proactive_cleanup_percent = 50        # Start proactive cleanup at 50% disk
rust_target_max_age_days = 14          # Only remove targets untouched for 14+ days
proactive_cleanup_interval_cycles = 120 # Run every 120 guard cycles (~1h at 30s interval)
```

**Throttling:** Proactive cleanup also requires at least `interval_cycles × interval_secs` seconds since the last proactive pass, preventing redundant scans even if cycles run fast.

### dracon-sync Repo Discovery

Repo discovery searches up to **4 levels deep** from each watch root. Dot-prefixed directories (e.g. `.config/`, `.dracon/`) are descended into if they contain a `.git` directory — only skipped after the `.git` check fails. The hardcoded exclusions are `objects` and whatever is in `exclude_dir_names` from policy.

### dracon-sync Push Behavior

Push operations use `push_with_retries` with SSH hardening (`ConnectTimeout`, `ConnectionAttempts`) and automatic HTTPS fallback on persistent timeout. The `push_retries` policy setting is respected. All transient network failures should now trigger retries rather than failing immediately.

### dracon-sync Merge Strategy

dracon-sync uses `git pull --no-rebase` (merge) instead of `git pull --rebase`. This preserves both local and remote histories without rewriting commits. Benefits:

- **Less likely to conflict**: Merge handles parallel commits gracefully; rebase fails if the same lines were modified
- **No history rewriting**: Commits are not rebased, so there's no risk of losing commits if the rebase is aborted
- **Clear history**: Merge commits clearly show where branches diverged and merged

When `auto_pull = true` and a repo is behind upstream, sync will create a merge commit rather than rebasing. This prevents the "rebase-abort causes true divergence" scenario.

### dracon-sync Automatic Remote Creation

When `auto_github_private = true` in `dracon-sync.toml`, any repo in a watched root without an origin remote will automatically get:

1. A private GitHub repo created via `gh repo create --private`
2. HTTPS remote added: `https://github.com/<account>/<repo>.git`
3. Initial commit pushed: `git push -u origin HEAD`

Requirements: `gh` CLI installed and authenticated (`gh auth status`).

```toml
[sync]
auto_github_private = true
auto_github_private_account = "YourOrgOrUsername"
```

**⚠️ CRITICAL: NEVER create suffixed repos (repo-1, repo-2, repo-N).**
If the GitHub repo already exists, reuse it. A previous suffix loop in `create_github_private_remote` created 15+ orphan repos (`dracon-demons-1` through `-9`). This happens when `gh repo create` fails with "Name already exists" and the code appends `-1`, `-2` instead of just reusing the existing repo. This pattern is explicitly banned in all repo creation functions.

### Per-Remote Repo Name Mapping

Some platforms (GitLab, Forgejo) reject dots in project names. The `.dracon` repo (dot-prefixed) would fail on GitLab. Use `repo_name_map` to map local directory names to remote project names:

```toml
[[remotes]]
name = "gitlab"
push_url = "git@gitlab.com:myorg/{repo}.git"
auto_create = true
[remotes.repo_name_map]
".dracon" = "dracon-home"
```

This maps local `.dracon` → `dracon-home` on GitLab while keeping `.dracon` on GitHub/Codeberg.

### Codeberg/Forgejo Limitation

**Push-to-create is disabled** for Codeberg because Forgejo (the underlying software on Codeberg.org) does not allow `git push` to create new repos. You must manually create repos on Codeberg first, or enable push-to-create in Forgejo settings. Set `auto_create = false` for the Codeberg remote (the default).

### Webhook Notifications

On push failures (origin or mirror remotes), `dracon-sync` can send a fire-and-forget HTTP POST to a configured webhook URL:

```toml
webhook_url = "https://your-webhook-endpoint.example/notify"
```

Payload:
```json
{
  "event": "push_failure",
  "repo": "/path/to/repo",
  "remote": "origin",
  "error": "connection timeout after 300s",
  "timestamp": 1714896000
}
```

The request runs in a background thread with a 5s timeout — webhook failures do not block sync operations.

### Mirror Visibility & Metadata Sync

When enabled, `dracon-sync` automatically mirrors GitHub's public/private status and repository metadata (description, topics/tags) to GitLab and Codeberg mirrors.

```toml
# dracon-sync.toml
sync_visibility = true               # Mirror GitHub visibility to Codeberg/GitLab
sync_metadata = true                 # Mirror description and topics/tags
sync_visibility_interval_hours = 24  # Check at most once per day per repo
```

**How it works:**
- Visibility and metadata are queried from GitHub via `gh api`
- Mirrors are updated via their REST APIs (GitLab: `PRIVATE-TOKEN`, Codeberg: `Authorization: token`)
- Timestamp-gated cache in `~/.local/state/dracon/visibility-sync/` prevents API overuse
- `auth_type` is auto-detected from push URL (GitLab/Codeberg URLs are recognized)
- Missing tokens for a mirror skip that mirror gracefully

**At creation time:** If `sync_visibility = true`, new mirror repos inherit GitHub's visibility. If `false` (default), all mirrors are created as private.

### Release Pipeline (Tags, Releases, Publishing)

After a version bump, `dracon-sync` can automatically create git tags, GitHub Releases, publish to package registries, and update Nix flake versions via PR. Four separate toggles control each step:

| Toggle | Default | Risk | Reversible? |
|--------|---------|------|-------------|
| `auto_tag` | `true` | Low | Yes (`git tag -d`) |
| `auto_release` | `false` | Medium | Yes (`gh release delete`) |
| `auto_publish` | `[]` | High | **No** (registries are immutable) |
| `nix_auto_update` | `false` | Low | Yes (close PR) |

**Per-repo opt-in:** Tags, releases, publishing, and Nix flake PRs require a `.dracon/dracon-sync.toml` in the repo:

```toml
# .dracon/dracon-sync.toml
auto_tag = true              # default: on
auto_release = true          # default: off — creates GitHub Release on major bumps
auto_publish = ["crates-io"] # default: empty = no publishing
nix_auto_update = true       # default: off — creates PR updating flake.nix version
```

**Global publish targets** are configured in the main `dracon-sync.toml`:

```toml
auto_publish = false  # master toggle (default: off)

[[publish_targets]]
name = "crates-io"
registry = "crates-io"    # crates-io | npm | pypi
[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBTWlZpZjJYeitpbUJSUHZORlVwbHc3KzRNWU05RHlJcjZPKzdLZ1FSL1dRCkxud3Y2MmtSbnpNZS93QnV4L1FJMzNPczV1bXBhZ2c5ZzRpSUtDZFMzeWcKLT4gWDI1NTE5IFN5azdMOTFybTZCTHVHK2RVNUQrWUhJYUU3NHB5VzBkMDJaQlQyN0Z5M0UKcHcyLy9ZRkNocE1UeXBwYm1MQVV5dyt4ZUFqTllQdkd1d1RlejMySm43UQotPiBYMjU1MTkgQUU3M2pBTVZ6c3VVUFFRS3l0Q28zUWF2eEhDKzBMaXAxYUhXTldYdGMxZwpjMTVOZERraEdTYURQWGF2VHB0KzhPN3hocGlidVp2cHhsMnordjdqa01vCi0+IFgyNTUxOSBhUHZiWVpFTUU2UGJPdUt2cFJKQ3NVS1k4YlBhSkxMUWNCOEw1VnIyd1JZCk95ZEZ0d2RsMnRTUVArcXd3cElVajRVbFJUT3orUk9RUFN6RjM0blJlN00KLT4gWDI1NTE5IEp4WHU2dUE2VkttSWZZc2RvNHdKR1NmZ3JDK1FCeCt2WFR2WFIrS1RyMTQKK1RuS2tyZWY1QTZ1aG9INkFsK3p3Sm5QLzQvamg2WHA4ek9ST0Uwd2JpRQotPiBYMjU1MTkgOEd2eVBVM2dsT29nK0VaWlRmU0RTNjN5cmRGVjZMNE1td3EyRUNkWGNUNApDTUQ3TFFZbFpTd3A0VnExd1EyNmh0WGRRMnh4NGFONjdXUGxRWkNhbjNzCi0+IFd3Siw3MVt5LWdyZWFzZSB8PltOaFtAVQpMamhHVTFlS0hNL0NTVG0rcHZmbGxCQVFEQ1dDTUpnTVh4dGxZcTZDZTZLUEZEa1YxUFB1WUoxVVM5Y3lnY3BYCkJKaU5HNHVQa0YzMElabzlqRHdldDdZQXJKTEdyU1BjdFZudWg5N1AzKzAxNXBaUHdNUXFSSWMKLS0tIHA1TUllRklJa2pFeWlPV0tkbmQwWGdoRDBNdGIwcUZKUk1ENXlVYzJGaGMK5vyIggevDuRdpuKUOPs6Eyhfqx+VDbN8ySNKDTJbJiPbfSAUQ0zyMBnTxfjfZtb14BxHVj3zOoABnhng02Q0zcAF/Pe8]
publish_timeout_secs = 300
```

**Safety:** Dry-run publish (`cargo publish --dry-run`, `npm publish --dry-run`) runs before real publish. Registry pre-check skips already-published versions. Publish failures log incidents but don't break the sync cycle.

## CLI Reference

All binaries support `-V, --version` and `-v, --verbose` (repeatable up to 2x for `-vv`).

### dracon-sync

```
dracon-sync [OPTIONS] <COMMAND>
Commands:
  status    Show resolved policy path and sync scope
  repos     One-off report across discovered repositories
  health    Check daemon health (policy valid, daemon responsive, repos healthy)
  metrics   Print Prometheus-style metrics
  once      Run one sync pass
  daemon    Run continuous sync loop
  sync-now  Sync one or more repositories now
  pause     Pause sync (creates freeze marker)
  resume    Resume sync (removes freeze marker)
  config    Manage sync configuration
  repair    Repair and manage repositories (concerns, warns, origins, stuck repos, dual-branch)
  publish   Publish to package registries and check publish status
  scaffold  Scaffold standard files (LICENSE) into repositories
```

**Subcommands of `config`:**
- `dracon-sync config edit` — open sync policy in the system editor
- `dracon-sync config validate` — validate the sync policy for errors and warnings

**Subcommands of `repair` (all dry-run by default; pass `--apply` to execute):**
- `dracon-sync repair concerns` — repair concern repos
- `dracon-sync repair warns` — repair warn repos (dirty-only triage)
- `dracon-sync repair origins` — detect and repair origin URLs pointing to orphan `-N` suffixed repos
- `dracon-sync repair stuck-list` — list repos that are permanently stuck on push
- `dracon-sync repair stuck-unstuck <repo>` — unstuck a specific repo
- `dracon-sync repair dual-branch-list` — list repos with dual main/master
- `dracon-sync repair dual-branch-repair <repo>` — consolidate to main

**Subcommands of `publish`:**
- `dracon-sync publish run <repo>` — publish a repo to configured registries
- `dracon-sync publish status <repo>` — show publish status across configured registries

**Subcommands of `scaffold`:**
- `dracon-sync scaffold` — scaffold into all discovered repos (or `--repo <path>`)
- Options: `--files <NAMES>`, `--overwrite`, `--dry-run`

**Global flags:** `-v` / `-vv` increase verbosity; `-V` prints version.

### Safety Behaviors

**Primary safety mechanism:** `IndexLock` (`.git/index.lock` coordination) prevents sync/warden from writing working-tree files while git's checkout is in progress. Git's own revert capability serves as the safety net for any committed changes.

**Removing a large number of files intentionally:** Use `git add -A && git commit -m 'delete files'` directly — no daemon involvement needed.

**Incident response after a block:** Read the incident ledger at `~/.local/state/dracon/dracon-sync-incidents.jsonl` to understand what was blocked and why.

**dracon-sync commit message generation:** Commit messages are simple mechanical facts (e.g., "update 3 file(s)") extracted from the diff. No AI, no LLM, no prose.

### dracon-system

```
dracon-system [OPTIONS] <COMMAND>
Commands:
  status    Show core path and service status
  doctor    Run deterministic diagnostics for canonical dracon setup
  events    Show recent events from the shared event stream
  storage   Analyze storage hotspots and optionally clean safe build/cache dirs
  link      Manage deterministic symlink ownership for system setup
  symlinks  Scan filesystem for broken symlinks (report-only)
  zram      Zram management: show stats and generate NixOS config for tuning
  guard     Guard runtime: monitor disk/process pressure and notify/mitigate
```

**Subcommands of `link`:**
- `dracon-system link status` — show link reconciliation status from policy
- `dracon-system link doctor` — diagnose link drift and invalid targets
- `dracon-system link apply` — apply link policy by creating/fixing symlinks

**Subcommands of `guard` (destructive ones default to dry-run; pass `--apply` to execute):**
- `dracon-system guard once` — run one guard evaluation pass
- `dracon-system guard daemon` — run continuous guard loop (systemd)
- `dracon-system guard prune` — prune system caches and Docker resources
- `dracon-system guard clean` — clean all reclaimable space (targets, trash, nix, caches, node_modules)

**Subcommands of `zram`:**
- `dracon-system zram --status` — show current zram stats
- `dracon-system zram --gen-config` — generate NixOS config for tuning

### dracon-warden

```
dracon-warden [OPTIONS] <COMMAND>
Commands:
  status         Show resolved policy path and watch roots
  once           Run one hardening pass and exit
  scrub-markers  Scan plaintext JSON files for DRACON_SECRET markers and optionally scrub them
  resmudge       Fix working-tree files that are still ciphertext (contain DRACON_SECRET markers)
  repair         System-wide repair pass for secret-related corruption
  filter-clean   Git filter clean operation (stdin -> stdout). Called by git, not for direct use
  filter-smudge  Git filter smudge operation (stdin -> stdout). Called by git, not for direct use
  keygen         Generate a new age keypair for this machine
  setup-hooks    Install git hooks globally for warden encryption enforcement
```

**Git hooks (installed by `setup-hooks`):**
- `pre-commit`: Blocks commits if warden filter is not configured
- `pre-push`: Scans for plaintext secrets as defense-in-depth (catches `--no-verify` bypass)

## Tokens & Secrets

All secrets are stored in `~/.dracon/utilities/sync/secrets/*.env` (sync). See the secrets directory
README for the full inventory and creation instructions.

| Token | File | Purpose | Source |
|-------|------|---------|--------|
| `GITLAB_TOKEN` | `gitlab.env` | HTTPS push fallback, repo creation, visibility/metadata sync | https://gitlab.com/-/profile/personal_access_tokens |
| `CODEBERG_TOKEN` | `codeberg.env` | HTTPS push fallback, repo creation, visibility/metadata sync | https://codeberg.org/user/settings/applications |
| `GH_TOKEN` | env or `gh auth` | GitHub repo creation, visibility queries, GitHub Releases | https://github.com/settings/tokens or `gh auth login` |
| `CARGO_REGISTRY_TOKEN` | user creates | Publish to crates.io | https://crates.io/settings/tokens |
| `NPM_TOKEN` | user creates | Publish to npm | https://www.npmjs.com/settings/tokens/create (Automation type) |
| `TWINE_PASSWORD` | user creates | Publish to PyPI | https://pypi.org/manage/account/token/ |

**Token resolution**: `load_secret("NAME")` checks env var first, then scans
`*.env` files in the secrets directory. Missing tokens are skipped gracefully.

## Commit Messages

Commit messages are **deterministic facts extracted from the diff**. No AI, no LLMs, no prose.

### Core Principle: No AI-Generated Messages

- **No LLM at the commit boundary** — zero AI calls when generating commit messages
- **No inference, no guessing** — just regex on git diff output
- **No prose** — structured key-value pairs only
- **AI reads the diff, not the message** — the message is just an INDEX for searching

### What Gets Extracted

From the diff, we deterministically extract:

1. **Task state transitions** (from any file — markdown, text)
   - `[x]` → `CLOSED: task name`
   - `[~]` → `WIP: task name`
   - Works in: `- [x]`, `* [x]`, `[x]` (markdown/text only, not code comments)
   - Task names are compacted to short route-key fragments; explanatory clauses after `:`, `;`, `—`, or `–` are dropped to avoid prose-like subjects

2. **Blast radius** (from `git diff --numstat`)
   - `FILES:N` — total files changed
   - `DIRS:X,Y` — top-level directories (scope)
   - `[file1, file2]` — top 3 changed files (searchable)
   - `DELTA:+A/-B` — lines added/removed

3. **Metrics** (also from diff)
   - `TEST:T` — lines changed in test files
   - `BIN:B` — binary files changed
   - `NEW:file1,file2` — newly created files (searchable)
   - `DEL:file1,file2` — deleted files (searchable)
   - `DEPS:+reqwest,-serde` — specific dependencies added/removed
   - `MERGE:` — merge commit prefix; merge commits start with `MERGE: | ...`
   - `REVERT:` — revert commit prefix; revert commits start with `REVERT: | ...`
   - `TAG:v1.2.0` — this commit is tagged (release milestone)
   - `TESTONLY:` — all changed files are test files (no production code)
   - `ENV:` — env files changed (`.env`, `.envrc`, etc.)

### Commit Format

```
[INTENT] | N file(s) in DIRS [files] DELTA:+A/-B [METRICS]
```

**Examples:**

```bash
# Task completed + test code written
CLOSED: Implement JWT | 3 file(s) in src [auth.py, jwt.py] DELTA:+140/-12 | TEST:45

# Task in progress (partial work)
WIP: Refactor DB pool | 2 file(s) in src [db.py] DELTA:+50/-10

# New files added
CLOSED: Add auth module | 5 file(s) in src [auth.py, jwt.py] DELTA:+200/-0 | NEW:src/auth.py,src/jwt.py TEST:80

# Dependency change (security audit signal)
2 file(s) in . [Cargo.toml, Cargo.lock] DELTA:+50/-10 | DEPS:+reqwest,-hyper

# Binary file added (context window warning for AI)
1 file(s) in assets [logo.png] DELTA:+0/-0 | BIN:1

# Files deleted (refactoring)
3 file(s) in src [old.py, legacy.py] DELTA:+0/-150 | DEL:src/old.py,src/legacy.py

# Release commit (tagged)
CLOSED: Release v1.0.0 | 10 file(s) in src [lib.rs] DELTA:+500/-100 | TAG:v1.0.0

# Merge commit
MERGE: | 5 file(s) in src [auth.py, db.py] DELTA:+200/-50

# Test-only commit (no production code changes)
CLOSED: Add auth tests | 2 file(s) in src [auth_test.py] DELTA:+100/-0 | TEST:100 TESTONLY:

# Env file changed (security audit)
1 file(s) in . [.env.example] DELTA:+5/-2 | ENV:
```

### How AI Searches This

```bash
# Find when a specific task was completed
git log --grep="JWT"

# Find all paused/in-progress work
git log --grep="WIP:"

# Find commits touching auth code
git log --grep="DIRS:auth"

# Find commits with binary files (skip in context window)
git log --grep="BIN:"

# Find commits where new files were added
git log --grep="NEW:"

# Find commits where files were deleted
git log --grep="DEL:"

# Find commits with dependency changes (security audit)
git log --grep="DEPS:"

# Find merge commits
git log --grep="MERGE:"

# Find revert commits
git log --grep="REVERT:"

# Find release commits
git log --grep="TAG:"

# Find test-only commits (no production code)
git log --grep="TESTONLY:"

# Find commits with env file changes
git log --grep="ENV:"

### What This Is NOT

- NOT AI-scribed messages (removed — they were useless)
- NOT conventional commits (`feat:`, `fix:`) — human bias
- NOT natural language summaries — AI reads the diff

## Environment Variables

### dracon-sync

| Variable | Purpose | Example |
|----------|---------|---------|
| `DRACON_SYNC_GIT_BIN` | Override path to git binary (checked every call, not cached) | `/run/current-system/sw/bin/git` |

### dracon-system

### Test Environment

Use scoped environment guards when a test mutates process environment:

- `dracon-sync`: `crate::test_helpers::EnvRestorer`.
- `dracon-security` integration tests: `crate::tests::common::EnvRestorer`.
- `dracon-system`: no shared env-mutating helper is needed because system tests do not mutate global process environment.

```rust
use crate::test_helpers::EnvRestorer;

// Set an env var (restored on drop)
let _guard = EnvRestorer::new("VAR_NAME", "value");

// Remove an env var (restored on drop)
let _guard = EnvRestorer::remove("VAR_NAME");
```

## Dependency Hygiene

- Per-crate manifests use `workspace = true` for shared dependency versions declared in root `Cargo.toml`.
- `cargo deny check` is the dependency-policy gate. `deny.toml` documents unavoidable transitive duplicate-version exceptions from the age/i18n-embed encryption stack, proptest/tempfile/uuid test stack, zbus/notify-rust desktop-notification stack, and toml parser stack.
- `cargo tree -d` may still show transitive duplicate versions after `cargo deny check` passes. Do not force-align transitive crates unless a compatible direct dependency upgrade removes the duplicate without changing behavior.

## Testing

### dracon-sync

**431 tests** across 2 suites (`cargo test -p dracon-sync -- --test-threads=1`). Tests use `tempfile::TempDir` for isolation.

Whole-workspace: **692 passed, 6 ignored** across 22 suites (`cargo test --workspace -- --test-threads=1`). Per-crate latest counts:

- `dracon-sync`: 431 passed, 2 suites.
- `dracon-system`: 83 passed, 1 suite.
- `dracon-warden`: 79 passed, 2 suites.
- `dracon-security` (`dracon-warden/src/security`): 99 passed, 6 ignored, 17 suites.
- `dracon-ai` standalone: 7 passed, 1 suite (`cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1`).

```bash
export DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git

# Reliable (serial execution — no flaky race conditions):
cargo test --workspace -- --test-threads=1

# Fast but may have flaky failures from shared global state:
cargo test --workspace
```

**Known parallel-test issues:** some tests can fail unpredictably when running with default parallelism. Root causes:
1. `std::process::Command::new("git")` resolves from `PATH`, which concurrent tests modify for mock binaries
2. `acquire_path_lock()` only serializes the subset of tests that explicitly acquire it
3. Some sync tests start TCP listeners on fixed ports for mock registries

**Env var hygiene:** `dracon-sync` uses `crate::test_helpers::EnvRestorer`; `dracon-security` integration tests use `crate::tests::common::EnvRestorer`. Use the scoped guard for any env mutation, or avoid mutating process environment when possible. The guard restores on drop.

**Key env vars:**
- `DRACON_SYNC_GIT_BIN` — overrides git binary path (checked every call, not cached)
- `PATH` — mutations require `acquire_path_lock()` first

### dracon-system & dracon-warden

```bash
cargo test -p dracon-system
cargo test -p dracon-warden
```

---

## Related Documentation

- [README.md](README.md) — User-facing quick start and usage guide
- [CONTRIBUTING.md](CONTRIBUTING.md) — Development workflow and contribution guidelines
- [CHANGELOG.md](CHANGELOG.md) — Version history and release notes

