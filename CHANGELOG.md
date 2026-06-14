# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Bounded parallel sync**: `dracon-sync` daemon now dispatches
  `sync_repo` calls in parallel, bounded by the new
  `sem_max_concurrent_sync` policy field (default 4). Previously,
  the main loop was serial: a 60s `push_op_timeout_secs` on one
  slow repo (e.g. kiki-sassy github divergence, one-mil-girls
  gitlab) blocked all other repos from being committed and pushed.
  With 17 watched repos, a fresh dirty state on multiple repos
  now clears in ~35s instead of 10+ min. Live evidence captured
  in `docs/design/dirty-files-investigation.md`. The apply phase
  intentionally simplifies the deeply-nested stuck-ahead/behind
  /mirror notification logic; that gets restored in a follow-up.
  Set `sem_max_concurrent_sync = 1` to restore the original
  serial behavior. The apply phase is also bounded by a
  per-cycle deadline (`pulse_interval_secs * 2`, min 2s) so a
  slow push on one repo cannot block the next cycle indefinitely;
  unfinished tasks remain in the in-flight queue and are drained
  in subsequent cycles.

- **Canonical git identity section in `dracon-sync.example.toml`**: a
  new SECTION 12 documents the canonical `DraconDev <dracsharp@gmail.com>`
  profile, the global/per-repo resolution order, the search command
  to detect drift, and a note that the daemon does not rewrite
  identity at runtime. Operators should keep their profile consistent
  across `~/.gitconfig` and per-repo `.git/config` files.

- **All operator-owned warden pub keys are now tracked**: the
  previous goal tracked only `owner_nixos.pub`. This goal
  audits `~/.dracon/data/keys/`, force-tracks every operator-owned
  `*.pub` (`master.pub`, `micro2_git_key.pub`, `micro2_libs_key.pub`,
  `owner_age15xjl.pub`, `owner_age1f7y5.pub`, `owner_nixos.pub`),
  and pushes to all three public remotes. Private keys (`*.age`,
  `id_age`, `*.key`) remain blocklisted by `.gitignore`. The
  tracking rationale and recovery procedure are documented in
  `docs/design/owner-nixos-pub-tracking.md`.

- **`dracon-ai-lib` profile fix**: the per-repo
  `~/.git/config` had `user.name = Dracon` and
  `user.email = dracon@void`, which was a config drift from the
  canonical `DraconDev <dracsharp@gmail.com>`. Now matches the
  global gitconfig. Future commits from this repo will use the
  canonical profile.

- **`owner_nixos.pub` is now tracked in `dracon-utilities`**: the warden
  public key at `.dracon/data/keys/owner_nixos.pub` is committed and
  pushed to all three public remotes. Operators can recover the
  encryption key from git history if the local `.dracon/data/keys/`
  is ever lost. The `!.dracon/data/keys/*.pub` allowlist in the
  warden-managed `.gitignore` correctly force-tracks pub keys while
  keeping the private key (`*.key`, `id_age`) out of tracking.

- **`auto_stage_untracked` policy field**: a new `auto_stage_untracked`
  boolean (default `true`) and `untracked_exclude_patterns` list
  (default safe patterns for notes, scratch, audit, screenshots, etc.)
  in `dracon-sync.toml`. Together they make the daemon auto-stage
  newly-created untracked working files on the next sync cycle while
  keeping user notes, scratch research, and audit evidence
  permanently untracked. Set `auto_stage_untracked = false` to opt
  out completely. See `docs/design/dirty-files-investigation.md`
  for the per-file classification and the known cases where the
  dirty state persists longer than `inactivity_push_delay_secs`.

- **`dracon-sync repos` `STATE` column**: The `repos` table now includes a
  derived "STATE" column that combines last-commit time, last-push time,
  dirty state, ahead/behind, and push status into a small fixed
  vocabulary the user can scan at a glance. The vocabulary covers
  `working`, `committing`, `pushing`, `synced`, `stalled`, `dirty`,
  `untracked-only`, `intentional`, `failed`, `idle`, `cold`, and
  `healthy`. The `stalled` label specifically surfaces the
  "we changed files but then stopped" case the user asked about:
  dirty tracked/staged work older than `committing_commit_minutes`.
  Recent dirty work is labelled `dirty` so normal sync can pick it up
  after the configured settling delay without a red stalled alarm;
  `sync-now --warns` forces the same dirty-only triage immediately.
