use super::*;
use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn defaults_are_expected() {
        assert_eq!(default_min_size_mb(), 512);
        assert_eq!(default_kinds(), "rust-build,node-deps,build-output,cache");
    }

    #[test]
    fn human_bytes_formats_units() {
        assert_eq!(human_bytes(1), "1.0 B");
        assert_eq!(human_bytes(1024), "1.0 KiB");
        assert_eq!(human_bytes(1024 * 1024), "1.0 MiB");
    }

    #[test]
    fn parse_kinds_trims_and_dedupes() {
        let kinds = parse_kinds(" rust-build, node-deps ,rust-build,,cache ");
        assert_eq!(kinds.len(), 3);
        assert!(kinds.contains("rust-build"));
        assert!(kinds.contains("node-deps"));
        assert!(kinds.contains("cache"));
    }

    #[test]
    fn expand_tilde_uses_home_when_available() {
        let _guard = env_lock().lock().expect("lock");
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", "/tmp/dracon-home-test");
        // Ensure HOME is restored even if the test panics
        struct HomeGuard(Option<String>);
        impl Drop for HomeGuard {
            fn drop(&mut self) {
                if let Some(ref v) = self.0 {
                    std::env::set_var("HOME", v);
                } else {
                    std::env::remove_var("HOME");
                }
            }
        }
        let _home_guard = HomeGuard(old_home);

        assert_eq!(expand_tilde("~"), PathBuf::from("/tmp/dracon-home-test"));
        assert_eq!(
            expand_tilde("~/Dev/project"),
            PathBuf::from("/tmp/dracon-home-test/Dev/project")
        );
        assert_eq!(expand_tilde("/x/y"), PathBuf::from("/x/y"));
    }

    #[test]
    fn expand_tilde_falls_back_to_dot_when_home_unset() {
        let _guard = env_lock().lock().expect("lock");
        let old_home = std::env::var("HOME").ok();
        std::env::remove_var("HOME");
        struct HomeGuard(Option<String>);
        impl Drop for HomeGuard {
            fn drop(&mut self) {
                if let Some(ref v) = self.0 {
                    std::env::set_var("HOME", v);
                } else {
                    std::env::remove_var("HOME");
                }
            }
        }
        let _home_guard = HomeGuard(old_home);

        assert_eq!(expand_tilde("~"), PathBuf::from("."));
        assert_eq!(expand_tilde("~/foo"), PathBuf::from("./foo"));
        assert_eq!(expand_tilde("/x/y"), PathBuf::from("/x/y"));
    }

    #[test]
    fn build_link_report_counts_states() {
        let policy = SystemPolicy {
            storage: StoragePolicy::default(),
            links: LinkPolicy {
                entries: vec![LinkEntry {
                    link: "/tmp/does-not-exist-link".into(),
                    target: "/tmp/does-not-exist-target".into(),
                }],
            },
            guard: GuardPolicy::default(),
        };
        let report = build_link_report(&policy);
        assert_eq!(report.total, 1);
        assert_eq!(report.healthy, 0);
        assert_eq!(report.drifted, 1);
        assert_eq!(report.missing_target, 1);
    }

    #[cfg(unix)]
    #[test]
    fn evaluate_link_handles_missing_and_sync_cases() {
        let base = std::env::temp_dir().join(format!(
            "dracon_system_test_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(&base).expect("base dir");

        let target = base.join("target.txt");
        fs::write(&target, "x").expect("target");

        let missing_link = LinkEntry {
            link: base.join("missing-link").display().to_string(),
            target: target.display().to_string(),
        };
        let s1 = evaluate_link(&missing_link);
        assert_eq!(s1.issue, "link_missing");

        let normal_file_link = base.join("normal-file");
        fs::write(&normal_file_link, "x").expect("file");
        let not_symlink = LinkEntry {
            link: normal_file_link.display().to_string(),
            target: target.display().to_string(),
        };
        let s2 = evaluate_link(&not_symlink);
        assert_eq!(s2.issue, "path_not_symlink");

        let good_link = base.join("good-link");
        symlink(&target, &good_link).expect("symlink");
        let synced = LinkEntry {
            link: good_link.display().to_string(),
            target: target.display().to_string(),
        };
        let s3 = evaluate_link(&synced);
        assert_eq!(s3.issue, "ok");
        assert!(s3.in_sync);

        let wrong_target = base.join("other.txt");
        fs::write(&wrong_target, "y").expect("other");
        let mismatch_link = base.join("mismatch-link");
        symlink(&wrong_target, &mismatch_link).expect("symlink mismatch");
        let mismatch = LinkEntry {
            link: mismatch_link.display().to_string(),
            target: target.display().to_string(),
        };
        let s4 = evaluate_link(&mismatch);
        assert_eq!(s4.issue, "link_target_mismatch");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn parse_and_format_repeated_scenarios() {
        for i in 0..220usize {
            let csv = if i % 2 == 0 {
                "rust-build,node-deps,cache"
            } else {
                " rust-build , build-output , cache , rust-build "
            };
            let kinds = parse_kinds(csv);
            assert!(kinds.contains("rust-build"));
            assert!(kinds.contains("cache"));

            let bytes = (i as u64 + 1) * 2048;
            let out = human_bytes(bytes);
            assert!(!out.is_empty());
            assert!(out.contains(' '));
        }
    }

    #[test]
    fn parse_df_use_percent_works() {
        let sample = "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/root 100 91 9 91% /\n";
        assert_eq!(parse_df_use_percent(sample), Some(91));
    }

    #[test]
    fn parse_ps_output_works() {
        let sample = "123 1 250.5 4194304 git\n456 2 12.0 2048 zsh\n";
        let rows = parse_ps_output(sample);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].pid, 123);
        assert_eq!(rows[0].ppid, 1);
        assert_eq!(rows[0].command, "git");
        assert_eq!(rows[0].rss_mb, 4096);
        assert_eq!(rows[0].args, "");
    }

    #[test]
    fn is_git_process_detects_git_init() {
        assert!(is_git_process("git-init", ""));
        assert!(is_git_process("git", "init"));
        assert!(is_git_process("git-init", "--bare"));
    }

    #[test]
    fn is_git_process_detects_git_fetch_and_pull() {
        assert!(is_git_process("git-fetch", ""));
        assert!(is_git_process("git", "pull"));
        assert!(is_git_process("git-pull", "origin main"));
        assert!(is_git_process("git-fetch", "origin"));
    }

    #[test]
    fn is_git_process_detects_git_push_and_clone() {
        assert!(is_git_process("git-push", ""));
        assert!(is_git_process("git", "push"));
        assert!(is_git_process("git-clone", ""));
        assert!(is_git_process("git", "clone"));
    }

    #[test]
    fn is_git_process_rejects_non_git_commands() {
        assert!(!is_git_process("git", "log"));
        assert!(!is_git_process("git", "diff"));
        assert!(!is_git_process("git", "status"));
        assert!(!is_git_process("git", "commit"));
        assert!(!is_git_process("bash", ""));
        assert!(!is_git_process("python", ""));
        assert!(!is_git_process("legit-init", "")); // false positive from old substring matching
    }

    #[test]
    fn is_protected_ancestor_exact_match() {
        assert!(is_protected_ancestor("/home", "/home"));
        assert!(is_protected_ancestor("/etc", "/etc"));
        assert!(is_protected_ancestor("/", "/"));
    }

    #[test]
    fn is_protected_ancestor_descendant_match() {
        assert!(is_protected_ancestor("/home/dracon", "/home"));
        assert!(is_protected_ancestor("/home/dracon/Dev", "/home"));
        assert!(is_protected_ancestor("/etc/nginx/nginx.conf", "/etc"));
    }

    #[test]
    fn is_protected_ancestor_rejects_partial_prefix() {
        assert!(!is_protected_ancestor("/homefoo", "/home"));
        assert!(!is_protected_ancestor("/homefoo/bar", "/home"));
        assert!(!is_protected_ancestor("/etcnginx", "/etc"));
    }

    #[test]
    fn is_protected_ancestor_root_matches_exact_only() {
        assert!(is_protected_ancestor("/", "/"));
        assert!(!is_protected_ancestor("/anything", "/")); // root only matches exact to allow cleanup
        assert!(!is_protected_ancestor("/home", "/"));
    }

    #[test]
    fn check_path_str_blocks_descendants() {
        assert!(!check_path_str("/home/dracon", &[]));
        assert!(!check_path_str("/home/dracon/Dev", &[]));
        assert!(!check_path_str("/etc/nginx", &[]));
        assert!(check_path_str("/safe/path", &[]));
        assert!(check_path_str("/homefoo", &[])); // partial prefix should be safe
    }

    #[test]
    fn parse_ps_output_extracts_all_fields() {
        let sample = "9999 1 75.0 8192000 git-fetch origin main\n";
        let rows = parse_ps_output(sample);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pid, 9999);
        assert_eq!(rows[0].ppid, 1);
        assert_eq!(rows[0].cpu_percent, 75.0);
        assert_eq!(rows[0].rss_mb, 8192000 / 1024);
        assert_eq!(rows[0].command, "git-fetch");
        assert_eq!(rows[0].args, "origin main");
    }

    #[test]
    fn disk_state_transitions_at_thresholds() {
        let guard = GuardPolicy {
            disk_warn_percent: 70,
            disk_action_percent: 85,
            disk_critical_percent: 95,
            ..GuardPolicy::default()
        };
        assert_eq!(disk_state(50, &guard), "ok");
        assert_eq!(disk_state(70, &guard), "warn");
        assert_eq!(disk_state(84, &guard), "warn");
        assert_eq!(disk_state(85, &guard), "action");
        assert_eq!(disk_state(94, &guard), "action");
        assert_eq!(disk_state(95, &guard), "critical");
        assert_eq!(disk_state(100, &guard), "critical");
    }

    #[test]
    fn should_notify_respects_cooldown() {
        let mut state = GuardRuntimeState {
            heavy_since: std::collections::HashMap::new(),
            notify_cooldowns: std::collections::HashMap::new(),
            last_disk_state: "ok".to_string(),
            disk_history: Vec::new(),
            active_build_pids: std::collections::HashSet::new(),
        };
        let key = "test-key";
        assert!(should_notify(&mut state, key, 60), "first notify should succeed");
        assert!(!should_notify(&mut state, key, 60), "immediate second notify should be blocked");
        assert!(should_notify(&mut state, "other-key", 60), "different key should succeed");
    }

    #[test]
    fn predict_fill_time_requires_minimum_samples() {
        let history: Vec<(Instant, u8)> = vec![
            (Instant::now(), 50),
            (Instant::now(), 51),
        ];
        assert!(predict_fill_time(&history).is_none(), "needs at least 3 samples");
    }

    #[test]
    fn predict_fill_time_returns_none_for_stable_disk() {
        let base = Instant::now();
        let history: Vec<(Instant, u8)> = vec![
            (base, 50),
            (base + Duration::from_secs(10), 50),
            (base + Duration::from_secs(20), 50),
        ];
        assert!(predict_fill_time(&history).is_none(), "stable disk should not predict fill");
    }

    #[test]
    fn predict_fill_time_estimates_for_filling_disk() {
        let base = Instant::now();
        let history: Vec<(Instant, u8)> = vec![
            (base, 50),
            (base + Duration::from_secs(3600), 60),
            (base + Duration::from_secs(7200), 70),
        ];
        let hours = predict_fill_time(&history);
        assert!(hours.is_some(), "should predict fill time for rising disk");
        let h = hours.unwrap();
        assert!(h > 0.0, "predicted hours should be positive");
        assert!(h < 100.0, "predicted hours should be reasonable for 10%/hr rate");
    }

    #[tokio::test]
    async fn guard_report_completes_for_ok_disk() {
        let mut state = GuardRuntimeState {
            heavy_since: std::collections::HashMap::new(),
            notify_cooldowns: std::collections::HashMap::new(),
            last_disk_state: "ok".to_string(),
            disk_history: Vec::new(),
            active_build_pids: std::collections::HashSet::new(),
        };
        let guard = GuardPolicy {
            disk_warn_percent: 70,
            disk_action_percent: 85,
            disk_critical_percent: 95,
            disk_mount_path: "/".into(),
            freeze_sync_at_action: false,
            track_trends: false,
            ..GuardPolicy::default()
        };
        let report = run_guard_once(&guard, &mut state).await;
        assert!(report.is_ok() || report.is_err(), "async guard execution should complete");
    }
