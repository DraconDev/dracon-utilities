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

## Warden ↔ Sync Decoupling — Hook-Based Encryption Guarantee

### Context

Currently sync calls `dracon-warden once <repo>` via subprocess before every push (`ensure_warden_filter()`). This couples the two tools and has failure modes where warden isn't installed, fails silently, or the subprocess call is bypassed.

### Goal

- **Warden and sync are completely independent.** Sync never calls warden. Warden never calls sync.
- **Encryption guarantee comes from a git pre-commit hook.** If the filter isn't configured, commits are blocked.
- **Warden daemon is no longer required.** The hook is the enforcement layer. Warden is just a binary + hooks.
- **Warden can be deployed standalone** to servers without needing sync or committing anything.

### Architecture

```
Git hooks (installed globally via core.hooksPath):
  pre-commit  → validates filter exists → blocks commit if missing
  pre-push    → scans diff for plaintext secrets (catches --no-verify bypass)

Warden binary:
  dracon-warden once <repo>  → sets up filter + gitattributes
  dracon-warden setup-hooks  → installs hooks to ~/.config/git/hooks/

Sync daemon:
  stages files, commits, pushes — no warden knowledge at all
```

### Tasks

#### Warden: Add `setup-hooks` command

- [ ] Add `SetupHooks` variant to `Command` enum in `dracon-warden/src/main.rs`
- [ ] Implement `run_setup_hooks()`:
  - Create `~/.config/git/hooks/` directory if missing
  - Write `pre-commit` hook script that validates `filter=dracon` in `.gitattributes` and `filter.dracon.clean` in git config
  - Set executable permissions (`chmod +x`)
  - Run `git config --global core.hooksPath ~/.config/git/hooks/`
  - Print confirmation
- [ ] Add `--global` flag (default) and `--local` flag to `setup-hooks` command
  - `--global`: sets `core.hooksPath` globally, hooks apply to all repos
  - `--local`: installs hooks into a specific repo's `.git/hooks/`, sets `core.hooksPath` locally
- [ ] Add help text and examples to clap

#### Warden: Write pre-commit hook

- [ ] Create hook script (written by `setup-hooks`):
  ```bash
  #!/bin/sh
  # Validates warden filter is configured before commit
  REPO=$(git rev-parse --show-toplevel)
  
  # Check .gitattributes has filter=dracon patterns
  if ! grep -q "filter=dracon" "$REPO/.gitattributes" 2>/dev/null; then
    echo "❌ Warden filter missing from .gitattributes."
    echo "   Run: dracon-warden once $REPO"
    exit 1
  fi
  
  # Check git config has filter.dracon.clean set
  if ! git -C "$REPO" config filter.dracon.clean >/dev/null 2>&1; then
    echo "❌ Warden filter not configured in git config."
    echo "   Run: dracon-warden once $REPO"
    exit 1
  fi
  
  # Check filter binary is on PATH
  if ! command -v dracon-warden >/dev/null 2>&1; then
    echo "❌ dracon-warden binary not found on PATH."
    echo "   Install it or add to PATH."
    exit 1
  fi
  ```
- [ ] Hook checks three preconditions:
  1. `.gitattributes` contains `filter=dracon`
  2. `git config filter.dracon.clean` is set
  3. `dracon-warden` binary is on PATH
- [ ] If any check fails: print clear error with fix command, exit non-zero (blocks commit)

#### Warden: Add `pre-push` hook

- [ ] Write `pre-push` hook script:
  ```bash
  #!/bin/sh
  # Defense-in-depth: scan push for plaintext secrets
  # Catches --no-verify bypass of pre-commit hook
  REPO=$(git rev-parse --show-toplevel)
  # Check for common secret patterns in the diff being pushed
  git diff origin/main..HEAD 2>/dev/null | grep -qE \
    "(AKIA[A-Z0-9]{16}|-----BEGIN|password\s*=|secret\s*=|api_key\s*=)" && {
    echo "⚠️  Possible plaintext secrets detected in push."
    echo "   This means the warden filter was bypassed."
    echo "   Run: dracon-warden once $REPO"
    exit 1
  }
  ```
- [ ] Only fires if pre-commit was bypassed with `--no-verify`
- [ ] Scans the diff being pushed for common secret patterns (AWS keys, PEM headers, password assignments)
- [ ] Pattern list should be minimal and non-chatty — only real secrets, not false positives

#### Warden: Update `harden_repo()` to install hooks

- [ ] After setting up `.gitattributes` and filter config, also run hook installation
- [ ] This ensures `dracon-warden once <repo>` both configures the filter AND installs the hook
- [ ] Only install if not already present (check for hook file existence)

#### Warden: Remove or deprecate daemon

- [ ] Mark `daemon` command as deprecated in help text
- [ ] Keep daemon code functional for backward compatibility but not required
- [ ] Update `dracon-warden.service` systemd unit:
  - Option A: Remove the service entirely from `install.sh`
  - Option B: Keep service but add comment explaining it's optional
- [ ] The hook-based approach means the daemon is only needed for:
  - Proactive hardening of repos that haven't been committed to yet
  - Policy reload via SIGHUP
  - Periodic sweep as safety net (not required for security)

#### Warden: Add to `install.sh`

- [ ] Add `setup-hooks` call to `install.sh` after binary installation
- [ ] Ensure `core.hooksPath` is set globally during install
- [ ] One-time setup — no per-repo action needed

#### Sync: Remove `ensure_warden_filter()`

- [ ] Delete `ensure_warden_filter()` function from `dracon-sync/src/sync.rs` (lines 1367-1405)
- [ ] Remove the call at line 1851 in `stage_commit_and_push()`
- [ ] Remove any warden-related imports or references
- [ ] Sync becomes completely warden-unaware — no subprocess calls, no validation

#### Migration

- [ ] Existing repos with warden filter already configured continue working (hooks are additive)
- [ ] Existing repos without hooks get them on next `dracon-warden once` or install
- [ ] No breaking changes — old filter config still works, hooks are new enforcement layer
- [ ] Update `AGENTS.md` to reflect new architecture

#### Testing

- [ ] Test: fresh clone → commit → pre-commit hook blocks if filter missing
- [ ] Test: `dracon-warden once <repo>` → filter configured + hook installed → commit succeeds
- [ ] Test: delete `.gitattributes` filter entry → commit blocked with clear error
- [ ] Test: delete git config filter entry → commit blocked with clear error
- [ ] Test: `--no-verify` bypasses hook (expected — user chose to skip)
- [ ] Test: `setup-hooks --local` installs to specific repo
- [ ] Test: `setup-hooks --global` sets global `core.hooksPath`
- [ ] Test: `--no-verify` bypasses pre-commit but pre-push catches plaintext secrets
- [ ] Test: sync works without warden installed (no errors, no warnings)
- [ ] Test: warden works without sync installed (standalone)

#### Documentation

- [ ] Update `AGENTS.md` — remove warden daemon dependency from sync section
- [ ] Update `dracon-warden.example.toml` — add `setup-hooks` documentation
- [ ] Add `SETUP.md` or section in README for standalone warden deployment
- [ ] Document server deployment: copy binary, run `dracon-warden setup-hooks`, done
- [ ] Document the two hooks: pre-commit (core guarantee) and pre-push (defense-in-depth)
