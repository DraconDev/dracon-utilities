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

---

## Skipped

- [ ] H5: Extract duplicated shutdown signal setup (3x) — over-engineering
- [ ] M3: `once_cell` → `std::sync::OnceLock` — unstable API
- [ ] L5: Test for new branch auto-push — needs git mock infrastructure
- [ ] L6: Test for filter_only_cleared cooldown — needs filter mock

---

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

- [x] Add `SetupHooks` variant to `Command` enum in `dracon-warden/src/main.rs`
- [x] Implement `run_setup_hooks()`:
  - Create `~/.config/git/hooks/` directory if missing
  - Write `pre-commit` hook script that validates `filter=dracon` in `.gitattributes` and `filter.dracon.clean` in git config
  - Set executable permissions (`chmod +x`)
  - Run `git config --global core.hooksPath ~/.config/git/hooks/`
  - Print confirmation
- [x] Add `--global` flag (default) and `--local` flag to `setup-hooks` command
  - `--global`: sets `core.hooksPath` globally, hooks apply to all repos
  - `--local`: installs hooks into a specific repo's `.git/hooks/`, sets `core.hooksPath` locally
- [x] Add help text and examples to clap

#### Warden: Write pre-commit hook

- [x] Create hook script (written by `setup-hooks`):
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
- [x] Hook checks three preconditions:
  1. `.gitattributes` contains `filter=dracon`
  2. `git config filter.dracon.clean` is set
  3. `dracon-warden` binary is on PATH
- [x] If any check fails: print clear error with fix command, exit non-zero (blocks commit)

#### Warden: Add `pre-push` hook

- [x] Write `pre-push` hook script:
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
- [x] Only fires if pre-commit was bypassed with `--no-verify`
- [x] Scans the diff being pushed for common secret patterns (AWS keys, PEM headers, password assignments)
- [x] Pattern list should be minimal and non-chatty — only real secrets, not false positives

#### Warden: Update `harden_repo()` to install hooks

- [x] After setting up `.gitattributes` and filter config, also run hook installation
- [x] This ensures `dracon-warden once <repo>` both configures the filter AND installs the hook
- [x] Only install if not already present (check for hook file existence)

#### Warden: Remove or deprecate daemon

- [x] Mark `daemon` command as deprecated in help text
- [x] Keep daemon code functional for backward compatibility but not required
- [x] Update `dracon-warden.service` systemd unit:
  - Option A: Remove the service entirely from `install.sh`
  - Option B: Keep service but add comment explaining it's optional
- [x] The hook-based approach means the daemon is only needed for:
  - Proactive hardening of repos that haven't been committed to yet
  - Policy reload via SIGHUP
  - Periodic sweep as safety net (not required for security)

#### Warden: Add to `install.sh`

- [x] Add `setup-hooks` call to `install.sh` after binary installation
- [x] Ensure `core.hooksPath` is set globally during install
- [x] One-time setup — no per-repo action needed

#### Sync: Remove `ensure_warden_filter()`

- [x] Delete `ensure_warden_filter()` function from `dracon-sync/src/sync.rs` (lines 1367-1405)
- [x] Remove the call at line 1851 in `stage_commit_and_push()`
- [x] Remove any warden-related imports or references
- [x] Sync becomes completely warden-unaware — no subprocess calls, no validation

#### Migration

- [x] Existing repos with warden filter already configured continue working (hooks are additive)
- [x] Existing repos without hooks get them on next `dracon-warden once` or install
- [x] No breaking changes — old filter config still works, hooks are new enforcement layer
- [x] Update `AGENTS.md` to reflect new architecture

#### Testing

- [ ] Test: fresh clone → commit → pre-commit hook blocks if filter missing
- [ ] Test: `dracon-warden once <repo>` → filter configured + hook installed → commit succeeds
- [ ] Test: delete `.gitattributes` filter entry → commit blocked with clear error
- [ ] Test: delete git config filter entry → commit blocked with clear error
- [ ] Test: `--no-verify` bypasses pre-commit hook (expected — user chose to skip)
- [ ] Test: `setup-hooks --local` installs to specific repo
- [ ] Test: `setup-hooks --global` sets global `core.hooksPath`
- [ ] Test: `--no-verify` bypasses pre-commit but pre-push catches plaintext secrets
- [ ] Test: sync works without warden installed (no errors, no warnings)
- [ ] Test: warden works without sync installed (standalone)

