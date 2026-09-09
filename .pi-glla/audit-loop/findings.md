# Audit-loop findings — README pass (2026-09-08)

Source: 4 parallel scouts (root/meta, dracon-sync, dracon-system, dracon-warden),
each claim re-verified against source/CLI before listing. No fabricated findings.

## New findings (this pass)

- [x] FIX: HIGH [F1]: Root README Releases section stale — stamps v0.113.53 (2026-08-22), sync 0.113.53 / system 0.112.38 / warden 0.113.5(RC); tags of 2026-09-01 are sync 0.113.55 / system 0.112.40 / warden 0.113.6 (README.md:125-135) — fixed in 722a51847
- [x] FIX: LOW [F2]: Root README claims ~1000 tests; CHANGELOG records 1419 passed (README.md:16-17) — fixed in 722a51847
- [x] FIX: LOW [F3]: Root README Install has no uninstall pointer though uninstall.sh ships at root (README.md:29-40) — fixed in 722a51847
- [x] FIX: LOW [F4]: Root README Install omits install.sh side effects — stops daemons, overwrites user service units, installs global git hooks, restarts services unless --no-restart (README.md:29-40) — fixed in 722a51847
- [x] FIX: HIGH [F5]: CONTRIBUTING "What Belongs Here" describes three standalone CLI repos with implementation in nested standalone git repos — false since 2026-08-22 subtree monorepo (CONTRIBUTING.md:7-16) — fixed in 722a51847
- [x] FIX: HIGH [F6]: CONTRIBUTING Setup tells contributors to git-clone standalone repos into dracon-sync/system/warden — clobbers tracked monorepo trees (CONTRIBUTING.md:28-36) — fixed in 722a51847
- [x] FIX: MEDIUM [F7]: CONTRIBUTING Validation drops --locked, drops clippy -D warnings, uses --test-threads=1 instead of AGENTS.md locked discipline (CONTRIBUTING.md:45-58) — fixed in 722a51847
- [x] FIX: LOW [F8]: CONTRIBUTING exports DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git unconditionally; path exists only on NixOS (CONTRIBUTING.md:49) — fixed in 722a51847
- [x] FIX: MEDIUM [F9]: install.sh comment + error text say utility source lives in nested standalone repos and to clone them — stale since monorepo conversion (install.sh:82-90) — fixed in 722a51847
- [x] FIX: MEDIUM [F10]: doctor.sh checks [ -d $utility/.git ] per utility — no nested .git exists in monorepo, always WARNs on healthy checkout (doctor.sh:30-35) — fixed in 722a51847
- [x] FIX: MEDIUM [F11]: docs/README says index saves reading 116 files — repo holds 264 files, 153 in docs/design/ alone (docs/README.md:5) — fixed in 722a51847
- [x] FIX: MEDIUM [F12]: docs/README Canonical-audits table + Archive bullet point at root files moved to docs/archive/audits-2026-07/ — every link 404s (docs/README.md:30-50) — fixed in 722a51847
- [x] FIX: MEDIUM [F13]: UTILITY_BOUNDARIES cites dracon-libs/docs/capability-boundaries.md and invokes dracon-security/persistence/code/ai/libs with no "not in this repo" scope note — newcomer hunts phantom crates (UTILITY_BOUNDARIES.md:5) — fixed in 722a51847
- [x] FIX: HIGH [F14]: dracon-sync README installs via sudo cp to /usr/local/bin while shipped service runs %h/.local/bin binary; build omits --locked (dracon-sync/README.md:14-22) — fixed in 722a51847
- [x] FIX: MEDIUM [F15]: dracon-sync README example-policy path dracon-sync/dracon-sync.example.toml is monorepo-relative; standalone/crates.io reader (example TOML excluded from package) needs ./ path + note (dracon-sync/README.md Runtime) — fixed in 722a51847
- [x] FIX: MEDIUM [F16]: dracon-sync README lists 4 of 19 subcommands, no --help pointer, omits banned-stop quiesce path (maintenance), policy path, service enablement (dracon-sync/README.md Runtime) — fixed in 722a51847
- [x] FIX: LOW [F17]: dracon-sync README "What is in this repo" says tests/ "(if present)" (present) and omits shipped dracon-sync.service, ai.example.toml, scripts/ (dracon-sync/README.md:24-33) — fixed in 722a51847
- [x] FIX: HIGH [F18]: All three utility READMEs claim "Source of truth: this standalone repository" + "Changes are made in this standalone repository" — standalone repos are frozen mirrors since 2026-08-22, development is in-tree (dracon-sync/README.md, dracon-system/README.md, dracon-warden/README.md Relationship/Maintenance) — fixed in 722a51847
- [x] FIX: MEDIUM [F19]: dracon-system README installs via sudo cp to /usr/local/bin while shipped guard unit runs %h/.local/bin binary (dracon-system/README.md:23) — fixed in 722a51847
- [x] FIX: MEDIUM [F20]: dracon-system README lists 4 commands of many (events/link/symlinks/zram/guard once-prune-clean missing), no live config path, no service enablement, no destructive-flag safety note (dracon-system/README.md:64-65) — fixed in 722a51847
- [x] FIX: MEDIUM [F21]: CODE BUG (help text): storage --min_size_mb help says [default: 50]; unset falls back to policy storage.min_size_mb whose built-in default is 512 (dracon-system/src/main.rs:191 vs policy.rs:328) — fixed in 581669394
- [x] FIX: MEDIUM [F22]: dracon-system example.toml is intentionally stricter than compiled defaults (thresholds 65/75/85/92+unfreeze 70 vs code 70/80/90/95+88; cleanup kinds false vs default_true) with no header note — no-config runs get aggressive defaults (dracon-system/dracon-system.example.toml:1-10) — fixed in 581669394
- [x] FIX: MEDIUM [F23]: dracon-warden README example-policy path dracon-warden/dracon-warden.example.toml is monorepo-relative; standalone clone needs ./ path (dracon-warden/README.md:59) — fixed in 722a51847
- [x] FIX: MEDIUM [F24]: dracon-warden example quick-start step 1 says ./install.sh — no install.sh ships in the utility dir (root only) (dracon-warden/dracon-warden.example.toml:7) — fixed in 581669394
- [x] FIX: MEDIUM [F25]: dracon-warden example plaintext_patterns comment suggests ["*.md", "*.txt", "LICENSE*"] — none are in is_allowed_plaintext_pattern allowlist, copy-paste fails validation (dracon-warden/dracon-warden.example.toml:38) — fixed in 581669394
- [x] FIX: MEDIUM [F26]: dracon-warden README omits once (the hardening command its own example tells users to run), keygen no-overwrite/backup essentials, protected_patterns posture, and pre-push hook + merge driver coverage (dracon-warden/README.md Purpose/Runtime) — fixed in 722a51847
- [x] FIX: LOW [F27]: dracon-warden SOURCE_OF_TRUTH invariant 4 says narrow defaults are "documented in the example policy" — example actually extends them with product-specific paths (dracon-warden/docs/SOURCE_OF_TRUTH.md:24-25) — fixed in 581669394
- [x] DECIDED: merge each utility README + monorepo-README into one guide (2026-09-09) — was: DECIDE [D1]: Each utility dir ships two guides — thin README.md (standalone-framed, goes to crates.io) vs rich monorepo-README.md (linked from nowhere). Choice: declare one canonical and cross-link (cost: one rewrite + mirror check) vs keep both with a cross-link header each (cost: permanent dual-maintenance drift risk).
- [x] DECIDED: remove flake.nix warden.enable module + doctor.sh warden.service check (2026-09-09) — was: DECIDE [D2]: flake.nix warden.enable module ships ExecStart=dracon-warden daemon but warden has no daemon subcommand (verified --help) — enabling crash-loops; doctor.sh enshrines dracon-warden.service. Choice: remove the module+check (cost: breaks NixOS configs that set warden.enable) vs rework module to hook-based activation (cost: new Nix design work) vs implement a daemon subcommand (cost: contradicts warden-has-no-daemon boundary).
- [x] DECIDED: keep install.sh PATH-shadow auto-remove (2026-09-09) — was: DECIDE [D3]: install.sh auto-deletes same-named dracon-sync/system/warden binaries elsewhere on PATH (e.g. /usr/local/bin) with bare rm -f, no confirm; permission-denied aborts under set -e. Choice: keep auto-remove (cost: surprises sysadmin installs) vs warn-and-skip (cost: users keep shadowing binaries and report stale-version confusion).
- [x] DECIDED: verify private vulnerability reporting is on and assert it in SECURITY.md (2026-09-09) — was: DECIDE [D4]: SECURITY.md reporting relies solely on GitHub private vulnerability reporting with no contact fallback or SLA. Choice: add maintainer mailbox (cost: spam surface, must be monitored) vs assert private reporting is enabled (cost: reports go nowhere if it is ever off).
- [x] DECIDED: ROADMAP is the map; shrink docs/README to design-index (2026-09-09) — was: DECIDE [D5]: docs/README.md and docs/ROADMAP.md overlap as docs maps. Choice: shrink docs/README to design-index only, ROADMAP as the map (cost: one rewrite, fix cross-links) vs keep both (cost: two maps drift).
- [x] DECIDED: per-utility CHANGELOGs canonical; note it in root CHANGELOG (2026-09-09) — was: DECIDE [D6]: Root CHANGELOG.md no longer tracks per-component releases (sync 0.113.55 / system 0.112.40 / warden 0.113.6 tagged 2026-09-01 updated only per-utility CHANGELOGs). Choice: resume root entries per release (cost: dual-changelog upkeep) vs declare per-utility CHANGELOGs canonical and note it in root (cost: root history loses the headline trail).

