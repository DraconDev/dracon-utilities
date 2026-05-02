# Work Plan: Multi-Remote Test Suite

## Goal
Achieve comprehensive test coverage for the GitLab and Codeberg multi-remote mirroring additions to dracon-sync, with all tests passing and clippy clean.

## Constraints
- **No `mockall`**: Stick with existing integration-test pattern (real git commands on temp dirs)
- **No external CLI mocking**: Test gh/glab error paths only; skip if tools unavailable
- **Clippy clean**: `-D warnings` must pass
- **All tests pass**: 257+ tests
- **Minimal refactors**: Only refactor what's needed for testability

---

## Success Criteria
- [ ] All functions in `git.rs` multi-remote module have tests
- [ ] `resolve_push_url` has tests
- [ ] `remove_stale_remotes` has tests (origin protection, stale cleanup)
- [ ] Codeberg HTTP path tested with wiremock
- [ ] Git push paths tested (SSH, fallback, retry)
- [ ] External CLI error paths tested ("already exists" parsing)
- [ ] Full `push_mirror_remotes` integration flow tested
- [ ] `cargo test --all-targets` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes

---

## Phase 1: Easy Wins (No Refactors Needed)

### 1.1 Test `resolve_push_url` (policy.rs)
**Estimated**: 5 min  
**Why**: Pure string manipulation, zero dependencies.

Add tests for:
- `{repo}` substitution
- `{account}` substitution
- Both together
- No templates (passthrough)

```rust
#[test]
fn test_resolve_push_url_with_templates() {
    let config = RemoteConfig {
        name: "github".to_string(),
        push_url: "git@github.com:{account}/{repo}.git".to_string(),
        auto_create_account: "DraconDev".to_string(),
        ..Default::default()
    };
    assert_eq!(
        config.resolve_push_url("my-repo"),
        "git@github.com:DraconDev/my-repo.git"
    );
}
```

### 1.2 Test `remove_stale_remotes`
**Estimated**: 10 min  
**Why**: Uses real git commands, follows existing pattern.

Add tests for:
- Removes stale remotes not in keep list
- Preserves `origin` (critical bug we fixed)
- Preserves remotes in keep list
- Empty keep list removes all except origin

### 1.3 Test `configure_all_remotes`
**Estimated**: 10 min  
**Why**: Uses `ensure_remote` internally, already tested.

Add tests for:
- Configures multiple remotes with resolved URLs
- Handles pre-existing remotes (updates URL)
- Skips on error (logs warning, continues)

### 1.4 Test `auto_create_all_remotes`
**Estimated**: 15 min  
**Why**: Filters by `auto_create`, delegates to platform functions.

Add tests for:
- Only processes `auto_create=true` remotes
- Returns correct structure `(name, Result)`
- Empty input returns empty output

### 1.5 Test `load_secret` with file-based secrets
**Estimated**: 10 min  
**Why**: Currently only env var tested. File loading is critical for Codeberg.

Add tests for:
- Loads from `~/.dracon/utilities/sync/secrets/*.env`
- Skips empty lines and comments
- Returns `None` if file doesn't exist
- Handles multiple `VAR=value` pairs in one file

---

## Phase 2: HTTP Testing (Codeberg)

### 2.1 Add wiremock dev-dependency
**Estimated**: 2 min  
**File**: `Cargo.toml`

```toml
[dev-dependencies]
tempfile = "3"
wiremock = "0.6"
```

### 2.2 Refactor `create_repo_on_codeberg` for testability
**Estimated**: 10 min  
**File**: `git.rs`

Current signature:
```rust
pub(crate) fn create_repo_on_codeberg(token: &str, account: &str, repo_name: &str, api_endpoint: &str) -> Result<String>
```

Refactor to accept client:
```rust
pub(crate) async fn create_repo_on_codeberg(
    client: &reqwest::Client,
    token: &str,
    account: &str,
    repo_name: &str,
    api_endpoint: &str,
) -> Result<String>
```

Update call sites in `auto_create_repo` to pass `&reqwest::Client::new()`.

### 2.3 Test `create_repo_on_codeberg`
**Estimated**: 20 min  
**File**: `git.rs` tests

Add tests using wiremock:
- **Success (201)**: Returns correct SSH URL
- **Already exists (409)**: Returns SSH URL without error
- **Already exists (422)**: Returns SSH URL without error
- **Auth failure (401)**: Returns error with status
- **Server error (500)**: Returns error with body
- **Network failure**: Returns error

### 2.4 Test `auto_create_repo` for Codeberg routing
**Estimated**: 15 min  
**File**: `git.rs` tests

Since `auto_create_repo` calls `create_repo_on_codeberg`, we need:
- Test with `AuthType::Codeberg` and mock client
- Test `AuthType::Generic` returns error
- Test missing token returns error

**Challenge**: `auto_create_repo` is sync, but `create_repo_on_codeberg` becomes async. Need to `block_on` in the sync context or make `auto_create_repo` async.

