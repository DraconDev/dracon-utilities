# Dracon Utilities

**Invisible git infrastructure for AI-powered development.** Three daemon services that make git, disk, and secrets management something you never think about.

dracon-sync is the core: an auto-commit, multi-mirror sync daemon that watches your repos, commits every change with AI-generated messages, and pushes to GitHub, GitLab, and Codeberg simultaneously. It's designed so that an AI coder (or a human) can just edit files and walk away — sync, commit, push, and mirror happen automatically.

**Why this exists:** Tools like `git-auto-sync` handle single-repo auto-commit. Mirror tools like `gitea-mirror` handle one-way replication. But nothing combines invisible auto-commit + multi-provider mirroring + AI scribe + release pipeline + safety guards into a single daemon. dracon-sync is the only tool that lets an AI coder never think about git while keeping every repo synced across 3 providers.

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

**For AI agents:** See [AGENTS.md](AGENTS.md) for detailed architecture and conventions.
**For contributors:** See [CONTRIBUTING.md](CONTRIBUTING.md) for development workflow.
**For changes:** See [CHANGELOG.md](CHANGELOG.md) for version history.

---

## Table of Contents

1. [What You Get](#what-you-get)
2. [Quick Start](#quick-start)
3. [dracon-sync — Invisible Git Sync](#dracon-sync--invisible-git-sync)
4. [dracon-system — Disk & Process Guard](#dracon-system--disk--process-guard)
5. [dracon-warden — Security Hardening](#dracon-warden--security-hardening)
6. [Configuration Examples](#configuration-examples)
7. [Troubleshooting](#troubleshooting)

---

## What You Get

| Binary | What It Does | Why You Want It |
|--------|-------------|-----------------|
| **dracon-sync** | Auto-commits, mirrors to 3 providers, AI commit messages, release pipeline | Never think about git. Edit files, walk away. |
| **dracon-system** | Watches disk space and renices runaway processes | Prevents "disk full" crashes and runaway CPU. |
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
| dracon-system-guard | — | 250M | 20% | 64 |
| dracon-warden | 384M | 1G | 10% | 64 |

`MemoryHigh` is a soft limit that triggers memory reclaim before the hard `MemoryMax` limit is reached. This prevents sudden OOM kills while still constraining memory usage.

### Verify

```bash
dracon-sync status      # Shows discovered repos
dracon-system status    # Shows disk usage
dracon-warden status    # Shows watch roots
```

---

## dracon-sync — Invisible Git Sync

**Purpose:** Makes git invisible. You edit files, sync handles everything else — auto-commit, auto-push, multi-mirror, AI messages, release pipeline.

### Design Philosophy

dracon-sync is **invisible infrastructure** for an AI coder. The AI works on one repo at a time, makes changes, and sync handles the rest — the AI never needs to think about commits, pushes, or cross-repo coordination.

**The workflow:**
1. You (or an AI agent) edit files in a repo
2. Sync daemon detects changes within seconds
3. After a brief inactivity delay (5s default), sync commits with an AI-generated message
4. Pushes to origin (GitHub) and all mirror remotes (GitLab, Codeberg)
5. Done — no manual git commands needed

**What sync provides:**
- Auto-commit on every change (every edit gets a checkpoint)
- AI-generated commit subjects from diffs (unique, semantic messages each cycle)
- Multi-provider mirroring (GitHub + GitLab + Codeberg simultaneously)
- Auto-create GitHub repos for new projects
- Visibility sync (mirrors GitHub's public/private status to GitLab/Codeberg)
- Version bumping, tagging, and release pipeline
- Self-healing: repairs broken tracking refs, stuck pushes, dual branches
- Safety guards: blocks mass deletions, respects freeze markers

**What sync doesn't do:**
- Interactive prompts — everything runs non-interactively
- Session management — each daemon cycle is independent
- Dashboard — status is available via `repos` and `health` commands

### Transport: HTTPS + PAT (Primary), SSH (Fallback)

dracon-sync uses **HTTPS with Personal Access Tokens** as the primary transport for GitHub. This is more reliable than SSH — no SSH agent timeouts, no key rotation, and `gh auth` handles token refresh automatically. GitLab and Codeberg mirrors use SSH by default, with HTTPS PAT fallback on SSH failures.

**GitHub:** HTTPS via `gh auth` credential helper. Configure once:
```bash
gh auth login
```

**GitLab & Codeberg:** SSH by default. Store PATs for HTTPS fallback:
```bash
# GitLab PAT (for HTTPS fallback and API operations)
echo "GITLAB_TOKEN=glpat-xxxxxxxxxxxxxxxxxxxx" > ~/.dracon/utilities/sync/secrets/gitlab.env

# Codeberg PAT (for HTTPS fallback and API operations)
echo "CODEBERG_TOKEN=cbp_xxxxxxxxxxxxxxxxxxxx" > ~/.dracon/utilities/sync/secrets/codeberg.env
```

### How It Works

1. Discovers all git repos under `~/Dev` (up to 4 levels deep)
2. Every pulse (default 1s), checks each repo for uncommitted changes
3. After inactivity delay (default 5s since last change), auto-commits with AI-generated messages
4. Pulls remote changes (with merge, not rebase — safer, less likely to conflict)
5. Pushes to origin (GitHub) and mirror remotes (GitLab, Codeberg)

### Essential Commands

```bash
# Run one sync pass manually
dracon-sync once

# Sync a specific repo immediately (with --force to bypass safety checks)
dracon-sync sync-now /path/to/repo
dracon-sync sync-now --force /path/to/repo  # Bypass mass-deletion guard

# Check daemon health
dracon-sync health

# View all repos and their sync status
dracon-sync repos

# View metrics
dracon-sync metrics
```

### Safety: Mass-Deletion Prevention

`dracon-sync` refuses to auto-commit deletions that remove 85% or more of tracked files. This guards against accidental mass deletions caused by filesystem issues, filter misconfigurations, or destructive operations.

When triggered, you'll see:
```
⚠️ SAFETY: 46 files missing from working tree (85% of 46 tracked)
⚠️ Refusing to stage mass deletion - this looks like a mistake or destructive operation
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

### Report Accuracy

The `repos` command shows **real dirty file counts** — the MOD/STG columns reflect the actual number of modified/staged files, regardless of whether they're excluded by policy. The OK/WARN/CONCERN status still uses the effective filter, so a repo with 30 files in `target/` shows MOD=30 but status=OK. Previously, the report zeroed out the counts when `effective_dirty` was false, making repos with dozens of uncommitted files appear clean.

### Daemon Reliability

- **Push timeouts** (60s default): A hanging mirror push (e.g. GitLab unreachable) blocks the entire daemon. 60s per push / 120s per repo keeps things responsive
- **Stale lock cleanup**: On startup, removes orphan `.git/index.lock` files that block all git operations in a repo
- **Filter-only cooldown**: Repos with clean/smudge filter changes (e.g. dracon-warden) show as dirty but have no diff — the daemon detects this and cools down instead of tight-looping
- **Fingerprint scheduling**: Only syncs after 5s of no fingerprint change, preventing partial-change commits

### Configuration

**Path:** `~/.dracon/utilities/sync/dracon-sync.toml`

```toml
[sync]
watch_roots = ["/home/dracon/Dev"]
pulse_interval_secs = 1
inactivity_push_delay_secs = 5
auto_commit = true
auto_pull = true
auto_push = true

# Alert threshold: warn if repo has more than N unpushed commits
alert_unpushed_threshold = 10

# Automatically create GitHub repos for new projects
auto_github_private = true
auto_github_private_account = "DraconDev"

# Push to multiple remotes (GitHub uses HTTPS + PAT, others use SSH)
[[remotes]]
name = "github"
push_url = "https://github.com/DraconDev/{repo}.git"
auto_create = false

[[remotes]]
name = "gitlab"
push_url = "git@gitlab.com:dracondev/{repo}.git"
auto_create = false

[[remotes]]
name = "codeberg"
push_url = "git@codeberg.org:dracondev/{repo}.git"
auto_create = false  # Codeberg/Forgejo doesn't support push-to-create
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

**Purpose:** Monitors disk usage and CPU-heavy processes. Auto-cleans when disk is full and auto-renices runaway processes (**never kills** — only deprioritizes).

### How It Works

1. Every 30 seconds, checks disk usage
2. At 65%: early warning
3. At 75%: freezes dracon-sync and starts cleanup
4. At 85%: aggressive cleanup
5. At 92%: critical — emergency cleanup
6. Monitors processes using >50% CPU (180% in live config) for >30s, graduates nice to deprioritize (**never kills**)

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

# Auto-renice heavy processes (graduated: higher CPU/memory = higher nice value)
auto_renice = true
renice_value = 5
release_after_secs = 120

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

# System-wide repair (scrub + re-hardening)
dracon-warden repair /path/to/repo

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
pulse_interval_secs = 1
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
auto_renice = true
renice_value = 5
release_after_secs = 120
auto_cleanup_rust = true
```

### Multi-Remote Mirror

```toml
# ~/.dracon/utilities/sync/dracon-sync.toml
[sync]
auto_push = true

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
auto_create = false  # Codeberg doesn't support push-to-create
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

## License

This project is dual-licensed:

- **AGPL-3.0-only** — See [LICENSE](LICENSE) for the full text. This is the default license for open source use.
- **Commercial License** — For organizations that prefer not to comply with AGPLv3's source disclosure requirements. See [COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md) for details.

By contributing to this project, you agree to the terms in [CLA.md](CLA.md).