## Code pass (2026-09-09)

Source: 3 parallel scouts (dracon-sync, dracon-system, dracon-warden+security),
each claim re-verified against source before listing. Dropped after verification
(documented-deliberate or no real failure): sync dead settling/dirty fields (intentional
future-policy, CONCERN #6); warden keygen 12-char window (~40-bit entropy, needs ~1M
machines to collide); warden pre-push modified-blob + SECRET_RE shape gaps (both
documented trade-offs with rationale in hook comments); warden .plaintext CWD-relative
(no demonstrated failure; merge half subsumed by F49); warden backfill contains/CRLF
gates (safe-direction imprecision, no demonstrated failure); warden text_merge perms
(tempdir is 0700 — files not world-visible); warden ~/.arcane/keys fallback (decrypt-only
fallback under own $HOME, no exfil path); warden hygiene defaults narrower than example
(security crate is generic/published — must not hardcode product game paths); warden
xargs -r/mktemp (project is GNU/Linux-only per READMEs); warden revoke contains-match
(full-key substring; deleting proofs signed by a revoked key is correct); warden binary
inline-tag smudge (clean never creates inline tags in binaries — whole-file-or-passthrough);
warden EnvironmentManager (pub API of published dracon-security crate — removal breaks
semver); system resolve_bin hardcoded dracon path (harmless non-matching fallback entry);
system CleanTargets tmp absence (CLI surface scope, not a bug).