- **`dracon-sync repos` shows daemon activity**: added a new `DAEMON`
  column to the live `repos` table that shows the daemon's most recent
  recorded action per repo (e.g. `32s ago sync_commit ok`,
  `5m ago sync_triage ok`, `none`). This is sourced from the incident
  ledger and is wired into `sync_repo` so every auto-commit is recorded.
  The `last_when` / `last_push` columns reset to the moment of the
  daemon's own commit, so the `DAEMON` column closes the gap between
  "is the user still editing" and "is the daemon actively syncing".
  Thresholds
  (`active_commit_minutes`, `committing_commit_minutes`,
  `cold_commit_minutes`) live in the global policy with optional
  per-repo overrides in `RepoPolicyOverride`. The `--json` output
  includes the new `state_cause` and `state_cause_label` fields on
  every row. Documented in `docs/design/repos-state-cause.md`.

### Fixed
- **`dracon-sync` `STATE` semantics**: Recent dirty tracked/staged work
  now classifies as `dirty`, not `stalled`, so normal sync or
  `repair warns --apply` can pick it up without a red alarm. A repo only
  becomes `stalled` when tracked/staged work has sat longer than
  `committing_commit_minutes` without push progress. The `working` label
  means "the daemon is currently working through this repo" (clean, in
  sync, commit and push both within `active_commit_minutes`), not
  "the user is still editing right now". The `synced` label is the
  longer-term clean state (commit/push within `committing_commit_minutes`
  but outside the active window); the `working` vs `synced` split
  means the user can see at a glance which repos the daemon is
  currently working through versus which are merely in a long-term
  clean state.
  Documented in `docs/design/repos-state-cause.md`.

- **`dracon-sync repos` `STATE` docs clarified**: The design docs and
  example config now explain the live table meanings in user-facing
  terms: `idle` is the normal clean quiet state, `cold` is the
  >24h quiet state, `stalled` is dirty tracked/staged work with no
  unpushed commits, and `intentional` is the per-repo no-upstream
  opt-out.

- **`dracon-sync` `PUSHED` column missing for freshly-cloned repos**:
  The `last_push_for_branch` helper used `git reflog show origin/<branch>
  --format=%cr -1`, which returns empty output for repos whose
  remote-tracking reflog has no entries (i.e. freshly cloned and
  never re-fetched). The PUSHED column showed `-` for those repos
  even though the remote-tracking ref was perfectly valid. The helper
  now uses `git log -1 --format=%cr origin/<branch>`, which returns
  the committer date of the remote tip in both the populated-reflog
  and empty-reflog cases. Regression test added: builds a bare repo,
  seeds a commit, clones it, and asserts the helper returns a real
  date.

- **`dracon-sync` per-repo `intentional_no_upstream` opt-out**: A repo
  whose `.dracon/dracon-sync.toml` sets `intentional_no_upstream = true`
  is now recognized as intentionally isolated (e.g., a legacy private
  mirror that the operator no longer wants auto-tracked). The
  `repos` table replaces the `NO_UPSTREAM` flag with the explicit
  `INTENTIONAL_NO_UPSTREAM` flag, the `PUSHED` column shows
  `INTENTIONAL` (rendered green), and the hint says
  `"intentional legacy isolation, no upstream configured"`. The
  `dracon-sync repair concerns` command skips the repo entirely and
  the auto-repair path never runs `git push -u origin HEAD` for it.
  This is a logic defect (the previous "run repair-concerns --apply
  (set upstream)" hint was misleading for repos the operator has
  intentionally left unconnected). Documented as invariant #6 in
  `docs/design/sync-push-classification.md`.

- **`install.sh` dry-run daemon verification**: The running-daemon
  verification block used `local` at top level, which broke
  `./install.sh --dry-run` under shells where `local` is only valid
  inside functions. The service-name variable is now plain shell state.

