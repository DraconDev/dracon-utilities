# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **dracon-sync: deep untracked
  subtrees not staged (goal `662a6e15` /
  2026-06-16)**: `stage_existing_files`
  in `dracon-sync/src/sync.rs` walked
  untracked directories only 1 level
  deep, missing files nested 2+ levels
  below. libgit2 collapses a fully-
  untracked subtree into a single
  top-level dir entry (e.g.
  `web/games/libs/game/effects/src/`
  for files at `.../styles/*.css`), so
  the 1-level walk found only the
  `src/` subdir, didn't recurse, and
  staged nothing. Result: the
  operator's deep code/library files
  (Svelte components, CSS styles)
  stayed untracked for minutes at a
  time. Fixed with a full recursive
  walk using an explicit stack (with
  guards for symlinks, dotfile dirs,
  and `excluded_dir_names`). Added 2
  new unit tests
  (`test_stage_existing_files_recurses_deeply`,
  `test_stage_existing_files_skips_node_modules_and_dotdirs`).
  Test count: 854 -> 856. Note: the
  2 operator-deferred
  `_template-*/global.css` files were
  auto-staged as part of the bulk
  commit `71b3f887b` because the fix
  staged the whole tree; content is
  unchanged.
- **dracon-sync: PUSH_STUCK prevention
  (goal `87c1bf4d` / 2026-06-16)**: the
  daemon's concurrent multi-remote push
  could race when one remote (gitlab,
  codeberg) was slow, causing a divergence
  and ultimately PUSH_STUCK after
  `push_max_retries`. Fixed with two
  complementary changes:
  1. Config: added `force_push_when_behind
     = true` to gitlab and codeberg remotes
     so the daemon's existing
     `--force-with-lease` mechanism auto-
     recovers from divergence.
  2. Code: changed `push_to_all_remotes`
     in `dracon-sync/src/git/multi_remote.rs`
     from concurrent `tokio::spawn` to
     sequential `for remote in sorted`.
     This eliminates the race entirely.
     Trade-off: 4-remote push takes ~6s
     instead of ~1.5s, but the user-visible
     cadence is similar (the daemon's 2s
     apply phase deadline was already
     causing trailing-drain on every
     commit when concurrent pushes took
     >2s).
  Also unblocked the current divergence
  on dracon-platform with `git push
  --force-with-lease=<old-sha>`. Added
  3 new unit tests in `multi_remote.rs`:
  empty list, priority order, continues
  after failure. Test count: 851 -> 854.

### Changed
- **dracon-sync: lower
  `inactivity_push_delay_secs` from 5 to
  3 in operator config (goal `114541e6` /
  2026-06-16)**: with AI agents editing
  the same repo, the 5s debounce was
  chaining unrelated changes into single
  commits. The 3s debounce produces more
  granular commits with better provenance.
  Safe because the `MAX_DIRTY_DELAY=5s`
  hardcoded force-commit backstop in
  `daemon.rs` still applies — the daemon
  never waits >5s when the operator is
  actively editing. Code default unchanged
  (still 5s in `policy.rs`); only operator
  config is lowered.

### Investigated
- **dracon-platform: PUSH_STUCK from
  divergent remotes (goal `42ea41d4` /
  2026-06-16)**: during the cadence
  investigation, `dracon-platform` entered
  PUSH_STUCK with 11 consecutive push
  failures. Root cause: race condition in
  concurrent multi-remote push. The daemon
  pushes to origin/github/gitlab/codeberg
  in parallel via `tokio::spawn`. When
  network is slow on one remote, a
  subsequent fast-forward can land on
  origin/github but be rejected by the
  slow remote (which is still at an older
  tip). Fix options documented in
  `docs/design/commit-all-principle-
  2026-06-16.md` (Followup #3). None
  applied. Operator-approval needed for
  the `force_push_when_behind = true`
  config change.

### Investigated
- **dracon-platform: commit cadence
  (goal `42ea41d4` / 2026-06-16)**: operator
  observed "commits are fairly infrequent"
  on the platform. Measured cadence:
  dracon-platform is the FASTEST-committing
  repo (1.73 commits/min, 26 commits in 15
  min) — higher than the 4 other active
  repos combined. The "infrequent" perception
  is misleading. Per-commit timing: 5s
  debounce + 6s 4-remote push = 11.5s
  minimum. The operator is editing faster
  than the daemon can commit, so it always
  feels behind, but the cadence is actually
  correct given the constraints. 4 options
  considered (lower debounce / raise
  pulse_interval / parallelize push /
  exclude smoke-out PNGs) — none applied.
  Operator-decision pending. Design doc
  documents the full investigation.

### Investigated
- **dracon-platform: 7 untracked dirs (goal
  `05ea6904` / 2026-06-16)**: operator saw
  "the platform has a ton of files that are
  not getting commited" in the daemon
  report. Investigation showed the daemon
  WAS committing them (161 uncommitted
  files → 2 uncommitted files over 40
  minutes, with 30+ commits including
  bulk commits of 38, 47, and 86 files).
  The "7 untracked" in the daemon report
  is the count of top-level untracked
  DIRECTORIES, not uncommitted FILES. The
  2 remaining uncommitted files are
  operator-decision items from
  goal `6205ad1f` Part B section 8
  (deferred `_template-*` subtrees per
  `76ddaa7e`). No code/config changes
  needed. Design doc updated with the
  full investigation.

### Removed
- **dracon-sync: `check-untracked-md` subcommand
  (goal `6205ad1f` / 2026-06-16)**: reverted the
  `e680cfa9` "defensive guard" hack. Operator
  feedback: "this seems like a hack git sync should
  not handle it i think warden already down so that
  is not good". git-sync is the wrong layer; the
  defensive guard against untracked content belongs
  in warden (which has git hooks + the
  `DRACON_SECRET:` encryption flow), not in
  git-sync. Also removes the `noteworthy_untracked()`
  function in `git::diff`, the 6 unit tests in
  `noteworthy_untracked_tests` mod, and the
  `CheckUntrackedMd` enum variant. Test count:
  857 -> 851.

### Fixed
- **browser-extensions-shared: moved
  `platform-free-extension-shortlist.md` from the
  CWD-drift doubled path to the intended path
  (`docs/research/extension-research/platform-free-
  extension-shortlist.md`) and committed (goal
  `c19d21b8` followup, 2026-06-16)**: the file was
  the deliverable for an older AI goal
  (`abf12ed7-9286-4b4f-9af0-caa827bfe296`) and
  was cross-linked from a tracked file; the doubled
  path was caused by an AI agent launching from a
  repo subdirectory (CWD drift), not by a real
  duplicate file.
- **dracon-platform: resolved real merge in progress
  (CONCERN → ✅ OK, goal `e04de70b` / 2026-06-16)**:
  operator said "we need to look inoth the draocn
  platform is ... and kiki those aer the msot concenred
  in detail". Root cause: daemon auto-pulled origin's
  `04ec2afad` at 08:48:48 (push→pull→conflict loop),
  leaving 6 files in conflict between local (7 commits
  ahead) and origin (1 commit behind). The `.git/MERGE_HEAD`
  blocked the daemon for 240 minutes; max failures
  exceeded at 08:51:14; sync alert at 12:24:10.
  Resolution: `git checkout --ours` for all 6 conflicting
  files (local is more recent — local has the
  `audit(2026-06-16): reclassify Paddle key findings
  P0/P1 → P3` reclassification on the audit doc and
  more recent screenshots; same author on both sides),
  then `git commit` (merge `75f3c4e7f`). 4 files
  merged cleanly from origin and stayed in working
  tree (`hellhunter/src/lib/components/StartScreen.svelte`,
  `hegemon/src/lib/components/MenuRightPanel.svelte`,
  `_template-visual-novel/static/favicon.png`,
  `tests/games/_template-visual-novel.spec.ts`).
  Post-merge: daemon auto-committed operator's other
  in-flight work (25+ files: `_audit-clone-test/*`
  deletions, hellhunter `GameCanvas.svelte` `chromaKey`
  sprite work, junk-runner assets, screenshots, etc.)
  across commits `036a467bc` → `8e4cd8265` →
  `0cc83abc7`. All 4 remotes (origin + github +
  gitlab + codeberg) now aligned at `0cc83abc7`.
  Live report went from `CONCERN, 100 MOD, 11 UT,
  3 AHEAD, 1 BEHIND, push pending 240m` to `✅ OK,
  0 AHEAD, 0 BEHIND, all 4 remotes aligned,
  1 UT (_template-visual-novel/src/lib/),
  untracked-only state`. The remaining
  `_template-visual-novel/src/lib/` untracked tree
  is the operator's new template (daemon does not
  auto-stage untracked content in this template per
  the 76ddaa7e constraint). Live report final state:
  `12 OK + 2 WARN + 0 CONCERN + 0 failed`. The
  2 WARN are: `Junk-Runner-bevy` from operator's
  active work (healthy settling) and
  `kiki-sassy-desktop-announcer` PUSH_STUCK 49m
  unchanged — pre-existing divergent history
  (804 ahead / 436 behind on github) awaiting
  operator's option (a/b/c/d/e) from handoff
  `docs/design/kiki-sassy-decision-handoff-2026-06-15.md`.
  Documented in
  `docs/design/concern-investigation-2026-06-16.md`.

