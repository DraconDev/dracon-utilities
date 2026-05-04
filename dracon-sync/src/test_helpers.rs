/// Shared test utilities for dracon-sync tests.
///
/// # EnvRestorer
/// `EnvRestorer` saves an environment variable, sets it to a new value,
/// and restores the original on drop. This prevents env var leaks between
/// tests when running in parallel.
///
/// # Usage
/// ```ignore
/// let _guard = EnvRestorer::new("MY_VAR", "new_value");
/// // ... test uses MY_VAR = "new_value" ...
/// // On drop, MY_VAR is restored to its original value (or removed).
/// ```
pub(crate) struct EnvRestorer {
    key: String,
    old_value: Option<String>,
}

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
