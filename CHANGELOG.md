# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **dracon-sync**: Scribe refactor — commit messages from diffs, not `project-state.md`
  - `generate_commit_message()`: AI receives current diff (main) + 10 previous diffs (background) + recent subjects → returns subject line
  - `local_fallback_message()`: file-pattern fallback (e.g., "update auth, jwt and 2 files") when AI unavailable
  - Removed `scribe_update()` and `stage_project_state()` — replaced by direct commit message generation
  - Removed `read_project_focus`, `extract_category_scope_from_focus`, `extract_scope_from_focus`, `git_log_recent_subjects`
  - `project-state.md` is now manual-only: sync no longer auto-generates, stages, or commits it
  - `parse_conventional_commit()` extracts (category, scope, description) from AI subject to prevent double-prefix

### Added
- **dracon-sync**: Mirror visibility sync (`sync_visibility` config)
  - Mirrors on Codeberg/GitLab automatically match GitHub's public/private status
  - Cache-gated: at most one API check per repo per `sync_visibility_interval_hours` (default: 24h)
  - `gh api` for GitHub reads, `curl` for GitLab/Codeberg writes
  - `strip_ansi()` helper for `gh api` JSON output (GitHub CLI injects color codes)
- **dracon-sync**: Mirror metadata sync (`sync_metadata` config)
  - Mirrors get GitHub's description and topics/tags synced automatically
  - Shares the same cache-gate as visibility sync
- **dracon-sync**: Three-toggle release pipeline (`auto_tag`, `auto_release`, `auto_publish`)
  - `auto_tag = true` (default on): Git tag `v{version}` on every version bump
  - `auto_release = false` (default off): GitHub Release on major bumps via `gh release create`
  - `auto_publish = []` (default empty): Publish to crates.io/npm/PyPI (per-registry opt-in)
  - All three require per-repo opt-in via `.dracon/dracon-sync.toml`
  - Dry-run safety: `cargo publish --dry-run` / `npm publish --dry-run` before real publish
  - Idempotent: skips if version already exists on registry
  - Non-fatal: publish failures log incidents but don't break the sync cycle
- **dracon-sync**: `publish` and `publish-status` CLI subcommands
  - `publish <repo>`: Manually publish to configured registries
  - `publish-status <repo>`: Check current version and registry status
- **dracon-sync**: `SyncOutcome` enum (`Synced`/`NothingToDo`/`Blocked`)
  - Replaces `Result<bool>` — daemon only increments failure count on actual errors
  - Clean repos no longer accumulate false failure counts
- **dracon-sync**: `GIT_ASKPASS` for GitLab/Codeberg HTTPS PAT push
  - Replaces URL-embedded `oauth2:TOKEN@` and `git:TOKEN@` patterns
  - Tokens no longer visible in process listings or logs
- **dracon-sync**: `effective_auth_type()` and `resolve_account()` on `RemoteConfig`
  - Auto-detects GitLab/Codeberg from push URL when `auth_type` not explicitly set
  - Extracts account name from push URL pattern for API calls
- **dracon-sync**: Permission checks on secrets directory
  - `load_secret` rejects world-writable directories and warns on world-readable files
- **dracon-sync**: AI major-bump cap — `parse_ai_bump_response` downgrades `Major` → `Minor`
  - Major version bumps require manual intervention
- **dracon-sync**: `MASS_DELETION_GUARD_BLOCKED` Prometheus counter
  - Counter `dracon_sync_mass_deletion_guard_blocked_total` incremented on each guard trigger
  - View with `dracon-sync metrics`
- **dracon-sync**: `sync-now --force` flag for intentional mass deletions
  - Bypasses the mass-deletion safety guard completely
  - Use with caution — commits ALL deletions without prompting
- **dracon-sync**: HTTPS+PAT fallback for GitLab and Codeberg pushes
  - `gitlab_https_url()` and `codeberg_https_url()` functions convert SSH URLs to HTTPS
  - `GITLAB_TOKEN` and `CODEBERG_TOKEN` used for authentication over HTTPS
  - Applied to both `push_to_named_remote` and `push_with_transport_fallbacks`
- **dracon-sync**: `GIT_TERMINAL_PROMPT=0` set on all git push commands
  - Prevents interactive SSH login prompts in daemon, CLI, and tests
- **dracon-sync**: Repo discovery optimization
  - `discover_git_repos_recursive` now skips descending into subdirectories of already-discovered repos
- **dracon-sync**: `HashSet`-based filter-only path matching
  - `git_diff_head_files` returns `HashSet<PathBuf>` for exact path matching
  - Prevents substring collision (e.g., `main.rs` matching `src/main.rs`)
- **dracon-sync**: Visibility cache uses repo path hash as key
  - Prevents same-name repo collisions across different watch roots
- **dracon-system**: `is_protected_ancestor` replaces exact-match path protection
  - `/home` now protects `/home/dracon/Dev`, `/etc` protects `/etc/nginx`, etc.
  - Root path `/` is exact-match only (prevents protecting everything)