### Fixed
- **`dracon-sync` system-repo path bug**: The example template's
  `system_repo` default pointed at a non-git legacy directory. The actual git
  repo where the sync daemon's state lives is `~/.dracon`. The example
  template and the installed `dracon-sync.toml` are now both set to the
  correct path.
- **`dracon-sync` `STUCK_PUSH` flag now requires a recorded push failure**:
  The flag used to fire on any `ahead > 0`, including repos the daemon
  had not yet tried to push in the current cycle. It now consults the
  incident ledger for a recent `result: "fail"` entry within the last
  10 minutes. Repos with unpushed commits but no recorded failure show
  as `PENDING` instead. This is a logic defect, not a behavioural
  change to the daemon's actual sync work. Refined in commits
  `1135d6bb8` and `bac8316cc`.
- **`dracon-sync` multi-remote push no longer retries permanent rejections**:
  Pushing to a GitLab/Codeberg protected branch, or any server-side
  `pre-receive hook declined` rejection, used to burn the full
  `push_retries` budget on an outcome that cannot change. The new
  `is_permanent_push_rejection()` classifier detects five canonical
  error patterns (`pre-receive hook declined`, `protected branch`,
  `not allowed to push`, `deny updating`, `hook declined`) and returns
  immediately on match, logging one incident per cycle. This is a
  logic defect, not a change to which remotes are pushed. Added in
  commit `fd93b943f`.
- **`dracon-sync` `repair concerns` aligned with `repos` table**: The
  repair command used the old `ahead > 0 → concern` rule after the
  `repos` table had been refined, so the two surfaces disagreed on
  which repos were concerns. Both now use
  `repo_is_concern_with_push_failure()`: a repo is a concern when it
  has no origin/upstream, or is `behind > 0`, or is `ahead > 0` AND has
  a recent push failure in the incident ledger. The `stuck-push`
  repair filter also uses the same recent-push-failure requirement as
  the table's `STUCK_PUSH` flag. This is a logic defect (inconsistency
  between two views of the same data). Fixed in commit `bac8316cc`.
- **`dracon-system` doctor "dracon-libs" check is now correctly labeled
  dev-only**: The check used to say "dracon-libs (sibling)" with no
  hint that it is optional for installed binaries. It is now labeled
  `dracon-libs (dev sibling)` and the remediation explains that the
  sibling is required only for `cargo build` from source. This is a
  logic defect (mislabeling an optional check as required). Fixed in
  commit `fd93b943f`.
- **`dracon-sync` stage cooldowns are now enforced**: The daemon previously
  inserted a `stage_cooldowns` entry after `git add` timeout but never
  consulted it on later cycles. It now skips repos with active cooldowns
  and removes expired entries, preventing repeated timeout attempts while
  the cooldown is active.
- **`dracon-sync` multi-remote push retries after HTTPS fallback failure**:
  A failed HTTPS fallback used to return immediately and skip the SSH
  retry loop. The retry loop now runs after fallback failure, so transient
  SSH failures can still recover.
- **`dracon-sync` origin push stops immediately on permanent rejections**:
  `push_with_retries()` and the lower-level transport fallback path now
  check `is_permanent_push_rejection()` before auto-pull/retry/fallback,
  matching the multi-remote push path and avoiding retry-budget burn on
  protected branches.
- **`dracon-sync` config validation now warns on unsafe timing/ledger
  values**: `stage_cooldown_secs`, `pull_op_timeout_secs`,
  `push_op_timeout_secs`, `repo_sync_timeout_secs`,
  `inactivity_push_delay_secs`, `repair_cooldown_secs`, and ledger
  retention values now warn before they cause incident-ledger spam or
  misleading push-failure windows.
- **`dracon-sync` recent-push-failure lookup now reads the ledger tail**:
  The `STUCK_PUSH` classification no longer scans the entire append-only
  incident ledger on every `repos` call. It reads a bounded tail window
  (500 lines) and still uses the same 10-minute `recent_push_failure`
  semantics.
