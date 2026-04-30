use std::sync::Mutex;
use tempfile::TempDir;

pub struct HomeGuard {
    _temp_home: TempDir,
}

static ENV_MUTEX: Mutex<()> = Mutex::new(());

impl HomeGuard {
    pub fn new() -> Self {
        let _lock = ENV_MUTEX.lock().expect("env lock poisoned");
        let temp_home = TempDir::new().expect("create temp dir");
        if let Some(old) = std::env::var_os("HOME") {
            std::env::set_var("HOME", temp_home.path());
        } else {
            std::env::set_var("HOME", temp_home.path());
        }
        Self { _temp_home: temp_home }
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        std::env::remove_var("HOME");
    }
}
