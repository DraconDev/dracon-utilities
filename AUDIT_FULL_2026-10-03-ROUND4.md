# AUDIT FULL — ROUND4 (2026-10-03)

Date: 2026-10-03
Scope: dracon-utilities post-R3 releases — sync 0.113.93, system 0.112.44, warden 0.113.15, security 0.4.0
Method: read-only source audit merged from six component reports; all file:line refs verified by read in source reports; two checker findings proven by fixture execution; exclude.rs findings proven by replica repro; warden clean refusal proven by live binary probe.
Sources: /tmp/r4-sync-core.md, /tmp/r4-sync-report.md, /tmp/r4-system.md, /tmp/r4-warden.md, /tmp/r4-meta.md, /tmp/r4-regression.md
Dedupe: by root cause. One merge applied: R4-02 (regression) is the same defect as R4-SYS-07 (doctor legacy-config hint); folded into R4-SYS-07 below.
Severity: HIGH = data loss / wrong push / secret leak; MEDIUM = correctness gap; LOW = hardening / docs / tests.

## Counts (79 unique findings)

| Component | HIGH | MEDIUM | LOW | Total |
|---|---|---|---|---|
| sync-core (SC) | 0 | 6 | 11 | 17 |
| sync-report (SR) | 0 | 5 | 12 | 17 |
| system (SYS) | 0 | 6 | 12 | 18 |
| warden (W) | 1 | 3 | 7 | 11 |
| meta/shell/release (M) | 1 | 8 | 6 | 15 |
| regression-new (R4-01) | 0 | 1 | 0 | 1 |
| Total | 2 | 29 | 48 | 79 |

## HIGH (2)

### R4-W-01 — HIGH — pre-commit does not verify filter.dracon.required=true; fail-closed clean degrades to silent plaintext passthrough
Evidence: dracon-warden/src/main.rs:5193 (pre-commit checks only process/clean --local); :1721 (ensure sets required=true, never re-verified); :3873-3884,:4464-4484 (oversize/encrypt failures rely on git aborting); man gitattributes (missing/nonzero filter is no-op passthru unless required=true; process-protocol error/abort exit follows required flag).
Impact: if required drifts (hand-edit, partial config, --unset), an oversize refusal or encrypt error commits the file UNENCRYPTED with exit 0 — the exact leak the refusal exists to prevent.
Fix: add `git -C "$REPO" config --local filter.dracon.required | grep -qx true` to PRE_COMMIT_HOOK alongside the line-5193 check.

### R4-M-01 — HIGH — AWS secret passed as CLI argv (process-table leak)
Evidence: scripts/rotate-dracon-platform-aws-key.sh:15,:184-185 (NEW_SECRET="$2"; usage $0 <ID> <SECRET>). Script is careful elsewhere (OLD key substring :191; redacted output :286) but argv intake defeats that.
Impact: NEW AWS secret visible to ps on a shared host and persisted in shell history.
Fix: read the secret from env (read -rs), a file descriptor, or age file; never argv. Same for the key ID.

## MEDIUM (29)

### R4-SC-01 — MEDIUM — clean mirror-only repos never retry pushes; no-op cycle wipes stuck ledger
Evidence: dracon-sync/src/sync.rs:5673-5697 (ahead CLI fallback only when detached); :5727-5732 (should_push = ahead>0 || upstream_ref_missing; both false with no upstream); :5876-5879 (no-push returns Attempted{ok:true} → NothingToDo); daemon.rs:860-862 (NothingToDo clears stuck ledger); daemon.rs:7884-7895,:8323-8324 (daemon dispatches clean repos on mirror-ahead override, so the no-op repeats every cycle). Same root disarms the backstop (sync.rs:5100-5117 initial_ahead=0 for attached mirror-only).
Impact: failed mirror push on clean mirror-only repo never retried; each dispatched no-op clears stuck-ledger entry + failure_count (false-healthy; retry budget never engages).
Fix: make handle_ahead_push mirror-aware (count_pushable_unpushed_vs_mirrors, or pass daemon mirror-ahead override into sync_repo); or return a retain-activity outcome instead of NothingToDo when mirror-ahead > 0.

### R4-SC-02 — MEDIUM — 15-min wedge abort burns failure budget and wipes per-remote pause memory
Evidence: daemon.rs:9055-9063 (abort request); :8807-8819 (wrapper converts JoinError to Err + EMPTY remote_failures); :826 (apply_outcome overwrites entry.remote_failures unconditionally); :938-945 (Err arm → Failure, no cancellation check); :8892-8901,:9004-9009 (failure_count += 1). Contrast push-level fix sync.rs:5841-5848 (R3-L13 maps cancellation to AllPaused).
Impact: aborted sync task counts as real failure (feeds MAX_FAILURES backoff) and discards sick-remote pause counters; next cycle re-hammers backing-off remotes.
Fix: check push_error_is_cancellation in the apply path (treat as PushPaused-like retain; do not overwrite entry.remote_failures with the empty map).

### R4-SC-03 — MEDIUM — sem_max_concurrent_sync documented but unenforced (unbounded sync spawn)
Evidence: policy.rs:1288-1290 (default 4); daemon.rs:8668-8674 (comment claims dispatch bounded by policy.sem_max_concurrent_sync); :8739-8764 (every eligible repo spawned; no semaphore/acquire/limit in daemon.rs); daemon.rs:148,:156 (status/classification got global caps; sync workers did not).
Impact: fully-dirty fleet spawns N concurrent sync_repo workers with git subprocesses and multi-minute pack measurements — the self-stampede the caps were built to stop.
Fix: enforce the semaphore (or a to_sync length cap) around collection/spawn.