- **`dracon-sync repair-warns` no longer uses a coarse sync timeout**:
  Large but healthy repos can exceed the legacy `repo_sync_timeout_secs`
  wrapper while individual git operations are still making progress. Warn
  repair now delegates to `sync_repo`'s per-operation timeouts instead of
  aborting the whole triage pass with a synthetic timeout.
- **`scripts/scaffold_feature_repos.py` `--init-git` flag for self-contained
  workflow**: Generates the façade files, initializes a local git repo,
  commits them with `--no-verify`, and adds `DraconDev/<name>` as the
  `origin` remote. The operator only has to `git push -u origin main` after
  the GitHub repository exists.
- **`scripts/scaffold_feature_repos.py` `--monorepo-root` defaults to the
  script's own directory**: The previous default of `Path.cwd()` only
  worked when the operator ran the script from the monorepo root. The new
  default is `--monorepo-root` resolves to the directory that contains
  `scripts/`, so the script behaves the same regardless of cwd. CLI flags
  still take precedence.
- **`dracon-sync repos` last-push query skips unsafe branch names**:
  `last_push_for_branch()` now short-circuits when the current branch is
  empty (detached HEAD) or contains shell-special characters that would
  break the `git reflog show origin/{branch}` argument. Previously the
  command was run unconditionally and the column silently showed "-".
- **`dracon-sync repos` `git log` subject parser preserves unit separators**:
  `parse_git_log_meta_line()` rejoins any extra unit-separated fields back
  into the subject, so a commit subject that itself contains `\x1f` is
  reconstructed verbatim instead of being truncated at the first extra
  field.
- **`dracon-sync repos` hint text now matches WARN vs CONCERN semantics**:
  A dirty repo with unpushed commits but no recent push failure is still
  `WARN`, so its hint now says the daemon will push after changes settle
  instead of suggesting `repair-concerns`.
- **`dracon-sync repos --json` keeps stdout machine-readable on repo failures**:
  Repo init/status failures are still counted and reported, but in JSON
  mode their human failure lines are sent to stderr so stdout remains valid
  JSON.
- **`dracon-sync` broken-tracking repair log now shows the real old
  tracking ref**: The startup repair used to print a fake
  `branch/branch -> origin/branch` message. It now parses the actual
  `[origin/master: gone]` ref and prints the real old/new mapping.
- **`dracon-warden` plaintext-sibling hatch checks now use the repo path**:
  `scrub-markers` and `resmudge` previously checked for `<file>.plaintext`
  relative to the current working directory. They now check under the repo
  being scanned, so the hatch works when the command is run from outside
  the repo.
- **`dracon-system` renice state now updates only after `renice` succeeds**:
  The guard used to record a PID as reniced even if the external `renice`
  command failed. It now treats renice failure as a failed action and does
  not update in-memory state.
- **Cargo.lock refreshed for the current dependency graph**: `cargo build
  --locked` failed because the committed lockfile was stale. Refreshing it
  keeps CI/build validation reproducible without requiring lockfile writes
  during validation.

### Added
- **GitHub utility feature façade scaffolding**: Added
  `scripts/scaffold_feature_repos.py` and
  `docs/design/github-feature-repos.md` so `dracon-sync`,
  `dracon-system`, and `dracon-warden` can be presented as separate GitHub
  feature surfaces without duplicating or moving implementation code out of
  the monorepo.
- **`dracon-sync` `stage_op_timeout_secs` policy field**: Configurable
  idle timeout (default 60s, min 10s) for `git add -A` and other
  staging operations on a single repo. The previous hardcoded 30s
  timeout was too tight for large repos (e.g.,
  `browser-extensions-shared` with 2500+ dirty paths took 88s) and
  caused the daemon to log a "staging timeout" incident on every
  cycle. The default of 60s gives headroom for typical work without
  making the daemon feel stuck. Added in commit `1135d6bb8`.
- **`dracon-sync` `stage_cooldown_secs` policy field**: When
  `git add` exceeds `stage_op_timeout_secs`, the daemon pauses
  further staging attempts on that repo for the configured duration
  (default 3600s = 1 hour). The point is to stop incident-ledger
  spam: a single repo that consistently times out will otherwise log
  a new incident every cycle. After the cooldown elapses, the daemon
  tries `git add` again; if it times out once more, the cooldown
  resets. The cooldown is per-repo; other repos are unaffected. Added
  in commit `00bba440d`.