#### Documentation

- [x] Update `AGENTS.md` — remove warden daemon dependency from sync section
- [x] Update `dracon-warden.example.toml` — add `setup-hooks` documentation
- [ ] Add `SETUP.md` or section in README for standalone warden deployment
- [x] Document server deployment: copy binary, run `dracon-warden setup-hooks`, done
- [x] Document the two hooks: pre-commit (core guarantee) and pre-push (defense-in-depth)

---

## Audit: HIGH Priority Tasks

### H-PUSH-TEST: Fix Push Test Suite (7 failures)

**Priority:** HIGH
**Component:** dracon-sync
**Impact:** 7 push-related tests failing, blocking CI confidence

**Root Cause:**
Tests create mock git binaries via `std::process::Command::new("git")` and modify `PATH` to point to them. Even with `--test-threads=1`, concurrent `Command` resolution can pick up stale PATH values from previous tests. The `acquire_path_lock()` mechanism only serializes tests that explicitly acquire it — push tests don't.

**Affected Tests:**
1. `test_push_to_named_remote_auto_force_when_behind`
2. `test_push_to_named_remote_ssh_fails_https_fallback`
3. `test_push_to_named_remote_ssh_success`
4. `test_push_with_retries_retries_then_succeeds`
5. `test_push_with_retries_succeeds_first_attempt`
6. `test_push_with_transport_fallbacks_ssh_fails_https_fallback_succeeds`
7. `test_push_with_transport_fallbacks_ssh_succeeds_no_fallback`

**Recommended Fix:**
- Option A: Mock `Command::new("git")` at the function level using a test wrapper, not PATH manipulation
- Option B: Create a `GitMock` fixture that wraps all git calls and runs in-process
- Option C: Use process-level isolation (fork per test) — heavy but guaranteed isolation

**Acceptance Criteria:**
- All 7 push tests pass reliably with `cargo test -- --test-threads=1`
- No PATH manipulation in test setup
- Tests pass in parallel (`cargo test` without `--test-threads=1`)

---

### H-SEC-LIB: Split Warden Security Lib (3,500 lines)

**Priority:** HIGH
**Component:** dracon-warden
**Impact:** Maintainability, code navigation, test isolation

**Current State:**
`dracon-warden/src/security/src/lib.rs` is a single 3,500-line file containing:
- `DemonSecurity` struct with 20+ methods
- V1/V2 encryption format handling
- Smart clean/smudge filter logic
- Secret scanning regex patterns
- Backup/restore system
- Team key management
- Key generation

**Recommended Splits:**
```
dracon-warden/src/security/src/
├── lib.rs              (re-exports, DemonSecurity struct definition)
├── crypto.rs           (encrypt_v2, decrypt_v2, encrypt_with_repo_key, decrypt_with_repo_key)
├── filter.rs           (smart_clean, smart_smudge, seal_clean, seal_smudge)
├── scanner.rs          (secret patterns, scan_for_secrets, scrub_markers)
├── backup.rs           (backup_file, backup_secret, restore_file)
├── team.rs             (create_team, load_team_key, team_key_exists)
└── keygen.rs           (generate_master_identity, ensure_current_user_key)
```

**Acceptance Criteria:**
- Each module is < 800 lines
- All 65 warden tests still pass
- Public API unchanged (DemonSecurity struct methods remain accessible)
- No circular dependencies between modules

---

### H-DAEMON: Extract Sync Daemon Cooldown Manager

**Priority:** HIGH
**Component:** dracon-sync
**Impact:** Maintainability, reduces daemon.rs complexity

**Current State:**
`daemon.rs` (1,324 lines) manages 5+ tracking maps:
- `activity: HashMap<PathBuf, RepoActivity>` — fingerprint + dirty_since + ahead_since + behind_since
- `repair_cooldowns: HashMap<PathBuf, Instant>` — prevents repair storms
- `filter_cooldowns: HashMap<PathBuf, Instant>` — prevents tight re-check loops
- `stuck_push_repos: HashMap<PathBuf, u64>` — permanently stuck repos
- `remote_notify_cooldowns: HashMap<String, Instant>` — webhook dedup
- `pending_repos: HashMap<PathBuf, Instant>` — repos waiting for initial scan