**Decision**: Make `auto_create_repo` async (it's only called from `auto_create_all_remotes` which is already async-adjacent).

---

## Phase 3: Git Push Testing

### 3.1 Test `push_to_named_remote`
**Estimated**: 20 min  
**File**: `git.rs` tests

Add tests for:
- **SSH push success**: Use a temp bare repo as remote
- **HTTPS fallback**: Mock with local HTTP server or skip (complex)
- **Retry behavior**: Configure remote that fails first N times
- **Remote not found**: Returns error

**Approach**: Set up a local bare repo, add it as a remote, push to it.

```rust
#[tokio::test]
async fn test_push_to_named_remote_success() {
    let tmp = tempfile::tempdir().unwrap();
    let remote_repo = tmp.path().join("remote");
    let local_repo = tmp.path().join("local");
    
    // Init bare remote
    std::process::Command::new("git")
        .args(["init", "--bare", "-b", "master"])
        .arg(&remote_repo)
        .status()
        .unwrap();
    
    // Init local with commit
    std::process::Command::new("git")
        .args(["init", "-q", "-b", "master"])
        .arg(&local_repo)
        .status()
        .unwrap();
    // ... add remote, commit, push
}
```

### 3.2 Test `push_to_all_remotes`
**Estimated**: 15 min  
**File**: `git.rs` tests

Add tests for:
- **Priority ordering**: Lower priority pushed first
- **Error collection**: One remote fails, others succeed
- **All fail**: Returns all errors
- **Empty remotes**: Returns empty results

### 3.3 Test `push_mirror_remotes`
**Estimated**: 20 min  
**File**: `git.rs` tests

Add integration test:
- Configures remotes
- Attempts auto-create (will fail without credentials, that's OK)
- Cleans stale remotes
- Pushes to configured remotes
- Verifies remotes exist and have correct URLs

---

## Phase 4: External CLI Error Paths

### 4.1 Test `create_repo_on_github` error parsing
**Estimated**: 10 min  
**File**: `git.rs` tests

Since we can't mock `Command::new`, test:
- Skip if `gh` unavailable (check with `which gh` or `gh --version`)
- If available, test "already exists" path (create repo twice)
- Test error message parsing on failure

**Guardrail**: Use `#[ignore]` or conditional compilation for tests requiring `gh`.

### 4.2 Test `create_repo_on_gitlab` error parsing
**Estimated**: 10 min  
**File**: `git.rs` tests

Same approach as GitHub.

### 4.3 Test `auto_create_repo` platform routing
**Estimated**: 10 min  
**File**: `git.rs` tests

Test without external tools:
- `AuthType::Generic` returns error
- `AuthType::Codeberg` with missing token returns error
- `AuthType::GitHub`/`GitLab` when tool unavailable returns error

---

## Phase 5: sync.rs Integration Tests

### 5.1 Test multi-remote push in `sync_repo`
**Estimated**: 20 min  
**File**: `sync.rs` tests

Add test:
- Repo with origin and 1 mirror remote
- Commit changes
- `sync_repo` pushes to origin then mirror
- Verify both remotes have the commit

### 5.2 Test remote failure tracking in daemon
**Estimated**: 15 min  
**File**: `daemon.rs` tests

Add test:
- Simulate push failure to mirror remote
- Verify `remote_failures` map incremented
- Verify cooldown notification logic

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `auto_create_repo` async refactor breaks callers | Medium | High | Check all call sites, compile after each change |
| Wiremock tests flaky in CI | Low | Medium | Use fixed ports, add retries |
| Git push tests require SSH keys | Medium | Medium | Use `file://` protocol for local remotes |
| External CLI tests fail in CI | Medium | Low | Mark with `#[ignore]` or skip conditionally |
| Scope creep (testing unrelated code) | Medium | Medium | Strict function list, stop at 5 phases |

## Decision Log

1. **No mockall**: Stick with integration-test pattern. Adding mockall is scope creep.
2. **No CLI mocking**: Test error paths only. Mocking `Command::new` is too invasive.
3. **Wiremock for Codeberg**: Only external dependency added. Justified because HTTP is deterministic.
4. **Make `create_repo_on_codeberg` async**: Required for wiremock testing. Propagates to `auto_create_repo`.
5. **Use `file://` protocol for git push tests**: Avoids SSH key requirements in CI.

## Files to Modify

1. `dracon-sync/Cargo.toml` - add wiremock
2. `dracon-sync/src/git.rs` - refactor + add tests
3. `dracon-sync/src/policy.rs` - add tests for `resolve_push_url`
4. `dracon-sync/src/sync.rs` - add integration tests
5. `dracon-sync/src/daemon.rs` - add remote failure tests

## Total Estimated Time

- Phase 1: 50 min
- Phase 2: 47 min
- Phase 3: 55 min
- Phase 4: 30 min
- Phase 5: 35 min
- **Total**: ~3.5 hours

## Next Steps

1. ✅ Plan complete
2. ⏭️ Run `/start-work` to begin implementation
3. ⏭️ Execute phases in order
4. ⏭️ Run `cargo test && cargo clippy` after each phase