- **`dracon-sync` push-rejection classification design note**:
  `docs/design/sync-push-classification.md` documents the
  `STUCK_PUSH` vs `PENDING` semantics, the 10-minute
  `recent_push_failure` window derived from the incident ledger, the
  `is_permanent_push_rejection` regex set, the retry policy, and the
  `repos` ↔ `repair concerns` invariant. Added in this release.


### Changed
- **CLI print style**: All three binaries (`dracon-sync`, `dracon-warden`,
  `dracon-system`) now use a consistent visual language for human-facing
  output. The `status` tables include a summary one-liner and grouped
  sections; byte counts and timeouts are formatted as human-readable
  (e.g. `50.0 MiB`, `1m 30s`); freeze/doctor indicators are coloured
  (suppressed when `NO_COLOR` is set). `dracon-system doctor` now emits
  per-check remediation hints. Design note:
  `docs/design/cli-print-style.md`.
- **CLI print polish (round 2)**: Four specific surfaces that were still
  weak have been upgraded. `dracon-sync repos` now has a legend line,
  multi-line icon+label headers, ✅/⚠️/❌ status cells, and a color-aware
  summary (no raw ANSI when piped). `dracon-sync health` now uses a single
  table with a summary one-liner; warnings are grouped into their own
  block with a count. `dracon-warden scrub-markers`/`resmudge`/`repair`/
  `keygen`/`setup-hooks` each print a 2-3 line informative summary, even
  when nothing was changed. `dracon-system events` shows a severity-counts
  footer and a one-line summary before the table. See the
  `docs/design/cli-print-style.md` design note for the full set of
  conventions.

### Added
- **Warden plaintext-sibling escape hatch**: `dracon-warden` now supports an
  opt-in escape hatch for files that should be stored verbatim (not encrypted).
  Touch a `<file>.plaintext` sibling next to any tracked file to opt it in.
  The clean filter returns the file unchanged, the pre-push hook silently
  skips it, and `scrub-markers` / `resmudge` leave it alone. Threat model,
  revocation story, and what the hatch does NOT protect against are in
  `docs/design/warden-plaintext-sibling.md`. Default install behaviour is
  unchanged: no hatch, no plaintext.
- **CI/CD pipeline**: `.github/workflows/ci.yml` — fmt check, clippy, build, serial tests
- **Lint gates**: `#![warn(missing_docs)]` on all 4 crate roots
- **dracon-libs docs**: Fixed all 95 missing-doc warnings in dracon-git
- **Module extraction** (dracon-system): `events.rs`, `links.rs`, `zram.rs`, `doctor.rs`, `safety.rs` — 850 lines, 20% main.rs reduction
- **Module extraction** (dracon-sync): `branch.rs`, `config.rs`, `diff.rs`, `discovery.rs`, `misc.rs`, `multi_remote.rs`, `ops.rs`, `push.rs`, `staging.rs`, `status.rs`, `urls.rs` — 1,846 lines, 45% git/mod.rs reduction
- **Startup cleanup**: Sync daemon prunes stale state on every start/restart — stuck repos, incident ledger retention, visibility cache orphans, guard log rotation
- **Broken tracking repair**: `repair_broken_tracking()` detects `origin/master: gone` refs and re-points to `origin/{branch}` — runs at daemon startup
- **GitHub orphan cleanup script**: `scripts/cleanup-github-orphans.sh` — lists and deletes 83 suffixed orphan + test repos (needs `delete_repo` scope)
- **dracon-libs get_diff fallback**: `get_diff()` now falls back to CLI on libgit2 errors (binary blobs, nul bytes)