- [x] FIX: HIGH [F28]: standard_files ~/ escape still open — is_safe allows ~/..., expand_tilde resolves under $HOME, so source="~/.ssh/id_rsa" copies a HOME key into every watched repo for auto-commit+push; SYNC-H5 comment names this exact attack yet the rule blesses ~ (dracon-sync/src/policy.rs:102, dracon-sync/src/standard_files.rs:37) — fixed in 88d78649b
- [x] FIX: HIGH [F29]: documented per-repo auto_repair_concerns=false silently does nothing — knob is GLOBAL_ONLY, RepoPolicyOverride lacks the field, no deny_unknown_fields, daemon gates globally; AGENTS.md promises it protects sacred-history repos (dracon-sync/src/policy.rs:2071, dracon-sync/src/daemon.rs:3367) — fixed in b091ead8f
- [x] DECIDED [D7] (2026-09-09 — operator approved "redact all userinfo"): redact_origin_credentials now strips the ENTIRE userinfo (https://host/...) — choice made: redact ALL userinfo incl. bare usernames (cost: loses username context in logs) over keep verbatim (cost: token-as-username PATs leak into terminal/JSON reports) (dracon-sync/src/ownership.rs:462) — fixed in 97301d26f/996b1895c
- [x] FIX: MED [F30]: pause/resume touch only marker 1 — frozen via marker 2 (~/.dracon/freeze/dracon-sync) or env reports "not paused"/false "resumed" while still frozen (dracon-sync/src/main.rs:905, dracon-sync/src/policy.rs:1841) — fixed in b091ead8f
- [x] FIX: LOW [F31]: run_maintenance comment claims >24h freeze TTL, code is 1h since 2026-08-24 (dracon-sync/src/main.rs:46 vs dracon-sync/src/policy.rs:1864) — fixed in b091ead8f
- [x] FIX: LOW [F32]: scale_push_timeout cap is identity (cap>=value always) + base*6 can overflow on absurd configs (dracon-sync/src/sync.rs:224) — fixed in 20c0db619
- [x] FIX: LOW [F33]: daemon call-site comment describes removed standalone materialization (function is remote-config-only since 730eaf2a) (dracon-sync/src/daemon.rs:3733 vs dracon-sync/src/daemon.rs:3160) — fixed in 20c0db619
- [x] FIX: LOW [F34]: kill_process_group returns early when SIGTERM fails so SIGKILL is never attempted, contradicting its own TERM-wait-KILL doc (dracon-sync/src/git/ops.rs:42) — fixed in 20c0db619
- [x] FIX: LOW [F35]: integration tests default to NixOS-only /run/current-system/sw/bin/git with unwrap — non-NixOS cargo test panics (dracon-sync/tests/integration_test.rs:10) — fixed in 20c0db619
- [x] DECIDED [D8] (2026-09-09 — operator chose KEEP warn-and-load): world-readable secret files warn-and-load. Rationale: fail-closed stoppage contradicts the fleet's liveness-first design (same philosophy as eager-encrypt over getting stuck — a halted daemon that syncs nothing is itself an error state). Refusing to load would trade a theoretical confidentiality risk for a certain availability loss across the whole fleet. Live secrets are 600 anyway. (dracon-sync/src/secrets.rs:180)
- [x] FIX: MED [F36]: storage --kinds help lists (targets, trash, nix, caches, node_modules, docker) — zero overlap with real default kinds rust-build,node-deps,build-output,cache (dracon-system/src/main.rs:195 vs dracon-system/src/policy.rs:333) — fixed in 97d202568
- [x] FIX: MED [F37]: storage --json returns before the cleanup block — --cleanup/--apply silently ignored with exit 0 (dracon-system/src/main.rs:5353) — fixed in 97d202568
- [x] FIX: MED [F38]: .git backstop matches only file_name==".git" — /repo/.git/objects passes despite the defense-in-depth comment (dracon-system/src/main.rs:5623) — fixed in 97d202568
- [x] FIX: MED [F39]: log truncation uses strict check_safe_to_delete which rejects everything under /home and /var — the example's own log_dirs can never be truncated (dracon-system/src/main.rs:4859 vs dracon-system/src/safety.rs:24) — fixed in 97d202568
- [x] FIX: LOW [F40]: clean_tmp_paths symlink pre-filter uses entry.metadata() which follows symlinks, so is_symlink() is never true and the "Never follow" comment is false (dracon-system/src/main.rs:4131) — fixed in 97d202568
- [x] FIX: LOW [F41]: trash/tmp cutoff math (now - age) underflows and panics in debug on absurd min_age configs; normalize clamps neither knob (dracon-system/src/main.rs:2928, dracon-system/src/main.rs:4120) — fixed in b494ffe22
- [x] FIX: LOW [F42]: SIGHUP corrupt-policy path logs/emits "using defaults" but keeps the old policy (dracon-system/src/main.rs:5975) — fixed in b494ffe22
- [x] FIX: LOW [F43]: normalize_guard_policy never clamps disk_early_warn_percent — early>warn makes the early band permanently empty (dracon-system/src/main.rs:5110 vs dracon-system/src/main.rs:3401) — fixed in b494ffe22
- [x] FIX: LOW [F44]: cmd_guard_clean calls docker_prune(apply, apply, ...) — --apply forces docker --all (deletes ALL unused images) with no opt-out, while the prune path ties --all to its own flag (dracon-system/src/main.rs:6258 vs dracon-system/src/main.rs:2617) — fixed in b494ffe22
- [x] FIX: LOW [F45]: cmd_guard_prune no-flag path prints human-readable disk text even with --json, then also prints the JSON report — mixed output (dracon-system/src/main.rs:6052) — fixed in b494ffe22
- [x] FIX: LOW [F46]: zram --algorithm help lists 4 algos, code accepts 7 incl. lzo-rle/deflate/842 (dracon-system/src/main.rs:226 vs dracon-system/src/zram.rs:17) — fixed in b494ffe22
- [x] FIX: LOW [F47]: events --severity help advertises critical, EventSeverity has only Info/Warn/Error — nothing emitted can match it (dracon-system/src/main.rs:165 vs dracon-system/src/events.rs:16) — fixed in b494ffe22
- [x] FIX: LOW [F48]: example.toml documents none of clean_tmp/tmp_search_paths/tmp_min_age_hours/trash_min_age_days/trash_credential_guard/process_stuck_after_secs/disk_rapid_fill_gbph/rust_target_max_age_days/clean_node_modules (dracon-system/dracon-system.example.toml vs dracon-system/src/policy.rs:175) — fixed in cfa12835b
- [x] FIX: HIGH [F49]: merge driver re-encrypts via git's %A TEMP path so the protected-patterns gate misses and clean writes PLAINTEXT into %A — git commits it (decrypt ignores path, encrypt is path-gated: asymmetric); %A/%B are temp files, not worktree files as the comment claims (dracon-warden/src/main.rs:2428, dracon-warden/src/security/src/lib.rs:1400) — fixed in 0df86fcdb
- [x] FIX: MED [F50]: RepoKey holds 32-byte AES-GCM keys with no Zeroize/ZeroizeOnDrop while TeamKey has it — key material lingers after drop (dracon-warden/src/security/src/modules/keys.rs:12 vs :42) — fixed in e8f8b5bfa
- [x] FIX: LOW [F51]: resmudge silently continues past files over STREAM_IO_MAX_BYTES with no warning — large ciphertext files stay unrestored indefinitely (dracon-warden/src/main.rs:2077) — fixed in e8f8b5bfa

## Code pass (2026-09-09, collect-only)

Source: 3 parallel read-only scouts (dracon-sync, dracon-system,
dracon-warden+security), followed by source verification. Existing F1–F51 and
D1–D8 were excluded. One scout claim (unused `anyhow::Result` import in
`dracon-system/src/doctor.rs`) was dropped: the import is used by
`cmd_doctor`'s `Result<()>` return type. No code was changed in this pass.

### dracon-sync

- [ ] FIX: MEDIUM [F52]: startup cleanup checks only `repo/.git/index.lock`, so it misses stale locks in linked worktrees and nested submodules whose `.git` is a pointer file; later `IndexLock::acquire` sees the real resolved lock and skips the checkout indefinitely (dracon-sync/src/daemon.rs:3118)
- [ ] FIX: HIGH [F53]: startup lock cleanup treats any `fuser` spawn/permission/error as “not in use” and removes the lock; an unavailable or failing `fuser` can therefore delete an active Git index lock and allow concurrent index writes (dracon-sync/src/daemon.rs:3125)
- [ ] FIX: MEDIUM [F54]: `ever_pushed` reads refs below the checkout’s literal `.git`, so linked worktrees/submodules with remote refs in the common gitdir appear never-pushed and can pass the 900-second gone guard into unwanted mirror creation (dracon-sync/src/report.rs:6689)
- [x] FIX: HIGH [F55]: `standard_files[].target = "."` passes the lexical safety check; with overwrite enabled, `ensure_standard_files` removes the repository directory recursively before the copy fails, deleting the checkout and `.git` (dracon-sync/src/policy.rs:110, dracon-sync/src/standard_files.rs:77) — fixed in 93d429ba9
- [x] FIX: MEDIUM [F56]: standard-file target checks are lexical only; a tracked symlink directory such as `.github -> /tmp/out` lets daemon/CLI scaffold writes (and overwrite deletes) resolve outside the repository (dracon-sync/src/standard_files.rs:39, dracon-sync/src/main.rs:1877) — fixed in e4a4f339e
- [x] FIX: LOW [F57]: publish-upstream setup reports success after `git config` exits nonzero because it checks process spawn rather than `ExitStatus::success`; unwritable/read-only gitdirs remain unconfigured and are retried misleadingly (dracon-sync/src/daemon.rs:421) — fixed in 62de22618

### dracon-system

- [x] FIX: HIGH [F58]: `nix_keep_generations` passes `5` to `nix-env --delete-generations`, which deletes generation 5 rather than keeping the last five; apply then runs `nix-collect-garbage -d`, which deletes all old profile generations despite the keep setting (dracon-system/src/main.rs:3035) — fixed in 5a490f46b
- [ ] FIX: HIGH [F59]: the shipped guard service has `PrivateTmp=true`, so its default `/tmp` cleanup sees only the service-private namespace and cannot reclaim stale host `/tmp` entries that filled the monitored root filesystem (dracon-system/dracon-system-guard.service:33)
- [ ] FIX: MEDIUM [F60]: bare `dracon-system guard clean` is documented as reclaiming/previewing all cleanup targets but `resolve_clean_targets` returns no targets unless `--all` or an individual flag is supplied; it exits successfully after doing nothing (dracon-system/src/main.rs:6193)
- [ ] FIX: MEDIUM [F61]: `Restart=always` with only exit statuses 2 and 78 prevented means a valid `enabled=false` policy exits 0 and a malformed policy exits 1, causing the shipped guard service to restart every 10 seconds instead of remaining disabled or surfacing a stable error (dracon-system/dracon-system-guard.service:13)
- [ ] FIX: MEDIUM [F62]: auto-renice computes an absolute target from policy tiers without taking the current process nice value into account; a process already at nice 10 can be reset to target nice 5, raising its priority contrary to the “lower priority” contract (dracon-system/src/main.rs:1398)
- [ ] FIX: MEDIUM [F63]: active-build protection covers Rust target cleanup but package-cache cleanup has no activity check and can recursively remove cargo/npm/pip/go caches during an active build when apply is enabled (dracon-system/src/main.rs:2317, dracon-system/src/main.rs:2694)
- [ ] FIX: MEDIUM [F64]: `/tmp` open-path protection scans `/proc/*/fd` but not `/proc/<pid>/cwd`; a process chdir’d into an old top-level tmp directory with no open fd can have that directory recursively removed (dracon-system/src/main.rs:4079)
- [ ] FIX: MEDIUM [F65]: configurable `tmp_search_paths` accepts arbitrary roots, and guard deletion rejects only exact system roots; `tmp_search_paths="~"` can therefore recursively remove sufficiently old top-level home entries under apply (dracon-system/src/main.rs:4256, dracon-system/src/safety.rs:80)
- [ ] FIX: MEDIUM [F66]: the documented `guard_log_file = "~/.local/state/dracon/..."` is passed directly to `PathBuf` in logging and rotation without tilde expansion, so telemetry writes to a literal relative `~` path or fails under the service (dracon-system/src/main.rs:1235, dracon-system/dracon-system.example.toml:64)
- [ ] FIX: LOW [F67]: `status` honors `DRACON_SYSTEM_POLICY` when loading policy but always reports the canonical policy path/existence, so an active valid override is displayed as missing or misidentified (dracon-system/src/main.rs:5096, dracon-system/src/main.rs:5145)
- [ ] FIX: LOW [F68]: `events --json` prints `(no matching events)` when the filtered result is empty, violating the advertised JSONL output and breaking consumers that parse every line as JSON (dracon-system/src/events.rs:153)
- [ ] FIX: LOW [F69]: persistent events append to `~/.dracon/events.jsonl` without size rotation, while the guard emits errors on repeated cycles and `events` reads the entire file before tailing; a persistent failure can grow the disk-protection daemon’s own log without bound (dracon-system/src/events.rs:60)

### dracon-warden + security

- [ ] FIX: HIGH [F70]: shipped `repo_roots`/`discover_roots` values such as `"~/.dracon"` and `"~/Dev"` are passed to `PathBuf` without tilde expansion, silently filtering out the intended roots and making once/repair/resmudge operate on zero repositories (dracon-warden/src/main.rs:467)
- [ ] FIX: MEDIUM [F71]: discovery is documented as recursive but `discover_git_repos` reads only immediate children of each root, omitting nested repositories such as nested game/submodule checkouts from hardening and repair (dracon-warden/src/main.rs:137, dracon-warden/dracon-warden.example.toml:16)
- [ ] FIX: HIGH [F72]: protected patterns such as the shipped `secrets/*` and `.ssh/*` generate Git filter attributes but `path_is_protected` does not implement single-star path globs, so clean passes matching secret files through plaintext (dracon-warden/src/main.rs:719, dracon-warden/src/security/src/modules/filter.rs:57)
- [ ] FIX: HIGH [F73]: hardening reads tracked `.gitignore`/`.gitattributes` symlinks with `fs::read_to_string`; a checkout-controlled link to a local secret can be read and preserved into the generated repository file before auto-commit/push (dracon-warden/src/main.rs:1241)
- [ ] FIX: MEDIUM [F74]: repair’s Git-index-driven resmudge and env-header backfill loops read/write tracked paths without rejecting symlinks, so a committed link can decrypt or rewrite an external file during `repair --apply` (dracon-warden/src/main.rs:2064, dracon-warden/src/main.rs:2190)
- [ ] FIX: MEDIUM [F75]: owner public-key publication follows a repository-controlled target symlink on `fs::read`/`fs::write`, allowing hardening to overwrite an external writable file with the local public key (dracon-warden/src/main.rs:963)
- [ ] FIX: MEDIUM [F76]: local hook setup and generated foreign-hook chaining use `repo/.git/hooks` directly; linked worktrees and nested submodules expose `.git` as a pointer file, so `setup-hooks --local` fails and existing local hooks are skipped (dracon-warden/src/main.rs:2746, dracon-warden/src/main.rs:2773)
