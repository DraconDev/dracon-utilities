# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **`dracon-sync` system-repo path bug**: The example template's
  `system_repo` default pointed at a non-git legacy directory. The actual git
  repo where the sync daemon's state lives is `~/.dracon`. The example
  template and the installed `dracon-sync.toml` are now both set to the
  correct path.

### Changed
- **CLI print style**: All three binaries (`dracon-sync`, `dracon-warden`,
  `dracon-system`) now use a consistent visual language for human-facing
  output. The `status` tables include a summary one-liner and grouped
  sections; byte counts and timeouts are formatted as human-readable
  (e.g. `50.0 MiB`, `1m 30s`); freeze/doctor indicators are coloured
  (suppressed when `NO_COLOR` is set). `dracon-system doctor` now emits
  per-check remediation hints. Design note:
  `docs/design/cli-print-style.md`.
- **CLI print polish (round 2)**: Four specific surfaces that were still
  weak have been upgraded. `dracon-sync repos` now has a legend line,
  multi-line icon+label headers, ✅/⚠️/❌ status cells, and a color-aware
  summary (no raw ANSI when piped). `dracon-sync health` now uses a single
  table with a summary one-liner; warnings are grouped into their own
  block with a count. `dracon-warden scrub-markers`/`resmudge`/`repair`/
  `keygen`/`setup-hooks` each print a 2-3 line informative summary, even
  when nothing was changed. `dracon-system events` shows a severity-counts
  footer and a one-line summary before the table. See the
  `docs/design/cli-print-style.md` design note for the full set of
  conventions.

### Added
- **Warden plaintext-sibling escape hatch**: `dracon-warden` now supports an
  opt-in escape hatch for files that should be stored verbatim (not encrypted).
  Touch a `<file>.plaintext` sibling next to any tracked file to opt it in.
  The clean filter returns the file unchanged, the pre-push hook silently
  skips it, and `scrub-markers` / `resmudge` leave it alone. Threat model,
  revocation story, and what the hatch does NOT protect against are in
  `docs/design/warden-plaintext-sibling.md`. Default install behaviour is
  unchanged: no hatch, no plaintext.
- **CI/CD pipeline**: `.github/workflows/ci.yml` — fmt check, clippy, build, serial tests
- **Lint gates**: `#![warn(missing_docs)]` on all 4 crate roots
- **dracon-libs docs**: Fixed all 95 missing-doc warnings in dracon-git
- **Module extraction** (dracon-system): `events.rs`, `links.rs`, `zram.rs`, `doctor.rs`, `safety.rs` — 850 lines, 20% main.rs reduction
- **Module extraction** (dracon-sync): `branch.rs`, `config.rs`, `diff.rs`, `discovery.rs`, `misc.rs`, `multi_remote.rs`, `ops.rs`, `push.rs`, `staging.rs`, `status.rs`, `urls.rs` — 1,846 lines, 45% git/mod.rs reduction
- **Startup cleanup**: Sync daemon prunes stale state on every start/restart — stuck repos, incident ledger retention, visibility cache orphans, guard log rotation
- **Broken tracking repair**: `repair_broken_tracking()` detects `origin/master: gone` refs and re-points to `origin/{branch}` — runs at daemon startup
- **GitHub orphan cleanup script**: `scripts/cleanup-github-orphans.sh` — lists and deletes 83 suffixed orphan + test repos (needs `delete_repo` scope)
- **dracon-libs get_diff fallback**: `get_diff()` now falls back to CLI on libgit2 errors (binary blobs, nul bytes)

### Changed
- **Dead code cleanup**: Removed `git_list_paths` (zero callers), `Level::as_str`/`Event`/`timestamp_secs` from log.rs (unused after JSON→human refactor), gated `fallback_status_rank`/`acquire_path_lock` with `#[cfg(test)]`, fixed all clippy unused-import/never-constructed warnings across all 3 crates
- **Scratch file cleanup**: Removed local task/scratch files and stale task directories from git tracking; added matching `.gitignore` rules.
- **Service restart policy**: All 3 services changed from `Restart=on-failure` to `Restart=always` — daemons now restart even after clean exits, preventing 5+ hour outages
- **CLI output style**: All status commands now use Title Case keys (`Policy:` not `POLICY:`) for consistency with JSON output and health check format
- **Daemon log noise**: Silent when healthy — concern/warn summaries only print when `found > 0`
- **Structured logging**: `log.rs` now prints human-readable `⚠️ message` to stderr instead of raw JSON — JSON incident records stay in the ledger file only
- **Link status**: Prints "No configured links" instead of empty table when 0 links exist
- **dracon-sync**: Scribe refactor — commit messages from diffs, not `project-state.md`
  - `generate_commit_message()`: AI receives current diff (main) + 10 previous diffs (background) + recent subjects → returns subject line
  - `local_fallback_message()`: file-pattern fallback (e.g., "update auth, jwt and 2 files") when AI unavailable
  - Removed `scribe_update()` and `stage_project_state()` — replaced by direct commit message generation
  - Removed `read_project_focus`, `extract_category_scope_from_focus`, `extract_scope_from_focus`, `git_log_recent_subjects`
  - `project-state.md` is now manual-only: sync no longer auto-generates, stages, or commits it
  - `parse_conventional_commit()` extracts (category, scope, description) from AI subject to prevent double-prefix
