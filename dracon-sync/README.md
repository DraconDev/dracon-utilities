# dracon-sync

Deterministic git sync daemon with automatic remote creation, AI-powered commit messages, and self-healing repository management.

## Features

### Automatic Remote Creation
When `auto_github_private = true`, newly initialized repos without an origin remote automatically get a private GitHub repository created via `gh`, with the remote added and initial commit pushed.

### Deterministic Sync
- Monitors repositories for changes across watched roots
- Commits, pulls, and pushes automatically based on policy
- Respects freeze markers (e.g., during deployments)

### Self-Healing
- Detects and repairs common git issues (conflicted remotes, stuck pushes)
- Consolidates dual main/master branch repos to main
- Manages permanently stuck repos

### AI Scribe Integration
Generates meaningful commit messages using AI when providers are configured.

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

# Run continuous sync daemon (default: 60s pulse interval)
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
pulse_interval_secs = 60

# Delay after last change before auto-push (seconds)
inactivity_push_delay_secs = 120

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
# adds the SSH remote, and pushes the initial commit.
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
2. Add the SSH remote: `git remote add origin git@github.com:account/repo.git`
3. Push the initial commit: `git push -u origin HEAD`

Requirements:
- `gh` CLI must be installed and authenticated (`gh auth status`)
- `auto_github_private_account` must match your GitHub username or org

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

## The Scribe: AI Working Memory

The daemon maintains `.dracon/project-state.md` in each repo. This is the AI's **working memory** — the primary interface between sync and the AI coder.

The AI reads this file on session start to understand past work. Sync commits every change so the AI can recover from any point in history.

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

**Rules:**
- **Current Focus** must be one line — it becomes the commit body
- **Context** helps the AI recover understanding after time away
- **Blockers** tell the AI what it can't proceed on
- **Next Steps** give the AI a clear path forward
- Be specific: "Fix TOCTOU race in warden keygen" not "fix bugs"
- Don't document mechanical changes — only semantic state

## Version

```bash
dracon-sync --version
```