**Recommended Extraction:**
```rust
struct CooldownManager {
    repair: HashMap<PathBuf, Instant>,
    filter: HashMap<PathBuf, Instant>,
    remote_notify: HashMap<String, Instant>,
    pending: HashMap<PathBuf, Instant>,
}

impl CooldownManager {
    fn is_repair_cooldown_active(&self, repo: &Path, cooldown_secs: u64) -> bool { ... }
    fn set_repair_cooldown(&mut self, repo: PathBuf, cooldown_secs: u64) { ... }
    fn is_filter_cooldown_active(&self, repo: &Path) -> bool { ... }
    fn set_filter_cooldown(&mut self, repo: PathBuf, cooldown_secs: u64) { ... }
    fn prune_stale(&mut self, max_age: Duration) { ... }
}
```

**Acceptance Criteria:**
- `daemon.rs` drops below 1,000 lines
- All 329 sync tests still pass
- Cooldown logic is testable in isolation
- No behavioral changes — pure refactor

---

## Audit: MEDIUM Priority Tasks

### M-PUSH-DOC: Document Push Failure Decision Tree

**Priority:** MEDIUM
**Component:** dracon-sync
**Impact:** Developer understanding, debugging push failures

**Current State:**
Push logic spans 3 files with complex fallback chains:
1. `push_with_retries()` → SSH retry loop
2. `push_with_transport_fallbacks()` → SSH → HTTPS fallback
3. `push_https_fallback()` → GitHub → GitLab → Codeberg

**Recommended Documentation:**
Add to README or inline docs:
```
Push Decision Tree:
  1. SSH push with retry (1-5s backoff, N attempts)
     └─ Success → done
     └─ Failure → continue
  2. Transport fallback (SSH → HTTPS)
     └─ GitHub HTTPS (no token needed for public repos)
     └─ GitLab HTTPS (GITLAB_TOKEN required)
     └─ Codeberg HTTPS (CODEBERG_TOKEN required)
     └─ All fail → log incident, mark repo as stuck if clean+ahead
```

**Acceptance Criteria:**
- Decision tree documented in README or inline docs
- Covers all failure modes: timeout, auth, network, diverged
- Includes recovery steps for each failure type

---

### M-PARALLEL-PUSH: Parallel Mirror Pushes

**Priority:** MEDIUM
**Component:** dracon-sync
**Impact:** Sync speed for multi-mirror repos

**Current State:**
Mirror pushes are sequential. For a repo with origin + 2 mirrors:
- Origin push: 60s timeout
- Mirror 1: 60s timeout
- Mirror 2: 60s timeout
- Worst case: 180s per repo

**Recommended Change:**
```rust
// Push to all remotes concurrently
let mut handles = vec![];
handles.push(tokio::spawn(push_to_origin(repo, timeout)));
for remote in &policy.remotes {
    handles.push(tokio::spawn(push_to_mirror(repo, remote, timeout)));
}
// Wait for all, collect results
```

**Acceptance Criteria:**
- Origin + mirrors pushed concurrently
- Per-remote timeout still enforced (60s each)
- Failure in one mirror doesn't block others
- All 329 sync tests still pass

---

### M-CLIPPY: Fix Warden Clippy Warning

**Priority:** MEDIUM
**Component:** dracon-warden
**Impact:** Code quality, zero-warning builds

**Warning:**
```
warning: the borrowed expression implements the required traits
  → dracon-warden/src/main.rs
help: change this to: `repo_path`
```

**Fix:**
```rust
// Before
let output = std::process::Command::new("git")
    .arg("config")
    .arg("core.hooksPath")
    .arg(&repo_path)  // ← needless borrow
    .output()?;

// After
let output = std::process::Command::new("git")
    .arg("config")
    .arg("core.hooksPath")
    .arg(repo_path)   // ← direct move
    .output()?;
```

**Acceptance Criteria:**
- `cargo clippy -p dracon-warden` produces 0 warnings
- All 65 warden tests still pass

---

### M-INTEGRATION: Add Integration Test Suite

**Priority:** MEDIUM
**Component:** dracon-sync + dracon-warden
**Impact:** Catch end-to-end bugs

**Current State:**
Most tests are unit tests with mock git. No tests exercise the full sync cycle with real git repos.

**Recommended Tests:**
```
tests/
├── sync_integration.rs     (full sync cycle: dirty → stage → commit → push)
├── warden_integration.rs   (full cycle: plaintext → encrypt → commit → decrypt)
└── cross_tool.rs           (sync + warden independence verification)
```

