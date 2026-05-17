# dracon-sync

**Invisible git sync for AI-powered development.** An auto-commit, multi-mirror daemon that watches your repos, commits every change with AI-generated messages, and pushes to GitHub, GitLab, and Codeberg simultaneously.

## Why This Exists

Other tools solve parts of the problem:
- **git-auto-sync**: Auto-commits a single repo, no mirroring
- **gitea-mirror**: One-way mirror to a single Forgejo instance, no auto-commit
- **git-bridge**: Multi-provider sync, but no auto-commit or AI
- **swarf**: Invisible sync for AI agents, but only a side-band directory

**dracon-sync** is the only tool that combines all of these into one daemon:

| Capability | git-auto-sync | gitea-mirror | git-bridge | swarf | **dracon-sync** |
|---|:-:|:-:|:-:|:-:|:-:|
| Auto-commit on change | ✅ | ❌ | ❌ | ✅ | ✅ |
| Multi-repo watch | ❌ | ✅ | ✅ | ❌ | ✅ |
| Multi-provider mirror | ❌ | ✅ (→1) | ✅ | ❌ | ✅ (3+) |
| AI commit messages | ❌ | ❌ | ❌ | ❌ | ✅ |
| Version bump + release | ❌ | ❌ | ❌ | ❌ | ✅ |
| Safety guards | ❌ | ❌ | ❌ | ❌ | ✅ |
| Visibility sync | ❌ | ❌ | ❌ | ❌ | ✅ |
| Broken tracking repair | ❌ | ❌ | ❌ | ❌ | ✅ |

## Features

### Invisible Infrastructure
The AI (or human) works on one repo at a time, makes changes, and sync handles the rest — the AI never needs to think about commits, pushes, or cross-repo coordination.

1. You edit files
2. Sync detects changes within seconds
3. After a brief inactivity delay (5s default), sync commits with an AI-generated message
4. Pushes to origin (GitHub) and all mirror remotes (GitLab, Codeberg)
5. Done — no manual git commands needed

### HTTPS + PAT Transport (GitHub)
GitHub origin uses **HTTPS with Personal Access Tokens** — more reliable than SSH (no agent timeouts, no key rotation). GitLab and Codeberg mirrors use SSH by default, with HTTPS PAT fallback on SSH failures.

### Automatic Remote Creation
When `auto_github_private = true`, newly initialized repos without an origin remote automatically get a private GitHub repository created via `gh`, with the remote added and initial commit pushed.

### Deterministic Sync
- Monitors repositories for changes across watched roots
- Commits, pulls, and pushes automatically based on policy
- Respects freeze markers (e.g., during deployments)

### Self-Healing
- Detects and repairs common git issues (conflicted remotes, stuck pushes)
- Repairs broken upstream tracking refs (e.g. `origin/master: gone`)
- Consolidates dual main/master branch repos to main
- Manages permanently stuck repos
- Prunes stale operational state on daemon restart (stuck repos, incident ledger, visibility cache)

### AI Scribe Integration
Generates meaningful commit messages using AI when providers are configured. Falls back to local file-pattern messages when AI is unavailable.

## Installation

### Quick Install (User Service)

```bash
cd dracon-sync
./install.sh
```

This will:
1. Build the release binary
2. Install to `~/.local/bin/dracon-sync`
3. Set up and start the systemd user service

### Manual Install

```bash
# Build
cargo build --release

# Copy binary
cp target/release/dracon-sync ~/.local/bin/

# Install systemd service
mkdir -p ~/.config/systemd/user
cp dracon-sync.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable dracon-sync.service
systemctl --user start dracon-sync.service
```

## Usage

### Commands

```bash
# Show policy path, watched roots, and discovered repos
dracon-sync status

# One-shot sync across all discovered repos
dracon-sync once

# Run continuous sync daemon (default: 1s pulse interval)
dracon-sync daemon

# Override the pulse interval from CLI
dracon-sync daemon --interval-secs 30

# Sync a specific repository now
dracon-sync sync-now ~/Dev/my-project

# Edit the sync policy
dracon-sync edit-config

# Test AI provider connectivity
dracon-sync test-ai

# Report across all repos
dracon-sync repos
dracon-sync repos --only-concern
dracon-sync repos --json

# Repair concern repos (dry-run by default)
dracon-sync repair-concerns
dracon-sync repair-concerns --apply

# Repair warn repos
dracon-sync repair-warns
dracon-sync repair-warns --apply

# Manage stuck repos
dracon-sync stuck list
dracon-sync stuck unstuck ~/Dev/repo

# Manage dual-branch repos
dracon-sync dual-branch list
dracon-sync dual-branch repair ~/Dev/repo

# Repair orphan origin URLs (e.g. after remote rename)
dracon-sync repair-origins
dracon-sync repair-origins --apply

# Scaffold standard files (LICENSE, CLA, etc.)
dracon-sync scaffold
dracon-sync scaffold --repo ~/Dev/repo --files LICENSE,CLA.md

# Manually publish to registries
dracon-sync publish ~/Dev/repo
dracon-sync publish-status ~/Dev/repo

# Check daemon health and metrics
dracon-sync health
dracon-sync metrics
```

