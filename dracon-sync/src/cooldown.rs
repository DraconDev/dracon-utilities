//! Cooldown management for the sync daemon.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Manages cooldown timers for various daemon operations.
#[derive(Debug, Default)]
pub(crate) struct CooldownManager {
    /// Per-repair cooldowns (prevents repair storms)
    repair: HashMap<PathBuf, Instant>,
    /// Per-repo filter cooldowns (prevents tight re-check loops)
    filter: HashMap<PathBuf, Instant>,
    /// Per-remote notification cooldowns (webhook dedup)
    remote_notify: HashMap<String, Instant>,
    /// Repos waiting for initial scan
    pending: HashMap<PathBuf, Instant>,
}

impl CooldownManager {
    /// Create a new empty cooldown manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a repair cooldown is active for a repo.
    pub fn is_repair_cooldown_active(&self, repo: &PathBuf) -> bool {
        if let Some(&until) = self.repair.get(repo) {
            Instant::now() < until
        } else {
            false
        }
    }

    /// Set a repair cooldown for a repo.
    pub fn set_repair_cooldown(&mut self, repo: PathBuf, cooldown_secs: u64) {
        self.repair.insert(repo, Instant::now() + Duration::from_secs(cooldown_secs));
    }

    /// Check if a filter cooldown is active for a repo.
    pub fn is_filter_cooldown_active(&self, repo: &PathBuf) -> bool {
        if let Some(&until) = self.filter.get(repo) {
            Instant::now() < until
        } else {
            false
        }
    }

    /// Set a filter cooldown for a repo.
    pub fn set_filter_cooldown(&mut self, repo: PathBuf, cooldown_secs: u64) {
        self.filter.insert(repo, Instant::now() + Duration::from_secs(cooldown_secs));
    }

    /// Check if a remote notification cooldown is active.
    pub fn is_remote_notify_cooldown_active(&self, key: &str) -> bool {
        if let Some(&until) = self.remote_notify.get(key) {
            Instant::now() < until
        } else {
            false
        }
    }

    /// Set a remote notification cooldown.
    pub fn set_remote_notify_cooldown(&mut self, key: String, cooldown_secs: u64) {
        self.remote_notify.insert(key, Instant::now() + Duration::from_secs(cooldown_secs));
    }

    /// Add a repo to the pending list.
    pub fn add_pending(&mut self, repo: PathBuf) {
        self.pending.insert(repo, Instant::now());
    }

    /// Check if a repo is pending and how long it's been pending.
    pub fn get_pending_duration(&self, repo: &PathBuf) -> Option<Duration> {
        self.pending.get(repo).map(|&start| start.elapsed())
    }

    /// Remove a repo from the pending list.
    pub fn remove_pending(&mut self, repo: &PathBuf) {
        self.pending.remove(repo);
    }

    /// Retain only repos that are in the given set.
    pub fn retain_repos(&mut self, repos: &std::collections::BTreeSet<PathBuf>) {
        self.repair.retain(|repo, _| repos.contains(repo));
        self.filter.retain(|repo, _| repos.contains(repo));
        self.pending.retain(|repo, _| repos.contains(repo));
    }

    /// Clear all cooldowns (e.g., on policy reload).
    pub fn clear(&mut self) {
        self.repair.clear();
        self.filter.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cooldown_manager_new() {
        let manager = CooldownManager::new();
        assert!(manager.repair.is_empty());
        assert!(manager.filter.is_empty());
        assert!(manager.remote_notify.is_empty());
        assert!(manager.pending.is_empty());
    }

    #[test]
    fn test_repair_cooldown() {
        let mut manager = CooldownManager::new();
        let repo = PathBuf::from("/test/repo");
        
        // Initially not active
        assert!(!manager.is_repair_cooldown_active(&repo));
        
        // Set cooldown
        manager.set_repair_cooldown(repo.clone(), 60);
        
        // Now active
        assert!(manager.is_repair_cooldown_active(&repo));
    }

    #[test]
    fn test_filter_cooldown() {
        let mut manager = CooldownManager::new();
        let repo = PathBuf::from("/test/repo");
        
        // Initially not active
        assert!(!manager.is_filter_cooldown_active(&repo));
        
        // Set cooldown
        manager.set_filter_cooldown(repo.clone(), 30);
        
        // Now active
        assert!(manager.is_filter_cooldown_active(&repo));
    }

    #[test]
    fn test_remote_notify_cooldown() {
        let mut manager = CooldownManager::new();
        let key = "github:repo".to_string();
        
        // Initially not active
        assert!(!manager.is_remote_notify_cooldown_active(&key));
        
        // Set cooldown
        manager.set_remote_notify_cooldown(key.clone(), 10);
        
        // Now active
        assert!(manager.is_remote_notify_cooldown_active(&key));
    }

    #[test]
    fn test_pending_repos() {
        let mut manager = CooldownManager::new();
        let repo = PathBuf::from("/test/repo");
        
        // Initially not pending
        assert!(manager.get_pending_duration(&repo).is_none());
        
        // Add to pending
        manager.add_pending(repo.clone());
        
        // Now pending
        assert!(manager.get_pending_duration(&repo).is_some());
        
        // Remove from pending
        manager.remove_pending(&repo);
        
        // No longer pending
        assert!(manager.get_pending_duration(&repo).is_none());
    }

    #[test]
    fn test_retain_repos() {
        let mut manager = CooldownManager::new();
        let repo1 = PathBuf::from("/test/repo1");
        let repo2 = PathBuf::from("/test/repo2");
        
        manager.set_repair_cooldown(repo1.clone(), 60);
        manager.set_repair_cooldown(repo2.clone(), 60);
        
        let mut keep = std::collections::BTreeSet::new();
        keep.insert(repo1.clone());
        
        manager.retain_repos(&keep);
        
        assert!(manager.is_repair_cooldown_active(&repo1));
        assert!(!manager.is_repair_cooldown_active(&repo2));
    }

    #[test]
    fn test_clear() {
        let mut manager = CooldownManager::new();
        let repo = PathBuf::from("/test/repo");
        
        manager.set_repair_cooldown(repo.clone(), 60);
        manager.set_filter_cooldown(repo.clone(), 30);
        
        manager.clear();
        
        assert!(!manager.is_repair_cooldown_active(&repo));
        assert!(!manager.is_filter_cooldown_active(&repo));
    }
}
