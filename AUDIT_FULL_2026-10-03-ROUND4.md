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


