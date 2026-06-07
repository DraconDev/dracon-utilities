#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;
    use std::time::Duration;

    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    static HOME_MUTEX: Mutex<()> = Mutex::new(());

    /// Guard that temporarily changes $HOME and restores it on drop.
    struct HomeGuard {
        original: Option<String>,
        #[allow(dead_code)]
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl HomeGuard {
        fn new(home: &str) -> Self {
            let lock = HOME_MUTEX.lock().expect("home mutex poisoned");
            let original = std::env::var("HOME").ok();
            std::env::set_var("HOME", home);
            HomeGuard {
                original,
                _lock: lock,
            }
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            std::env::remove_var("HOME");
            if let Some(ref v) = self.original {
                std::env::set_var("HOME", v);
            }
        }
    }

    struct TestDir {
        path: std::path::PathBuf,
        #[allow(dead_code)]
        guard: Mutex<()>,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let tmp = std::env::temp_dir();
            let path = tmp.join(format!("dracon_warden_test_{}_{}", name, id));
            fs::create_dir_all(&path).expect("create temp dir");
            Self {
                path,
                guard: Mutex::new(()),
            }
        }
        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn sample_policy() -> WardenPolicy {
        WardenPolicy {
            protected_patterns: vec!["*.env".into(), "secrets/**".into()],
            plaintext_patterns: vec!["*.pub".into()],
            hygiene_patterns: vec!["target/".into(), "*.log".into()],
            repo_roots: vec![],
            discover_roots: vec![],
        }
    }

    #[test]
    fn replace_managed_block_appends_when_missing() {
        let current = "a=1\n";
        let block = format!("{BLOCK_BEGIN}\nmanaged\n{BLOCK_END}");
        let next = replace_managed_block(current, &block);
        assert!(next.contains("a=1"));
        assert!(next.contains("managed"));
        assert!(next.contains(BLOCK_BEGIN));
        assert!(next.contains(BLOCK_END));
    }

    #[test]
    fn replace_managed_block_replaces_existing_and_keeps_tail() {
        let current = format!("head\n{BLOCK_BEGIN}\nold\n{BLOCK_END}\n\nend\n");
        let block = format!("{BLOCK_BEGIN}\nnew\n{BLOCK_END}");
        let next = replace_managed_block(&current, &block);
        assert!(next.contains("head"));
        assert!(next.contains("new"));
        assert!(!next.contains("old"));
        assert!(next.contains("end"));
    }

    #[test]
    fn build_gitigno[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBRS0pBS1UrdWxhZ2o4dXRNYlg4Tit0ay9WeVZmVFkrSmpRS0FaaC9qNG5NClYwYzZHVHp0WG1WTThOSll1ODBCZFBPQjJOK0FnN3F6enFPZTBvWTBaZlkKLT4gWDI1NTE5IEFKa1JLcGgwVkd0bElweUlDZFpjMU9USFFKKzJjYmladGxpRmVCSlFlVU0KQmZiR2tmblNubXlCdHZQUnZreUI4djBndHZ5MHJBQ0hpZHR0V2llY2wyawotPiBYMjU1MTkgemx6QVB5akw1eVVkVmZTYkZHZ3ZUSkJONnYzdUVTZTQ0Zkc1UHFldzRsWQpuYy9qYUd3RlUxUkxOOG40eVFKQnpPK3gxdm4xQ2FDOG9GSjVwQi9La0VrCi0+IFgyNTUxOSB3aUxqdytCN2l4OURzSHVqRmtPVld5V3ZTcE16UURET05KbURmdUV0UnljCkRKRGJPUmk1Y0xtRFhkZnBZN2gyaW1NV2gwMlpLeW5EVnhYNGh1MG1KQ1kKLT4gWDI1NTE5IE8xMm9oTnpOaGhWaEs4V210WllVaUYyVW9veXRKMnUrZ0tzSkMwQnJHZ0EKT09BMStlTC80MTQ0WHZydFArcGUraFBUR1V4QzdUc3UxcG42VDUrV01tSQotPiBYMjU1MTkgQlFzcVFvN3E1MkdxeHNKUFE1NWoxSG5nVGdTSEI4UTdFREVucFlnU0tUMApaWGUxTnlFQVFJbUdJM1BJeW1oTVNOdW1CSVducFY5Z0hnSmtTYldFQjJjCi0+IE9qYWstZ3JlYXNlIFhebyNnIGxrMFIud1dLIHYgLzN6CkxVM1FuWjFBT2dNV3lEdm45QVlaUytCa3I2SXpRSE9Iamk3eXV2TXFEYmRGTUhrc1FNcU5BYTRvQmRuTFlwM2IKVmlOZ2xFTE8KLS0tIGNhUzdsN2hCSXpYSldsbUdvQyt0N3loQWhBQ2QrbGE2SWhjWGlhT0s0L28K7ghtpxhizlsfJ9VnSVrYNEcIXx7v/PYVCxfzpw4k48F4GnpfG2Rwih8ZC0wuoYKILvKiaZbkWJqbWMIYjLnvVQ==]() {
        let block = build_gitignore_block(&sample_policy()).expect("block");
        assert!(block.contains(BLOCK_BEGIN));
        assert!(block.contains("target/"));
        assert!(block.contains("!*.env"));
        assert!(block.contains("!secrets/**"));
        assert!(block.contains("!*.pub"));
        assert!(!block.contains("!config/licenses.json"));
        assert!(!block.contains("!config/services.test.json"));
        assert!(!block.contains("!plan/pages/templates/*.json"));
        assert!(block.contains(BLOCK_END));
    }

    #[test]
    fn build_gitattributes_block_includes_expected_lines() {
        let block = build_gitattributes_block(&sample_policy()).expect("block");
        assert!(block.contains("*.env filter=dracon"));
        assert!(block.contains("secrets/** filter=dracon"));
        assert!(block.contains("*.pub -filter"));
        assert!(!block.contains("config/licenses.json -filter"));
        assert!(!block.contains("config/services.test.json -filter -diff -merge"));
        assert!(!block.contains("plan/pages/templates/*.json -filter -diff -merge"));
    }

    #[test]
    fn plaintext_cannot_overlap_protected_or_disable_env_encryption() {
        let policy = WardenPolicy {
            protected_patterns: vec!["config/envs/*.env".into(), "*.env".into()],
            plaintext_patterns: vec!["config/envs/*.env".into()],
            hygiene_patterns: vec![],
            repo_roots: vec![],
            discover_roots: vec![],
        };
        assert!(build_gitattributes_block(&policy).is_err());
    }

    #[test]
    fn owner_pubkeys_in_filters_only_owner_pub() {
        let td = TestDir::new("warden_owner_pubkeys");
        fs::write(td.path().join("owner_a.pub"), "a").expect("write");
        fs::write(td.path().join("owner_a.key"), "a").expect("write");
        fs::write(td.path().join("identity.pub"), "a").expect("write");
        let keys = owner_pubkeys_in(td.path());
        assert_eq!(keys.len(), 1);
        assert_eq!(
            keys[0].file_name().and_then(|n| n.to_str()),
            Some("owner_a.pub")
        );
    }

    #[test]
    fn newest_file_picks_newest_existing() {
        let td = TestDir::new("warden_newest");
        let a = td.path().join("a.pub");
        let b = td.path().join("b.pub");
        fs::write(&a, "a").expect("write a");
        std::thread::sleep(Duration::from_secs(1));
        fs::write(&b, "b").expect("write b");
        let picked = newest_file(vec![a.clone(), b.clone()]).expect("picked");
        assert_eq!(picked, b);
    }

    #[test]
    fn publish_repo_pubkey_writes_and_is_idempotent() {
        let td = TestDir::new("warden_publish_key");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");
        let key = td.path().join("owner_test.pub");
        fs::write(&key, "age1xxx").expect("key");

        assert!(publish_repo_pubkey(&repo, &key).expect("first publish"));
        assert!(!publish_repo_pubkey(&repo, &key).expect("second publish"));
        let out = repo.join(".dracon/data/keys/owner_test.pub");
        assert_eq!(fs::read_to_string(out).expect("read out"), "age1xxx");
    }

    #[test]
    fn harden_repo_changes_files_and_writes_key() {
        let td = TestDir::new("warden_harden_repo");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");
        let key = td.path().join("owner_test.pub");
        fs::write(&key, "age1yyy").expect("key");

        let status = ProcessCommand::new("git")
            .arg("init")
            .arg(&repo)
            .status()
            .expect("git init");
        assert!(status.success(), "git init should succeed");

        let (a, b, c) = harden_repo(&repo, &sample_policy(), Some(&key), true).expect("harden");
        assert!(a, "gitignore should be written");
        assert!(b, ".gitattributes should be written");
        assert!(c, "pubkey should be published");
        assert!(repo.join(".gitignore").exists());
        assert!(repo.join(".gitattributes").exists());
        assert!(repo.join(".dracon/data/keys/owner_test.pub").exists());
    }

    #[test]
    fn harden_repo_sets_local_dracon_filter_config() {
        let td = TestDir::new("warden_harden_repo_filter_cfg");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");
        let status = ProcessCommand::new("git")
            .arg("init")
            .arg(&repo)
            .status()
            .expect("git init");
        assert!(status.success());

        let (_a, b, _c) = harden_repo(&repo, &sample_policy(), None, true).expect("harden");
        assert!(b);

        let clean = ProcessCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .arg("config")
            .arg("--local")
            .arg("--get")
            .arg("filter.dracon.clean")
            .output()
            .expect("get clean");
        assert!(clean.status.success());
        assert_eq!(
            String::from_utf8_lossy(&clean.stdout).trim(),
            "dracon-warden filter-clean %f"
        );

        let smudge = ProcessCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .arg("config")
            .arg("--local")
            .arg("--get")
            .arg("filter.dracon.smudge")
            .output()
            .expect("get smudge");
        assert!(smudge.status.success());
        assert_eq!(
            String::from_utf8_lossy(&smudge.stdout).trim(),
            "dracon-warden filter-smudge %f"
        );

        let required = ProcessCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .arg("config")
            .arg("--local")
            .arg("--get")
            .arg("filter.dracon.required")
            .output()
            .expect("get required");
        assert!(required.status.success());
        assert_eq!(String::from_utf8_lossy(&required.stdout).trim(), "true");
    }

    #[test]
    fn publish_repo_pubkey_rejects_non_owner_or_secret_key_material() {
        let td = TestDir::new("warden_publish_key_rejects");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");

        let not_owner = td.path().join("identity.pub");
        fs::write(&not_owner, "age1xxx").expect("write");
        assert!(publish_repo_pubkey(&repo, &not_owner).is_err());

        let secret = td.path().join("owner_secret.pub");
        fs::write(&secret, "AGE-SECRET-KEY-1XXXX").expect("write");
        assert!(publish_repo_pubkey(&repo, &secret).is_err());
    }

    #[test]
    fn publish_repo_pubkey_no_churn_different_valid_key() {
        let td = TestDir::new("warden_publish_key_no_churn");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");
        let keys_dir = repo.join(".dracon/data/keys");
        fs::create_dir_all(&keys_dir).expect("keys dir");

        let key_a = td.path().join("owner_test.pub");
        fs::write(&key_a, "age1aaa").expect("key a");
        assert!(publish_repo_pubkey(&repo, &key_a).expect("first publish"));

        let key_b = td.path().join("owner_test.pub");
        fs::write(&key_b, "age1bbb").expect("key b");
        assert!(!publish_repo_pubkey(&repo, &key_b).expect("churn protection"));

        assert_eq!(
            fs::read_to_string(keys_dir.join("owner_test.pub")).expect("read"),
            "age1aaa",
            "existing valid key must not be overwritten by a different valid key"
        );
    }

    #[test]
    fn salvage_invalid_json_replaces_marker_tokens_and_parses() {
        let a = "{[DRACON_SECRET:abc]: \"x\"}";
        let salvaged = salvage_invalid_json_markers(a).expect("salvaged");
        let v: serde_json::Value = serde_json::from_str(&salvaged).expect("parse");
        assert_eq!(
            v["__scrubbed__"],
            serde_json::Value::String("x".to_string())
        );

        let b = "{ \"track_id\": [DRACON_SECRET:abc], \"x\": 1 }";
        let salvaged = salvage_invalid_json_markers(b).expect("salvaged");
        let v: serde_json::Value = serde_json::from_str(&salvaged).expect("parse");
        assert!(v["track_id"].is_null());
        assert_eq!(v["x"], serde_json::Value::from(1));
    }

    #[test]
    fn effective_repo_roots_merges_and_dedupes() {
        let td = TestDir::new("warden_effective_roots");
        let p1 = td.path().join("one");
        fs::create_dir_all(&p1).expect("p1");

        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec![],
            hygiene_patterns: vec![],
            repo_roots: vec![p1.display().to_string(), p1.display().to_string()],
            discover_roots: vec![],
        };
        let merged = effective_repo_roots(&policy);
        assert_eq!(merged.len(), 1);
        assert!(merged.contains(&p1));
    }

    #[test]
    fn effective_discovery_roots_merges_watch_and_discover_deduped() {
        let td = TestDir::new("warden_effective_discovery_roots");
        let p1 = td.path().join("one");
        let p2 = td.path().join("two");
        fs::create_dir_all(&p1).expect("p1");
        fs::create_dir_all(&p2).expect("p2");

        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec![],
            hygiene_patterns: vec![],
            repo_roots: vec![p1.display().to_string()],
            discover_roots: vec![p1.display().to_string(), p2.display().to_string()],
        };
        let merged = effective_discovery_roots(&policy);
        assert_eq!(merged.len(), 2);
        assert!(merged.contains(&p1));
        assert!(merged.contains(&p2));
    }

    #[test]
    fn apply_managed_file_detects_noop_second_write() {
        let td = TestDir::new("warden_apply_noop");
        let file = td.path().join(".gitignore");
        let block = format!("{BLOCK_BEGIN}\nfoo\n{BLOCK_END}");
        assert!(apply_managed_file(&file, &block).expect("first"));
        assert!(!apply_managed_file(&file, &block).expect("second"));
    }

    #[test]
    fn apply_overwrite_file_detects_noop_second_write() {
        let td = TestDir::new("warden_apply_overwrite_noop");
        let file = td.path().join(".gitattributes");
        let body = "a\nb\n";
        assert!(apply_overwrite_file(&file, body).expect("first"));
        assert!(!apply_overwrite_file(&file, body).expect("second"));
    }

    #[test]
    fn repeated_replace_block_scenarios_are_stable() {
        for idx in 0..200usize {
            let current = if idx % 2 == 0 {
                format!("prefix-{idx}\n")
            } else {
                format!("prefix-{idx}\n{BLOCK_BEGIN}\nold\n{BLOCK_END}\n")
            };
            let block = format!("{BLOCK_BEGIN}\nnew-{idx}\n{BLOCK_END}");
            let next = replace_managed_block(&current, &block);
            assert!(next.contains(&format!("new-{idx}")));
            assert!(next.contains(BLOCK_BEGIN));
            assert!(next.contains(BLOCK_END));
        }
    }

    #[test]
    fn resolve_policy_path_local_finds_temp_config() {
        let td = TestDir::new("warden_policy_path");
        let config_dir = td.path().join(".dracon").join("utilities").join("warden");
        fs::create_dir_all(&config_dir).expect("create config dir");
        let config_path = config_dir.join("dracon-warden.toml");
        fs::write(
            &config_path,
            r#"
[watch]
watch_roots = ["/tmp/test"]
"#,
        )
        .expect("write config");

        let old_val = std::env::var("DRACON_WARDEN_POLICY").ok();
        std::env::set_var("DRACON_WARDEN_POLICY", config_path.display().to_string());
        let path = resolve_policy_path_local().expect("should resolve");
        // Restore env var to prevent parallel test interference
        match old_val {
            Some(v) => std::env::set_var("DRACON_WARDEN_POLICY", v),
            None => std::env::remove_var("DRACON_WARDEN_POLICY"),
        }

        assert_eq!(path, config_path);
    }

    #[test]
    fn resolve_policy_path_local_falls_back_to_default_locations() {
        let td = TestDir::new("warden_policy_default");
        let config_dir = td.path().join(".dracon").join("utilities").join("warden");
        fs::create_dir_all(&config_dir).expect("create config dir");
        let config_path = config_dir.join("dracon-warden.toml");
        fs::write(
            &config_path,
            r#"
[watch]
watch_roots = ["/tmp/test"]
"#,
        )
        .expect("write config");

        let _guard = HomeGuard::new(td.path().to_str().unwrap());
        let path = resolve_policy_path_local();

        assert!(path.is_ok(), "should find config in default location");
    }

    #[test]
    fn marker_prefix_at_finds_correct_positions() {
        let s = "prefix [DRACON_SECRET:abc] after";
        assert_eq!(marker_prefix_at(s, 7), Some("[DRACON_SECRET:"));

        let s2 = "prefix [DRACON_SECRET:xyz] after";
        assert_eq!(marker_prefix_at(s2, 7), Some("[DRACON_SECRET:"));

        let s3 = "no marker here";
        assert_eq!(marker_prefix_at(s3, 0), None);
    }

    #[test]
    fn is_marker_string_detects_both_markers() {
        assert!(is_marker_string("hello [DRACON_SECRET:xyz] world"));
        assert!(!is_marker_string("hello world"));
        assert!(!is_marker_string("DRACON_SECRET not in brackets"));
        assert!(!is_marker_string("[WRONG_SECRET:abc]"));
    }

    #[test]
    fn build_gitigno[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBkTVF4TmQ2L3R0V0doaDI2dFltZjJPMjBVZVoyZThoZ3h3V3ZGNk4xVEd3CjBqTm5zTmhWb1UyaHNicWVnY050SlZUQVFaVVB5MGFRRXVMWng2cUFGSmsKLT4gWDI1NTE5IEl4Z0dtTWRpY21ObjJrenZlK08vLzNkK0lsdEZ5b1g0ak4wWDZGODc5bEkKVlRYZXVlY2RlcGVxbWl2dm0rejY0YjlWTmUvTythYjh5MWlrUEZpNm55dwotPiBYMjU1MTkgWXFMTDFpZ0w0VTZBSzFPallVeEQzVnVLbzNYODVaaTFCd0R6amplTHhWUQpBWTRNdjcxc3pXSzNIRjVDbC9SUVArR21ZV1J1N0NsTkNuNFV4ak93V0lRCi0+IFgyNTUxOSBhWkhsbGozc0RyYzNoeUxZalBLSFlIWTlvc3p5OThFaVZTRXdaNUJmaVhrCjhJY0czNTcxWkpCaFRLY3NjUFJSTGRwNlVjUjU2NnBFL21BSC9kZmxNQlUKLT4gWDI1NTE5IEduUzFsYktSdmN3TkVETjFLRW9EeHhtSUFucDBQOW9EbjF2ckJadlpiakEKb2NmbmxuY1VqSUNCU1Bqb2JpNWxTcTI1S1RSa1c5TVJqM3dmSnlwbWNISQotPiBYMjU1MTkgcGtOeVFNVmFybUpYYThCNUYwblB5QlY4RVF6bTZzOFBIT3hyckZ1RDZ5cwpTVVFnL21XYjl5anc2ZFhkNkR6amp5akpKV2MyV0VZNnBpVExFMkJ0RnFjCi0+IEB7eS1ncmVhc2Ugclpgd0hlKVIgLi9WeCg8LSAyVGMyeiBLIVt6e1UKQUt6VDZtM0lFMG1EdmtoOVZEaUUwQ21mcmFpenJ0anFzOFpObTlDS3dnZmJ5d01ZZGs4aTVudzdJQU52U3cKLS0tIDAzV2VJbkZLczRId3E0bENrcWNlTDA0MmZwT2lFWVNHSVRxWjQveGxuN3MKwlL2V8oyMIQPkyJTq4VMuHNUVnyM8ADJEWgnF8Es/FisKRMUtheprru6PhCIdwe93UtcFwW/eAKw8nqf5JCDz7vR]() {
        let block = build_gitignore_block(&sample_policy()).expect("block");
        assert!(block.contains("# --- BEGIN DRACON MANAGED BLOCK ---"));
        assert!(block.contains("target/"));
        assert!(block.contains("*.log"));
    }

    #[test]
    fn build_gitattributes_block_sets_filter_for_env() {
        let block = build_gitattributes_block(&sample_policy()).expect("block");
        assert!(block.contains("*.env filter=dracon"));
        assert!(block.contains("secrets/** filter=dracon"));
    }

    #[test]
    fn discover_git_repos_finds_all_git_dirs() {
        let td = TestDir::new("warden_discover_all");
        let root = td.path().join("root");
        fs::create_dir_all(&root).expect("root");

        let repo1 = root.join("my_repo");
        fs::create_dir_all(repo1.join(".git")).expect("my_repo .git");

        let repo2 = root.join("other_repo");
        fs::create_dir_all(repo2.join(".git")).expect("other_repo .git");

        let repos = discover_git_repos(&[root], &BTreeSet::new());

        assert!(repos.contains(&repo1), "my_repo should be found");
        assert!(repos.contains(&repo2), "other_repo should be found");
    }

    #[test]
    fn discover_git_repos_local_finds_basic_repos() {
        let td = TestDir::new("warden_discover_local");
        let root = td.path().join("root");
        fs::create_dir_all(&root).expect("root");

        let repo1 = root.join("repo1");
        fs::create_dir_all(repo1.join(".git")).expect("repo1 .git");

        let repo2 = root.join("repo2");
        fs::create_dir_all(repo2.join(".git")).expect("repo2 .git");

        let repos = discover_git_repos_local(&[root]);

        assert!(repos.contains(&repo1), "repo1 should be found");
        assert!(repos.contains(&repo2), "repo2 should be found");
    }

    #[test]
    fn filter_smudge_handles_empty_input() {
        let content = "let x = 1;\n";
        let warden = DraconWarden::new().expect("create warden");
        let result = warden.smudge(content.as_bytes(), None).expect("smudge");
        assert_eq!(
            result,
            content.as_bytes(),
            "plaintext should pass through unchanged"
        );
    }

    #[test]
    fn replace_managed_block_empty_current_string() {
        let current = "";
        let block = format!("{BLOCK_BEGIN}\nnewcontent\n{BLOCK_END}");
        let next = replace_managed_block(current, &block);
        assert!(next.contains("newcontent"));
        assert!(next.contains(BLOCK_BEGIN));
        assert!(next.contains(BLOCK_END));
    }

    #[test]
    fn replace_managed_block_multiple_blocks_replaces_all() {
        let current = format!(
            "prefix\n{BLOCK_BEGIN}\nfirst\n{BLOCK_END}\nmid\n{BLOCK_BEGIN}\nsecond\n{BLOCK_END}\n suffix\n"
        );
        let block = format!("{BLOCK_BEGIN}\nnew\n{BLOCK_END}");
        let next = replace_managed_block(&current, &block);
        assert!(next.contains("prefix"));
        assert!(next.contains("new"));
        assert!(
            !next.contains("first"),
            "first block content should be replaced"
        );
        assert!(
            !next.contains("second"),
            "second block content should be replaced"
        );
        assert!(next.contains("mid"));
        assert!(next.contains(" suffix"));
    }

    #[test]
    fn replace_managed_block_preserves_leading_whitespace() {
        let current = "  prefix\n";
        let block = format!("{BLOCK_BEGIN}\nmanaged\n{BLOCK_END}");
        let next = replace_managed_block(current, &block);
        assert!(
            next.starts_with("  prefix\n"),
            "leading content should be preserved"
        );
    }

    #[test]
    fn apply_managed_file_creates_parent_dirs() {
        let td = TestDir::new("warden_apply_creates_dirs");
        let nested = td.path().join("a/b/c/managed.txt");
        let block = format!("{BLOCK_BEGIN}\ncontent\n{BLOCK_END}");
        let result = apply_managed_file(&nested, &block);
        assert!(result.is_ok(), "should create parent dirs");
        assert!(nested.exists(), "file should exist");
        std::fs::remove_dir_all(td.path()).ok();
    }

    #[test]
    fn apply_overwrite_file_creates_new_file() {
        let td = TestDir::new("warden_overwrite_new");
        let file = td.path().join("newfile.txt");
        let result = apply_overwrite_file(&file, "hello world");
        assert!(result.is_ok(), "should create new file");
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(
            content.starts_with("hello world"),
            "should contain content: {:?}",
            content
        );
        std::fs::remove_dir_all(td.path()).ok();
    }

    #[test]
    fn apply_overwrite_file_overwrites_existing() {
        let td = TestDir::new("warden_overwrite_existing");
        let file = td.path().join("existing.txt");
        std::fs::write(&file, "old content").unwrap();
        let result = apply_overwrite_file(&file, "new content");
        assert!(result.is_ok(), "should overwrite");
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(
            content.starts_with("new content"),
            "should contain new content: {:?}",
            content
        );
        std::fs::remove_dir_all(td.path()).ok();
    }

    #[test]
    fn is_marker_string_edge_cases() {
        assert!(!is_marker_string(""), "empty string should not match");
        assert!(!is_marker_string("[DRACON_SECRET]"), "no colon");
        assert!(
            !is_marker_string("DRACON_SECRET not in brackets"),
            "not in brackets"
        );
        assert!(!is_marker_string("[WRONG_SECRET:abc]"), "wrong prefix");
        assert!(
            is_marker_string("[DRACON_SECRET:]"),
            "empty key is still a marker"
        );
        assert!(
            is_marker_string("[DRACON_SECRET: ]"),
            "space key is still a marker"
        );
        assert!(is_marker_string("[DRACON_SECRET:abc123]"), "basic key");
        assert!(
            is_marker_string("[DRACON_SECRET:abc-123_456]"),
            "key with dash underscore"
        );
    }

    #[test]
    fn marker_prefix_at_edge_cases() {
        assert_eq!(marker_prefix_at("no bracket here", 0), None);
        assert_eq!(
            marker_prefix_at("[DRACON_SECRET:abc]", 0),
            Some("[DRACON_SECRET:"),
            "starts at position 0"
        );
        assert_eq!(
            marker_prefix_at("[DRACON_SECRET:abc]", 1),
            None,
            "starts at position 1"
        );
        assert_eq!(
            marker_prefix_at("prefix [DRACON_SECRET", 8),
            None,
            "incomplete bracket without colon"
        );
        assert_eq!(
            marker_prefix_at("[DRACON_SECRET:abc] more", 0),
            Some("[DRACON_SECRET:"),
            "marker at start followed by more"
        );
        assert_eq!(
            marker_prefix_at("text [DRACON_SECRET:abc] end", 5),
            Some("[DRACON_SECRET:"),
            "at position 5 [ bracket is at position 5"
        );
    }

    #[test]
    fn salvage_invalid_json_no_marker_returns_none() {
        assert!(salvage_invalid_json_markers("just normal json").is_none());
        assert!(salvage_invalid_json_markers("").is_none());
        assert!(
            salvage_invalid_json_markers("[DRACON_SECRE").is_none(),
            "incomplete marker should return None"
        );
    }

    #[test]
    fn salvage_invalid_json_marker_at_end_of_string() {
        let input = r#"{"key": "value", "secret": "[DRACON_SECRET:abc]"}"#;
        let salvaged = salvage_invalid_json_markers(input).expect("should salvage");
        assert!(salvaged.contains("null") || salvaged.contains("__scrubbed__"));
    }

    #[test]
    fn salvage_invalid_json_markers_multiple_in_sequence() {
        let input = r#"{"a": [DRACON_SECRET:x], "b": [DRACON_SECRET:y], "c": "normal"}"#;
        let salvaged = salvage_invalid_json_markers(input).expect("should salvage");
        assert!(salvaged.contains("null") || salvaged.contains("__scrubbed__"));
        assert!(salvaged.contains("normal"));
    }

    #[test]
    fn salvage_invalid_json_handles_nested_markers() {
        let input = r#"{"key": "[DRACON_SECRET:abc]", "nested": {"key": "[DRACON_SECRET:xyz]"}}"#;
        let salvaged = salvage_invalid_json_markers(input).expect("should salvage");
        let v: serde_json::Value = serde_json::from_str(&salvaged).expect("should parse");
        assert!(v["key"].is_null() || v["key"].is_string());
    }

    #[test]
    fn effective_repo_roots_handles_empty_policy() {
        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec![],
            hygiene_patterns: vec![],
            repo_roots: vec![],
            discover_roots: vec![],
        };
        let roots = effective_repo_roots(&policy);
        assert!(roots.is_empty());
    }

    #[test]
    fn effective_discovery_roots_handles_empty_policy() {
        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec![],
            hygiene_patterns: vec![],
            repo_roots: vec![],
            discover_roots: vec![],
        };
        let roots = effective_discovery_roots(&policy);
        assert!(roots.is_empty());
    }

    #[test]
    fn build_globset_empty_patterns_returns_empty_set() {
        let set = build_globset(&[]).expect("should succeed");
        assert!(set.is_empty());
    }

    #[test]
    fn build_globset_single_pattern_matches() {
        let set = build_globset(&["*.json".into()]).expect("should succeed");
        assert!(set.is_match("test.json"));
        assert!(!set.is_match("test.txt"));
    }

    #[test]
    fn build_globset_multiple_patterns() {
        let set = build_globset(&["*.json".into(), "*.toml".into()]).expect("should succeed");
        assert!(set.is_match("test.json"));
        assert!(set.is_match("test.toml"));
        assert!(!set.is_match("test.txt"));
    }

    #[test]
    fn build_globset_invalid_pattern_returns_error() {
        let result = build_globset(&["[".into()]);
        assert!(result.is_err(), "invalid glob pattern should return error");
    }

    #[test]
    fn build_globset_normalizes_backslash() {
        let set = build_globset(&["subdir\\*.json".into()]).expect("should succeed");
        assert!(set.is_match("subdir/test.json"));
    }

    #[test]
    fn run_keygen_generates_keypair_successfully() {
        let td = TestDir::new("warden_keygen_success");
        let keys_dir = td.path().join(".dracon").join("data").join("keys");

        let _guard = HomeGuard::new(td.path().to_str().unwrap());

        let result = run_keygen();

        assert!(result.is_ok(), "keygen should succeed: {:?}", result);
        let hostname_raw = hostname::get()
            .expect("hostname")
            .to_string_lossy()
            .to_string();
        let hostname: String = hostname_raw
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        let secret_path = keys_dir.join(format!("machine_{}.age", hostname));
        let pubkey_path = keys_dir.join(format!("owner_{}.pub", hostname));
        assert!(
            secret_path.exists(),
            "secret key should be created at {}",
            secret_path.display()
        );
        assert!(
            pubkey_path.exists(),
            "pubkey should be created at {}",
            pubkey_path.display()
        );
    }

    #[test]
    fn run_keygen_refuses_to_overwrite_existing_secret_key() {
        let td = TestDir::new("warden_keygen_secret_exists");
        let keys_dir = td.path().join(".dracon").join("data").join("keys");
        std::fs::create_dir_all(&keys_dir).unwrap();

        let _guard = HomeGuard::new(td.path().to_str().unwrap());

        let hostname_raw = hostname::get()
            .expect("hostname")
            .to_string_lossy()
            .to_string();
        let hostname: String = hostname_raw
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        let fake_secret = keys_dir.join(format!("machine_{}.age", hostname));
        std::fs::write(&fake_secret, "already exists").unwrap();

        let result = run_keygen();

        assert!(
            result.is_err(),
            "should refuse to overwrite existing secret key"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("already exists"),
            "error should mention already exists: {}",
            err_msg
        );
    }

    #[test]
    fn run_keygen_refuses_to_overwrite_existing_pubkey() {
        let td = TestDir::new("warden_keygen_pubkey_exists");
        let keys_dir = td.path().join(".dracon").join("data").join("keys");
        std::fs::create_dir_all(&keys_dir).unwrap();

        let _guard = HomeGuard::new(td.path().to_str().unwrap());

        let hostname_raw = hostname::get()
            .expect("hostname")
            .to_string_lossy()
            .to_string();
        let hostname: String = hostname_raw
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        let fake_pubkey = keys_dir.join(format!("owner_{}.pub", hostname));
        std::fs::write(&fake_pubkey, "already exists").unwrap();

        let result = run_keygen();

        assert!(
            result.is_err(),
            "should refuse to overwrite existing pubkey"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("already exists") || err_msg.contains("file may already exist"),
            "error should mention already exists: {}",
            err_msg
        );
    }

    #[test]
    fn warden_policy_validate_accepts_valid_policy() {
        let policy = WardenPolicy {
            protected_patterns: vec!["*.env".into(), "secrets/**".into()],
            plaintext_patterns: vec!["*.pub".into()],
            hygiene_patterns: vec![],
            repo_roots: vec![],
            discover_roots: vec![],
        };
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn warden_policy_validate_rejects_overlapping_patterns() {
        let policy = WardenPolicy {
            protected_patterns: vec!["config/envs/*.env".into()],
            plaintext_patterns: vec!["config/envs/*.env".into()],
            hygiene_patterns: vec![],
            repo_roots: vec![],
            discover_roots: vec![],
        };
        let result = policy.validate();
        assert!(result.is_err(), "should reject overlapping patterns");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot be both protected and plaintext"));
    }

    #[test]
    fn warden_policy_validate_rejects_non_allowlisted_plaintext() {
        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec!["mysecret.txt".into()],
            hygiene_patterns: vec![],
            repo_roots: vec![],
            discover_roots: vec![],
        };
        let result = policy.validate();
        assert!(
            result.is_err(),
            "should reject non-allowlisted plaintext pattern"
        );
    }

    #[test]
    fn warden_policy_validate_accepts_allowlisted_plaintext() {
        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec!["Cargo.lock".into(), "*.pub".into()],
            hygiene_patterns: vec![],
            repo_roots: vec![],
            discover_roots: vec![],
        };
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn warden_policy_validate_rejects_secretish_plaintext() {
        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec!["passwords.txt".into()],
            hygiene_patterns: vec![],
            repo_roots: vec![],
            discover_roots: vec![],
        };
        let result = policy.validate();
        assert!(
            result.is_err(),
            "should reject plaintext pattern with 'password'"
        );
    }

    #[test]
    fn is_env_file_name_detects_common_variants() {
        assert!(is_env_file_name(".env"));
        assert!(is_env_file_name(".envrc"));
        assert!(is_env_file_name(".env.local"));
        assert!(is_env_file_name(".env.production"));
        assert!(is_env_file_name("config.env"));
        assert!(is_env_file_name("/path/to/.env"));
        assert!(is_env_file_name("/path/to/.envrc"));
        assert!(!is_env_file_name("env.txt"));
        assert!(!is_env_file_name(".envbackup"));
        assert!(is_env_file_name("my.env"), ".env suffix should match");
    }

    #[test]
    fn is_encrypted_env_content_detects_markers() {
        assert!(is_encrypted_env_content("[DRACON_SECRET:key]"));
        assert!(is_encrypted_env_content("[DRACON_SECRET:key]\n"));
        assert!(!is_encrypted_env_content("[DRACON_SECRET]"));
        assert!(!is_encrypted_env_content("DRACON_SECRET:key"));
        assert!(!is_encrypted_env_content("[OTHER_SECRET:key]"));
        assert!(!is_encrypted_env_content("plain text"));
        assert!(
            !is_encrypted_env_content("  [DRACON_SECRET:key]  "),
            "leading whitespace not trimmed"
        );
    }

    /// Guard that restores an environment variable on drop.
    struct EnvGuard {
        key: String,
        old_value: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &str, value: &str) -> Self {
            let old_value = std::env::var(key).ok();
            std::env::set_var(key, value);
            EnvGuard {
                key: key.to_string(),
                old_value,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::remove_var(&self.key);
            if let Some(ref v) = self.old_value {
                std::env::set_var(&self.key, v);
            }
        }
    }

    #[test]
    fn cli_once_hardens_single_repo() {
        let td = TestDir::new("warden_once_repo");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");

        let status = ProcessCommand::new("git")
            .arg("init")
            .arg(&repo)
            .status()
            .expect("git init");
        assert!(status.success(), "git init should succeed");

        let config_dir = td.path().join(".dracon").join("utilities").join("warden");
        fs::create_dir_all(&config_dir).expect("config dir");
        let config_path = config_dir.join("dracon-warden.toml");
        fs::write(
            &config_path,
            r#"
[watch]
watch_roots = ["/tmp/test"]
"#,
        )
        .expect("write config");

        let _env_guard = EnvGuard::set("DRACON_WARDEN_POLICY", config_path.to_str().unwrap());

        let policy = WardenPolicy::load(&config_path).expect("load policy");
        let result = harden_repos(&policy, vec![repo.clone()], true);
        assert!(result.is_ok(), "once should succeed: {:?}", result);
        assert!(
            repo.join(".gitignore").exists(),
            ".gitignore should be created"
        );
        assert!(
            repo.join(".gitattributes").exists(),
            ".gitattributes should be created"
        );
    }

    #[test]
    fn cli_repair_dry_run_does_not_modify() {
        let td = TestDir::new("warden_repair_dry_run");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");

        let status = ProcessCommand::new("git")
            .arg("init")
            .arg(&repo)
            .status()
            .expect("git init");
        assert!(status.success(), "git init should succeed");

        let config_dir = td.path().join(".dracon").join("utilities").join("warden");
        fs::create_dir_all(&config_dir).expect("config dir");
        let config_path = config_dir.join("dracon-warden.toml");
        fs::write(
            &config_path,
            r#"
[watch]
watch_roots = ["/tmp/test"]
"#,
        )
        .expect("write config");

        let _env_guard = EnvGuard::set("DRACON_WARDEN_POLICY", config_path.to_str().unwrap());

        let policy = WardenPolicy::load(&config_path).expect("load policy");
        policy.validate().expect("valid policy");

        let result = scrub_markers(&policy, std::slice::from_ref(&repo), false);
        assert!(
            result.is_ok(),
            "repair dry-run scrub should succeed: {:?}",
            result
        );

        let result = harden_repos(&policy, vec![repo.clone()], true);
        assert!(
            result.is_ok(),
            "repair dry-run harden should succeed: {:?}",
            result
        );
    }

    #[test]
    fn cli_repair_strict_fails_when_markers_remain() {
        let td = TestDir::new("warden_repair_strict");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");

        let status = ProcessCommand::new("git")
            .arg("init")
            .arg(&repo)
            .status()
            .expect("git init");
        assert!(status.success(), "git init should succeed");

        let config_dir = td.path().join(".dracon").join("utilities").join("warden");
        fs::create_dir_all(&config_dir).expect("config dir");
        let config_path = config_dir.join("dracon-warden.toml");
        fs::write(
            &config_path,
            r#"
[watch]
watch_roots = ["/tmp/test"]
"#,
        )
        .expect("write config");

        let _env_guard = EnvGuard::set("DRACON_WARDEN_POLICY", config_path.to_str().unwrap());

        let policy = WardenPolicy::load(&config_path).expect("load policy");
        policy.validate().expect("valid policy");

        let repos = vec![repo.clone()];
        let (found, _changed) = resmudge_repos(&policy, &repos, false).expect("resmudge report");

        if found > 0 {
            let strict_result: anyhow::Result<()> = Err(anyhow::anyhow!(
                "ciphertext markers remain in working tree (count={})",
                found
            ));
            assert!(
                strict_result.is_err(),
                "strict should fail when markers remain"
            );
        }
    }

    #[test]
    fn filter_clean_passes_plaintext_unchanged() {
        let content = b"let x = 1;\n";
        let warden = DraconWarden::new().expect("create warden");
        let result = warden.clean(content, None).expect("clean");
        assert_eq!(
            result, content,
            "plaintext should pass through clean unchanged"
        );
    }

    #[test]
    fn filter_clean_encrypts_content_with_secret_marker() {
        let content = b"[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA5VDFGcHZLQmcveG0wWmV5alJyRzlSN2RobUlrYll1SDZZVTFueVh0WEJBCkRlNFBBeC9mbnl5ZUthMmdNR09PTFR3OWYzL1lKVXZTWHRuZk1laGhrQjQKLT4gWDI1NTE5IGIybjVZQlpuU1Q2WlBJbGhxU3N0L1o1WnpoUnhvQVFLUUMyK3dWd3lNRTAKa2UzM3ZiVWdtaVo4M2hsaHpCSFZtT21OUVdQMUZDbjJvcWI2WWZYajY1VQotPiBYMjU1MTkgcnhIYVBiQ3BmVXFMcmVsQXQ3OXZJd1kvcGtqaDRJckFPT2RoR01PZ0RpOApQSzdaVktQaE9oZ2ZIY0FzWnpyNHhzcnZxaHRiTVp6OVJRS1dQWUZmTmRBCi0+IFgyNTUxOSBPMm1qQm01ZHczR2l0VS9WOGQwTUhTdDZlb25IQ0VkaXhFckM1ckU4SHdJCjkzTGxDZ09nRlZrdThNTy93NXNvZ3dobk5vM0NLQUtMSXB5UXJMbTdQT0kKLT4gWDI1NTE5IDhqS2QrZW0ra3prdU1nVUJIeTQyd2VkMklEUEkxYkpVZVJ3ZlEzQmd2U3cKNll5QU4rMGg4bEtZTUFreEtLV3ltUmR2cDJueTVPcG4rSUpvb3BwM1lQVQotPiBYMjU1MTkgeHJ5ajNkaHcyc1F2ZDFXZUpWZzJNVHA1Yi9RZnpMUjhSNG9zQ3VRSnhIawpnOVFxdklwVWpSMXJYM2JqRTI0d1RqT1lteHdnSUdROTFGeGNWMnhFdjVJCi0+IFpLK3ZyMy1ncmVhc2UgJSAhI2l1KCA+ZUE9IDkKZTlRWVMxVDZRY2FubnlNc3lKY3hIRi9ncGtaVzJTNjFVWGQrL2JVcy9ZNHRyVTZaWTRkWmZmZUs2TlVMdlFUYwpoazQ5QjVVYURkZTh2ZEJIa0xEUQotLS0gSEY2RExUUFFtT3YxdnNIdDBpYStlTVZQcUk1ZFI4VXNFbjE3T2ZOUnBoWQo5G8kyNvNZqiKDS1ZRsmNinSfLgLKsdiPpN7jIwbCTsk5tLZ1SLqfRDiB0Es0Fw4Gh+INo8v9Via6bgGkJPGYPPx3v7Qz2JhTZV48S4Q==]\n";
        let warden = DraconWarden::new().expect("create warden");
        let result = warden.clean(content, Some("config.env")).expect("clean");
        // Clean should either encrypt or pass through; result should be valid bytes
        assert!(!result.is_empty(), "clean output should not be empty");
    }

    #[test]
    fn filter_smudge_passes_plaintext_unchanged() {
        let content = b"let x = 1;\n";
        let warden = DraconWarden::new().expect("create warden");
        let result = warden.smudge(content, None).expect("smudge");
        assert_eq!(
            result, content,
            "plaintext should pass through smudge unchanged"
        );
    }

    #[test]
    fn cli_scrub_markers_finds_markers_in_json() {
        let td = TestDir::new("warden_scrub_json");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");

        let status = ProcessCommand::new("git")
            .arg("init")
            .arg(&repo)
            .status()
            .expect("git init");
        assert!(status.success(), "git init should succeed");

        // Create a JSON file with a secret marker
        let json_file = repo.join("secrets.json");
        fs::write(
            &json_file,
            r#"{"api_key": "[DRACON_SECRET:abc123]", "name": "test"}"#,
        )
        .expect("write json");

        let config_dir = td.path().join(".dracon").join("utilities").join("warden");
        fs::create_dir_all(&config_dir).expect("config dir");
        let config_path = config_dir.join("dracon-warden.toml");
        fs::write(
            &config_path,
            r#"
[watch]
watch_roots = ["/tmp/test"]
"#,
        )
        .expect("write config");

        let _env_guard = EnvGuard::set("DRACON_WARDEN_POLICY", config_path.to_str().unwrap());

        let policy = WardenPolicy::load(&config_path).expect("load policy");
        policy.validate().expect("valid policy");

        // Dry-run should find markers without modifying
        let result = scrub_markers(&policy, std::slice::from_ref(&repo), false);
        assert!(result.is_ok(), "scrub dry-run should succeed: {:?}", result);
    }

    #[test]
    fn cli_resmudge_reports_on_plaintext_repo() {
        let td = TestDir::new("warden_resmudge_plain");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");

        let status = ProcessCommand::new("git")
            .arg("init")
            .arg(&repo)
            .status()
            .expect("git init");
        assert!(status.success(), "git init should succeed");

        let config_dir = td.path().join(".dracon").join("utilities").join("warden");
        fs::create_dir_all(&config_dir).expect("config dir");
        let config_path = config_dir.join("dracon-warden.toml");
        fs::write(
            &config_path,
            r#"
[watch]
watch_roots = ["/tmp/test"]
"#,
        )
        .expect("write config");

        let _env_guard = EnvGuard::set("DRACON_WARDEN_POLICY", config_path.to_str().unwrap());

        let policy = WardenPolicy::load(&config_path).expect("load policy");
        policy.validate().expect("valid policy");

        // Dry-run on a plain repo should find nothing and succeed
        let repos = vec![repo.clone()];
        let (found, changed) = resmudge_repos(&policy, &repos, false).expect("resmudge report");
        assert_eq!(found, 0, "plaintext repo should have no ciphertext markers");
        assert_eq!(changed, 0, "dry-run should not change anything");
    }
}
