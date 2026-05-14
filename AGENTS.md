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
12. [The Scribe](#the-scribe-ai-commit-message-generator)
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
2. AI reads `dracon-utilities/.dracon/project-state.md` (if present, for manual context)
3. AI makes changes
4. Sync daemon auto-commits and pushes
5. Done

**What sync provides:**
- Auto-commit on every change (AI doesn't need to think about git)
- AI-generated commit subjects from diffs (unique messages each cycle)
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

## Systemd Service Files

Service files are installed to `~/.config/systemd/user/` by `install.sh`.

### dracon-sync.service

| Setting | Value | Purpose |
|---------|-------|---------|
| `ExecStart` | `dracon-sync daemon` | Runs sync daemon |
| `Restart` | `on-failure` | Restarts on crash |
| `RestartSec` | `5` | Wait 5s before restart |
| `Nice` | `10` | Lower CPU priority |
| `CPUQuota` | `15%` | Max 15% CPU usage |
| `MemoryMax` | `2G` | Max 2GB RAM |
| `TasksMax` | `96` | Max 96 threads |
| `Environment` | `DRACON_SYNC_POLICY` | Points to config file |
| `Environment` | `GIT_TERMINAL_PROMPT=0` | Disables interactive git prompts |

**Pre-start cleanup:** Kills stale `dracon-git pulse` processes to prevent lockups.

### dracon-system-guard.service

| Setting | Value | Purpose |
|---------|-------|---------|
| `ExecStart` | `dracon-system guard daemon` | Runs guard daemon |
| `Restart` | `on-failure` | Restarts on crash |
| `RestartSec` | `10` | Wait 10s before restart |
| `MemoryMax` | `100M` | Max 100MB RAM |
| `CPUQuota` | `10%` | Max 10% CPU usage |
| `NoNewPrivileges` | `true` | Security hardening |
| `ProtectSystem` | `strict` | Read-only system fs |
| `ProtectHome` | `read-only` | Read-only home (except allowed paths) |
| `ReadWritePaths` | `~/.dracon, ~/Dev, ~/.local/state/dracon` | Writable directories |

### dracon-warden.service

| Setting | Value | Purpose |
|---------|-------|---------|
| `ExecStart` | `dracon-warden daemon` | Runs warden daemon |
| `Restart` | `on-failure` | Restarts on crash |
| `RestartSec` | `3` | Wait 3s before restart |
| `Nice` | `10` | Lower CPU priority |
| `CPUQuota` | `10%` | Max 10% CPU usage |
| `MemoryMax` | `1G` | Max 1GB RAM |
| `TasksMax` | `64` | Max 64 threads |

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
├── dracon-sync-incidents.jsonl   # Append-only incident ledger
├── dracon-sync-stuck-push-repos.json  # Stuck push tracking
└── visibility-sync/              # Cache for visibility/metadata sync (per-repo timestamps)

The incident ledger is appended every sync cycle. Keeping it at `~/.local/state/dracon/` instead of inside `.dracon` prevents the sync daemon from auto-committing its own operational data.

### Incident Response

When the safety guard triggers or other incidents occur, entries are written to the incident ledger:

```bash
# View recent incidents
cat ~/.local/state/dracon/dracon-sync-incidents.jsonl | tail -20
```

Each line is a JSON object:
```json
{"ts_unix":1714896000,"scope":"safety","repo":"/path/to/repo","reason":"3 files missing from working tree (100% of 3 tracked)","action":"mass_deletion_guard","backup_branch":null,"result":"blocked","details":"total_tracked=3 missing_count=3"}
```

**After an incident:**
1. Read the incident ledger to understand what happened
2. Check the repo status: `git status` and `git log --oneline -5`
3. If mass deletion was blocked, decide whether it was intentional
4. For intentional deletions: manually commit with `git add -A && git commit -m 'delete files'`
5. Review the safety guard code if the block was unexpected

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

When enabled, git processes (init, fetch, pull, clone, push) that sustain high CPU for the configured duration receive SIGTERM. Before sending SIGKILL, the guard verifies the PID still belongs to the same git process via `/proc/{pid}/cmdline` to prevent killing a recycled PID. Disabled by default for safety.

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

After a version bump, `dracon-sync` can automatically create git tags, GitHub Releases, and publish to package registries. Three separate toggles control each step:

| Toggle | Default | Risk | Reversible? |
|--------|---------|------|-------------|
| `auto_tag` | `true` | Low | Yes (`git tag -d`) |
| `auto_release` | `false` | Medium | Yes (`gh release delete`) |
| `auto_publish` | `[]` | High | **No** (registries are immutable) |

**Per-repo opt-in:** Tags, releases, and publishing require a `.dracon/dracon-sync.toml` in the repo:

```toml
# .dracon/dracon-sync.toml
auto_tag = true              # default: on
auto_release = true          # default: off — creates GitHub Release on major bumps
auto_publish = ["crates-io"] # default: empty = no publishing
```

**Global publish targets** are configured in the main `dracon-sync.toml`:

```toml
auto_publish = false  # master toggle (default: off)

[[publish_targets]]
name = "crates-io"
registry = "crates-io"    # crates-io | npm | pypi
token_secret = "CARGO_REGISTRY_TOKEN"
publish_timeout_secs = 300
```

**Safety:** Dry-run publish (`cargo publish --dry-run`, `npm publish --dry-run`) runs before real publish. Registry pre-check skips already-published versions. Publish failures log incidents but don't break the sync cycle.

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
  repair-warns     Repair warn repos [--apply] [--repo <path>] [--json]
  once             Run one sync pass
  daemon           Run continuous sync loop [--interval-secs override]
  sync-now         Sync one or more repositories now [--dry-run] [--force] [repos...]
  pause            Pause sync (creates freeze marker)
  resume           Resume sync (removes freeze marker)
  edit-config      Open sync policy in the system editor
  test-ai          Test AI providers connectivity
  health           Check daemon health [--json]
  metrics          Print Prometheus-style metrics
  stuck            Manage repos permanently stuck on push
  dual-branch      Manage repos with both main and master branches
  repair-origins   Detect and repair orphan origin URLs [--apply]
  publish          Manually publish a repo to configured registries [--dry-run]
  publish-status   Check current version and registry publish status
```

**Nested subcommands:**
- `dracon-sync stuck list` — list stuck repos
- `dracon-sync stuck unstuck <repo>` — unstuck a specific repo
- `dracon-sync dual-branch list` — list repos with dual main/master
- `dracon-sync dual-branch repair <repo>` — consolidate to master

**Global flags:** `-v` / `-vv` increase verbosity; `-V` prints version.

### Safety Behaviors

**dracon-sync mass-deletion prevention:** The sync daemon will refuse to auto-commit deletions that remove 85% or more of tracked files in a repository. This guards against accidental mass wipes caused by filesystem issues, filter misconfigurations, or destructive operations.

When triggered, sync prints a warning and skips the commit:
```
⚠️ SAFETY: 46 files missing from working tree (85%+ of 46 tracked)
⚠️ Refusing to stage mass deletion - this looks like a mistake or destructive operation
⚠️ If you really want to delete these files, do: git add -A && git commit -m 'delete files'
```

To bypass: manually stage and commit the deletions with `git add -A && git commit`.

**Metrics:** A Prometheus counter `dracon_sync_mass_deletion_guard_blocked_total` is incremented each time the guard triggers. View it with `dracon-sync metrics`.

**Force bypass:** For intentional total wipes, use `dracon-sync sync-now --force <repo>` to skip the safety guard entirely. Use with caution — this will auto-commit ALL deletions without prompting.

**Incident response after a block:** Read the incident ledger at `~/.local/state/dracon/dracon-sync-incidents.jsonl` to understand what was blocked and why.

**dracon-sync commit message generation:** Instead of writing project-state.md and extracting a focus line, the scribe now generates commit subjects directly from diffs. The AI receives the current diff (highlighted as the main change), 10 previous diffs (background context), and recent commit subjects, then returns a single subject line. This produces unique, specific messages every cycle. When AI is unavailable, a local file-pattern fallback generates messages like "update auth, jwt and 2 files".

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

### All Tokens & Secrets

All secrets are stored in `~/.dracon/utilities/sync/secrets/*.env` (sync) and
`~/.dracon/utilities/sync/ai/secrets/*.env` (AI). See the secrets directory
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

### Test AI Providers

```bash
dracon-sync test-ai
```

## The Scribe: AI Commit Message Generator

The scribe generates unique, semantic commit subjects from diffs each sync cycle. It no longer writes or reads `project-state.md` — commit messages are generated directly from the actual code changes.

### How It Works

1. Collect current staged diff + 10 previous diffs + recent commit subjects
2. AI receives the current diff (highlighted as THE main change) and previous diffs (background only)
3. AI returns a single subject line in conventional commit format (e.g., `fix(auth): validate JWT expiry before accepting tokens`)
4. Category/scope are extracted from the AI subject; `build_commit_message` assembles the final commit with footer

### Fallback When AI Unavailable

If no AI providers are configured or all fail, a local file-pattern fallback generates messages like `update auth, jwt and 2 files` from changed file stems.

### Why Frequent Commits?

Sync commits every change because:
- The AI reads git history to understand past work
- Every commit is a checkpoint the AI can recover to
- More commits = better context for the AI's "what was I doing?"
- Commits are cheap; context is valuable

### Manual project-state.md

The AI can still maintain `.dracon/project-state.md` manually for its own working memory across sessions. Sync no longer auto-generates, stages, or commits this file. If the AI wants it tracked, it must `git add` it explicitly.

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

**406 tests** in `src/` (git.rs, sync.rs, report.rs, policy.rs, main.rs, visibility.rs, release.rs, bump.rs, secrets.rs). Tests use `tempfile::TempDir` for isolation.

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

---

## Related Documentation

- [README.md](README.md) — User-facing quick start and usage guide
- [CONTRIBUTING.md](CONTRIBUTING.md) — Development workflow and contribution guidelines
- [CHANGELOG.md](CHANGELOG.md) — Version history and release notes