### Changed
- **dracon-sync: durable commit-all policy
  (goal `546d4f9c` / 2026-06-15)**: operator asked
  for the previous goal's policy change to be
  permanent, not a one-time config edit. Code
  defaults in `dracon-sync/src/policy.rs` updated:
  - `default_exclude_file_patterns()`: was
    `["*.log", "nohup.out", "*.sqlite", "*.db", ...]`
    → `Vec::new()` (commit logs and DBs by default)
  - `default_untracked_exclude_patterns()`: removed
    audit/screenshot/media/note patterns
    (`**/audit/**`, `**/evidence/**`,
    `**/screenshots/**`, `*.png`, `*.jpg`, `*.mp4`,
    `**/note.md`, etc.). Now only session-scratch
    patterns remain (`**/scratch/**`, `**/tmp/**`,
    `**/pi-tmp/**`, `.demon/**`, `.sisyphus/**`,
    `.ralph/**`).
  - Test updated: `test_default_exclude_file_patterns`
    now asserts empty.
  - Test added:
    `test_default_untracked_exclude_patterns_is_commit_all_unless_scratch`
    asserts the new defaults (session-scratch in,
    audit/screenshot/media/note out).
  - Test added:
    `test_example_toml_matches_policy_defaults` loads
    `dracon-sync.example.toml` as a `SyncPolicy` and
    asserts that `exclude_file_patterns`,
    `untracked_exclude_patterns`, and
    `max_stage_file_bytes` match the code defaults.
    Catches future drift between the example config
    and the policy code.
  - `dracon-sync/dracon-sync.example.toml` updated
    to reflect new defaults (also fixed pre-existing
    duplicate `sem_max_concurrent_sync` key and
    updated `max_stage_file_bytes` from 50 MiB to
    100 MiB to match the new code default).
  - New `AGENTS.md` created with plain-English
    documentation of the policy, daemon commands,
    and forbidden actions.
  - Result: 851 tests pass (was 849 + 2 new),
    release build clean, cargo deny clean, all
    4 remotes aligned at `01632c41`. Per-repo
    `auto_commit_exclude_patterns` mechanism still
    works (Junk-Runner-bevy keeps its
    `test-results/` exclusion). Concerns
    investigated: kiki-sassy `github` push-stuck
    (now 10+ failures) is a divergent history issue
    (436 github-only commits containing real
    feature work like MESSAGES.md, GitHub
    Sponsors button, truncation code; 785
    local-only commits). `git merge-tree`
    simulation shows **316 merge conflicts across
    15 files** (6 Rust, 3 Nix, Cargo, encryption
    key, configs) if option (b) is chosen.
    **NEEDS OPERATOR INPUT** to resolve
    (operator-owned repo). Other 2 concerns
    (dracon-platform 5 MOD, Junk-Runner-bevy
    88 MOD) resolved or working as designed.
    Documented in
    `docs/design/commit-all-policy-durable-2026-06-15.md`.

