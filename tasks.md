# Dracon Utilities — Task List

## Completed

- [x] C1: Fix hardcoded `/home/dracon/` paths in dracon-ai
- [x] C2: Fix mutex poison crash in dracon-system guard
- [x] H1: Dead code cleanup (17 suppressions)
- [x] H2: Remove `todo_commit_messages` compiler warning
- [x] H3: Fix clippy warnings (3 issues)
- [x] H4: Extract duplicated cache cleanup helper in dracon-system
- [x] L1: Add `DRACON_SYNC_GIT_BIN` to clap help text
- [x] L2: Add `sha256sum` to install.sh output
- [x] L3: Add TOML field ordering warning to example config
- [x] L4: Add size guard to incident ledger startup
- [x] Fix TOML field ordering — fields after `[[remotes]]` silently ignored
- [x] Fix validator false positives on `[[remotes]]` fields
- [x] Fix `auto_create_account` validation — downgrade to warning
- [x] Fix diverged repo recovery — re-fetch status after pull in repair-concerns
- [x] Fix `create_repo_on_github` — add `--default-branch main` (then API fallback)
- [x] Fix commit message: root files counted as directories
- [x] Fix commit message: DEL/NEW show top 10 instead of top 3
- [x] Fix commit message: `CLOSED:` truncation (60 chars per task, 10 tasks max)
- [x] Fix commit message: `TESTONLY:` shows file names
- [x] Fix `truncate_task` UTF-8 crash — multi-byte char boundary panic
- [x] Remove `scribe` and `ai-bumper` features from sync
- [x] Remove dracon-ai from workspace
- [x] Clean up 25 repos with dual master/main branches
- [x] Scaffold missing LICENSE files
- [x] Add regression tests for `truncate_task`

## Skipped

- [ ] H5: Extract duplicated shutdown signal setup (3x) — over-engineering
- [ ] M3: `once_cell` → `std::sync::OnceLock` — unstable API
- [ ] L5: Test for new branch auto-push — needs git mock infrastructure
- [ ] L6: Test for filter_only_cleared cooldown — needs filter mock

## Backlog

- [ ] M1: Split dracon-system/src/main.rs (3,469 lines)
- [ ] M2: Break long functions — `run_daemon()` (191 lines), `emit_event()` (180 lines)
- [ ] M4: curl → reqwest migration

## Warden Filter Expansion

### Problem

Warden only encrypts files matching `protected_patterns` (`.env`, `config.json`, `*.pem`). Secrets in source code pass through unencrypted. `SecretScanner` with 50+ regex patterns exists but isn't applied to code files.

### Goal

Every `git add` scans file content for secrets and encrypts any found — regardless of file type.

### Tasks

- [ ] Update warden policy `protected_patterns` to include source code: `*.rs`, `*.py`, `*.ts`, `*.js`, `*.go`, `*.sh`, `*.yml`, `*.yaml`, `*.toml`, `*.md`, `*.sql`
- [ ] Update `build_gitattributes_block()` to generate patterns for new file types
- [ ] Update `ensure_warden_filter()` in sync to include new patterns
- [ ] Add `post-index-change` git hook — detects unencrypted secrets after `git add`, re-adds with filter
- [ ] Install hook via git template (`~/.git-templates/hooks/post-index-change`)
- [ ] Set `git config --global init.templateDir ~/.git-templates` for new repos
- [ ] One-time migration: run `dracon-warden once` on all existing repos
- [ ] Test: create repo, add `.rs` file with hardcoded AWS key, verify encryption on `git add`
- [ ] Test: verify files with no secrets pass through unchanged (no false positives)
