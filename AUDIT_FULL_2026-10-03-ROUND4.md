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

### R4-SR-05 — MEDIUM — flip_repo_visibility mixes owner/repo identity sources
Evidence: visibility.rs:723 (GitHub owner parsed from origin_url) but repo name remapped through possibly different remote's repo_name_map :726-728; GitLab/Codeberg use local basename :746-747,:775-776 (doc :708), not URL-derived name.
Impact: with stale origin (live cases visibility.rs:292-295, report.rs:1430) or renamed local dir, explicit make-public/make-private can address the wrong owner/repo pair — including flipping an unrelated same-account repo. gh/API 404s make the common case noisy-but-safe; the collision case is not.
Fix: derive ONE (owner, repo) identity per target remote from that remote's own resolve_account+resolve_repo_name; refuse when origin-derived and config-derived identities disagree (same fail-closed posture as same_host_project_divergence :404-432).

### R4-SYS-01 — MEDIUM — checker step 6 ignores DRACON_SYSTEM_POLICY, checks wrong file
Evidence: dracon-system/scripts/check-unit-deployment.sh:305 (policy_file falls back to HOME default, no DRACON_SYSTEM_POLICY) while the daemon resolves the override (src/main.rs:7125-7130, honoured by doctor src/doctor.rs:34-41). Observed: with DRACON_SYSTEM_POLICY pointing at a policy naming an RO quarantine root and no default policy present, the script exits 0 "OK" (fixture probe /tmp/r4fix).
Impact: runtime contract checked against roots the daemon never uses — false pass, or false fail on unused roots.
Fix: policy_file="${POLICY_FILE:-${DRACON_SYSTEM_POLICY:-${HOME:-}/.dracon/utilities/system/dracon-system.toml}}".