### Systemd Service Management

```bash
# Check status
systemctl --user status dracon-sync.service

# View logs
journalctl --user -u dracon-sync -f

# Restart after config changes
systemctl --user restart dracon-sync.service
```

## Configuration

Create `~/.dracon/utilities/sync/dracon-sync.toml`:

```toml
[sync]
# Watch directories for git repositories
watch_roots = ["/home/user/Dev", "/home/user/work"]

# Pulse interval in seconds (how often to scan for changes)
pulse_interval_secs = 1

# Delay after last change before auto-push (seconds)
inactivity_push_delay_secs = 5

# Auto git operations
auto_commit = true
auto_pull = true
auto_push = true
auto_bump_versions = true

# Auto-repair concerns and warnings
auto_repair_concerns = true
auto_repair_warns = true

# Automatic private GitHub remote creation
# When a repo has no origin remote, creates a private GitHub repo,
# adds the HTTPS remote, and pushes the initial commit.
auto_github_private = true
auto_github_private_account = "YourOrgOrUsername"

# Exclude specific repos or directories
exclude_repos = ["/home/user/Dev/archived"]
exclude_dir_names = ["node_modules", "target", ".venv"]

# Sync freeze (for use with dracon-system disk guard)
freeze_sync_at_action = true
```

### Automatic Remote Creation

If `auto_github_private = true` in your policy, any `git init` in a watched root will automatically:

1. Create a private GitHub repo via `gh repo create --private`
2. Add the HTTPS remote: `git remote add origin https://github.com/account/repo.git`
3. Push the initial commit: `git push -u origin HEAD`

Requirements:
- `gh` CLI must be installed and authenticated (`gh auth login`)
- `auto_github_private_account` must match your GitHub username or org

### Multi-Provider Mirrors

Push to multiple providers simultaneously. GitHub uses HTTPS + PAT; others use SSH with HTTPS fallback:

```toml
[[remotes]]
name = "github"
push_url = "https://github.com/DraconDev/{repo}.git"

[[remotes]]
name = "gitlab"
push_url = "git@gitlab.com:dracondev/{repo}.git"
auto_create = true

[[remotes]]
name = "codeberg"
push_url = "git@codeberg.org:dracondev/{repo}.git"
auto_create = false  # Codeberg/Forgejo doesn't support push-to-create
```

Store PATs for HTTPS fallback and API operations:
```bash
# GitLab
echo "GITLAB_TOKEN=glpat-xxxxxxxxxxxxxxxxxxxx" > ~/.dracon/utilities/sync/secrets/gitlab.env

# Codeberg
echo "CODEBERG_TOKEN=cbp_xxxxxxxxxxxxxxxxxxxx" > ~/.dracon/utilities/sync/secrets/codeberg.env
```

### AI Providers

dracon-sync uses AI for commit messages and version bumping. Configure in `~/.dracon/utilities/sync/ai.toml`:

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
```

Store API keys in `~/.dracon/utilities/sync/ai/secrets/`:
- `mistral.env` → `MISTRAL_API_KEY=...`
- `nvidia.env` → `NVIDIA_API_KEY=...`

## The Scribe: AI Commit Messages

The scribe generates unique, semantic commit subjects from diffs each sync cycle. It does NOT write or read `project-state.md` — commit messages are generated directly from the actual code changes.

### How It Works

1. Collect current staged diff + 10 previous diffs + recent commit subjects
2. AI receives the current diff (highlighted as THE main change) and previous diffs (background only)
3. AI returns a single subject line in conventional commit format (e.g., `fix(auth): validate JWT expiry before accepting tokens`)
4. Category/scope are extracted from the AI subject; `build_commit_message` assembles the final commit with footer

### Fallback When AI Unavailable

If no AI providers are configured or all fail, a local file-pattern fallback generates messages like `update auth, jwt and 2 files` from changed file stems.

### Manual project-state.md

The AI can still maintain `.dracon/project-state.md` manually for its own working memory across sessions. Sync no longer auto-generates, stages, or commits this file. If the AI wants it tracked, it must `git add` it explicitly.

## Startup Cleanup

On daemon start/restart, sync prunes stale operational state:

- **Stuck repos**: Removes entries from stuck-push tracking for repos no longer stuck
- **Incident ledger**: Enforces retention policy (keeps last N entries per `incident_retention`)
- **Visibility cache**: Removes orphan `.last` files for repos no longer watched
- **Broken tracking**: Repairs `origin/master: gone` refs → re-points to `origin/{branch}`

Broken tracking repair also runs every ~300 cycles (~5 min) in the daemon loop, since new `:gone` tracking breaks can appear at runtime.

## Version

```bash
dracon-sync --version
```
