use std::sync::Mutex;
use tempfile::TempDir;

pub struct HomeGuard {
    _temp_home: TempDir,
    original_home: Option<String>,
}

static ENV_MUTEX: Mutex<()> = Mutex::new(());

impl HomeGuard {
    pub fn new() -> Self {
        let _lock = ENV_MUTEX.lock().expect("env lock poisoned");
        let original_home = std::env::var("HOME").ok();
        let temp_home = TempDir::new().expect("create temp dir");
        std::env::set_var("HOME", temp_home.path());
        Self {
            _temp_home: temp_home,
            original_home,
        }
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        std::env::remove_var("HOME");
        if let Some(h) = &self.original_home {
            std::env::set_var("HOME", h);
        }
    }
}