### Changed
- **Dead code cleanup**: Removed `git_list_paths` (zero callers), `Level::as_str`/`Event`/`timestamp_secs` from log.rs (unused after JSON→human refactor), gated `fallback_status_rank`/`acquire_path_lock` with `#[cfg(test)]`, fixed all clippy unused-import/never-constructed warnings across all 3 crates
- **Scratch file cleanup**: Removed local task/scratch files and stale task directories from git tracking; added matching `.gitignore` rules.
- **Service restart policy**: All 3 services changed from `Restart=on-failure` to `Restart=always` — daemons now restart even after clean exits, preventing 5+ hour outages
- **CLI output style**: All status commands now use Title Case keys (`Policy:` not `POLICY:`) for consistency with JSON output and health check format
- **Daemon log noise**: Silent when healthy — concern/warn summaries only print when `found > 0`
- **Structured logging**: `log.rs` now prints human-readable `⚠️ message` to stderr instead of raw JSON — JSON incident records stay in the ledger file only
- **Link status**: Prints "No configured links" instead of empty table when 0 links exist
- **dracon-sync**: Scribe refactor — commit messages from diffs, not `project-state.md`
  - `generate_commit_message()`: AI receives current diff (main) + 10 previous diffs (background) + recent subjects → returns subject line
  - `local_fallback_message()`: file-pattern fallback (e.g., "update auth, jwt and 2 files") when AI unavailable
  - Removed `scribe_update()` and `stage_project_state()` — replaced by direct commit message generation
  - Removed `read_project_focus`, `extract_category_scope_from_focus`, `extract_scope_from_focus`, `git_log_recent_subjects`
  - `project-state.md` is now manual-only: sync no longer auto-generates, stages, or commits it
  - `parse_conventional_commit()` extracts (category, scope, description) from AI subject to prevent double-prefix
- **dracon-warden**: Secret scanner pattern fixes
  - Added "Hex Secret (Quoted)" pattern: catches 32+ char mixed-case hex strings in quotes
  - Added "High-Entropy Secret (Quoted)" pattern: catches 24+ char alphanumeric strings in quotes
  - Added "Slack Bot Token (Compact)" pattern: catches `Slack token prefixes without numeric ID segments
  - GitHub token patterns (`GitHub token prefixes): accept 30-40 chars (was exactly 36)
  - Mailgun API Key pattern: accept 28-34 chars (was exactly 32)

### Added
- **dracon-sync**: Mirror visibility sync (`sync_visibility` config)
  - Mirrors on Codeberg/GitLab automatically match GitHub's public/private status
  - Cache-gated: at most one API check per repo per `sync_visibility_interval_hours` (default: 24h)
  - `gh api` for GitHub reads, `curl` for GitLab/Codeberg writes
  - `strip_ansi()` helper for `gh api` JSON output (GitHub CLI injects color codes)
- **dracon-sync**: Mirror metadata sync (`sync_metadata` config)
  - Mirrors get GitHub's description and topics/tags synced automatically
  - Shares the same cache-gate as visibility sync
- **dracon-sync**: Three-toggle release pipeline (`auto_tag`, `auto_release`, `auto_publish`)
  - `auto_tag = true` (default on): Git tag `v{version}` on every version bump
  - `auto_release = false` (default off): GitHub Release on major bumps via `gh release create`
  - `auto_publish = []` (default empty): Publish to crates.io/npm/PyPI (per-registry opt-in)
  - All three require per-repo opt-in via `.dracon/dracon-sync.toml`
  - Dry-run safety: `cargo publish --dry-run` / `npm publish --dry-run` before real publish
  - Idempotent: skips if version already exists on registry
  - Non-fatal: publish failures log incidents but don't break the sync cycle
- **dracon-sync**: `publish` and `publish-status` CLI subcommands
  - `publish <repo>`: Manually publish to configured registries
  - `publish-status <repo>`: Check current version and registry status
- **dracon-sync**: `SyncOutcome` enum (`Synced`/`NothingToDo`/`Blocked`)
  - Replaces `Result<bool>` — daemon only increments failure count on actual errors
  - Clean repos no longer accumulate false failure counts
- **dracon-sync**: `GIT_ASKPASS` for GitLab/Codeberg HTTPS PAT push
  - Replaces URL-embedded `oauth2:TOKEN@` and `git:TOKEN@` patterns
  - Tokens no longer visible in process listings or logs
