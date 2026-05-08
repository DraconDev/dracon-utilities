# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **dracon-system**: Auto-kill policy for runaway git processes
  - `auto_kill_git` config option (disabled by default)
  - `git_kill_threshold_secs` config option (default: 60s)
  - `kill_process()` function with SIGTERM → SIGKILL escalation
  - `is_git_process()` function detecting git-init, git-fetch, git-pull, git-clone, git-push
  - 5 unit tests for process detection
- **dracon-system**: Persistent guard logging
  - `guard_log_file` config option (default: `~/.local/state/dracon/dracon-system-guard.log`)
  - `guard_log_max_mb` config option (default: 1 MiB, auto-rotates)
  - JSONL format with `heavy-brief` and `heavy-sustained` events
  - Full command line (`args`) and parent PID (`ppid`) in process samples
- **Documentation**: Complete README.md rewrite with quick start, per-utility guides, troubleshooting
- **Documentation**: Example config files for all utilities
  - `dracon-sync/dracon-sync.example.toml`
  - `dracon-system/dracon-system.example.toml`
  - `dracon-warden/dracon-warden.example.toml`
  - `dracon-ai/dracon-ai.example.toml`
- **Scripts**: Enhanced install.sh with --help, --dry-run, --force, --upgrade, --verbose flags
- **Scripts**: New doctor.sh for prerequisite validation
- **Scripts**: Enhanced uninstall.sh with --help, --configs, --logs, --purge flags
- **Scripts**: install.sh now copies example configs on first install

### Changed
- **dracon-system**: Process monitoring now logs ALL heavy processes (not just sustained)
- **dracon-system**: `ps` output changed from `comm=` (15-char truncated) to `args` (full command line)
- **AGENTS.md**: Added environment variables section
- **AGENTS.md**: Added process monitoring & logging documentation
- **AGENTS.md**: Updated policy files table with example config links

### Fixed
- **dracon-sync**: Critical bug in mass-deletion safety check — `git ls-files --count` is not a valid git command, causing `.unwrap_or(0)` to silently bypass the guard. Fixed by counting lines of `git ls-files` output.
- **dracon-sync**: Strengthened mass-deletion guard with secondary threshold (>50% of tracked files missing triggers prevention, not just 100%)
- **dracon-sync**: Merge strategy changed from `git pull --rebase` to `git pull --no-rebase`
- **install.sh**: Now creates all config directories (sync, system, warden, ai)
- **install.sh**: Copies example configs if they don't exist

## [0.2.0] - 2024-05-03

### Added
- **dracon-system**: Guard daemon for disk/process monitoring
  - Disk usage thresholds (70/80/90/95%)
  - Auto-freeze dracon-sync at 90% disk usage
  - Auto-cleanup Rust target directories
  - Process CPU monitoring with notifications
  - Zombie process detection
  - Inode usage monitoring
- **dracon-warden**: Security hardening daemon
  - Git filter encryption for secrets
  - `DRACON_SECRET` marker support
  - `scrub-markers` recovery tool
  - `resmudge` working tree repair

### Changed
- Restructured as cargo workspace with separate crates

## [0.1.0] - 2024-04-28

### Added
- **dracon-sync**: Initial release
  - Auto-commit, auto-pull, auto-push
  - AI-powered commit messages (scribe)
  - Version bumping (ai-bumper)
  - Incident ledger for debugging
  - Stuck repo management
  - Dual-branch repair (main/master)
  - Orphan origin URL repair
  - GitHub private repo auto-creation
  - Multi-remote push support
  - Webhook notifications

---

## Versioning Notes

- **MAJOR**: Breaking changes to config format or CLI interface
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, documentation updates