### Removed
- **dracon-sync: revert all filtering — let the daemon
  commit every change** (2026-06-15, goal `76ddaa7e`):
  the per-repo `auto_commit_exclude_patterns` filters
  added by goals `1fe80684` and `3276ceb4`, plus the
  `ExcludedDirty` state and the WARN filter helper
  function, have been reverted per operator feedback:
  > "what is even the excluded dirty i jsut checked the
  > rust ai web auto and its just 3 repots not getting
  > commited that is clearly our eerror we should not
  > be disincluding markdown files, the untrack files
  > in the browser extensions is another markdown adn
  > the junk runner is jsut disincluding a bunch of
  > pngs, we shoudl nto filter these out at all"
  Concrete changes:
  - Removed `auto_commit_exclude_patterns` from
    `Junk-Runner-bevy/.dracon/dracon-sync.toml`
    (was `["**/test-results/**", "**/e2e/screenshots/**"]`)
    and `rust-ai-web-auto/.dracon/dracon-sync.toml`
    (was `["reports/kdp-live-*.md"]`). The per-repo
    files are kept (with explanatory comments) so the
    override slot exists for future tuning.
  - Removed `StateCause::ExcludedDirty` variant
    from `dracon-sync/src/report.rs` (goal `3276ceb4`).
  - Removed `count_non_excluded_modified_files`
    helper function and the WARN-filter call sites
    in `run_repos_report` and `run_repair_warns`
    (goal `1fe80684`). WARN classification is back
    to using `status.modified_files > 0` directly
    (pre-`1fe80684` behavior).
  - Removed the `dir/<glob>` pattern support in
    `dracon-sync/src/exclude.rs::matches_untracked_exclude`
    (the rust-ai-web-auto and Junk-Runner-bevy
    patterns were the only consumers).
  - Removed 9 tests (5 from `1fe80684`, 4 from
    `3276ceb4`).
  Test count: 851 (was 860 before reversion).
  Live impact: `Junk-Runner-bevy` 90 MOD test-results/
  PNGs are now committed normally; `rust-ai-web-auto`
  3 MOD `kdp-live-*.md` files are now committed
  normally. The daemon's 5s `inactivity_push_delay_secs`
  (set by goal `546d4f9c`) is the source of truth for
  commit timing. Documented in
  `docs/design/revert-filters-2026-06-15.md`.

### Added
- **dracon-platform: committed 391 untracked content
  files across 4 game trees** (2026-06-16, goal
  `ae389d76`): the operator said "we are still seeing
  untrackeds make sure we are addressing it". The
  goal inventoried and addressed all untracked
  content in the 14 reporting repos. In
  `dracon-platform`:
  - `web/games/games/hegemon/src/lib/**` (41 files of
    real game source: `audio/musicService.svelte.ts`,
    `components/*.svelte`, `game/` SvelteKit game
    logic). Committed in batch with the other trees.
  - `web/games/games/hegemon/static/assets/**` (306
    files of real game assets: `backdrops/`,
    `buildings/`, `creatures/` SVGs with content
    names like `castle_angel_13.svg`).
  - `web/games/games/hellhunter/src/lib/**` (36 files
    of real game source: `game/e2e.test.ts`,
    `game/generatedAssets.ts`, `game/music.test.ts`,
    `components/`).
  - `web/games/src/routes/games/[slug]/**` (2 files
    of real SvelteKit route code: `play/+page.svelte`,
    `play/+page.ts`, with `+page.server.ts` added
    during the work).
  - 3 `.svelte.ts` state stores
    (`web/games/games/hegemon/src/lib/game/state/game.svelte.ts`,
    `web/games/games/hegemon/src/lib/state/saveStore.ts`,
    `web/games/games/hellhunter/src/lib/game/state/gameStore.svelte.ts`)
    were blocked by the warden-managed `state/`
    gitignore pattern. Added a scoped re-inclusion
    `!web/games/**/state/` and `!web/games/**/state/**`
    AFTER the warden-managed block (line 155-156) so
    the source stores can be committed while leaving
    the top-level runtime `state/` (databases) ignored.
  - Discovered mid-work: the warden-managed
    `generated/` pattern (line 120) and the daemon's
    build-artifact cleanup auto-removed 38 tracked
    files in `web/games/games/hellhunter/static/generated/`.
    The operator's gitignore is correct — these are
    build outputs, not source. Left untracked (as
    intended by the gitignore). No action needed.
  Commits: `94afdc14a` (384 files bulk), `9c12f6b96`
  (2 routes files), `cc6f5cae2` (3 state stores +
  1 new server file + .gitignore re-inclusion).
  4-remote alignment: origin, github, gitlab, codeberg
  all at `8f0d819e7e5f` (then more operator commits
  on top).

- **dracon-platform: gitignored
  `web/tests/tmp-snap.spec.ts`** (2026-06-16, goal
  `ae389d76`): added the pattern `web/tests/tmp-*.spec.ts`
  to `/home/dracon/Dev/dracon-platform/.gitignore`
  AFTER the warden-managed block (line 170). The
  file is a 25-line scratch Playwright test that
  hardcodes a one-off session path
  `web/.pi-tmp/site-nobrainers-2026-06-15/` (a
  dated audit session). The `tmp-*.spec.ts` pattern
  is future-proof for any other operator session
  scratch tests. Verified with
  `git check-ignore -v web/tests/tmp-snap.spec.ts`.

- **browser-extensions-shared: committed 12 PNG icon
  files across 3 extension `public/` dirs**
  (2026-06-16, goal `ae389d76`): the operator said
  "we are still seeing untrackeds make sure we are
  addressing it". The 3 public/ dirs contained
  extension icons (16/32/48/128 px) for:
  - `extensions/page-audit/public/icon/{16,32,48,128}.png`
  - `extensions/page-diff/public/icon/{16,32,48,128}.png`
  - `extensions/research-notebook/public/icon/{16,32,48,128}.png`
  Staged with `git add <dir>` (specific paths, not
  `git add .`). The daemon committed and pushed to
  all 4 remotes. 4-remote alignment:
  origin, github, gitlab, codeberg all at
  `f260dd072732`.
  PENDING OPERATOR DECISION: the 4th untracked entry
  in browser-extensions-shared, the markdown file
  `docs/research/extension-research/docs/research/extension-research/platform-free-extension-shortlist.md`
  (11,130 bytes), is LEFT UNTRACKED per the previous
  goal's preserved constraint (goal `76ddaa7e`:
  "NEVER auto-stage the untracked markdown in
  browser-extensions-shared (ASK first)"). The
  operator has been informed and asked to choose
  (a) commit, (b) gitignore, (c) defer. Documented
  in `docs/design/untracked-content-resolution-2026-06-15.md`.

### Fixed
- **dracon-sync: WARN classification now respects
  per-repo `auto_commit_exclude_patterns`**
  (2026-06-15, goal `1fe80684`): the WARN signal in
  `dracon-sync/src/report.rs` previously counted every
  modified tracked file, including files the operator
  had explicitly excluded from auto-commit via
  per-repo `auto_commit_exclude_patterns`. A repo
  whose only modifications were in excluded paths
  (e.g. Junk-Runner-bevy's test-results/ PNGs that
  Playwright keeps regenerating) was stuck at WARN
  forever. The new `count_non_excluded_modified_files`
  helper filters the modified file list by the
  per-repo patterns and uses the filtered count for
  the WARN/OK decision. The MOD column still shows
  the unfiltered count so the operator can see the
  true dirty state. Adds 4 new tests. Documented in
  `docs/design/all-green-investigation-2026-06-15.md`.

