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

## Phase 2: HTTP Testing (Codeberg) — REVISED

### 2.1 Refactor `create_repo_on_codeberg` to use blocking client
**Estimated**: 5 min  
**File**: `git.rs`

Current code uses `tokio::runtime::Handle::current().block_on()` inside a sync function — fragile.

Refactor to use `reqwest::blocking::Client`:

```rust
pub(crate) fn create_repo_on_codeberg(
    token: &str,
    account: &str,
    repo_name: &str,
    api_endpoint: &str,
) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(api_endpoint)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "name": repo_name,
            "private": true,
            "default_branch": "master"
        }))
        .send()
        .with_context(|| "codeberg repo create failed")?;

    let status = response.status();
    if status.as_u16() == 409 || status.as_u16() == 422 {
        return Ok(format!("git@codeberg.org:{}/{}.git", account, repo_name));
    }

    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        anyhow::bail!("codeberg repo create failed ({}): {}", status, body);
    }

    Ok(format!("git@codeberg.org:{}/{}.git", account, repo_name))
}
```

**Dependency change**: Add `blocking` feature to existing reqwest:
```toml
reqwest = { version = "0.12", features = ["json", "blocking"] }
```

### 2.2 Test `create_repo_on_codeberg` with local TCP mock
**Estimated**: 15 min  
**File**: `git.rs` tests

**No wiremock needed.** Use `std::net::TcpListener` to create a minimal mock HTTP server:

```rust
#[test]
fn test_create_repo_on_codeberg_success() {
    // Start a thread that listens on a random port and returns HTTP 201
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let response = "HTTP/1.1 201 Created\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).unwrap();
    });
    
    let url = format!("http://127.0.0.1:{}/api/v1/repos", port);
    let result = create_repo_on_codeberg("token", "account", "repo", &url);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("git@codeberg.org"));
}
```

Add tests for:
- **Success (201)**: Returns correct SSH URL
- **Already exists (409)**: Returns SSH URL without error
- **Already exists (422)**: Returns SSH URL without error
- **Auth failure (401)**: Returns error with status

### 2.3 Test `auto_create_repo` platform routing
**Estimated**: 10 min  
**File**: `git.rs` tests

Test without external tools:
- `AuthType::Generic` returns error
- `AuthType::Codeberg` with missing token returns error (no env, no secrets file)
- `AuthType::GitHub` when `gh` unavailable returns error (check via `which gh` or skip)

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