**Acceptance Criteria:**
- At least 10 integration tests per component
- Tests use real git repos (tempdir), not mocks
- Tests verify: file changes → git state → remote state
- Runs in CI with `cargo test --test integration`

---

## Audit: LOW Priority Tasks

### L-REMOVE-DAEMON: Remove Deprecated Warden Daemon Code

**Priority:** LOW
**Component:** dracon-warden
**Impact:** Reduce maintenance burden

**Current State:**
Warden daemon is deprecated but code still exists:
- `daemon` subcommand handler
- Filesystem watcher (notify crate)
- Event debounce logic
- Policy reload via SIGHUP

**Recommended Action:**
- Remove `daemon` subcommand from CLI
- Remove filesystem watcher code
- Keep `setup-hooks` as the only daemon-like functionality
- Remove `dracon-warden.service` from `install.sh`

**Acceptance Criteria:**
- `dracon-warden daemon` prints "deprecated, use setup-hooks instead"
- No filesystem watcher dependency
- All 65 warden tests still pass
- Binary size reduced by ~50KB

---

### L-HEALTH-ENDPOINT: Add Daemon Health Check Socket

**Priority:** LOW
**Component:** dracon-sync
**Impact:** Monitoring, systemd integration

**Current State:**
Daemon health is checked via `dracon-sync health` CLI command. No programmatic endpoint.

**Recommended Change:**
- Create Unix socket at `~/.local/state/dracon/sync.sock`
- Listen for JSON health requests
- Return: policy valid, daemon responsive, repo counts, last sync time
- Systemd can use `ExecStartPost` to check socket

**Acceptance Criteria:**
- Socket created on daemon start, removed on stop
- `curl --unix-socket ~/.local/state/dracon/sync.sock http://localhost/health` returns JSON
- Systemd service uses socket for health checks

---

### L-ASYNC-UNIFY: Unify Sync Git Calls to Async

**Priority:** LOW
**Component:** dracon-sync
**Impact:** Consistency, performance

**Current State:**
Some git calls use `tokio::process::Command`, others use `std::process::Command`. Mixed async/sync.

**Recommended Change:**
- Audit all `std::process::Command::new("git")` calls in sync
- Convert to `tokio::process::Command` where possible
- Keep `std::process::Command` only for blocking operations in sync context

**Acceptance Criteria:**
- All git calls in sync use async variants
- No blocking in tokio runtime
- All 329 sync tests still pass

---

### L-SEC-PATTERNS: Review Smart Clean Regex Patterns

**Priority:** LOW
**Component:** dracon-warden
**Impact:** Security coverage

**Current State:**
Smart clean uses regex patterns to detect secrets. Patterns are hardcoded string constants.

**Recommended Action:**
- Audit current patterns against OWASP Secrets Cheat Sheet
- Add patterns for: GCP service accounts, Azure keys, Discord tokens, Slack webhooks
- Move patterns to config file for user extensibility
- Add test for each new pattern

**Acceptance Criteria:**
- All common cloud provider secret formats detected
- Patterns configurable via `dracon-warden.toml`
- Each pattern has a test case

---

## Backlog

- [ ] M1: Split dracon-system/src/main.rs (3,469 lines)
- [ ] M2: Break long functions — `run_daemon()` (191 lines), `emit_event()` (180 lines)
- [ ] M4: curl → reqwest migration
- [ ] H7: Fix release test `test_release_pipeline_tag_only_when_auto_tag_true`
- [ ] H8: Fix diff test `test_fallback_entries_recalculate_staged_files`

---

## Audit Summary

| Priority | Tasks | Status |
|----------|-------|--------|
| HIGH | 3 | H-PUSH-TEST, H-SEC-LIB, H-DAEMON |
| MEDIUM | 4 | M-PUSH-DOC, M-PARALLEL-PUSH, M-CLIPPY, M-INTEGRATION |
| LOW | 4 | L-REMOVE-DAEMON, L-HEALTH-ENDPOINT, L-ASYNC-UNIFY, L-SEC-PATTERNS |
| **Total** | **11** | |

**Test Health:**
- dracon-sync: 376/402 pass (93.5%) — 26 failures (mostly push tests)
- dracon-warden: 64/65 pass (98.5%) — 1 flaky failure
- Combined: 440/467 pass (94.2%)
