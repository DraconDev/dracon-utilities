/// Shared test utilities for dracon-sync tests.
///
/// # EnvRestorer
///
/// `EnvRestorer` saves an environment variable, sets it to a new value (or removes it),
/// and restores the original on drop. This prevents env var leaks between tests when
/// running in parallel.
///
/// ## Use `new()` when you need to SET an env var for a test:
/// ```
/// let _guard = EnvRestorer::new("MY_VAR", "new_value");
/// // MY_VAR is "new_value" for the duration of this test
/// ```
///
/// ## Use `remove()` when you need to CLEAR an env var for a test:
/// ```
/// let _guard = EnvRestorer::remove("SOME_VAR");
/// // SOME_VAR is unset for the duration of this test
/// ```
///
/// # Parallel Test Constraints
///
/// Tests pass reliably with `--test-threads=1` (458/458 pass as of 2026-05-22). In parallel mode,
/// ~10-20 tests fail unpredictably due to these shared global states:
///
/// 1. **PATH**: Tests that add mock binary dirs to PATH (for gh/glab mocking)
///    use `acquire_path_lock()` + `EnvRestorer::new("PATH", ...)`. But other
///    tests that call `std::process::Command::new("git")` directly resolve `git`
///    from PATH and can race with concurrent PATH modifications.
///
/// 2. **`DRACON_SYNC_GIT_BIN`**: Some tests set this env var to mock the git binary.
///    While `git_binary()` no longer caches the value, tests using
///    `std::process::Command::new("git")` directly (not through `git_binary()`)
///    don't check this env var at all. Use `test_git_cmd()` from this module
///    instead of `std::process::Command::new("git")` to ensure consistency.
///
/// 3. **Registry/port state**: Integration-style tests `test_create_repo_on_github_*`,
///    `test_create_repo_on_gitlab_*` that start local TCP listeners can conflict.
///
/// ## Mitigations already in place
///
/// - `git_binary()` in `policy.rs` and `real_git_path()` in `git.rs`: no longer use
///   `OnceLock` caching for `DRACON_SYNC_GIT_BIN` — checked every call
/// - All env var mutations in tests are now gated behind `EnvRestorer` to prevent leaks
/// - `acquire_path_lock()` (parking_lot Mutex) serializes PATH-modifying tests
///
/// ## Running tests
///
/// ```bash
/// # Reliable (serial):
/// cargo test -- --test-threads=1
///
/// # Fast but may have flaky failures:
/// cargo test
/// ```
///
/// # Git Command Helper
///
/// Use `test_git_cmd()` instead of `std::process::Command::new("git")` in tests.
/// This respects `DRACON_SYNC_GIT_BIN` and prevents PATH resolution races in parallel runs.
///
/// ```ignore
/// let output = test_git_cmd().current_dir(&repo).args(["status"]).output()?;
/// ```
#[allow(dead_code)]
pub(crate) fn test_git_cmd() -> std::process::Command {
    let git_path = crate::policy::git_binary();
    std::process::Command::new(git_path)
}

#[allow(dead_code)]
pub(crate) struct EnvRestorer {
    key: String,
    old_value: Option<String>,
}

#[allow(dead_code)]
impl EnvRestorer {
    /// Saves current value of `key`, sets it to `new_value`.
    /// On Drop: restores the original value (or removes if unset).
    pub(crate) fn new(key: &str, new_value: &str) -> Self {
        let old_value = std::env::var(key).ok();
        std::env::set_var(key, new_value);
        EnvRestorer {
            key: key.to_string(),
            old_value,
        }
    }

    /// Saves current value of `key`, removes the variable entirely.
    /// On Drop: restores the original value (or removes if unset).
    pub(crate) fn remove(key: &str) -> Self {
        let old_value = std::env::var(key).ok();
        std::env::remove_var(key);
        EnvRestorer {
            key: key.to_string(),
            old_value,
        }
    }
}

impl Drop for EnvRestorer {
    fn drop(&mut self) {
        std::env::remove_var(&self.key);
        if let Some(ref v) = self.old_value {
            std::env::set_var(&self.key, v);
        }
    }
}