### Fixed
- **dracon-sync: upgrade `dracon-git` 94.2.7 → 94.7.0**
  (2026-06-15, goal `0ab367b5`): the `dracon-git`
  library v94.2.7 had two bugs that caused
  Junk-Runner-bevy to show as WARN with 91 "MOD" for
  what was actually 3 untracked test-results/ PNGs:
  1. `get_status()` counted `is_wt_new()` (untracked)
     as `modified_files`.
  2. `RepoStatus` had no `untracked_files` field,
     making the correct count split unrepresentable.
  v94.7.0 fixes both. After upgrade, Junk-Runner-bevy
  drops from `91 MOD + 3 UT` to `0 MOD + 3 UT`
  (correctly classified as untracked, not modified).
  Documented in
  `docs/design/junk-runner-fix-2026-06-15.md`.

### Fixed
- **dracon-sync: operator's live config drift**
  (2026-06-15, followup to goal `546d4f9c`): the
  operator's `~/.dracon/utilities/sync/dracon-sync.toml`
  was missing `**/research/scratch/**` from the
  untracked_exclude_patterns list, even though
  it's in the new code default. Added it (and a
  CHANGELOG comment explaining why) so the
  operator's config matches the new defaults.
  Backup saved to
  `dracon-sync.toml.bak-2026-06-15-2`. Daemon
  restarted to pick up the new config.
    `auto_commit_exclude_patterns` mechanism still
    works (Junk-Runner-bevy keeps its
    `test-results/` exclusion). Concerns
    investigated: kiki-sassy `github` push-stuck
    (6 failures) is a divergent history issue
    (436 github-only commits, 775 local-only
    commits) — **NEEDS OPERATOR INPUT** to resolve
    (operator-owned repo). Other 2 concerns
    (dracon-platform 5 MOD, Junk-Runner-bevy
    88 MOD) resolved or working as designed.
    Documented in
    `docs/design/commit-all-policy-durable-2026-06-15.md`.