- **dracon-system**: `auto_cleanup_apply` guard config (default: `false`)
  - Daemon runs cleanup in dry-run mode by default
  - `auto_truncate_logs` also gated behind `auto_cleanup_apply`
- **dracon-system**: Docker prune respects `apply` gate
  - `docker_prune(false, ...)` returns 0 without invoking docker
- **dracon-system**: PID verification before SIGKILL
  - Reads `/proc/{pid}/cmdline` after SIGTERM wait to confirm PID still belongs to same git process
- **dracon-system**: Strict git process command matching
  - Replaced substring `contains("git")` with `starts_with("git ")` + exact subcmd whitelist
- **dracon-system**: `expand_tilde` fallback changed from `/home` to `.` with logged warning
- **dracon-system**: `process_cpu_percent` default changed from `180.0` to `50.0`
- **dracon-warden**: Binary file passthrough in smudge filter
  - `is_binary_content()` detects null bytes; binary files pass through unchanged
- **dracon-warden**: Individual regex patterns with memory limits
  - `RegexBuilder` with `dfa_size_limit(1_000_000)` and `size_limit(10_000_000)`
  - Prevents ReDoS via catastrophic backtracking
- **dracon-warden**: Path-component matching for sensitive directories
  - `path_components_match` uses `.windows()` for multi-component dirs like `.config/gcloud`
  - `smart_clean_with_path` uses `path.components()` instead of substring `contains`
- **dracon-warden**: Exact filename matching (fixes `coreutils` false positive)
  - `starts_with("core")` replaced with exact match or `"{name}."` prefix
- **dracon-warden**: Regex flag consistency between combined and individual patterns
  - Individual regexes now built from same processed strings as combined regex
- **dracon-warden**: `HomeGuard` test struct with `Drop` impl
  - Locks `HOME_MUTEX`, restores `HOME` on panic — prevents test env leakage
- **CI**: GitHub Actions workflow with `cargo fmt`, `clippy -D warnings`, `test --test-threads=1`
- **Workspace**: `Cargo.toml` with coordinated versions and shared dependencies
- **Workspace**: Root `Cargo.lock` locks 452 packages across all workspace members
- **Services**: `RestartPreventExitStatus=2 78` on all services
- **Services**: `TasksMax=32` on `dracon-system-guard.service`
- **Services**: Narrowed `pkill` pattern to exact match in `ExecStartPre`
- **Services**: `MemoryHigh` documented in README for all 3 services
- **Documentation**: Complete README.md rewrite with quick start, per-utility guides, troubleshooting
- **Documentation**: Example config files for all utilities
- **Documentation**: Comprehensive secrets reference (`~/.dracon/utilities/sync/secrets/README.md`)
- **Documentation**: Token inventory in AGENTS.md with creation URLs and expiry info
- **Scripts**: Enhanced install.sh with --help, --dry-run, --force, --upgrade, --verbose flags
- **Scripts**: New doctor.sh for prerequisite validation
- **Scripts**: Enhanced uninstall.sh with --help, --configs, --logs, --purge flags
- **Scripts**: install.sh now copies example configs on first install
- **Tests**: 104 new tests across all crates (509 total, up from ~405)

### Changed
- **dracon-sync**: Mass-deletion guard graduated threshold — blocks at ≥85% (was 100%)
- **dracon-sync**: Filter-only detection skips `cli_diff_entries` fallback when entries deliberately cleared
  - Prevents encrypted files from being committed as plaintext
- **dracon-sync**: Version bumper prevents double-bump when both `scribe` and `ai-bumper` features enabled
- **dracon-sync**: Scribe runs after version bumper (sees post-bump diff)
- **dracon-sync**: Daemon re-fetches `RepoStatus` before stuck-repo detection
- **dracon-sync**: Index reset (`git reset HEAD --`) when mass-deletion guard blocks
- **dracon-system**: Process monitoring logs ALL heavy processes (not just sustained)
- **dracon-system**: `ps` output changed from `comm=` (15-char truncated) to `args` (full command line)
- **dracon-warden**: Warden example config uses actual settings (`protected_patterns`, `plaintext_patterns`, `hygiene_patterns`)
- **install.sh**: Removed dracon-ai build (not in workspace); fixed nonexistent file references

### Fixed
- **dracon-sync**: Critical bug in mass-deletion safety check — `git ls-files --count` is not a valid git command, causing `.unwrap_or(0)` to silently bypass the guard. Fixed by counting lines of `git ls-files` output.
- **dracon-sync**: `get_bump_info` now supports `pyproject.toml` and gracefully handles first-commit repos (HEAD~1 nonexistent)
- **dracon-sync**: `extract_repo_name` now handles SSH URLs with port (`ssh://git@host:22/owner/repo.git`)
- **dracon-sync**: Divergence diagnosis no longer silently falls back to `unwrap_or(0)` on parse errors
- **dracon-sync**: Merge strategy changed from `git pull --rebase` to `git pull --no-rebase`
- **dracon-system**: Docker prune bypassed dry-run gate — now properly skipped when `!apply`
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