- **dracon-sync**: `effective_auth_type()` and `resolve_account()` on `RemoteConfig`
  - Auto-detects GitLab/Codeberg from push URL when `auth_type` not explicitly set
  - Extracts account name from push URL pattern for API calls
- **dracon-sync**: Permission checks on secrets directory
  - `load_secret` rejects world-writable directories and warns on world-readable files
- **dracon-sync**: AI major-bump cap — `parse_ai_bump_response` downgrades `Major` → `Minor`
  - Major version bumps require manual intervention
- **dracon-sync**: HTTPS+PAT fallback for GitLab and Codeberg pushes
  - `gitlab_https_url()` and `codeberg_https_url()` functions convert SSH URLs to HTTPS
  - `GITLAB_TOKEN` and `CODEBERG_TOKEN` used for authentication over HTTPS
  - Applied to both `push_to_named_remote` and `push_with_transport_fallbacks`
- **dracon-sync**: `GIT_TERMINAL_PROMPT=0` set on all git push commands
  - Prevents interactive SSH login prompts in daemon, CLI, and tests
- **dracon-sync**: Repo discovery optimization
  - `discover_git_repos_recursive` now skips descending into subdirectories of already-discovered repos
- **dracon-sync**: `HashSet`-based filter-only path matching
  - `git_diff_head_files` returns `HashSet<PathBuf>` for exact path matching
  - Prevents substring collision (e.g., `main.rs` matching `src/main.rs`)
- **dracon-sync**: Visibility cache uses repo path hash as key
  - Prevents same-name repo collisions across different watch roots
- **dracon-system**: `is_protected_ancestor` replaces exact-match path protection
  - `/home` now protects `/home/dracon/Dev`, `/etc` protects `/etc/nginx`, etc.
  - Root path `/` is exact-match only (prevents protecting everything)
- **dracon-system**: `auto_cleanup_apply` guard config (default: `false`)
  - Daemon runs cleanup in dry-run mode by default
  - `auto_truncate_logs` also gated behind `auto_cleanup_apply`
- **dracon-system**: Docker prune respects `apply` gate
  - `docker_prune(false, ...)` returns 0 without invoking docker
- **dracon-system**: PID verification before SIGKILL
  - Reads `/proc/{pid}/cmdline` after SIGTERM wait to confirm PID still belongs to same git process
- **dracon-system**: Strict git process command matching
  - Replaced substring `contains("git")` with `starts_with("git ")` + exact subcmd whitelist
- **dracon-system**: `expand_tilde` fallback changed from `/home` to `.` with logged warning
- **dracon-system**: `process_cpu_percent` default changed from `180.0` to `50.0`
- **dracon-warden**: Binary file passthrough in smudge filter
  - `is_binary_content()` detects null bytes; binary files pass through unchanged
- **dracon-warden**: Individual regex patterns with memory limits
  - `RegexBuilder` with `dfa_size_limit(1_000_000)` and `size_limit(10_000_000)`
  - Prevents ReDoS via catastrophic backtracking
- **dracon-warden**: Path-component matching for sensitive directories
  - `path_components_match` uses `.windows()` for multi-component dirs like `.config/gcloud`
  - `smart_clean_with_path` uses `path.components()` instead of substring `contains`
- **dracon-warden**: Exact filename matching (fixes `coreutils` false positive)
  - `starts_with("core")` replaced with exact match or `"{name}."` prefix

## [0.112.4] - 2026-06-07

### Fixed
- `dracon-sync/README.md` and `docs/OPERATIONS.md`: replaced flat CLI paths
  (`repair-concerns`, `repair-warns`, `stuck list`, `dual-branch list`,
  `publish-status`, `repair-origins`) with the correct nested subcommand
  syntax (`repair concerns`, `repair stuck-list`, `publish run`, etc.).
  Resolves audit-2026-06-07 P1-2.
- `dracon-warden status` help text and README "Quick Commands" sections
  now say "repo roots" (matching the v0.3.0 `watch_roots` → `repo_roots`
  field rename). Resolves audit-2026-06-07 N-4.