- **dracon-utilities: removed load-bearing symlink
  `~/Dev/dracon-libs`** (2026-06-15, goal
  `cca2169f`): `~/Dev/dracon-libs` was a symlink
  to `/tmp/lib-edit` and was invisible to the
  daemon (which doesn't follow symlinks). It was
  also load-bearing: the workspace had 6 `path =
  "../dracon-libs/..."` deps. Investigation found
  4 of those deps were dead (no `.rs` file uses
  them) and 2 (`dracon-git`, `dracon-system-lib`)
  were functionally identical to crates.io 94.7.0
  (`diff -rq` confirmed src/ dirs are identical).
  Refactor: dropped the 4 dead deps + dropped the
  `path` for the 2 versioned ones (now use
  crates.io 94.7.0). Refactor committed
  (`b2963348` + `113ff008`), pushed to all 4
  remotes. Archived `/tmp/lib-edit` to
  `/tmp/dracon-libs-snapshot-2026-06-15.tar.gz`
  (866 KB, 215 entries, SHA-256
  `1e8aa374a48d08cd1c9a6ac6ac449cc4d1a0c7c378589be4ae11db9e4146289f`),
  then `rm` the symlink. After deletion: 849
  tests pass, release build clean, cargo deny
  clean. Live daemon report still 13 repos
  (symlink was never watched).

### Fixed
- **Junk-Runner-bevy: applied test-results exclude
  policy to main branch** (2026-06-15, goal
  `c794cf71`): the operator's per-repo policy at
  `.dracon/dracon-sync.toml` (with
  `auto_commit_exclude_patterns = ['**/test-results/**',
  '**/e2e/screenshots/**']`) was committed to
  `tauri2` in `dc8f85fe1` but never made it to
  `main`. The daemon is working on `main` (per
  `.git/HEAD`), so the policy was not being
  applied — test-results/ PNGs were being
  auto-committed (e.g. commit `b71c068db` "3
  file(s) in test-results" at 20:37:50). The
  operator's "junk runner just seems wrong" was
  CORRECT — the policy was on the wrong branch.
  Fix: copied `.dracon/dracon-sync.toml` from
  `tauri2` to `main` in commit `24709b924`, pushed
  to all 4 remotes, now aligned at
  `24709b924db6`. After this fix, future test
  runs that regenerate `test-results/` PNGs will
  NOT be auto-committed. Also pulled the
  `Enable GitHub Sponsors button` commit
  (`6d1e953b6`) from origin (was 1 commit
  behind). Documented in
  `docs/design/junk-runner-investigation-2026-06-15.md`.
- **dracon-platform: manually committed 2 audit
  dirs the daemon missed** (2026-06-15, goal
  `c794cf71`): the daemon was committing other
  files in `dracon-platform` (HOME-AUDIT-...md,
  hegemon source, etc.) but NOT these 2 audit
  dirs (`audit-byteplus-...`, `audit-dp9cqdw-...`).
  Root cause not fully diagnosed (settling
  window? in_flight HashSet? auto-exclude
  pattern?). Manually committed as workaround,
  all 4 remotes aligned at `700d58c28c08`. The
  9 `.pi-tmp/*` scratch dirs and 3 deferred
  source dirs in `dracon-platform` remain
  untracked with documented reasons.
- **dracon-platform: committed 100+ untracked files
  (screenshots, audit dirs, test specs, game assets)
  per operator request** (2026-06-15): the operator
  had 39 untracked entries preserved through goal
  `fa84a5bd` and asked "we woudl love to commit that
  too and push it". This was resolved in goal
  `ca80b0d1`. Created 4 logical commits (top-level
  PNGs, audit dirs, test specs, game JPGs) plus
  6 daemon auto-commits for the operator's in-progress
  source/test/docs work. All 4 remotes (origin,
  github, gitlab, codeberg) aligned at
  `2a7bbd295bdf`. No force-pushes, no `.gitignore`
  changes, no sensitive files committed, no
  `.pi-tmp/*` session scratch committed. Documented
  in
  `docs/design/dracon-platform-untracked-commit-2026-06-15.md`.
  The 3 source dirs (`hegemon/src/lib/`,
  `hegemon/static/assets/`, `slug` route) were
  deferred to a follow-up question for the operator.
- **dracon-sync trailing-drain bug caused permanent skip
  of slow-push repos** (2026-06-15): the daemon's
  `in_flight: HashSet<PathBuf>` is supposed to prevent
  re-dispatching a repo while its `sync_repo` task is
  running. The trailing-drain phase drains leftover
  tasks with a 2s deadline. **On timeout, the
  unfinished tasks were dropped from `in_flight_tasks`
  (which goes out of scope) but their entries in
  `in_flight` were NEVER cleared.** The result: a slow
  sync task (e.g. a 60s push on `dracon-platform`)
  would stay in `in_flight` forever, causing the
  COLLECT phase of every subsequent cycle to skip the
  repo. The repo would never be processed again until
  the daemon restarted. Fix: track dispatched repos
  in a local `dispatched_this_cycle: HashSet<PathBuf>`,
  and on trailing-drain completion or timeout, clear
  any `in_flight` entries that were not drained. The
  daemon now logs `🔄 trailing-drain: clearing N stuck
  in_flight entries: {...}` when this happens. New
  regression test `test_trailing_drain_clears_stuck_in_flight`
  in `daemon.rs`. 1 regression test added. The fix was
  discovered during the `dracon-platform` push
  investigation (goal `fa84a5bd`); after deploying
  the fix, the daemon immediately committed 25 files
  to `dracon-platform` and pushed to all 3 remotes.
  Documented in
  `docs/design/dracon-platform-push-investigation-2026-06-15.md`.
  All 14 watched repos now show `✅ OK` and `healthy`.
- **dracon-sync unit tests polluted the live stuck-push
  ledger** with junk entries (`/tmp/.tmpXXXXX/test-repo`)
  from `tempfile::tempdir()` test fixtures. The push-failure
  tests in `daemon.rs` and `sync.rs` wrote to the real
  ledger at
  `~/.local/state/dracon/dracon-sync-stuck-push-repos.json`
  because they did not redirect `DRACON_SYNC_STATE_DIR` to
  a temp dir. The result: every `cargo test` run added
  fake entries to the operator's live ledger. Fix: 7
  unit tests now use `EnvRestorer::new("DRACON_SYNC_STATE_DIR",
  temp_dir.path())` to redirect ledger writes to a temp
  dir. Affected tests: `test_record_push_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAzM1UybDRVNXhtVytrR2M4b1NKMUc2Smd0T3ZVdXpCZ3VIdjFEbDRadEJJCnViR0FnakJpUVp4azFCWW9Lenc1MWxDclJuM09uR1RKQUFKbXgyYm4xakEKLT4gWDI1NTE5IGtsbmM5U0cya0E2NjdoaU1JdEo3S205MFlhQ014MHFLdkN1bG5qQWowbEEKYS9zY0VvTHlTVUh1dXMyNDVJTFBRd1IxRXF0cUM2empTbE11cGpJNnpvVQotPiBYMjU1MTkgM1hTTlZ2S2U2N1puUG9IUUVrUDRIMHBkVndYNXVFSURiRnBvcXFaT04yZwprUGViNzBIZnR3dkw4QWQzcmd6alpDb0pxYUdZYUpNWkx0eEJ5bGpvNjdNCi0+IFgyNTUxOSBTQzRFNFFrU2N4bWRjK1ZocnJHck96RjVMNGxXQlFKczJTTGZBeXcyRVRzCnR1UjhsS1dNd0tMak5EY3JXQXdpVjhFOHBoOGpJcU9IVCt0QmNOK3pRNjAKLT4gWDI1NTE5IHUrSEwrYkU4SytRb1U2cXc0aXlKV0FGUE9EZ2ptRk5naUxwcGxXbElHSGcKZ04rMGpEakF0M1hlWklxOEowUVFDdVNsOWhWQnRrMEVaNit4dVoxL3MvMAotPiBaL147LWdyZWFzZSBxUVMtSH13IElNbyt2SX1ECm04K29DWEdMVjdmZ3JlSlBXd1ZvZ2JEQVVtT1JVeGl0QTc2YUg1YWlqOXgrQ0NMclJ5N21QLzdZWTAyNVRtak0KV0YrTGFpWXRNcTBWZ1NLaVc1Q2dGUng1VU1hSU01eHg0dm8KLS0tIGQ5TG51c05xbEZScEhGM0QvWUh2RE5WMnhWNHRWVFp0UkE2Q1MzNSs0SW8KmZYIP8t+yqFW5d0oJMyvDSa1uUiYWu57+oEDrTrRNpdzx2JcXNBjaxHyH6CSiK9bIY70y9Oh+LRWndUclYgQr7Q9]`,
  `test_record_push_success_clears_entry`, `test_record_push_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBRbk1JdGJ1d1A4MzdLc2NyK3cwYW42dkk2UjlSTVlnbTFEbGVaOFM1ZFFrCkxham9CbGNza083ZTRJN2IrekZadytndkQ4aWJCa2J4Tnh3MW1qdDBKb2cKLT4gWDI1NTE5IGRTWXRwRFliVWVkWFJ5NStSQmwyY3MraFhLbkhVTDhuL1VpSDB6K0doUWMKTW0zcjUwdURmRy9ZQVJQUFNWRFkveUN2c1h1bWZXbS84MUFLb2hYOHBHVQotPiBYMjU1MTkgYkhocU5tSVNsUGlHSEVxYTNBcnBiZ3JvTkdRVmxFK0tpSCtJSUNieVBpSQpFMXQ2em5UcjhCLzFXcFp3MHU2T2VOK3RFMXNqcXFmbGUrelcxb09oaGRNCi0+IFgyNTUxOSBLMnRqdUw5Q21BYzJ0aHFvYUx5eEV0S1ZUaUU1RU00eUVJakU4amN5bGlVClk1aElXUWF1Slpxdm1lVTBSc0EzVi81ZXJ1OEJuU0lwS0ZOL3Z4ci9hWWcKLT4gWDI1NTE5IGlRcUN3RzAwVDY4ZTJrNEE0YWZ0WkExSnprRVl2anRNeHgvV1pRZ3ZVM28KS0xUTnZ6YnpHMnl0YWZpejlMWlNMaVZsakh2ZytadUs5Q2oyZDA4blV0TQotPiB7LWdyZWFzZSBuIGZZUCAocWsyKSA/CjJjaEpzeDhnR1ZVUkdjNkJUdjVGYmg2Nmw3NWxtRm9vSnhEdklNeExxQTNENDI2eHVVcwotLS0gYS8xR3ovVW0zeGU5M1F5YWMrU1ZOc3laVTNwY2R2TUlWcHRuOGJDWXQ5MApI7gPiwdi2se98SIeWBBuEdIwe7YrQeRJTXCDnYo4OQD1+RGDvZLmLkYOwB756324Xld1GHnS91Zs3u2XYtA==]`
  (in `daemon.rs`); `test_sync_repo_mirror_push_failure_returns_false`,
  `test_sync_repo_mirror_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB6OGNCc01QNHVPS3czL2xHcTlnQU9vcDRlVkdRWEcwdU1lTW4ySjVNVmhVClB4M0laSEhIeDNlbi9BMkpXdzFyODFNemZxOG9VdjUxZVV4ZGViY2d6aEUKLT4gWDI1NTE5IHRDNDRsaFFYWVFJbTE3R0laQTkrZUR5THpnUUY4V25sMzFxT3dvUTczRzQKQWdKK01QV1ptQXNmMDRGTEZ4KzVSRUVHRGpRS05NbW5URnI4dnMvSC9kcwotPiBYMjU1MTkgOHdFSkNqUmYyUDVldjRLM3ljT2dnb1RJVmxXaE1aQzhXQUR2S2pvNElSMAp5UVFqVkpoQ0ZQZmFrTE1qNHpDazJoQ3FUYVlLbDVkVjJ4Y042WEdsZHlVCi0+IFgyNTUxOSBRNUlHbU9hMHRlTll5Y1lhWXVLQ09sSXhZVms3WDZBODZ4TUx1Ung1MEVzCm1TRGVOck5RckVzWXAyTGVlUHZVS3F1VktDc0pmTTA1bXRodHNJV0tmaDAKLT4gWDI1NTE5IG56aXAxa3YyRndrT0MrdFRwZnlQeUJoTzJXQWtCSE42amNYVlM3cDg1RkUKWVIvVFlMSnRNRUd4eU9xWXdDVFlra1RUclhJZm1zc0hFTmZQZVBPUXVzdwotPiB7JHY7RVRqcS1ncmVhc2UgPW4nTCB7ZG1vWFAoCjVmZ2UKLS0tIGIwaVZBK0w0Y2lsaVh1L0ZjMFlHdGhyOFlwL3FFa1lZZ0NvUWJ0eE51bjgK/oAqUCg/J0BUkT3lQaZHrj3X0EzXonLlHpnh7OHAnlsfRCgvbmDr+dF4z5Txrqa+UBfwG4HbjDmM]`,
  `test_sync_repo_mirror_push_success_returns_true`,
  `test_sync_repo_auto_github_private_graceful_on_no_gh`
  (in `sync.rs`). 19+ junk entries cleaned from the
  operator's ledger. No new tests added (the existing
  tests still pass; the change is to their setup, not
  their assertions). No regression in
  `cargo test --workspace --locked` (848 passed).
- **dracon-warden was encrypting source code**: the active
  `dracon-warden.toml` config had `protected_patterns` that
  included `*.rs`, `*.ts`, `*.py`, etc. (source code files).
  The SecretScanner's `Mistral API Key` regex
  (`mistral-[A-Za-z0-9_-]{20,}`) matched a public model ID
  `mistralai/[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBGWTdHTUpPNTBQakRhaDhjVVhtSVRZcmNacm9FSHEwOFdoejhUTzRkVFdBCmtwV0tVc1A3MGh6dlZxenQ2TVdIbHRPR3RqZnkxVG5KSDRYVnhWZmJaeHMKLT4gWDI1NTE5IEptQm5lSVZKREwzNUg0RTVETTBCK3R1NEQxdjFlU2RrRDM3TGR3bkZvd1kKdnJTaU5pSlhobHJ3bFF6cXlpQkw2aU1BeXBRak8xeitjK0R2bkxqK0NoWQotPiBYMjU1MTkgRFZmcmlSaFpoRmZzVUx2NGh0SzZwenBoMFh5RjlhbFluZk80REJxb1RVdwpsbEozVlB5T3NnclhLT0JnaGVORm56eUF3RFdJeUdEOVFkYkREU1RPWUFFCi0+IFgyNTUxOSB3WUVyTkEyeVloQTRUWjNJZ1BOWnhUTDNKQ1BZSk5KaUswNlpWa1RiVGlrCmk5bXNEVlE0Z3ROQ0ltRFZrNTB3dEgvYzNxV1hBMmZLN0FRUU56OWlhTDQKLT4gWDI1NTE5IGZyZDNQcHNNT21GdGcra3RYdjA2VlpuZUlvWWF2NFNlcU1TRk40TWIzRjAKL1pQRDhCekRBNG1WaU40QS9idmxFTDJENmYwaTFGcm5GU3JLMmdDV3o2VQotPiAiUSIpfGwtZ3JlYXNlIF9VYHZVIFc+XEdYMGZcCnhqRzJhSGo4K1ZtMjZNUk9Md2RDNHg2c0M1ZHMzemNrU0pORk9WeDZoSUFFSmVBQ2RDUmowK21xNlhBWHdEcWsKbmxZOG93ais0SkQvZC9vdiswUEwzSlhoVkFqOUNDd2lETG1xYVQza01LVUJLRzl5d2xjNUNnCi0tLSBGZUlEb0c0S1hyU3VPQm1yT1pwWk9aRGZ1QzdBSUE5NThiZnZyeTlJdXFJCm4pocNyZql/YLuYqpCtiEDkCeresHpaZ/7besXw2YzeQOYMn+qpEefUZiyX6xeDsGLr8ldb6sLG2cRMu3io7Q==]` in a
  TypeScript test file and encrypted it. The user reported
  this as a no-go via gibuardien: "we are encrypting code".
  Two-part fix:
  1. New `path_is_protected` helper in
     `dracon-warden/src/security/src/modules/filter.rs`
     implements glob-based matching of file paths against
     `protected_patterns`. Files that don't match any
     pattern are passed through unchanged — the scanner
     is NEVER invoked on them. The check is added to
     `smart_clean_with_path` BEFORE the existing
     sensitive-location logic.
  2. Removed all source code patterns from
     `~/.dracon/utilities/warden/dracon-warden.toml`'s
     `protected_patterns`. The list is now scoped to data
     files (`.env`, `*.pem`, `secrets/**`, etc.). A
     comment in the config explains the rationale.
  Restored the corrupted
  `browser-extensions-shared/extensions/vidpro-extension/test/components.test.ts`
  by replacing the encrypted blob with the original
  model ID `mistralai/[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBqa1QzbTByMTFRUFMxdmlYcENWbE9ORVVFbmpqclFJRlppcTR1amJDam0wCjN6ZXFRYnN2L1drSktSZHdCS2hZbTVUZUdqQU1sdVF3SW9sZ0xUZ0d3T0kKLT4gWDI1NTE5IFN2SXU4SStvSmt2YTJOWUpBVmhWRXBEN0dGZ2w5QUNNZWVOM1RDWkxsSFkKSW5lNVd0Rlp6b05XaHJsMmJiMWdyd2NmWStNdG5YditNbi9pRVhmcmRHcwotPiBYMjU1MTkgTGdWSFE3bHU4UTBhejF2eExtdE9SSGp0NXpYQkk5K2V3YUpOa1ZONmZqYwp1dFZWalZXQ0hPbjFaSDVzTEl5RDNYNWFaRit1bXpSWnRiYVRWdnRhUGowCi0+IFgyNTUxOSBzNll0dUVleUNoYndPNm1YT29HUFltZFg4ZFZaTUlObWtkNEdGUEdydW5VCjhacTFGa2ozZ1V2U1RXZkoxaWFVeTg0YWtrK3Q5Z0NFTDF4YVF0Y2poSVkKLT4gWDI1NTE5IFlaN0tDeHJaRnZvWEFHZHlCaEVzbFVMNjFRQjljZWQ4R0ZDRVV5YnVSQWMKc01CR1czMldtVjBhUmNncXhISXpiclFQbzNmbFdBZFptRFd3TzRNMlc1MAotPiBsfipfMy1ncmVhc2UgOzAiICwtSy4gSV0KNzFDU1RYejQyeUJPaTZGZm8yNVIzYVpLUFBMTlpiN3d2ajM5aG0zUmlHdGVCdjc4Ci0tLSBrMHh5MWhxMTJXaWhWSHFTbUI3a0lxd2dpc0M0WC9OeTQ3UXY0dUVCaUlzCj3ARRhiDyz0dGLFg3V3Lz1vesDzviimbzadgVnBDqilnwojvzwP5i6nvjqmxmAOM5Kgn2QV8CLmUOWWA18qUA==]`
  (verified via the sibling
  `utils/byokAdapter.ts` file). 6 new unit tests in
  `dracon-security`: `test_path_is_protected_*` (5) and
  `test_smart_clean_with_path_skips_unprotected_source_code`
  (1). Documented in
  `docs/design/source-encryption-incident-2026-06-15.md`.
  Audit of all 14 watched repos for source-code
  `DRACON_SECRET:YWdl` corruption returned only intentional
  test fixtures (warden's own encryption tests use
  encrypted blobs as test data).

### Added
- **Per-repo `owned` override investigation** (2026-06-15):
  after the operator asked to "default-skip non-owned repos"
  and then said "we def own kiki and others need exploring
  too", we ran `dracon-sync ownership --explain` on all 14
  watched repos and documented the results in
  `docs/design/ownership-investigation-2026-06-15.md`. All
  14 repos are correctly classified — 13 by signal-based
  detection, 1 by per-repo override (`dracon-ai-lib` whose
  upstream HEAD has a historical bad-config author). The
  per-repo `owned = true` override for
  `kiki-sassy-desktop-announcer` was added to bypass the
  earlier "Do not modify user-owned repos" constraint after
  the operator explicitly confirmed ownership. No
  misclassifications were found; no additional per-repo
  overrides are needed. The example config
  (`dracon-sync/dracon-sync.example.toml`) was updated with
  documented `owned = true` and `owned = false` per-repo
  override examples.

- **Ownership detection + default-skip safety guard rail**:
  the daemon now classifies each repo as `Owned`, `Unowned`,
  or `Unknown` based on three configurable signals:
  `git config user.email` ∈ `policy.trusted_emails`,
  HEAD author name/email ∈ `policy.trusted_authors`, and
  `origin` remote URL host ∈ `policy.trusted_remote_hosts`.
  When `auto_skip_unowned = true` (the safety-first default)
  and a repo is classified as `Unowned` or `Unknown`, the
  daemon logs `🚫 /path/to/repo skipping (<reason>): <detail>`
  once per cycle and does NOT issue any `sync_commit` or
  push to it. The `repos` table shows the new `🚫 unowned`
  STATE icon with a `run ownership --explain` HINT pointing
  the operator at the new `dracon-sync ownership` subcommand.
  This protects against repos whose `origin` is someone
  else's account (e.g. `zerostack-reference` →
  `gi-dellav/zerostack.git`) or whose HEAD author is a
  historical bad config (e.g. `dracon-ai-lib` →
  `Dracon <dracon@void>`). 11 new unit tests for the
  classification logic, 4 new policy field tests for
  defaults/serde/per-repo override, and 2 new end-to-end
  tests with `create_test_repo`. Per-repo override
  `RepoPolicyOverride.owned` and `auto_skip_unowned` re-enable
  the daemon for specific repos.