### R4-SYS-02 — MEDIUM — checker silently skips single-quoted storage roots (plus bare-~ gap)
Evidence: dracon-system/scripts/check-unit-deployment.sh:195-199 (sed matches only key = "value"; TOML accepts single quotes, so quarantine_dir = '/mnt/data/q' reads as empty → check_storage_root_writable :232 returns 0). Observed: identical RO-mountinfo fixture fails double-quoted, passes single-quoted (/tmp/r4fix). Related: :233-237 handles ~/* but bare ~ falls into *) → skipped, while the daemon expands bare ~ (src/policy.rs:1378-1382).
Impact: live config the checker reads as empty → false OK on unwritable roots.
Fix: match both quote styles in storage_root_for_key; map bare ~ to $HOME. (Fix direction depends on whether ops wants these styles supported or rejected daemon-side.)

### R4-SYS-03 — MEDIUM — doctor service probes blind on non-NixOS (NixOS-only bin paths)
Evidence: src/main.rs:3949-3978 (resolve_bin_opt searches only /run/current-system/sw/bin, /etc/profiles/per-user/dracon/bin, /nix/var/nix/profiles/default/bin); src/doctor.rs:46 (strict resolver for probe); src/main.rs:7132-7136 (returns false on miss). On a plain distro (systemctl at /usr/bin) every service/timer check reports n/a. Shipped unit explicitly serves plain distros (dracon-system-guard.service:23-27).
Impact: answerable checks report n/a on non-NixOS.
Fix: fall back to PATH lookup (command -v) when the store dirs miss. (Priority depends on confirming non-NixOS is a supported doctor target vs NixOS-only.)

### R4-SYS-04 — MEDIUM — is_git_tracked fail-OPEN when git is missing
Evidence: src/relocate.rs:196-204 (parent probe `_ => return Ok(false)` covers git spawn failure as well as "not a repo"), contradicting the fn contract "inspection failures bail (fail closed)" (:163-164). On a gitless host every tracked dir reads untracked, voiding refuse-to-relocate-tracked (:238-243) incl. the no-override auto path (src/main.rs:6736, allow_tracked=false); unwrap_or(true) in find_cold_candidates (:592) cannot help because this returns Ok(false).
Impact: tracked guard void on gitless hosts.
Fix: bail on spawn error; return false only on a clean non-zero rev-parse.

### R4-SYS-05 — MEDIUM — apply_relocate never re-checks free space (stale-plan strand)
Evidence: space checked at plan time only (src/relocate.rs:268-287); apply_relocate (:302-337) re-validates dest-absence and source shape but never fits. ENOSPC mid-copy bails with source intact but leaves a partial dest blocking every retry via "already exists" (:261-263,:312-317) — the strand the R3-L28 comment describes for unknown space, reachable here via a stale plan.
Impact: one stale plan strands all retries behind a partial dest.
Fix: re-run the avail check at apply start; and/or remove the partial dest on copy/verify failure.

### R4-SYS-06 — MEDIUM — uncovered_storage_roots omits log/guard-log/freeze-marker roots
Evidence: src/safety.rs:365-380 (checks only quarantine_dir and relocate_cold_root). Operator-repointed log_dirs (auto_truncate_logs), guard_log_file, or sync_freeze_marker outside ReadWritePaths= fails EROFS per candidate with no startup warning — the silent-reclaim-death class the unit comment documents (dracon-system-guard.service:61-67) and this machinery was built to catch.
Impact: silent reclaim death on repointed roots.
Fix: include the three roots (via effective_log_dirs for log_dirs).

### R4-W-02 — MEDIUM — "primary enforcement layer" is client-side hooks, bypassable end-to-end
Evidence: dracon-warden/README.md:3,:187-188 (hooks are "the primary enforcement layer"); src/main.rs:5211 (pre-push "catches --no-verify bypass of pre-commit", but nothing catches git push --no-verify); -c core.hooksPath=<empty> / GIT_CONFIG_* / fresh clone without setup-hooks skip all three hooks (git built-ins, no server-side control in scope).
Impact: commit --no-verify + push --no-verify pushes Tier-2/plaintext secrets with zero client checks.
Fix: document hooks as advisory defense-in-depth (not enforcement), and/or add a server-side scan; at minimum reword the README claim.

### R4-W-03 — MEDIUM — pre-commit verifies 2 of 5 managed keys; merge/diff driver drift undetected
Evidence: src/main.rs:1719-1737 (ensure sets process, required, diff.dracon.textconv, merge.dracon.driver, merge.dracon.name); :5178,:5193 (hook checks only .gitattributes filter line + filter process/clean keys); :1707-1711,:4535-4536 (code comment: without these keys "git would fall back to the text driver" and "a conflict yields undecryptable garbage").
Impact: drifted merge driver merges ciphertext (undecryptable conflict output); drifted textconv diffs ciphertext. (Whether git aborts vs falls back to text merge when the driver is undefined is unresolved — man page inconclusive; affects impact wording.)
Fix: verify merge.dracon.driver + diff.dracon.textconv in PRE_COMMIT_HOOK.

### R4-W-04 — MEDIUM — filter-process smudge ceiling diverges from one-shot: oversize blobs fail checkout under the installed driver
Evidence: src/main.rs:4427-4444 (past passthrough_ceiling_bytes=4x limit the driver emits status=error in EVERY direction incl. smudge); :3885-3895 (one-shot filter-smudge streams oversize blobs via std::io::copy, no ceiling); :1720 (installed driver IS filter-process, so the failing path is the common one).
Impact: blob over the ceiling (limit later lowered, pushed from higher-limit machine, exemption removed) makes checkout/diff of that file fail under filter-process while filter-smudge handles it. Narrow but real in a fleet.
Fix: document, or relay smudge-direction blobs to the git-side limit instead of erroring.

### R4-M-02 — MEDIUM — warden release runs zero AGENTS.md test gates
Evidence: dracon-warden/scripts/release.sh:111 (TOTAL_STEPS=7), :279-291 (step 1 is the version bump; no cargo test/build/clippy, no cargo deny, no run_gate anywhere in the file). AGENTS.md:758-762 mandates four gates (test, build --release, deny, clippy -D warnings). Sync (:296-338) and system (:316-348) run all four pre-mutation; warden bumps first and can publish an untested tree — the class sync's header calls out as fixed 2026-08-10.
Impact: untested/broken crate published to crates.io (wrong-push class).
Fix: port the sync/system step-1 gate block (incl. run_deny_gate) ahead of the warden bump.

### R4-M-03 — MEDIUM — sync/warden release notes ship broken GitHub links + 404 unit URL
Evidence: dracon-sync/scripts/release.sh:399 (compare ...v${VERSION} on hardcoded DraconDev/dracon-utilities); dracon-warden/scripts/release.sh:350 (same); dracon-sync/scripts/release.sh:393 (curl unit from .../dracon-utilities/main/dracon-sync/dracon-sync.service). Tags are <crate>-v<version> (all three TAG= lines), so v${VERSION} never exists; post-D1 the utility repos are standalone (.gitignore:151-153 ignores dracon-sync/ etc., so the parent has no dracon-sync/dracon-sync.service to curl → 404). dracon-system/scripts/release.sh:440,:434 already do this right (GH_PATH + ${TAG}).
Impact: every sync/warden GitHub release publishes dead links.
Fix: port system's GH_PATH resolution + ${TAG} compare link; point the systemd curl at the utility repo (${GH_PATH}/main/dracon-sync.service).

### R4-M-04 — MEDIUM — check-flake asserts Documentation the guard watchdog lacks both sides
Evidence: scripts/check-flake.sh:242,:246 assert services/timers.dracon-system-guard-watchdog.Unit.Documentation == "https://github.com/DraconDev/dracon-utilities", but flake.nix:460-465 (service) and :474-478 (timer) define only Description/After, and the shipped dracon-system/dracon-system-guard-watchdog.service/timer likewise carry no Documentation (sync/freeze pairs have it in both places). Harness types services as attrsOf anything with no defaults (:58-61), so a missing attribute is a Nix eval error, not a mismatch; author knew the ? guard (:124) but didn't use it here. CI wires this script (.github/workflows/ci.yml:425).
Impact: either a red gate or newly added and unexercised (nix eval not run to confirm; static mismatch is conclusive).
Fix: add Documentation to the flake guard-watchdog Units AND the shipped units (keeping the assertions), then run scripts/check-flake.sh.

### R4-M-05 — MEDIUM — repin recreates the whole flake lock inside a per-utility loop
Evidence: scripts/repin-nested-sources.py:125 (nix flake lock --recreate-lock-file inside for checkout, node, ...). Each drifted utility triggers a full lock recreation (up to 3x), moving UNRELATED inputs (nixpkgs) as a side effect of a utility repin. Comment admits --update-input wasn't available on some nix.
Impact: non-hermetic, unreviewable lock churn.
Fix: hoist to one recreation, or prefer nix flake lock --update-input <*-src> per drifted input.

### R4-M-06 — MEDIUM — install.sh rewrites global git config, undisclosed
Evidence: install.sh:132-141 (git config --global init.defaultBranch main). Runs on every mode incl. --binaries-only (only --dry-run guarded), yet --help (:4-20) never mentions it.
Impact: binary installer mutates global VCS config as a side effect.
Fix: restrict to interactive full installs (skip under --binaries-only), document in --help, or drop (git ≥2.28 default hint suffices).

### R4-M-07 — MEDIUM — rotate script corrupts/mishandles real-world secrets
Evidence: scripts/rotate-dracon-platform-aws-key.sh:231-232 (sed -i "s|^SES_SECRET_KEY=.*|SES_SECRET_KEY=$NEW_SECRET|" — $NEW_SECRET unescaped: | breaks the delimiter, &/backslash inject match text); :276-277 (cut -d= -f2 truncates base64 = padding, so a secret ending in = always fails the read-back check at :282 with exit 5). AWS secret keys are 40-char base64 — = padding and +/ chars are realistic.
Impact: corrupted .env writes; false read-back failures.
Fix: replace via python3/env (no sed interpolation); parse with ${line#*=} or cut -d= -f2-.

### R4-M-08 — MEDIUM — sync/warden releases accept downgrades; system refuses
Evidence: dracon-sync/scripts/release.sh:340-355, dracon-warden/scripts/release.sh:279-291 (no monotonicity check) vs dracon-system/scripts/release.sh:295-301 (sort -V refusal before any mutation, added 2026-10-01). Same-version re-run is intentional idempotency, but a LOWER version rewrites the manifest, closes the CHANGELOG under a misleading header, and only fails late at the registry — local release surfaces left mutated with manual recovery.
Impact: misleading release surfaces + manual recovery on downgrade typos.
Fix: port system's pre-mutation guard (allow == for re-runs if desired, refuse <).

### R4-M-09 — MEDIUM — orphan cleanup deletes GitHub repos on a bare flag, no confirm
Evidence: scripts/cleanup-github-orphans.sh:19-21 (--apply), :61-73 (gh repo delete --yes per repo, irreversible). Dry-run is the default (good), but one --apply deletes N repos with no count confirmation or backup; a stale gh listing or wrong-org typo (org hardcoded :26) becomes mass deletion.
Impact: irreversible mass deletion on one flag.
Fix: print the count + require typed confirmation (type DELETE N repos).

### R4-01 — MEDIUM (trigger unresolved) — clean filter hard-refuses absolute paths
Evidence: dracon-warden/src/main.rs:3477-3482 (refusal), :3873-3884 (one-shot Err), :3946-3957 (process driver Err); live probe: filter-clean /tmp/abs-probe.txt → exit 1 "refusing to clean absolute filter path", relative path → exit 0 passthrough.
Impact: IF any caller (lead claims cargo/gix on dirty trees) invokes clean with an absolute path, every git add aborts → cargo publish/package breaks in warden-managed repos. Real git always sends repo-relative %f/process paths, so the guard is dead code on the git path — pure availability risk, no leak. gix-absolute behavior UNVERIFIED (no vendored gix sources, no network).
Fix: relativize-then-guard — strip an absolute path to repo-relative when it resolves under the repo root (or accept basename + containment check), still refusing .. escapes and outside-root absolutes; add a regression test.

## LOW (48)

### R4-SC-07 — LOW — detached_discard stale-result marker never inserted (dead M1 path)
Evidence: daemon.rs:6571 (declared) + :6560-6562 (comment: "force-cleared and possibly re-dispatched"); :8948-8949 (only .get/.remove at check site); :9055-9063 (wedge path requests cancellation but never inserts a marker). Repo-wide grep confirms no insert site.
Impact: currently benign (ownership retained until join, so no force-clear race exists), but the M1 per-generation machinery is untestable-by-construction and would silently not fire if force-clearing ever returns.
Fix: insert the marker on abort, or remove the dead path and its test-only helper.

### R4-SC-08 — LOW — raw Command::new("git") bypasses DRACON_SYNC_GIT_BIN + prompt sealing
Evidence: sync.rs:956 (repo_has_warden_filter), :3982 + :4026 (auto_resolve_unmerged), :4102 (check_untracked_threshold) vs policy.rs:411-435 (git_binary honors DRACON_SYNC_GIT_BIN/Nix paths; GitCommand::new seals SSH prompt env). (Sibling pattern: R4-SR-13 covers the same class in ownership.rs.)
Impact: inconsistent git binary (breaks the DRACON_SYNC_GIT_BIN test seam and Nix layout), unprompt-sealed subprocess env.
Fix: use policy::std_git_command() at all four sites.

### R4-SC-09 — LOW — fetch-first auto-pull refspec skips the branch safety check
Evidence: git/push.rs:350-353 (pull_refspec from current_branch with no is_safe_branch_name gate) vs :290-301 (push refspec bails on unsafe names) and :194-206 (same in transport fallback).
Impact: exotic local branch name reaches `git pull origin <ref>` unvalidated (arg confusion); also pulls the explicit branch while pushing HEAD — a mid-detach race can merge an unexpected ref.
Fix: validate with is_safe_branch_name and bail like the push paths.

### R4-SC-10 — LOW — staging pathspecs are not :(literal)-quoted (glob filenames)
Evidence: sync.rs:1429-1432 (git add -A -- <raw paths>) vs git/staging.rs:154 (:(literal) prefix used for ls-files pathspecs for the same reason).
Impact: operator/expansion filenames containing glob magic ([, *) interpreted as pathspec patterns — mis-staged or silently skipped files.
Fix: prefix :(literal) (or --literal-pathspecs) on the git add argv.

### R4-SC-11 — LOW — conflict guard misses in-progress revert (and bisect)
Evidence: sync.rs:483-506 checks rebase-merge/rebase-apply/MERGE_HEAD/CHERRY_PICK_HEAD only; sync.rs:3628-3633 (compute_blast_radius detects .git/MERGE_HEAD + REVERT_HEAD for the message) proves mid-revert commits are reachable.
Impact: daemon stages/commits/pushes through an operator's in-progress git revert.
Fix: add is_revert_in_progress (.git/REVERT_HEAD, via state_path_exists) and bisect guards to check_conflict_state.

### R4-SC-12 — LOW — has_origin_remote config string-parse can false-negative with no fallback
Evidence: git/status.rs:84-99 (when <repo>/.git/config is readable, returns the line.trim() == "[remote \"origin\"]" verdict directly; trailing comments, casing, or include-based remotes read as absent; git-CLI fallback only runs when the file is unreadable).
Impact: origin exists but reads absent → skipped pulls, spurious "created remote" attempts (sync.rs:5091 ensure_origin_remote), wrong has_local_or_pending_work.
Fix: fall back to `git remote get-url origin` on a parse negative, or parse robustly.

### R4-SC-13 — LOW — origin retry budget exceeds the mirror budget (~5 pushes vs 3)
Evidence: git/push.rs:277-281 (attempts loop) + :427-437 (extra push_with_transport_fallbacks attempt after the loop exhausts; R3-L02 comment acknowledges retries=0 → up to 5 pushes) vs git/multi_remote.rs:697 (mirror budget counts TOTAL attempts, min 1).
Impact: sick origin hammered harder per cycle than a sick mirror; asymmetric load and inconsistent operator expectations.
Fix: count the fallback sweep inside the same total budget as the mirror path.

### R4-SC-14 — LOW — detect_large_blobs_ahead fails open (empty) when rev-list fails
Evidence: git/staging.rs:298-305 (@{u}..HEAD rev-list non-success → Ok(vec![])).
Impact: repos without upstream (mirror-only) or with transient rev-list errors silently disable the >100 MiB rewrite guard — exactly the repos that most need it.
Fix: propagate the error (caller decides) or fall back to a merge-base/whole-branch measure.

### R4-SC-15 — LOW — index.lock contention on git add fails the whole sync (no backoff)
Evidence: sync.rs:1433-1441 (normal-path git add error propagates as Err → sync Failure); only the filter-only reset path treats lock contention as non-fatal (:4347-4356, comment names sync-now/warden contention explicitly).
Impact: transient contention with the CLI or warden burns failure budget and can trip MAX_FAILURES backoff for a healthy repo.
Fix: retry index.lock failures with a short backoff and/or map them to a non-failure retain outcome.

### R4-SC-16 — LOW — operator-concurrent staging is swept into the auto-commit
Evidence: sync.rs:4336-4340 (commit set = full diff --cached, not the filtered to_stage list); Blocked arm deliberately leaves the index intact (:4403-4408) but the success path commits whatever is staged, including manual git adds landing between clean_staged_paths and commit.
Impact: operator-staged work committed under a mechanical message without consent window.
Fix: verify the staged set ⊆ intended paths before commit (or commit an explicit pathspec).

### R4-SC-17 — LOW — trailing-drain "still running" count includes previous-cycle pending repos
Evidence: daemon.rs:8920 (dispatched_this_cycle seeded from in_flight.clone(), which still holds previous cycles' detached-registry repos); :9030-9032 (message reports dispatched_this_cycle.len() as this cycle's still-running tasks).
Impact: misleading per-cycle drain message (over-counts); detached_since re-insert is guarded by or_insert so no timestamp harm.
Fix: seed dispatched_this_cycle from this cycle's to_sync repos only.

### R4-SR-06 — LOW — glob * branch is case-sensitive (raw pattern vs lowercased name)
Evidence: exclude.rs:991 (normalized.starts_with(&pattern[..len-1]) uses RAW pattern while normalized is lowercased; exact-match branch :974-977 normalizes both sides). Repro: is_excluded_dir_name("target-x", {"Target*"}) == false; docs/tests (:66-70) promise case-insensitive matching.
Impact: documented case-insensitive patterns silently fail to exclude.
Fix: slice normalized_pattern instead of pattern; add Target* regression test.

### R4-SR-07 — LOW — patterns with 2+ * never match
Evidence: matches_file_pattern generic arm requires parts.len() == 2; *-test-* → 3 parts → falls through to false. Repro: matches_file_pattern("my-test-file", "*-test-*") == false. Also affects per-segment matching in rel_* helpers (:1056-1130).
Impact: user multi-wildcard patterns silently never match.
Fix: implement multi-* ordered subsequence matching or reject/validate such patterns at config load with a warning.

### R4-SR-08 — LOW — repair-warns uses GLOBAL-ONLY auto-commit patterns
Evidence: report.rs:8882-8888 (has_sync_relevant_dirty_entries(..., &policy.auto_commit_exclude_patterns) — per-repo override ignored entirely, no load_repo_override in this path), unlike the UNION contract (policy.rs:1100-1116).
Impact: repos excluded only per-repo get spurious warn-repair plans + incident-ledger planned records for dirt the worker would never touch (real_is_dirty at :8912 still drives selection, so noise, not data loss).
Fix: load the per-repo override and pass effective_auto_commit_excludes.

### R4-SR-09 — LOW — repos --json emits null, contradicting the "never null" contract
Evidence: RepoReportRow (report.rs:1487-1645, plain Serialize derive, no skip_serializing_if) has codeberg_skip_reason :1548, git_size_bytes :1563, git_modules_bytes :1574, frozen_secs :1644 → serialize as null. Contract (:1661-1662) says "Absent values use the - sentinel … — never null."
Impact: contract/consumer confusion.
Fix: document the four nullable fields in the contract (preferred — changing to sentinels breaks consumers) or add skip_serializing_if.

### R4-SR-10 — LOW — codeberg skip reason: undocumented values + misleading multi-remote annotation
Evidence: field doc (report.rs:1537-1548) allows private/unknown/None, but construction (:4651-4675) can emit "quota" (:4670) and "public" (:4665, "shouldn't happen" TOCTOU between two cached_repo_visibility reads). Renderer (:737-744) folds the codeberg-specific reason onto the WHOLE excl list: github,codeberg excl + private renders [… [github,codeberg:private]], misattributing the reason to github.
Impact: undocumented values break consumers; misattributed reason misleads operators.
Fix: document quota (+ handle public explicitly, e.g. fall back to unknown); render the reason against codeberg only.

### R4-SR-11 — LOW — report embeds stuck-push last_error verbatim; no report-side redaction
Evidence: report.rs:4385-4394 (HINT) and :4428 (push_error) clone info.last_error with no redact_url_credentials call; report.rs contains zero redaction calls (verified by search). Mitigation (read, not assumed): daemon redacts at ledger-write time (daemon.rs:5271,:5291-5296). Residual: pre-2026-08-11 ledger entries and any future writer bypass flow unredacted into table + JSON.
Impact: residual credential-echo risk in report output.
Fix: one redact_url_credentials at report construction (defense in depth, cheap).

### R4-SR-12 — LOW — stale doc references non-existent global policy.exclude_remotes (doc-only)
Evidence: report.rs:1532-1533 "(or by the global policy.exclude_remotes)"; no such SyncPolicy field exists (verified by search — only RepoPolicyOverride.exclude_remotes, policy.rs:1018; daemon report paths all start from the per-repo list: daemon.rs:1045, sync.rs:2375). No behavior drift.
Fix: correct the comment to per-repo-only.

### R4-SR-13 — LOW — ownership.rs raw git, unbounded, blocking async workers
Evidence: git_config_user_email / git_head_author_email / git_head_author_name / git_origin_url (ownership.rs:572-638) use Command::new("git") (not policy::std_git_command), no timeout, synchronous; report calls detect_* fresh per repo inside async row futures (report.rs:4298-4314, 4 subprocesses × N repos). (Sibling pattern: R4-SC-08 covers the same class in sync.rs.)
Impact: DRACON_SYNC_GIT_BIN override ignored; a wedged git blocks a tokio worker (fast local reads, so hang risk is low — availability, not safety).
Fix: route through std_git_command() + bounded wait (or spawn_blocking).

### R4-SR-14 — LOW — report reads per-repo override twice per row (perf + TOCTOU skew)
Evidence: load_repo_override(&repo) at report.rs:3938 (drives excludes/remotes) and again at :4289 (ownership). Two file reads + parses per repo per run; a concurrent edit (or one transient parse failure) makes classification and ownership disagree.
Impact: perf waste + classification/ownership skew under concurrent edit.
Fix: load once, reuse the binding.

### R4-SR-15 — LOW — secrets.rs minimal .env dialect; misleading readability warning
Evidence: parser (:84-119) splits on first =, trims, keeps the rest verbatim: quoted values (GH_TOKEN="abc" → token includes quotes), export KEY=val lines (key mismatch → skipped), inline comments (GH_TOKEN=abc # x → value includes comment) all fail closed into auth errors. Separately, warn_if_world_readable (:180-191) triggers on 0o044 (group OR other) but always reports "world-readable". No leak (control-char refusal :94-113 verified + tested).
Impact: fail-closed confusion only.
Fix: strip matching quotes / export prefix / unquoted trailing comments (document the dialect); fix the warning to name group vs other.

### R4-SR-16 — LOW — --warn filter retains concern rows but the warn bucket excludes them
Evidence: counts (report.rs:3815-3818) define warn as warn && !active && !concern, but RepoFilter::Warn retains r.warn && !r.active (:4747) — concern rows with warn=true are listed under a header whose warn=N excludes them.
Impact: header/row count inconsistency.
Fix: retain r.warn && !r.active && !r.concern for consistency.

### R4-SR-17 — LOW — stale REPLACE/inherit docs post-UNION; "all consumers" claim is false (doc-only)
Evidence: RepoPolicyOverride.auto_commit_exclude_patterns doc "None means inherit the global value" (policy.rs:953-959); SyncPolicy doc "Defaults to empty: this is an opt-in per-repo mechanism" (policy.rs:541-543); classify_dirty_entries doc "effective per-repo (fallback global)" (report.rs:1296-1297) — all describe REPLACE, but the contract since R3-M2 is EXTEND (policy.rs:1100-1116), whose own "All consumers go through this helper now" is falsified by R4-SR-02 (report.rs:3958) and R4-SR-08 (report.rs:8882).
Impact: doc-only; implementers misled.
Fix: update the three docs to EXTEND semantics; soften the helper claim until report sites migrate.

### R4-SYS-07 — LOW — doctor legacy-config hint names the wrong path (merges regression R4-02)
Evidence: check tests ~/.config/dracon (src/doctor.rs:30-32) but the hint says "Move or remove the legacy ~/dracon configuration" (:156-164; regression cites :163). Mismatch present since introduction (commit 06a2021, verified via git show).
Impact: operator told to remove a path that was never checked; cosmetic.
Fix: align the hint with the checked path (or vice versa if ~/dracon was intended).

### R4-SYS-08 — LOW — sentinel comment teaches the opposite of tmp 0-behaviour (doc-only)
Evidence: src/policy.rs:977-978 says tmp_min_age_hours "0 = sweep /tmp regardless of age", but 0 disables cleanup (src/main.rs:5589-5591 early return) and the example agrees ("0 disables tmp hygiene", dracon-system.example.toml:280). Behaviour is safe.
Impact: footgun for the next editor.
Fix: correct the comment to "0 disables tmp hygiene".

### R4-SYS-09 — LOW — relocate crash window between staging rename and symlink
Evidence: src/relocate.rs:354-368 — after rename(source, staging) a crash leaves the source path MISSING with data only in <name>.dracon-relocate-staging, no marker and no documented recovery (the restore path covers symlink-error only).
Impact: confusing post-crash state; data present but path missing.
Fix: document recovery; optionally have setup detect stale staging dirs.

### R4-SYS-10 — LOW — loaded-unit check is an ExecReload-presence-only proxy
Evidence: scripts/check-unit-deployment.sh:146-156 — a copied-but-not-reloaded unit is caught only via missing ExecReload; any other directive drift (e.g. ReadWritePaths, PrivateTmp) with files agreeing passes steps 3-5, and step 6 covers only storage roots and only while running.
Impact: directive drift passes the checker.
Fix: also fail when systemctl --user show -p NeedDaemonReload is yes.

### R4-SYS-11 — LOW — fd-iteration error drops the REST of that process's fd list
Evidence: src/main.rs:5471 — while let Ok(Some(fd_entry)) exits the loop on Err, discarding all remaining fds of that process, broader than the documented "per-process/per-fd read failures stay skips" (:5445-5451), which reads as one fd skipped.
Impact: under-counted open files for that process.
Fix: restructure to continue-on-error per fd entry.

### R4-SYS-12 — LOW — open-file scan misses running executables / mmap'd files
Evidence: collect_open_paths_under_from (src/main.rs:5441-5495) collects fd targets and cwd only; /proc/<pid>/exe and maps are not consulted, so a stale binary executing from /tmp is deleted while running (process survives on the unlinked inode; the path is gone).
Impact: running binary's path unlinked from under it.
Fix: also record the exe link (one readlink per pid, same cost class as cwd).

### R4-SYS-13 — LOW — unreadable tmp root silently skipped, no diagnostic
Evidence: src/main.rs:5614-5617 — read_dir(root) failure is continue with no log line, unlike the loud proc-scan refusal (:5602-5605). Deletion-safe direction, but dry-run silently under-reports.
Impact: silent under-report.
Fix: one eprintln naming the skipped root.

### R4-SYS-14 — LOW — daemon/checker canonicalization parity (symlinked roots)
Evidence: checker matches on the canonical path (check-unit-deployment.sh:242-246, R3-L23) but unit_grants_write compares uncanonicalized paths (src/safety.rs:353-357, roots from uncanonicalized expand_tilde per src/quarantine.rs:110-117), so a symlinked storage root can warn in one and pass in the other.
Impact: checker/daemon verdict split on symlinked roots.
Fix: canonicalize (with literal fallback, as the checker does) in uncovered_storage_roots.

### R4-SYS-15 — LOW — parse_df_details * 1024 can overflow
Evidence: src/main.rs:798-800 — parse::<u64>() * 1024 panics in debug / wraps in release on absurd input, while the file's own convention is saturating_mul (:6704-6707). df output is local, so reachability is low, but a panic kills the daemon pass.
Impact: daemon-pass panic on absurd input.
Fix: saturating_mul(1024).

### R4-SYS-16 — LOW — watchdog .service has no hardening directives
Evidence: dracon-system-guard-watchdog.service:1-15 (no NoNewPrivileges / ProtectSystem / syscall filter) vs the guard unit's full sandbox (dracon-system-guard.service:48-119). It only shells systemctl, but it runs every 2 min as a second privileged-ish entry point.
Impact: wider-than-needed privileges on a periodic entry point.
Fix: add baseline NoNewPrivileges=true, ProtectSystem=strict, ProtectHome=read-only (it needs no writes).

### R4-SYS-17 — LOW — watchdog script-path parity unpinned by any test
Evidence: .service:10 ExecStart %h/.dracon/system-notify/dracon-system-guard-watchdog.sh must equal the install destination (install.sh:527, chmod :528), but no test references system-notify (only doctor timer names), unlike the pinned ExecReload contract (tests/guard_service.rs:220-253). Drift = 203/EXEC every 2 min with the guard un-backstopped.
Impact: silent watchdog death on path drift.
Fix: add a parity test asserting the ExecStart suffix matches the install path.

### R4-SYS-18 — LOW — doctor --json omits the strict verdict
Evidence: JSON mode prints the raw report (src/doctor.rs:220-226) whose schema (src/main.rs:439-465) carries no required/strict outcome, so machine consumers must hardcode the required set from doctor_checks.
Impact: machine consumers cannot read the strict verdict.
Fix: add strict_ok (and/or failed-required labels) to the JSON report.

### R4-W-05 — LOW — bare .dracon/ dir marks a repo warden-MANAGED; sync-only repos blocked as drift
Evidence: src/main.rs:5092 ([ -d "$REPO/.dracon" ] && MANAGED=1), then :5178-5197 demand warden filter config; dracon-sync/src/policy.rs:746 (<repo>/.dracon/dracon-sync.toml is a sync non-warden path; sync also uses .dracon/convos/, .dracon/assets.manifest).
Impact: sync-managed repo outside warden harden coverage has commits blocked until warden is set up — the H-10 false-positive class (comment :5075-5080) reintroduced via a shared dir name.
Fix: key MANAGED on a warden-specific marker (filter keys / attributes) instead of the shared .dracon/ dir.

### R4-W-06 — LOW — merge internal errors share exit 1 with conflicts, with different %A state
Evidence: src/main.rs:2367 (run_merge(...)? Err propagates to process exit 1); :4594-4601 (conflict writes plaintext markers to %A and returns Ok(1); on Err %A is untouched, still current-side ciphertext); :4543-4547 (contract comment covers only clean vs conflict, not internal error).
Impact: decrypt failure / merge-file crash presents as a routine conflict but leaves ciphertext in the worktree.
Fix: document, or use exit 2 for internal errors (git treats any nonzero as conflict, but the operator message can differ).

### R4-W-07 — LOW — merge tempdir holds all three decrypted sides in TMPDIR
Evidence: src/main.rs:4613-4622 (plaintext ancestor/current/other written to tempfile::tempdir(); removed only on clean drop, persists on SIGKILL, no zeroization). 0700 dir already limits exposure.
Impact: decrypted secrets at rest in /tmp on crash.
Fix: best-effort zeroize before drop; note in threat model.

### R4-W-08 — LOW — .plaintext hatch checks are CWD-relative with no containment; helper is pub fail-open
Evidence: dracon-warden/src/security/src/modules/filter.rs:25-31 (is_hatched does Path::new(&format!("{}.plaintext", path)).exists() with no absolute/.. rejection and no repo-root anchoring; any existence returns plaintext passthrough :161-163); src/main.rs:5404,:5450,:5476 (pre-push [ -f "$f.plaintext" ] likewise CWD-relative; hooks assume repo-root cwd).
Impact: filter path is pre-validated on the real clean path (main.rs:3475-3492 rejects absolute/..), so this is an API/hook footgun + TOCTOU (hatch added/removed between check and commit), not a live bypass. Hook cwd assumption (repo root) not verified against git source.
Fix: contain hatch resolution to the repo root and reject absolute/.. inside is_hatched.

### R4-W-09 — LOW — pkt_encode silently truncates oversize payloads
Evidence: src/main.rs:4025-4047 (payloads over PKT_MAX_TOTAL_LEN-4 truncated with only debug_assert!(false)). Unreachable today (all callers chunk at PKT_MAX_PAYLOAD).
Impact: a future caller gets silent stream corruption instead of Err.
Fix: return Result / assert in all builds.

### R4-W-10 — LOW — pre-push scan residuals (documented tradeoffs, still gaps)
Evidence: src/main.rs:5247-5251,:5287 (SECRET_RE covers Tier-1 shapes + assignments only; unquoted secret=/api_key=, AWS secret keys, generic high-entropy Tier-2 shapes push clean when the filter is bypassed); :5224-5226 (newline-in-filename edge explicitly accepted: tr '\0' '\n' + read -r); :5481-5491 (modified-binary check allows any new match string already present in a parent blob).
Impact: defense-in-depth residual; each is a conscious tradeoff in comments.
Fix: none required; track Tier-2 hook coverage as future work.

### R4-W-11 — LOW — hook entry does not check rev-parse --show-toplevel; harden warn-only on global-hook refresh failure
Evidence: src/main.rs:5012,:5314 (REPO=$(git rev-parse --show-toplevel) without || exit 1, unlike GIT_COMMON_DIR at :5020/:5315; failure yields empty REPO → MANAGED=0 → pre-commit exits 0 fail-open); :2092-2096 (global hook refresh failure only eprintln!s, then repos are reported hardened).
Impact: negligible in practice (hooks always run in a repo; refresh is bounded staleness).
Fix: add || exit 1; consider failing harden on refresh error.

### R4-M-10 — LOW — system release hardcodes REMOTE=origin; sync/warden auto-detect
Evidence: dracon-system/scripts/release.sh:88 (REMOTE=origin), help "(default: origin)"; system ships no resolve-github-remote.sh (sync:24, warden:27 have per-repo CANON auto-detect). Repeats the hardcoded-remote failure class sync documents at release.sh:277-280 (v0.113.9/10 push failures). Works today only because this repo happens to name its remote origin.
Impact: release push fails on any non-origin remote naming.
Fix: port resolve-github-remote.sh with the system CANON + auto-detect.

### R4-M-11 — LOW — sync/warden version bump not [package]-scoped
Evidence: dracon-sync/scripts/release.sh:342,:352; dracon-warden/scripts/release.sh:281,:288 (first ^version = in file) vs system :283-293 (crate_manifest_version, [package]-scoped, audit 2026-10-01). Safe today ([package] is line 1 in sync's manifest) but one [workspace.package] block above it silently bumps the wrong line.
Impact: latent wrong-line bump.
Fix: port crate_manifest_version + scoped sed.

### R4-M-12 — LOW — flake crateVersion fallbacks drifted; repin reads local manifest
Evidence: flake.nix:145 ("0.113.88" vs 0.113.93), :155 ("0.112.41" vs 0.112.44), :184 ("0.113.14" vs 0.113.15); scripts/repin-nested-sources.py:144-151 reads ROOT/checkout/Cargo.toml (live worktree) for the fallback, contradicting its own "REMOTE main, never a local worktree" rule (docstring + line 19). Fallback only fires on malformed input, but drift proves the "keep in step" path isn't running; local reads can inject mid-release versions.
Impact: stale fallbacks; mid-release version injection risk.
Fix: read the version from the pinned rev (git show <rev>:Cargo.toml).

### R4-M-13 — LOW — sync Nix build runs no tests; warden skip unexplained
Evidence: flake.nix:150 (doCheck = false for dracon-sync) vs system/warden which run test suites in-Nix; flake.nix:189 skips filter_clean_encrypts_content_with_secret_marker with no rationale comment (system documents every skip, :160-170).
Impact: untested Nix path for sync; unexplained warden skip.
Fix: document why sync can't run any tests in the sandbox (or run a subset); add the one-line reason for the warden skip.

### R4-M-14 — LOW — "monorepo" drift + dead parent resolve helper (docs/hygiene)
Evidence: install.sh:119-128 (calls the tree a monorepo; post-D1 they are nested standalone repos); dracon-warden/scripts/test_release_dry_run.sh:2 ("warden monorepo release preview"); dracon-system/scripts/test_release_pipeline.sh:2 ("monorepo release pipeline"); scripts/resolve-github-remote.sh (zero callers — parent release.sh is now a dispatcher and never invokes it).
Impact: doc drift; dead code.
Fix: reword to "nested standalone repositories"; delete the dead helper or wire a test.

### R4-M-15 — LOW — small shell/script nits (each one line)
Evidence: doctor.sh:121 ([ -f $HOME/... ] unquoted inside eval; breaks on spaced $HOME); rotate script :266,:270 (runs dracon-warden once twice — second run only to capture one echo line); rotate script :320 (git push codeberg main:master hardcodes a branch mapping); scripts/check-flake.sh (asserts watchdog Documentation/After/Type/ExecStart/Timeout/outputs but never Unit Description; descriptions match today but can re-drift invisibly); ci.yml:264 wires verify-spec.sh in one job while :385 says "NOT wired in yet" (stale note or job gap).
Impact: minor robustness/doc gaps.
Fix: quote $HOME; capture once; verify/parametrize the branch mapping; add six Description assertions; reconcile the ci.yml note.

## ROUND3 verification (regression re-check, 2026-10-03)

Method: read-only source inspection (every cited body opened) + live /tmp probes (filter-clean binary) + locked cargo test subsets. Tree clean at parent root. All ROUND2 items VERIFIED, none REOPENED.
- H1 VERIFIED — flake.nix:341-396 (- prefixes, ExecReload, CAP_SYS_NICE, EPERM sandbox); check-flake.sh:86-112 asserts. H2 VERIFIED — flake.nix:291-323 (Restart=always, CPUQuota=100%, MDWE absent + comment :311-316); asserted :116-124.
- M1 VERIFIED — multi_remote.rs:677-679 fail-fast (inspected). M2 VERIFIED — push.rs:52-55 per-forge retention (inspected). M3 VERIFIED — push.rs:238-250 and multi_remote.rs:707-718 SSH-cause chaining (inspected). Tests: push_ 102/102 PASS.
- M4 VERIFIED — staging.rs:124+ staged_blob_sizes_for (ls-files -s + cat-file --batch-check, fail-closed); staging tests 7/7 PASS.
- M5 VERIFIED — main.rs(system):6287-6289 auto_cleanup_apply gate (inspected). M6 VERIFIED — both shipped units contain SystemCallErrorNumber=EPERM (grep 1+1). M7 VERIFIED (documented) — dracon-sync.service:53 + flake.nix:315 comments intact. M8 VERIFIED — install.sh:519-526 ships 6 watchdog units + scripts; flake timers :406-486; check-flake :126-134 asserts.
- M9 VERIFIED — 3b companion loop check-unit-deployment.sh:115-119; CI wiring per ROUND3 (not re-read; advisory — see G2 below).
- M10 VERIFIED — SECRET_RE from hook_token_shapes_ere (warden main.rs:4680-4689). M11 VERIFIED — install.sh:337 cargo build --locked. M12 VERIFIED — report.rs:4487 full hash/msg rows (marker inspected).
- L1 VERIFIED — sync.rs:5075-5086 Err propagates (inspected); bootstrap tests 12/12 PASS incl. test_sync_repo_bootstrap_failure_is_error_not_nothing_to_do. L2 VERIFIED — sync.rs:5108-5114 fail-closed count (inspected). L3 VERIFIED — policy.rs:2214 tripwire + :2403 UNWIRED const (grep). L4 VERIFIED — ops.rs:661 loud unlink + :918-919 test (grep).
- L5 VERIFIED-as-residual — mirror path unchanged; origin budget doc-corrected per R3-L02 (push.rs:257-264 inspected). L6 VERIFIED — main.rs(system):3654 fail-closed scan (grep). L7 VERIFIED — :6717 df-failure pause (grep). L8 VERIFIED — :4310 writes-first truncate (grep).
- L9 VERIFIED — smudge tests 8/8 PASS (code moved filter.rs -> main.rs filter_transform_bytes/smudge; behavior pinned by tests; no body-level diff of the moved smudge — see G3 below).
- L10 VERIFIED — warden main.rs:5419-5452 (-M100%, diff-filter=a, blob-novelty). L11 VERIFIED (documented caveat) — warden main.rs:5547-5551 stale-ref warn. L12 VERIFIED — install.sh:414-425 atomic tmp+rename (+R3-L20 trap/echo fix).
- L13 VERIFIED — report.rs:2985 grapheme truncator (+R3-L33 per-grapheme width :2999-3005); truncate tests 14/14 PASS. L14-L19 VERIFIED — markers at report.rs:3083 (VS16), :6656 (L15 link-out), :2871 (L16 doc), :2927/:5676/:5801 (L17), :6408 (L18 budgets), :6642-6648 (L19 compact count).
- Same-day FIXED markers observed (not re-audited): R3-H1 (flake.nix:490-500, check-flake :141-143 .source asserts); R3-M1 (sync.rs:4782-4819 staged-blob sweep; M2 union helper policy.rs:1100 + 4 worker sites + daemon gates :8244/:8312 + test_auto_commit_excludes_union_global_and_per_repo PASS); R3-M3 (warden main.rs:5089-5091,:5175-5178 comment-aware probes + pre_commit_hook_blocks_when_only_commented_filters_remain PASS); R3-L01 (sync.rs:5687-5694); L02/L03 (push.rs:257-264,:422-429); L04 (multi_remote.rs:681-690); L05 (push.rs:110-116); L06 (push.rs:8-19,:384-392); L20 (install.sh:414-425); L21 (warden:5334); L26 (doctor.rs:14-17,:174-190); L33 (report.rs:2999).
- Lead verdicts: (a) warden absolute-%f refusal CONFIRMED (code + live exit-1 probe); cargo/gix-absolute trigger UNRESOLVED offline → R4-01. (b) Doctor "fails when absent" REFUTED (absent → Ok, doctor.rs:157-161); "fails --strict when present" CONFIRMED BY DESIGN (required:true :162, exit 1 :276-278) but advisory-only (no install/CI/script gate consumes doctor --strict; grep over install.sh, uninstall.sh, .github/, doctor.sh, scripts/ = zero hits); spin-off R4-SYS-07/R4-02. (c) U1-U4 all still carried/open: U1 dispatch cooldown/starvation machinery present (daemon.rs:100-317) but unaudited beyond apply_outcome; U2 bundle/lease path present (staging.rs:467-555) unaudited; U3 fleet state single-host only; U4 no fault-injection runs.

## Unresolved / omitted scope (12)

1. Whether an ambient git credential helper exists that would rescue the github HTTPS leg (no repo evidence found). Affects R4-SC-06 impact wording.
2. R4-SR-03 user-visible magnitude depends on daemon dispatch damping (fingerprints/quiet windows, out of scope): mechanism verified in exclude.rs + gate call sites, blast radius approximate.
3. Whether ops wants bare-~ and single-quote policy styles supported or rejected daemon-side (checker fix direction depends on it). Affects R4-SYS-02.
4. Confirm non-NixOS is a supported doctor target vs NixOS-only (decides R4-SYS-03 priority).
5. Whether git aborts vs falls back to text merge when merge.dracon.driver is undefined (man page inconclusive; affects R4-W-03 impact wording).
6. Hook cwd assumption (repo root) for CWD-relative .plaintext checks not verified against git source; treated as LOW. Affects R4-W-08.
7. Could not run shellcheck (binary absent) — CI gate at ci.yml:209 presumably covers it.
8. Could not run nix eval to confirm R4-M-04 gate-red vs unexercised; static mismatch is conclusive.
9. scripts/sync_convergence.py (77KB) + verify-ownership-mirrors.py not line-audited (budget); no HIGH indicators in scope scan.
10. G1: cargo/gix-absolute-path trigger for R4-01 unverifiable offline (no gix sources/network).
11. G2: M9 CI wiring not re-read (advisory; ROUND3 evidence stands).
12. G3: L9 via 8/8 smudge tests only (impl moved since ROUND3 refs).

## Explicit non-findings / verified healthy (condensed)

- Sync-report: visibility cache freshness boundaries match (visibility.rs:121-126 vs :201-203); GitLab/Codeberg tokens via curl stdin -H @-, never argv (:367-397,:532-647), errors carry no token material; .env control-char refusal blocks header injection (secrets.rs:94-113 + tests); Owned-verdict revalidation exists (R3-L14 two-strike, daemon.rs:1401-1416,:7271-7274), negatives redetect on TTL (:1384-1399); redact_url_credentials has no char-boundary panic (ownership.rs:515-546); emit_repo_failure routes failures to stderr in JSON mode (report.rs:3336-3343); no global exclude_remotes anywhere (report :804-828; daemon :1045; sync.rs:2375).
- System: tmp symlink handling (main.rs:5628-5632 + check_safe_to_delete_tmp_entry), fail-closed tree_has_fresh_content (+ tests.rs:2511), blind-proc refusal (+ tests.rs:2181), containment test (:2558) sound; relocate verify-on-plan-counts fails closed on source change; strict walk/copy (+ tests) sound; fits_in_avail(None)=false pinned; checker 3b companions, discovery, dangling-symlink, mount-shadow, root-mount cases have regression tests (cases 1-28); install/uninstall watchdog unit+script coverage present and symmetric.
- Warden: clean encrypt failure fails closed (lib.rs:1060-1064); inline smudge decrypt/base64/UTF-8 failures preserve the tag verbatim (filter.rs:446-464); whole-file smudge unlock failure warns + passes ciphertext through (lib.rs:1424-1427), stable with the double-encrypt guard (b64+age-magic, lib.rs:395-418); merge re-encrypt path-independent via ancestor format (main.rs:4548-4563, lib.rs:1086-1097); merge-file exit >1 is Err not conflict (:4631-4639, tests.rs:4774); clean path validation rejects absolute/.. fail-closed, smudge passes through safe (:3419-3494,:3896-3910); oversize clean refuses except exempt binaries (:3461-3474); status=error per-file fail-closed (:4519-4526); unsupported commands drain-then-error (:4447-4452); handshake violations abort (:4146+); no secret bytes in errors (unlock reports only age-magic+len, lib.rs:1361-1365; SecretScanner::scan snippet has no production callers); .gitattributes ordering (catch-all → -filter carve-outs → protected filter+diff+merge → plaintext -filter, :992-1045) matches last-match-wins intent.
- Meta: parent dispatcher scripts/release.sh:8,:38-43 (exec-only, no coordinated release) + scripts/test_release.sh covers help/dispatch/exit-2 sound; sync release tag-after-publish, idempotent re-runs, deny-fallback guard (:324-333), fixture-on-packaged-artifact (:443-461) sound; close-changelog.py triplicated but behavior-identical; install.sh service gating (:45-53,:615-657), atomic binary swap (:414-430), shadowing guards (:254-308), copy-failure tracking (:508-534) sound; uninstall.sh removes M8 watchdog units+scripts (:67-72,:118-131) and reverts global hooksPath (:133-158) sound; check-unit-deployment.sh sync(332)/system(318) share discovery/exit contract, step-4 sentinels correctly differ, both have regression suites; watchdog timers/services match flake values except R4-M-04's Documentation gap; python3 -m unittest scripts.tests.* works via namespace packages (import probe OK); CI shellchecks all scripts (ci.yml:201-209); hermetic release builds use --locked throughout (install.sh:337-341).




