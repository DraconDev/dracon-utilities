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
9. [AI Configuration](#ai-configuration)
10. [CLI Reference](#cli-reference)
11. [Environment Variables](#environment-variables)
12. [The Scribe](#the-scribe-ai-working-memory)
13. [Testing](#testing)

## Architecture

```
dracon-utilities/           <- CLI binaries (this repo)
├── dracon-sync/            -> ~/.local/bin/dracon-sync
├── dracon-system/          -> ~/.local/bin/dracon-system
└── dracon-warden/          -> ~/.local/bin/dracon-warden

dracon-libs/                <- Shared libraries (REQUIRED for building)
├── services/ai/            <- AI adapters, router, lanes
└── tools/sync/dracon-git/  <- git operations library
```

**Key point:** `dracon-utilities` contains the CLI wrappers. `dracon-libs` contains shared library code. Only the CLI binaries get installed.

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
2. AI reads `dracon-utilities/.dracon/project-state.md`
3. AI makes changes
4. Sync daemon auto-commits and pushes
5. Done

**What sync provides:**
- Auto-commit on every change (AI doesn't need to think about git)
- project-state.md as the AI's working memory (context survives sessions)
- Incident ledger for debugging (AI can read what went wrong)
- Freezing for pause (AI can pause sync during delicate operations)

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
| dracon-warden.service | dracon-warden daemon | Security hardening |

```bash
# Restart after install (install.sh does this automatically)
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
systemctl --user restart dracon-warden.service
```

## Policy Files

| Utility | Policy Path | Example Config |
|---------|-------------|----------------|
| dracon-sync | ~/.dracon/utilities/sync/dracon-sync.toml | [dracon-sync.example.toml](dracon-sync/dracon-sync.example.toml) |
| dracon-system | ~/.dracon/utilities/system/dracon-system.toml | [dracon-system.example.toml](dracon-system/dracon-system.example.toml) |
| dracon-warden | ~/.dracon/utilities/warden/dracon-warden.toml | [dracon-warden.example.toml](dracon-warden/dracon-warden.example.toml) |

## Operational State

Operational state (mutable files written at runtime) lives **outside the `.dracon` git tree** to prevent self-referential churn:

```
~/.local/state/dracon/
├── dracon-sync-incidents.jsonl   # Append-only incident ledger (2MB, 10k+ lines)
└── dracon-sync-stuck-push-repos.json  # Stuck push tracking
```

The incident ledger is appended every sync cycle. Keeping it at `~/.local/state/dracon/` instead of inside `.dracon` prevents the sync daemon from auto-committing its own operational data.

### dracon-system Protected Paths

`dracon-system` protects critical system directories from accidental deletion. The following are always protected (exact match):

`/`, `/home`, `/etc`, `/usr`, `/var`, `/boot`, `/nix`, `/run`, `/sys`, `/dev`, `/proc`

You can add custom protected paths in `dracon-system.toml`:

```toml
[guard]
# Additional directories to protect from cleanup operations (storage --cleanup, empty_trash, etc.)
# Use absolute paths. Paths are canonicalized before comparison.
# protected_paths = ["/mnt/data", "/opt/important"]
```

Safety: every `remove_dir_all` call site in `dracon-system` checks the path against both system and user-protected paths before executing. The `--apply` flag is required for destructive operations.

### dracon-system Process Monitoring & Logging

The guard monitors processes using >`process_cpu_percent`% CPU for >`process_sustain_secs` seconds. All heavy processes are logged to a persistent JSONL file regardless of duration.

**Persistent log file:** `~/.local/state/dracon/dracon-system-guard.log`
- Logs both `heavy-brief` (any spike) and `heavy-sustained` (after sustain threshold) events
- Auto-rotates when it exceeds `guard_log_max_mb`
- JSONL format: `{"ts":1234567890,"event":"heavy-brief","details":"pid=123 ppid=1 cmd=git args=git init cpu=61.7% ..."}`

**Auto-kill runaway git processes:**
```toml
[guard]
auto_kill_git = false           # Enable to auto-kill git processes
git_kill_threshold_secs = 60    # Kill after 60s of high CPU
```

When enabled, git processes (init, fetch, pull, clone, push) that sustain high CPU for the configured duration receive SIGTERM, then SIGKILL after 5 seconds if still alive. Disabled by default for safety.

**Log configuration:**
```toml
[guard]
guard_log_file = "~/.local/state/dracon/dracon-system-guard.log"
guard_log_max_mb = 1            # Rotate at 1 MiB
```

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
2. SSH remote added: `git@github.com:<account>/<repo>.git`
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

## CLI Reference

All binaries support `-V, --version` and `-v, --verbose` (repeatable up to 2x for `-vv`).

### dracon-sync

```
dracon-sync [OPTIONS] <COMMAND>
Commands:
  status           Show resolved policy path and sync scope
  validate-config  Validate the sync policy for errors and warnings
  repos            One-off report across discovered repositories
  repair-concerns  Repair concern repos (dry-run by default; use --apply to execute)
  repair-warns     Repair warn repos (dirty-only triage; dry-run by default)
  once             Run one sync pass
  daemon           Run continuous sync loop [--interval-secs override]
  sync-now         Sync one or more repositories now [--dry-run] [repos...]
  pause            Pause sync (creates freeze marker)
  resume           Resume sync (removes freeze marker)
  edit-config      Open sync policy in the system editor
  test-ai          Test AI providers connectivity
  health           Check daemon health [--json]
  metrics          Print Prometheus-style metrics
  stuck            Manage repos permanently stuck on push
  dual-branch      Manage repos with both main and master branches
  repair-origins   Detect and repair orphan origin URLs
```

**Nested subcommands:**
- `dracon-sync stuck list` — list stuck repos
- `dracon-sync stuck unstuck <repo>` — unstuck a specific repo
- `dracon-sync dual-branch list` — list repos with dual main/master
- `dracon-sync dual-branch repair <repo>` — consolidate to master

**Global flags:** `-v` / `-vv` increase verbosity; `-V` prints version.

### dracon-system

```
dracon-system [OPTIONS] <COMMAND>
Commands:
  status   Show core path and service status
  doctor   Run deterministic diagnostics
  storage  Analyze storage hotspots [--cleanup] [--apply]
  link     Manage symlink ownership (status | doctor | apply)
  guard    Guard runtime (once | daemon | prune | clean)
  events   Show recent events [-t N] [-s source] [-s severity]
  zram     Zram stats [--status] [--gen-config]
```

### dracon-warden

```
dracon-warden [OPTIONS] <COMMAND>
Commands:
  daemon         Run forever with filesystem event debounce
  once           Run one hardening pass [repo]
  status         Show resolved policy path and watch roots
  filter-clean   Git filter clean (stdin -> stdout)
  filter-smudge  Git filter smudge (stdin -> stdout)
  scrub-markers   Scan DRACON_SECRET markers [--apply] [repo]
  resmudge       Fix ciphertext stuck in working tree [--apply] [repo]
  repair         System-wide repair pass [--dry-run] [--strict] [repo]
  keygen         Generate new age keypair
```

## AI Configuration

dracon-sync uses AI for commit messages (scribe) and version bumping. Configure providers in `~/.dracon/utilities/sync/ai.toml`:

```toml
[[providers]]
name = "mistral"
env = "MISTRAL_API_KEY"
endpoint = "https://codestral.mistral.ai/v1"
model = "codestral-latest"

[[providers]]
name = "nvidia"
env = "NVIDIA_API_KEY"
endpoint = "https://integrate.api.nvidia.com/v1"
model = "stepfun-ai/step-3.5-flash"

[[providers]]
name = "openrouter"
env = "OPENROUTER_API_KEY"
endpoint = "https://openrouter.ai/api/v1"
model = "nvidia/nemotron-3-super-120b-a12b:free"

[[providers]]
name = "openrouter"
env = "OPENROUTER_API_KEY"
endpoint = "https://openrouter.ai/api/v1"
model = "openrouter/free"
```

### API Keys

Store keys in `~/.dracon/utilities/sync/ai/secrets/*.env`:
- `mistral.env` → `MISTRAL_API_KEY=...`
- `nvidia.env` → `NVIDIA_API_KEY=...`
- `openrouter.env` → `OPENROUTER_API_KEY=...`

### Test AI Providers

```bash
dracon-sync test-ai
```

## The Scribe: AI Working Memory

The scribe is **you** (AI). The daemon maintains `.dracon/project-state.md` in each repo. This file is the **primary interface** between sync and the AI coder.

### Why Frequent Commits?

Sync commits every change because:
- The AI reads git history to understand past work
- Every commit is a checkpoint the AI can recover to
- More commits = better context for the AI's "what was I doing?"
- Commits are cheap; context is valuable

### Format

```markdown
# Project State

## Current Focus
{one line: what you're working on right now}

## Context
{why: what problem are you solving? what prompted this change?}

## Completed
- [x] {what you finished, with context}

## In Progress
- [x] {what you're actively working on}

## Blockers
- {what's stopping progress: missing info, user decision needed, dependency}

## Next Steps
1. {immediate next action}
2. {what comes after}
```

### Rules

- **Current Focus** must be one line — it becomes the commit body
- **Context** helps the AI recover understanding after time away
- **Blockers** tell the AI what it can't proceed on
- **Next Steps** give the AI a clear path forward
- Be specific: "Fix TOCTOU race in warden keygen" not "fix bugs"
- Don't document mechanical changes — only semantic state
- If the file doesn't exist, create it when you have something meaningful to say

### Example

```markdown
# Project State

## Current Focus
Refactor incident ledger to XDG state directory

## Context
The 2MB incident ledger was inside the .dracon git repo, causing
self-referential churn. Every sync cycle added to the ledger,
which dirtied the repo, which triggered another commit. Moved
the ledger to ~/.local/state/dracon/ to break the cycle.

## Completed
- [x] Moved incident_ledger_path() from ~/.dracon/ to ~/.local/state/dracon/
- [x] Moved stuck_repos_path() to XDG-compliant location
- [x] 248 tests passing after path changes

## Blockers
- None

## Next Steps
1. Monitor for 24h to confirm no self-referential churn
2. Continue reviewing dracon-system for any orphaned state files
```

## Environment Variables

### dracon-sync

| Variable | Purpose | Example |
|----------|---------|---------|
| `DRACON_SYNC_GIT_BIN` | Override path to git binary (checked every call, not cached) | `/run/current-system/sw/bin/git` |

### dracon-system

| Variable | Purpose | Example |
|----------|---------|---------|
| `DRACON_AI_CONFIG` | Override dracon-ai config file path | `~/.dracon/utilities/ai/dracon-ai.toml` |
| `DRACON_AI_APPLY` | Set to `0` for plan-only mode (don't execute commands) | `0` |
| `DRACON_AI_DANGEROUS` | Set to `1` to allow dangerous commands (use with caution) | `1` |
| `DRACON_AI_ALLOW_CMD` | Set to `1` to enable `/cmd` tool execution in REPL | `1` |

### Test Environment

All env var mutations in tests should use `EnvRestorer` (from `crate::test_helpers::EnvRestorer`) to prevent leakage between tests.

```rust
use crate::test_helpers::EnvRestorer;

// Set an env var (restored on drop)
let _guard = EnvRestorer::new("VAR_NAME", "value");

// Remove an env var (restored on drop)
let _guard = EnvRestorer::remove("VAR_NAME");
```

## Testing

### dracon-sync

**349 tests** in `src/` (git.rs, sync.rs, report.rs, policy.rs, main.rs). Tests use `tempfile::TempDir` for isolation.

```bash
export DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git

# Reliable (serial execution — no flaky race conditions):
cargo test -- --test-threads=1

# Fast but may have ~10-20 flaky failures from shared global state:
cargo test
```

**Known parallel-test issues:** ~10-20 tests fail unpredictably when running with default parallelism. Root causes:
1. `std::process::Command::new("git")` resolves from `PATH`, which concurrent tests modify for mock binaries
2. `acquire_path_lock()` only serializes the subset of tests that explicitly acquire it
3. Some sync tests start TCP listeners on fixed ports for mock registries

**Env var hygiene:** All env var mutations in tests should use `EnvRestorer` (from `crate::test_helpers::EnvRestorer`) to prevent leakage between tests. Use `EnvRestorer::new("VAR", "value")` to set, or `EnvRestorer::remove("VAR")` to clear. The guard restores on drop.

**Key env vars:**
- `DRACON_SYNC_GIT_BIN` — overrides git binary path (checked every call, not cached)
- `PATH` — mutations require `acquire_path_lock()` first

### dracon-system & dracon-warden

```bash
cargo test -p dracon-system
cargo test -p dracon-warden
```