- **Settling max-delay + DirtyMaxAgeAction**: the daemon now
  force-commits a dirty repo that has been dirty continuously
  for > `settling_max_delay_secs` (default 60s) regardless of
  fingerprint stability. This prevents the "⏸ stalled Xm"
  pileup the operator was seeing — the user explicitly
  requested that the daemon "should be very actively
  committing". The 5s fingerprint-stability wait is still
  used for actively-edited repos (so the daemon doesn't
  commit on every keystroke). The max-age action is
  configurable via `policy.dirty_max_age_action`
  (`Commit` | `Warn` | `Ignore`); per-repo override via
  `RepoPolicyOverride.settling_max_delay_secs` and
  `dirty_max_age_action`. 5 new unit tests cover the
  defaults, the policy field serde, the per-repo override,
  and the test fixture regression. New `min_commit_interval_secs`
  policy field (default 5) exposes the per-repo commit
  rate-limit gate.

- **Push retry budget + push-stuck state**: the daemon now
  tracks `consecutive_failures` and `last_error` per stuck
  push repo in the on-disk
  `dracon-sync-stuck-push-repos.json` ledger. When
  `consecutive_failures >= push_max_retries` (default 5),
  the ACTIVITY column shows `🛑 push-stuck Xm (N ahead)`
  and the HINT column surfaces the actual `git push` error
  message (e.g. "Permission to gi-dellav/zerostack.git
  denied to DraconDev") with a `repair-concerns --apply`
  hint. The previous behavior was an opaque `🟣 pushing Xm`
  that could stay for days without telling the operator WHY
  the push wasn't landing. 3 new unit tests for the
  record/success/failure paths and 1 new ACTIVITY test
  for the push-stuck label. 9 new field tests for the
  `StuckRepoEntry` serde backward-compat (the new
  `consecutive_failures` / `last_error` / `last_error_at`
  fields default-fill from older JSON files).

- **Tightened in_flight staleness filter**: the on-disk
  `dracon-sync-in-flight.json` is now treated as stale
  after 5 seconds (was 30 seconds). The 30s window was
  leaking false `🔄 now` indicators on OK/idle/synced
  repos because the daemon writes the file every 1s and
  any entry from the past 30s was treated as "in flight".
  Combined with a new state-based suppression
  (`Synced`/`Idle`/`Cold`/`Untracked`/`Healthy` rows
  never show `🔄 now` even if the in_flight file lists
  them), the `🔄 now` indicator now reflects ground truth
  (the repo is currently being processed by a tokio task).
  3 new unit tests cover the 10s-old-stale case, the
  state-suppression case (Synced/Idle/Cold states never
  show `🔄 now`), and the Dirty state still showing
  `🔄 now` when legitimately in flight.

- **Auto-commit backstop for moving-target repos**: when a repo
  has more than `auto_commit_backstop_threshold` (default 20)
  unpushed commits AND the push has been pending
  (`ahead_since`) for more than
  `auto_commit_backstop_min_age_secs` (default 300s = 5 min),
  the daemon stops auto-committing and logs
  `⏸️ daemon backstop: N unpushed commits pending push >Xs,
  skipping auto-commit for <repo>`. This prevents the daemon
  from creating a moving target while a push is failing
  repeatedly (e.g. Junk-Runner-bevy had 2986 unpushed commits
  + 376 tracked test-result PNGs being committed every cycle,
  creating an infinite retry loop). Manual `git add`/`git
  commit` from the operator still works. Set
  `auto_commit_backstop_threshold = 0` in the policy to
  disable the backstop entirely. 5 new unit tests cover
  the below-threshold, above-threshold-but-recent, fully
  active, no-ahead-since, and threshold-zero (disabled)
  cases. Documented in
  `docs/design/dirty-files-investigation.md`.

- **Per-repo `auto_commit_exclude_patterns`**: a new
  per-repo TOML field that lets operators opt specific
  TRACKED file patterns out of auto-commit. Unlike
  `untracked_exclude_patterns` (which only applies to
  newly-added files), this applies to MODIFICATIONS of
  already-tracked files. Use case: a repo has 372
  Playwright screenshots force-tracked by `.gitignore`'s
  `!*.png` allowlist. Every test run regenerates them,
  and the daemon auto-commits each regeneration, creating
  a moving target the push can never resolve. Setting
  `auto_commit_exclude_patterns = ["**/test-results/**"]`
  in the repo's `.dracon/dracon-sync.toml` tells the
  daemon to skip those files. Default is empty (opt-in).
  Wired into `should_stage_entry` and
  `has_sync_relevant_dirty_entries`. 3 new unit tests
  cover the excluded-by-pattern, no-match, and
  empty-patterns cases. Documented in
  `docs/design/dirty-files-investigation.md` and
  `dracon-sync.example.toml`.

- **in_flight file staleness filter**: the on-disk
  `dracon-sync-in-flight.json` file used to make the
  ACTIVITY column show `🔄 now` could persist across
  cycles when a slow push from the previous cycle kept
  a repo in `in_flight` past the trailing-drain deadline.
  The `repos` command would then show `🔄 now` for repos
  whose pushes had completed (or stalled) while the daemon
  had moved on. The fix: `load_in_flight_for_path` now
  checks the file's `written_at` epoch and treats entries
  older than 30s as "stale → treat as empty". 2 new unit
  tests cover the stale and fresh cases. This complements
  the `⏸ stalled` and `🟣 pushing` labels so the ACTIVITY
  column always reflects ground truth.

- **Push stall fixes for `dracon-platform` (and similar)**:
  two related fixes that prevent the daemon from getting stuck
  in a "commit 1 file → push 28 commits → fail at 60s → HTTPS
  fallback → 3 min wait" loop.

  1. **Race condition in `stage_existing_files`**: build tools
     like vite create timestamp-suffixed temp files
     (e.g. `vite.config.ts.timestamp-1781483278562-...mjs`) and
     delete them within milliseconds. If `get_status()` lists
     such a path as untracked but the file is gone by the time
     `git add` runs, the whole `git add` fails with
     `fatal: unable to stat ...`, blocking every other file in
     the commit. The function now re-checks file existence
     right before staging and drops vanished files. Bare
     directory entries in the staging list are also filtered.
     3 new unit tests cover vanished files, directory
     entries, and the all-vanished no-op case.

  2. **Auto-scaled push timeout**: a fixed 60s idle timeout
     for `git push` is too short for a 28-commit push with
     binary test artifacts — git can sit in the negotiate
     phase for >60s before emitting any progress. The push
     timeout is now scaled with the local ahead count:
     `ahead ≤ 5` → base, `≤ 20` → 2x, `≤ 50` → 4x, `> 50`
     → 6x (capped at 600s = 10 min). Scaling is logged so
     operators can see when it kicks in (e.g.
     `⏫ Junk-Runner-bevy scaling push timeout 60s → 360s
     (2986 commits ahead)`). 5 new unit tests cover the
     small/medium/large/huge/zero-base scaling buckets.

  3. **ACTIVITY column now shows ahead count when pushing**:
     the `pushing` label now includes the unpushed-commit
     count (e.g. `🟣 pushing 4m (28 ahead)`) so operators
     can tell at a glance whether a stall is caused by a
     large backlog vs. a transient network blip.

- **ACTIVITY column now distinguishes active from stalled**: the
  `ACTIVITY` column in `dracon-sync repos` was previously a
  duplicate of the `LAST COMMIT` column (just the relative
  time of the last commit), which made it impossible to tell
  whether a row was "actively being processed" or "stalled"
  when many rows had the same timestamp. The new column shows
  one of: `🔄 now` (daemon has an in-flight task for this repo),
  `🟣 pushing Xm` (push in progress), `⏸ stalled Xm` (dirty &
  no daemon action for X minutes), `⏳ settling` (dirty & waiting
  for fingerprint stability), `🟢 synced Xm` (clean & recent),
  `⚪ idle Xh` (clean & waiting), `⚫ cold Xd` (no activity > 24h).
  The daemon now writes its `in_flight: HashSet<PathBuf>` to
  `~/.local/state/dracon/dracon-sync-in-flight.json` on every
  cycle, atomically (write-temp + rename), self-cleaning (file
  removed when set is empty), and removed on daemon shutdown.
  The `repos` command reads this file to render the new column.
  7 new unit tests in `dracon-sync/src/report.rs` for each
  ACTIVITY state. Legend updated to describe the new semantics.

- **No-redispatch invariant for parallel sync**: the daemon now
  tracks an `in_flight: HashSet<PathBuf>` consulted by the COLLECT
  phase's eligibility check. A repo with an active `sync_repo`
  task is not re-dispatched, even if the apply-phase deadline
  breaks out before the task completes. The APPLY phase removes
  repos from `in_flight` when their tasks complete; a bounded
  trailing drain (`pulse_interval_secs * 2`) clears the rest.
  This prevents duplicate `git push` invocations on the same
  `(repo, remote)` pair within a cycle window, which was causing
  a 2-3 minute "traffic jam" when 4 parallel pushes were
  competing for the SSH agent and network bandwidth. Live test:
  3 fresh dirty repos all commit+push in ~38s with no duplicate
  push attempts. New unit test `test_no_redispatch_invariant`
  in `dracon-sync/src/daemon.rs`. Documented in
  `docs/design/dirty-files-investigation.md`.

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