### R4-SC-04 — MEDIUM — 2 GiB pack guard blocks a tokio worker up to 600 s inline
Evidence: sync.rs:2213 (github_pack_too_large inline in async push_background); git/mod.rs:576-637 (compressed_pack_bytes: blocking try_wait + std sleep loop, 600 s ceiling); :684-759 (blob_size_sum: blocking child.wait + reads). Contrast git/staging.rs:295-297 (detect_large_blobs_ahead uses spawn_blocking + timeout); short-circuit git/mod.rs:168 helps only converged repos.
Impact: one big non-converged repo pins a runtime worker for minutes (600 s / 700 MB RSS cases in comments); with R4-SC-03 several such repos starve the runtime.
Fix: run the guard measurement in spawn_blocking with a timeout.

### R4-SC-05 — MEDIUM — steady-state path has no post-stage oversize sweep (TOCTOU commit)
Evidence: sync.rs:620-648 (clean_staged_paths unstages excluded/oversized BEFORE staging); :4312-4340 (stage → read staged set → commit, no re-measure); :1282 (directory-expansion gate stats worktree file only). Contrast bootstrap post-stage sweep sync.rs:4786-4863 (R3-M1).
Impact: stage-large-then-truncate in the same cycle commits an oversized blob past the gate; concurrent operator/git-add of a large file between clean and commit is swept in unchecked.
Fix: run a staged_blob_sizes_for sweep after staging, before commit (same as bootstrap).

### R4-SC-06 — MEDIUM — GitHub HTTPS fallback leg has no credential wiring, cannot succeed
Evidence: git/push.rs:57-73 (github leg pushes https URL with only GIT_TERMINAL_PROMPT=0, no askpass/token) vs :75-117,:119-159 (gitlab/codeberg legs build GIT_ASKPASS from GITLAB_TOKEN/CODEBERG_TOKEN).
Impact: when SSH fails, github HTTPS fallback fails auth (unless an ambient credential helper exists — none found in repo), so transport fallback is ineffective exactly for github-primary repos.
Fix: wire GH_TOKEN through git_askpass_script on the github leg like the other forges.

### R4-SR-01 — MEDIUM — exclude.rs .tmp- branch prefix check is tautological → over-exclusion
Evidence: dracon-sync/src/exclude.rs:980-989 (prefix derived FROM normalized, so starts_with always true; pattern chars never compared; only length + '-' at len-1 tested). Repro: is_excluded_dir_name("abcd-xyz", {".tmp-"}) == true (want false). Default .tmp-* takes the glob branch; only user patterns .<x>- (no *) hit this branch.
Impact: any dir name with '-' at the same index is excluded → dirty files the daemon should commit are silently skipped (fail-silent wrong-clean via is_excluded_change_path :998, should_stage_entry :1537).
Fix: compare against the pattern (prefix == &normalized_pattern[..k]); add regression test with non-matching same-shape name (abcd-xyz vs .tmp-).

### R4-SR-02 — MEDIUM — report.rs uses REPLACE, not the UNION helper → report/daemon divergence
Evidence: dracon-sync/src/report.rs:3958-3961 (as_deref().unwrap_or(&policy...) REPLACE) vs policy.rs:1107-1116 (effective_auto_commit_excludes UNION helper; doc :1100-1106 claims "All consumers go through this helper" — false here; daemon gates do use it :8244-8256,:8312-8322).
Impact: with a per-repo list, global patterns are dropped from classify_dirty_entries → excluded_dirty undercounted, committable overcounted → false dirty/WARN rows; repos --json disagrees with what the daemon will stage.
Fix: use effective_auto_commit_excludes(policy, &repo_override); update stale "fallback global" doc at report.rs:1296-1297.

### R4-SR-03 — MEDIUM — restore arm ignores exclusion policy + revert flag
Evidence: dracon-sync/src/exclude.rs:1887-1913 (has_sync_relevant_dirty_entries true if should_stage || can_restore || is_large_untracked); can_restore_entry :1640-1646 (_repo unused; true for ANY Modified/TypeChange/Renamed regardless of exclusion). Tracked Modified under excluded dir/pattern is always "relevant" even with revert_excluded_to_head=false (default), where the daemon never restores. Contrast Added-only test :238-260.
Impact: repos whose only dirt is policy-excluded tracked modifications never read clean at the gate (daemon.rs:8249,:8315; report.rs:8882) → perpetual dispatch/eligibility churn; user-visible magnitude depends on downstream damping (fingerprints/quiet windows, out of scope).
Fix: thread exclusion + effective revert_excluded_to_head into the restore arm (restore counts only when the daemon would restore), or document excluded-Modified as intentionally relevant and gate dispatch on committable-only.

### R4-SR-04 — MEDIUM — malformed per-repo TOML silently drops ALL overrides incl. owned=false
Evidence: policy.rs:1072-1081 (load_repo_override: any toml::from_str error → eprintln + RepoPolicyOverride::default()). One typo in <repo>/.dracon/dracon-sync.toml discards owned=false, exclude_remotes, auto_skip_unowned; daemon proceeds as if no override exists.
Impact: operator-intended opt-outs silently void → daemon auto-commits/pushes repos it was told to leave alone (wrong-push direction). The eprintln is the only signal.
Fix: fail closed per-repo on parse error (concern / skip with alert), or salvage-parse safety keys; at minimum record an incident-ledger entry, not just stderr.