- `dracon-system/README.md` server-deployment systemd snippet: corrected
  resource limits from `MemoryMax=100M CPUQuota=10%` to `MemoryMax=250M
  CPUQuota=20%` (matching `dracon-system-guard.service`).
- Removed `dracon-sync/note.md` (leftover investigation note from a May
  incident). Added gitignore rule so future `note.md` files are not
  tracked. Resolves audit-2026-06-07 P2-5.
- Untracked 4 stale tarpaulin coverage reports (~1.6 MB) across all 3
  binaries. Added `**/tarpaulin-report.*` to `.gitignore`. Resolves
  audit-2026-06-07 P2-4.
- Removed dead `let discover = effective_discovery_roots(&policy);`
  binding in `dracon-warden/src/main.rs:1356` (the result was never
  used; `explicit_discover` was built directly from `policy.discover_roots`).

### Changed
- Workspace version bumped 0.112.3 → 0.112.4 (hygiene-only release, no
  per-crate version changes).
- `dracon-system/src/print.rs` and `dracon-warden/src/print.rs`: added
  module-level `#![allow(dead_code)]` with a doc comment explaining the
  public-API intent (helpers for shared output formatting, awaiting
  callers). Resolves audit-2026-06-07 N-1.

### Audit
- **Audit hygiene**: internal audit artifacts were reviewed during release prep and are not included in the public tree. User-facing release notes and operational docs now carry the public guidance.

## [0.3.0] - 2026-06-07

### Breaking
- **`dracon-warden` `watch_roots` field renamed to `repo_roots`**: The old
  name was misleading (warden has no daemon mode and does not watch
  filesystems; the field is a list of directories to scan for git repos
  on demand). The canonical field is now `repo_roots`. The example toml,
  user guide, and BLUEPRINT all use the new name.

### Deprecated
- **`watch_roots` is still accepted** for backwards compatibility. When
  the old key is set (alone or alongside `repo_roots`), the policy still
  loads, but:
  - A deprecation warning is logged to stderr:
    `warning: 'watch_roots' is deprecated, use 'repo_roots' instead`
  - A yellow ⚠ row appears in `dracon-warden status`
  - When both keys are set, `repo_roots` wins and a different message
    indicates the conflict
  This alias will be removed in a future major release.

### Fixed
- **`dracon-warden` status no longer shows two identical root rows**:
  Previously the status table showed `🛡️ Watch roots` and
  `🧭 Discovery roots` as separate rows that were identical when
  `discover_roots` was unset. The status is now consolidated to a single
  `🔍 Repo roots` row, with an explicit `🧭 Discovery roots (additional)`
  row only when the user has set a non-empty `discover_roots` that
  extends the `repo_roots` set.

### Changed
- **`dracon-warden` legacy path removed from default config**: The
  example toml and the installed user config no longer include a legacy
  non-git directory. The directory itself is not deleted; the user can
  decide what to do with its contents.

## [0.2.0] - 2024-05-03

### Added
- **dracon-system**: Guard daemon for disk/process monitoring
  - Disk usage thresholds (70/80/90/95%)
  - Auto-freeze dracon-sync at 90% disk usage
  - Auto-cleanup Rust target directories
  - Process CPU monitoring with notifications
  - Zombie process detection
  - Inode usage monitoring
- **dracon-warden**: Security hardening daemon
  - Git filter encryption for secrets
  - `DRACON_SECRET` marker support
  - `scrub-markers` recovery tool
  - `resmudge` working tree repair

### Changed
- Restructured as cargo workspace with separate crates

## [0.1.0] - 2024-04-28

### Added
- **dracon-sync**: Initial release
  - Auto-commit, auto-pull, auto-push
  - AI-powered commit messages (scribe)
  - Version bumping (ai-bumper)
  - Incident ledger for debugging
  - Stuck repo management
  - Dual-branch repair (main/master)
  - Orphan origin URL repair
  - GitHub private repo auto-creation
  - Multi-remote push support
  - Webhook notifications

---

## Versioning Notes

- **MAJOR**: Breaking changes to config format or CLI interface
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, documentation updates

