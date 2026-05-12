# Dracon Utilities

CLI binaries for dracon system services. These install to `~/.local/bin/` and run as systemd user services.

**For AI agents:** See [AGENTS.md](AGENTS.md) for detailed architecture and conventions.
**For contributors:** See [CONTRIBUTING.md](CONTRIBUTING.md) for development workflow.
**For changes:** See [CHANGELOG.md](CHANGELOG.md) for version history.

---

## Table of Contents

1. [What You Get](#what-you-get)
2. [Quick Start](#quick-start)
3. [dracon-sync — Git Sync Automation](#dracon-sync--git-sync-automation)
4. [dracon-system — Disk & Process Guard](#dracon-system--disk--process-guard)
5. [dracon-warden — Security Hardening](#dracon-warden--security-hardening)
6. [Configuration Examples](#configuration-examples)
7. [Troubleshooting](#troubleshooting)

---

## What You Get

| Binary | What It Does | Why You Want It |
|--------|-------------|-----------------|
| **dracon-sync** | Auto-commits, pulls, and pushes your git repos | Never lose work. Auto-commit on every change. |
| **dracon-system** | Watches disk space and kills runaway processes | Prevents "disk full" crashes and runaway CPU. |
| **dracon-warden** | Encrypts secrets in git repos | Keep API keys safe in version control. |

All three run as background daemons via systemd user services.

---

## Quick Start

### Prerequisites

```bash
# You need dracon-libs as a sibling directory
git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs
```

### Install

```bash
# Clone this repo
git clone https://github.com/DraconDev/dracon-utilities.git
cd dracon-utilities

# Build and install
./install.sh
```

This installs binaries to `~/.local/bin/` and systemd services to `~/.config/systemd/user/`.

### Start Services

```bash
systemctl --user start dracon-sync.service
systemctl --user start dracon-system-guard.service
systemctl --user start dracon-warden.service

# Enable auto-start on login
systemctl --user enable dracon-sync.service
systemctl --user enable dracon-system-guard.service
systemctl --user enable dracon-warden.service
```

### Resource Limits

All services run with conservative systemd resource limits:

| Service | MemoryHigh | MemoryMax | CPUQuota | TasksMax |
|---------|-----------|-----------|----------|----------|
| dracon-sync | 768M | 2G | 15% | 96 |
| dracon-system-guard | — | 100M | 10% | 32 |
| dracon-warden | 384M | 1G | 10% | 64 |

`MemoryHigh` is a soft limit that triggers memory reclaim before the hard `MemoryMax` limit is reached. This prevents sudden OOM kills while still constraining memory usage.

### Verify

```bash
dracon-sync status      # Shows discovered repos
dracon-system status    # Shows disk usage
dracon-warden status    # Shows watch roots
```

---

## dracon-sync — Git Sync Automation

**Purpose:** Watches your repos and auto-commits/pushes changes. You never have to run `git commit` or `git push` manually.

### How It Works

1. Discovers all git repos under `~/Dev` (up to 4 levels deep)
2. Every 30 seconds, checks each repo for uncommitted changes
3. Auto-commits with AI-generated messages (or fallback to timestamp)
4. Pulls remote changes (with merge, not rebase — safer)
5. Pushes to origin and any mirror remotes

### Essential Commands

```bash
# Run one sync pass manually
dracon-sync once

# Sync a specific repo immediately (with --force to bypass safety checks)
dracon-sync sync-now /path/to/repo
dracon-sync sync-now --force /path/to/repo  # Bypass mass-deletion guard

# Check daemon health
dracon-sync health

# View metrics
dracon-sync metrics

# Check for repos with too many unpushed commits
dracon-sync metrics | grep unpushed
```

### Safety: Mass-Deletion Prevention

`dracon-sync` refuses to auto-commit deletions that remove 100% of tracked files. This guards against accidental total wipes caused by filesystem issues, filter misconfigurations, or destructive operations.

When triggered, you'll see:
```
⚠️ SAFETY: 46 files missing from working tree (100% of 46 tracked)
⚠️ Refusing to stage total wipe - this looks like a mistake or destructive operation
```

**To bypass manually:** Stage and commit the deletions yourself:
```bash
git add -A && git commit -m 'delete files'
```

**To bypass with force:** Use `--force` on `sync-now`:
```bash
dracon-sync sync-now --force /path/to/repo
```

**Metrics:** The counter `dracon_sync_mass_deletion_guard_blocked_total` tracks how many times the guard has triggered. View it with:
```bash
dracon-sync metrics
```

### Configuration

**Path:** `~/.dracon/utilities/sync/dracon-sync.toml`

```toml
[sync]
watch_roots = ["/home/dracon/Dev"]
interval_secs = 30
auto_commit = true
auto_pull = true
auto_push = true

# Alert threshold: warn if repo has more than N unpushed commits
# (should rarely trigger with auto_push enabled)
alert_unpushed_threshold = 10

# Automatically create GitHub repos for new projects
auto_github_private = true
auto_github_private_account = "DraconDev"

# Push to multiple remotes
[[remotes]]
name = "origin"
push_url = "git@github.com:DraconDev/{repo}.git"
auto_create = true

[[remotes]]
name = "codeberg"
push_url = "git@codeberg.org:DraconDev/{repo}.git"
auto_create = false  # Codeberg doesn't support push-to-create
```

**PAT-based HTTPS fallback:** If SSH authentication fails, dracon-sync automatically falls back to HTTPS using Personal Access Tokens (PATs). Store your tokens in `~/.dracon/utilities/sync/secrets/`:

```bash
# GitLab PAT (for HTTPS fallback when SSH fails)
echo "GITLAB_TOKEN=glpat-xxxxxxxxxxxxxxxxxxxx" > ~/.dracon/utilities/sync/secrets/gitlab.env

# Codeberg PAT (for HTTPS fallback when SSH fails)  
echo "CODEBERG_TOKEN=cbp_xxxxxxxxxxxxxxxxxxxx" > ~/.dracon/utilities/sync/secrets/codeberg.env
```

### AI Commit Messages

To enable AI-generated commit messages, create `~/.dracon/utilities/sync/ai.toml`:

```toml
[[providers]]
name = "openrouter"
env = "OPENROUTER_API_KEY"
endpoint = "https://openrouter.ai/api/v1"
model = "openrouter/free"
```

Set your API key:
```bash
echo "OPENROUTER_API_KEY=your_key_here" > ~/.dracon/utilities/sync/ai/openrouter.env
```

Test connectivity:
```bash
dracon-sync test-ai
```

---

## dracon-system — Disk & Process Guard

**Purpose:** Monitors disk usage and CPU-heavy processes. Auto-cleans when disk is full and can kill runaway processes.

### How It Works

1. Every 30 seconds, checks disk usage
2. At 90%: freezes dracon-sync and starts cleanup
3. At 95%: critical — aggressive cleanup
4. Monitors processes using >50% CPU for >30s
5. Can auto-renice or auto-kill runaway git processes

### Essential Commands

```bash
# Check current status
dracon-system status

# Run one guard pass (shows what it would do)
dracon-system guard once

# Run cleanup dry-run
dracon-system storage --cleanup

# Actually clean up
dracon-system storage --cleanup --apply

# Check for broken symlinks
dracon-system link doctor

# Fix broken symlinks
dracon-system link apply
```

### Configuration

**Path:** `~/.dracon/utilities/system/dracon-system.toml`

```toml
[guard]
# Disk thresholds (percent)
disk_warn_percent = 80
disk_action_percent = 90
disk_critical_percent = 95

# Process monitoring
process_cpu_percent = 50.0      # Alert if CPU > 50%
process_sustain_secs = 30       # For at least 30 seconds

# Auto-kill runaway git processes (disabled by default)
auto_kill_git = false
git_kill_threshold_secs = 60

# Notifications
notify = true
notify_cooldown_secs = 300

# Auto-cleanup Rust target directories when disk is full
auto_cleanup_rust = true
cleanup_min_size_mb = 256

# Persistent logging of heavy processes
guard_log_file = "~/.local/state/dracon/dracon-system-guard.log"
guard_log_max_mb = 1  # Rotate at 1 MiB
```

### Viewing Process Logs

When a process spikes CPU, it's logged to:
```bash
cat ~/.local/state/dracon/dracon-system-guard.log
```

Example output:
```json
{"ts":1778124364,"event":"heavy-brief","details":"pid=2215651 ppid=2215626 cmd=ps args=ps -eo pid,ppid,pcpu,rss,comm,args --no-headers cpu=300.0% rss=10MiB sustained=0s"}
```

### Protected Paths

These paths are protected from deletion (always):
```
/, /home, /etc, /usr, /var, /boot, /nix, /run, /sys, /dev, /proc
```

Add custom protected paths:
```toml
[guard]
protected_paths = ["/mnt/data", "/opt/important"]
```

---

## dracon-warden — Security Hardening

**Purpose:** Encrypts secrets (API keys, passwords) before they reach git. Uses `age` encryption.

### How It Works

1. You mark files with `DRACON_SECRET` markers
2. Warden encrypts them before `git add`
3. Only encrypted ciphertext is committed
4. On checkout, warden decrypts automatically

### Essential Commands

```bash
# Generate encryption keypair
dracon-warden keygen

# Scan for unencrypted secrets
dracon-warden scrub-markers /path/to/repo

# Fix ciphertext stuck in working tree
dracon-warden resmudge /path/to/repo

# Run one hardening pass
dracon-warden once /path/to/repo
```

### Setup in a Repo

1. Generate a key:
   ```bash
   dracon-warden keygen
   ```

2. Add to repo's `.gitattributes`:
   ```
   *.secret filter=dracon-warden
   ```

3. Mark secrets in files:
   ```python
   # DRACON_SECRET
   API_KEY = "sk-1234567890abcdef"
   # END_DRACON_SECRET
   ```

4. Warden auto-encrypts on commit, auto-decrypts on checkout.

---

## Configuration Examples

### Minimal Setup (Just Sync)

```toml
# ~/.dracon/utilities/sync/dracon-sync.toml
[sync]
watch_roots = ["/home/dracon/Dev"]
interval_secs = 30
auto_commit = true
auto_pull = true
auto_push = true
```

### Aggressive Disk Guard

```toml
# ~/.dracon/utilities/system/dracon-system.toml
[guard]
disk_warn_percent = 70
disk_action_percent = 80
disk_critical_percent = 90
process_cpu_percent = 30.0
auto_kill_git = true
git_kill_threshold_secs = 30
auto_cleanup_rust = true
```

### Multi-Remote Push

```toml
# ~/.dracon/utilities/sync/dracon-sync.toml
[sync]
auto_push = true

[[remotes]]
name = "github"
push_url = "git@github.com:DraconDev/{repo}.git"

[[remotes]]
name = "gitlab"
push_url = "git@gitlab.com:DraconDev/{repo}.git"
auto_create = true

[remotes.repo_name_map]
".dracon" = "dracon-home"
```

---

## Troubleshooting

### "command not found: dracon-sync"

```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.local/bin:$PATH"
```

### "dracon-libs not found"

```bash
cd /path/to/dracon-utilities
git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs
```

### Sync not auto-committing

1. Check if paused:
   ```bash
   dracon-sync status  # Look for "paused"
   dracon-sync resume  # If paused
   ```

2. Check policy path:
   ```bash
   dracon-sync validate-config
   ```

3. Check the incident ledger for blocked operations:
   ```bash
   cat ~/.local/state/dracon/dracon-sync-incidents.jsonl | tail -5
   ```

### Disk cleanup not running

```bash
# Check if guard is running
systemctl --user status dracon-system-guard.service

# Check disk state
dracon-system status

# Run manually to see what's happening
dracon-system guard once
```

### High CPU from git processes

Check the guard log:
```bash
cat ~/.local/state/dracon/dracon-system-guard.log | grep git
```

### Unpushed commits alert

If you see:
```
🚨 ALERT: /path/to/repo has 15 unpushed commits (threshold: 10)
```

This means auto-push is failing. Check:
```bash
# Check if repo is stuck
dracon-sync stuck list

# Check push errors
cd /path/to/repo && git push origin HEAD

# View recent incidents
cat ~/.local/state/dracon/dracon-sync-incidents.jsonl | tail -5
```

### Service won't start

```bash
# Check logs
journalctl --user -u dracon-sync.service -n 50
journalctl --user -u dracon-system-guard.service -n 50
journalctl --user -u dracon-warden.service -n 50

# Restart everything
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
systemctl --user restart dracon-warden.service
```

---

## Uninstall

```bash
./uninstall.sh
```

This removes binaries and systemd services. Your git repos and configs in `~/.dracon/` are preserved.