- **dracon-warden**: Secret scanner pattern fixes
  - Added "Hex Secret (Quoted)" pattern: catches 32+ char mixed-case hex strings in quotes
  - Added "High-Entropy Secret (Quoted)" pattern: catches 24+ char alphanumeric strings in quotes
  - Added "Slack Bot Token (Compact)" pattern: catches `Slack token prefixes without numeric ID segments
  - GitHub token patterns (`GitHub token prefixes): accept 30-40 chars (was exactly 36)
  - Mailgun API Key pattern: accept 28-34 chars (was exactly 32)

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

## [0.112.4] - 2026-06-07

### Fixed
- `dracon-sync/README.md` and `docs/OPERATIONS.md`: replaced flat CLI paths
  (`repair-concerns`, `repair-warns`, `stuck list`, `dual-branch list`,
  `publish-status`, `repair-origins`) with the correct nested subcommand
  syntax (`repair concerns`, `repair stuck-list`, `publish run`, etc.).
  Resolves audit-2026-06-07 P1-2.
- `dracon-warden status` help text and README "Quick Commands" sections
  now say "repo roots" (matching the v0.3.0 `watch_roots` → `repo_roots`
  field rename). Resolves audit-2026-06-07 N-4.
- `dracon-system/README.md` server-deployment systemd snippet: corrected
  resource limits from `MemoryMax=100M CPUQuota=10%` to `MemoryMax=250M
  CPUQuota=20%` (matching `dracon-system-guard.service`).
- Removed `dracon-sync/note.md` (leftover investigation note from a May
  incident). Added gitignore rule so future `note.md` files are not
  tracked. Resolves audit-2026-06-07 P2-5.
- Untracked 4 stale tarpaulin coverage reports (~1.6 MB) across all 3
  binaries. Added `**/tarpaulin-report.*` to `.gitignore`. Resolves
  audit-2026-06-07 P2-4.
- Removed dead `let discover = effective_discovery_roots(&policy);`
  binding in `dracon-warden/src/main.rs:1356` (the result was never
  used; `explicit_discover` was built directly from `policy.discover_roots`).

### Changed
- Workspace version bumped 0.112.3 → 0.112.4 (hygiene-only release, no
  per-crate version changes).
- `dracon-system/src/print.rs` and `dracon-warden/src/print.rs`: added
  module-level `#![allow(dead_code)]` with a doc comment explaining the
  public-API intent (helpers for shared output formatting, awaiting
  callers). Resolves audit-2026-06-07 N-1.

### Audit
- **Audit hygiene**: internal audit artifacts were reviewed during release prep and are not included in the public tree. User-facing release notes and operational docs now carry the public guidance.

## [0.3.0] - 2026-06-07

### Breaking
- **`dracon-warden` `watch_roots` field renamed to `repo_roots`**: The old
  name was misleading (warden has no daemon mode and does not watch
  filesystems; the field is a list of directories to scan for git repos
  on demand). The canonical field is now `repo_roots`. The example toml,
  user guide, and BLUEPRINT all use the new name.

### Deprecated
- **`watch_roots` is still accepted** for backwards compatibility. When
  the old key is set (alone or alongside `repo_roots`), the policy still
  loads, but:
  - A deprecation warning is logged to stderr:
    `warning: 'watch_roots' is deprecated, use 'repo_roots' instead`
  - A yellow ⚠ row appears in `dracon-warden status`
  - When both keys are set, `repo_roots` wins and a different message
    indicates the conflict
  This alias will be removed in a future major release.

### Fixed
- **`dracon-warden` status no longer shows two identical root rows**:
  Previously the status table showed `🛡️ Watch roots` and
  `🧭 Discovery roots` as separate rows that were identical when
  `discover_roots` was unset. The status is now consolidated to a single
  `🔍 Repo roots` row, with an explicit `🧭 Discovery roots (additional)`
  row only when the user has set a non-empty `discover_roots` that
  extends the `repo_roots` set.

### Changed
- **`dracon-warden` legacy path removed from default config**: The
  example toml and the installed user config no longer include a legacy
  non-git directory. The directory itself is not deleted; the user can
  decide what to do with its contents.

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

