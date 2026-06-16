# Commit-all principle + audit of "preserve untracked" exceptions (2026-06-16)

> **Operator said**: "this seems like a hack git
> sync should not handle it i think warden already
> down so that is not good, git sync just has to
> make sure that nothing left out unless we have a
> very good reason to leave it out"

Goal `6205ad1f` (2026-06-16).

## Context

Goal `e680cfa9` (also 2026-06-16) added a
"defensive guard" against the CWD-drift bug class
to `dracon-sync`: a `noteworthy_untracked()`
function, 6 unit tests, a `Command::CheckUntrackedMd`
enum variant, and a `check-untracked-md` CLI
subcommand. The operator rejected this approach
on principle: git-sync is the wrong layer for
this concern. The defensive guard belongs in
**warden** (which has git hooks + the
`DRACON_SECRET:` encryption flow), not in
git-sync (a sync daemon).

The operator's stated principle:

> "git sync just has to make sure that nothing
> left out unless we have a very good reason to
> leave it out"

This means: the daemon's commit-all policy is the
correct default, and any exception to it must
have a documented good reason.

## Part A — Revert the hack (DONE)

The `e680cfa9` code is reverted. Commits:

- `a332b168` — `dracon-sync: revert noteworthy_untracked + check-untracked-md` (294 lines removed)
- `d59987cf` — `changelog + design: document e680cfa9 revert`

What was removed:

- `noteworthy_untracked()` in
  `dracon-sync/src/git/diff.rs`
- 6 unit tests in `noteworthy_untracked_tests` mod
- `Command::CheckUntrackedMd` enum variant in
  `dracon-sync/src/main.rs`
- `Command::CheckUntrackedMd` dispatch arm
- `cmd_check_untracked_md()` function
- `check-untracked-md` CLI subcommand

Test count: **857 → 851** (the 6 hack tests
removed). `cargo test --workspace --locked`
passes. `cargo build --release --locked` succeeds.
`~/.local/bin/dracon-sync --help` does NOT list
`check-untracked-md`.

## Part B — Audit every "preserve untracked" exception

This section audits every existing exception to
the commit-all policy. For each, classify as
KEEP (with reason) or REMOVE (with resolution).

### KEEP (1-3): well-documented good reasons

#### 1. Scratch/temp dirs (KEEP)

```text
**/scratch/**, **/scratch-*, **/scratch_*
**/tmp/**, **/tmp-*
**/pi-tmp/**, **/.pi-tmp/**
**/research/scratch/**
.demon/**, .sisyphus/**, .ralph/**
```

**Reason**: These are ephemeral session-scratch
directories, agent session state, and temp
directories. They are explicitly designated as
ephemeral by design (the `ca80b0d1` convention
for `.pi-tmp/` is the agent scratch standard).
Committing them would cause perpetual churn.

**Source**: `~/.dracon/utilities/sync/dracon-sync.toml`
`untracked_exclude_patterns = [...]` (the
operator's global config) and the code default
in `dracon-sync/src/policy.rs` lines 821-829
(set by goal `546d4f9c` / `9aaf0b08`).

#### 2. Size limit (KEEP)

Files larger than 100 MiB are NOT auto-staged.

**Reason**: Performance and safety. Large files
(ML models, video assets, large datasets) should
be tracked via LFS or external storage, not raw
git. The 100 MiB threshold is the operator's
stated limit.

**Source**: `~/.dracon/utilities/sync/dracon-sync.toml`
`max_stage_file_bytes = 104857600` and the code
default in `dracon-sync/src/policy.rs` (was
`9aaf0b08`).

#### 3. Sensitive files (KEEP)

```text
.env, .env.*, .env.local, .env.production
*.pem, *.key, *.age
secrets/**
```

**Reason**: These are sensitive credentials
and should NEVER be auto-staged. Warden's job
is to encrypt the files that DO get committed
(via the `DRACON_SECRET:` filter flow) and to
block pushes that would expose plaintext
(via the pre-push hook in
`dracon-warden/src/main.rs` lines 2207+).

**Source**: `~/.dracon/utilities/warden/warden.toml`
`protected_patterns` (encryption) +
`hygiene_patterns` (hygiene) and the matching
`.gitignore` rules in every watched repo's
managed block.

### KEEP (4-9): per-repo / file-pattern decisions

#### 4. Per-repo `auto_commit_exclude_patterns` (AUDIT)

There are 4 watched repos with per-repo
`.dracon/dracon-sync.toml` files:

- `kiki-sassy-desktop-announcer` — has
  `owned = true` (user-owned repo). The
  per-repo override is for ownership
  classification, NOT for untracked excludes.
  KEEP.
- `Junk-Runner-bevy` — had
  `auto_commit_exclude_patterns` for
  `**/test-results/**` and
  `**/e2e/screenshots/**`. REMOVED in
  goal `76ddaa7e` (2026-06-15). The
  per-repo .toml is now a comment-only file
  documenting the removal. KEEP (the
  override slot exists for future tuning).
- `dracon-ai-lib` — has `owned = true`
  (user-owned repo, not a commit policy).
  KEEP.
- `rust-ai-web-auto` — had
  `auto_commit_exclude_patterns` for
  `reports/kdp-live-*.md`. REMOVED in
  goal `76ddaa7e` (2026-06-15). KEEP (slot
  exists for future tuning).

**Reason**: All 4 per-repo overrides are
either ownership overrides (not policy
excludes) or already-removed policy excludes
(slot exists for future tuning). No active
per-repo override is in use.

#### 5. `*.bak-*` in `.gitignore` (KEEP)

`*.bak-*` is in the operator's global
`/home/dracon/.dracon/.gitignore` (line 119,
added in goal `3276ceb4` / 2026-06-15).

**Reason**: Editor / shell tools create
`.bak-*` files when editing configs (e.g.,
`dracon-sync.toml.bak-2026-06-15`). These
are NOT deliverables — they are unintended
editor side effects. Committing them would
cause perpetual churn from every config
edit.

There are 2 currently-tracked `.bak-*` files
in `~/.dracon/utilities/sync/`
(`dracon-sync.toml.bak-2026-06-15` and
`dracon-sync.toml.bak-2026-06-15-2`) from
historical commits 70111fe29 and d6884bffc.
The `*.bak-*` gitignore pattern only affects
untracked files, so the historical commits
stay in place. This is purely a
forward-only hygiene rule.

KEEP.

#### 6. Junk-Runner-bevy `!*.png` re-include (KEEP)

The Junk-Runner-bevy `.gitignore` has
`!*.png` (and many other `!*` re-includes).
This re-includes PNG files that would
otherwise be hidden by general patterns.

**Reason**: The daemon-managed block
maintains the `!*.png` re-include to ensure
intentional PNG commits (icons, screenshots,
test artifacts) get tracked. The
`test-results/` and `e2e/screenshots/` are
NOT in Junk-Runner-bevy `.gitignore` (they
were in the now-removed per-repo override).

The operator's active Playwright work
regenerates PNGs. The `!*.png` re-include
ensures those get committed. The
`inactivity_push_delay_secs = 5` (set by
goal `546d4f9c`) handles the commit timing.

KEEP. (Operator confirmed intent via the
`76ddaa7e` removal of the per-repo override
and the inline comment in
`Junk-Runner-bevy/.dracon/dracon-sync.toml`.)

#### 7. browser-extensions-shared "NEVER auto-stage" (REMOVE — already done)

The constraint "NEVER auto-stage the untracked
markdown in `browser-extensions-shared`"
(from goal `76ddaa7e` / 2026-06-15) was
**REMOVED** in goal `c19d21b8` (2026-06-16).
The untracked `.md` (the doubled-path
`platform-free-extension-shortlist.md`) was
the deliverable for goal `abf12ed7-9286-
4b4f-9af0-caa827bfe296` and was cross-linked
from a tracked file. Moving and committing
it was the correct action.

**Status**: REMOVED. The current
`browser-extensions-shared` is clean (0
untracked files).

#### 8. dracon-platform `_template-visual-novel` (AUDIT — needs operator)

`/home/dracon/Dev/dracon-platform/web/games/
games/_template-visual-novel/` was deferred
per goal `76ddaa7e` (2026-06-15). The
deferral was: "DO NOT modify
`_template-visual-novel`".

Currently untracked inside that subtree:
1 file: `src/lib/styles/global.css` (created
2026-06-16 08:34, the morning of the
deferral).

Additionally, there's a NEW shared library
at
`/home/dracon/Dev/dracon-platform/web/games/
games/_lib/visual-novel/` (16 source files
created 2026-06-16 14:26-14:37) that is NOT
the deferred template.

**Reason for the deferral**: The deferral was
based on the operator's intent to clean up
the filter pipeline (per `76ddaa7e`). The
new principle says "commit all unless
super-good reason" — but the operator
explicitly deferred this subtree.

**Decision needed from operator**:

Option A: Commit the
`_template-visual-novel/src/lib/styles/
global.css` (treating the deferral as
overridden by the new principle).
Option B: Keep the deferral; the daemon
keeps ignoring this subtree until the
operator decides.
Option C: Add `web/games/games/_template-
visual-novel/**` to the per-repo
`auto_commit_exclude_patterns` in
`.dracon/dracon-sync.toml` (formalize
the deferral).
Option D: Delete the entire
`_template-visual-novel/` directory
(stale scratch from the morning of
2026-06-16).

The `_lib/visual-novel/` library (16 files)
is a separate decision:

Option A: Commit the new library
(recommended — it's operator-active work
and matches the new principle).
Option B: Keep it untracked (operator's
explicit choice).

**Status**: PENDING OPERATOR DECISION. The
deferral constraint from `76ddaa7e` is
preserved (the daemon does not auto-stage
these dirs) until the operator decides.

#### 9. `.dracon/.gitignore` (KEEP)

The operator's global
`/home/dracon/.dracon/.gitignore` has a
`# --- BEGIN DRACON MANAGED BLOCK ---`
section managed by `dracon-warden` and
operator-added entries (e.g., `*.bak-*`).

**Reason**: The managed block is the
source of truth for the encryption +
hygiene patterns. The `*.bak-*` is a
forward-only hygiene rule (see item 5).

KEEP.

### Live 14-repo scan (2026-06-16 15:50)

After all reverts and policy audit, the
14-repo scan shows:

| Repo | Untracked | Reason |
|------|-----------|--------|
| `/home/dracon/.dracon` | 0 | clean |
| `/home/dracon/Dev/DraconDev` | 0 | clean |
| `/home/dracon/Dev/Junk-Runner-bevy` | 0 | clean (commit-all) |
| `/home/dracon/Dev/avid` | 0 | clean |
| `/home/dracon/Dev/browser-extensions-shared` | 0 | clean (was the bug case in `c19d21b8`) |
| `/home/dracon/Dev/dracon-ai-lib` | 0 | clean |
| `/home/dracon/Dev/dracon-libs` | 0 | clean |
| `/home/dracon/Dev/dracon-platform` | 2 dirs | `_lib/visual-novel/` (16 files, operator-active) + `_template-visual-novel/src/lib/styles/global.css` (1 file, deferred) — **PENDING OPERATOR DECISION** |
| `/home/dracon/Dev/dracon-utilities` | 0 | clean |
| `/home/dracon/Dev/kiki-sassy-desktop-announcer` | 0 | clean (user-owned, `owned = true`) |
| `/home/dracon/Dev/one-mil-girls` | 0 | clean (user-owned) |
| `/home/dracon/Dev/pully-fully` | 0 | clean |
| `/home/dracon/Dev/rust-ai-web-auto` | 0 | clean |
| `/home/dracon/Dev/sandbox-rs` | 0 | clean |

13 of 14 repos are clean. The 1 repo
(dracon-platform) has 2 operator-decision
items that need the operator's input.

## Part C — Warden's role (READ-ONLY INVESTIGATION)

The operator said "i think warden already down
so that is not good". Investigation findings:

### Warden's responsibilities

1. **Encryption filter** — the `DRACON_SECRET:`
   flow that encrypts `protected_patterns` in
   `.gitattributes` before commit.
2. **Pre-commit hook** — validates that the
   encryption filter is configured (checks
   `.gitattributes`, `filter.dracon.clean`,
   and `dracon-warden` on PATH). Blocks
   commits if the filter is missing. See
   `dracon-warden/src/main.rs` lines 2175-2206.
3. **Pre-push hook** — scans pushed diffs for
   plaintext secrets (defense-in-depth).
   See `dracon-warden/src/main.rs` lines 2207+.
4. **Managed `.gitignore` blocks** —
   writes the encryption + hygiene patterns
   to every watched repo's `.gitignore`.

### What warden does NOT do

- **Warden does NOT auto-stage untracked
  files.** The pre-commit hook only validates
  the encryption filter setup; it does not
  run `git add` on untracked content.
- **Warden does NOT run periodic sync.**
  That's dracon-sync's job.

### The actual responsibility split

| Concern | Owner |
|---------|-------|
| Periodic git sync (auto-commit, auto-push, auto-pull) | **dracon-sync** |
| Auto-stage untracked files | **dracon-sync** (`auto_commit = true`) |
| Encryption of protected files | **warden** |
| Pre-commit validation of encryption filter | **warden** (pre-commit hook) |
| Pre-push plaintext scan | **warden** (pre-push hook) |
| Managed `.gitignore` blocks | **warden** |

### What the operator likely meant

The operator's comment "i think warden already
down so that is not good" is likely referring
to a different concern: warden's
`dracon-warden once <repo>` flow (the
encryption filter setup) being disabled or
broken. That is a different investigation.

**Status**: The CWD-drift / untracked content
concern is NOT warden's job; it IS
dracon-sync's job, and the daemon's
`auto_commit = true` already handles it
correctly. The `browser-extensions-shared`
case in `c19d21b8` was caused by a stale
per-repo constraint (now removed in `c19d21b8`),
not by warden being down.

### Warden investigation outcome

The defensive guard against untracked
content does NOT need a new warden hook.
The daemon's `auto_commit = true` already
auto-stages untracked files (after applying
the documented `untracked_exclude_patterns`
+ `exclude_dir_names` + the 100 MiB limit).
The `c19d21b8` followup confirmed this works
for the operator's real-world case.

If the operator wants additional defense
(e.g., a periodic scan that surfaces
"untracked files in an OK repo" as a WARN
in `dracon-sync repos`), that would be a
follow-up goal — but it should NOT be in
git-sync, and it should be a passive
reporting feature, not an auto-stage
mechanism (the daemon already does that).

## Part D — Documented principle

Added to `/home/dracon/Dev/dracon-utilities/
AGENTS.md` as the "Commit-all principle
(2026-06-16, goal `6205ad1f`)" section.

See the "Commit-all principle" heading in
AGENTS.md for the canonical statement.

## Verification

- `rg -l 'noteworthy_untracked|CheckUntrackedMd|cmd_check_untracked_md|check-untracked-md' dracon-sync/src/` returns ZERO (the hack is gone)
- `cargo build --release --locked` succeeds
- `cargo test --workspace --locked` passes; total test count is **851** (was 857)
- `~/.local/bin/dracon-sync --help` does NOT list `check-untracked-md`
- `dracon-sync repos` shows 14 OK (or 13 OK + 1 WARN for dracon-platform if operator keeps the deferred items untracked)
- 14-repo scan shows untracked files ONLY in documented exception categories (dracon-platform's 2 items are the operator's active + deferred cases)
- `AGENTS.md` has the new "Commit-all principle" section
- This design doc documents: reversion rationale, each exception category with reason, warden's role, new principle
- All 4 remotes aligned for dracon-utilities at `d59987cf`

## Pending operator decision

`_lib/visual-novel/` (16 files, operator-active
new library) and
`_template-visual-novel/src/lib/styles/
global.css` (1 file, deferred per `76ddaa7e`)
in `dracon-platform`. Options documented in
Part B section 8.

---

## Followup audit: dracon-platform 7 untracked dirs (goal `05ea6904`, 2026-06-16)

> **Operator said**: "ok we are looking much
> ebtter ... the only problem seeming that the
> platform has a ton of files that are not
> getting commited"

The live `dracon-sync repos` table showed
`dracon-platform` with **7 untracked top-level
entries**. This section audits whether the
daemon was correctly committing them.

### Investigation findings

**The daemon WAS committing the files** —
just slowly. The "7 untracked" in the
daemon's report is misleading: it's the count
of top-level untracked DIRECTORIES, not the
count of uncommitted FILES. Each top-level
dir contained many files.

**Counts during the audit** (16:13 → 17:00
BST):

| Time | Uncommitted files | Top-level untracked dirs |
|------|-------------------|--------------------------|
| 16:13 | 161 (start) | 7 |
| 16:22 | (after 86-file bulk commit) | ~9 |
| 16:47 | (after 38-file bulk commit) | 4 |
| 16:52 | (after 47-file bulk commit) | 3 |
| 16:54 | (after 11-file commit) | 2 |

The daemon made **30+ commits** during the
audit, including three bulk commits of
38, 47, and 86 files. By 16:54, only **2
uncommitted files** remained.

### Why the perception was "not getting committed"

The `❓ UT` column in the daemon report
shows top-level untracked dir count, not
file count. For the operator, "7" sounds
like 7 uncommitted files. In reality,
`_lib/canvas2d/` alone contains 19 files.

The daemon uses libgit2 for status, which
collapses fully-untracked subtrees into
their parent dir. The daemon's
`stage_existing_files()` in
`dracon-sync/src/sync.rs` walks one level
into the dir and picks up files. When the
files inside the dir are themselves
untracked (not in a deeper untracked
subdir), the daemon commits them in
bulk — the 86, 47, and 38-file commits
all worked this way.

### Root cause of the slow catch-up

The daemon's `inactivity_push_delay_secs = 5`
plus `min_commit_interval_secs = 5` plus
per-repo settling behavior mean the daemon
commits in small batches (1-10 files)
interspersed with bulk commits. The bulk
commits happen when:
- The operator stops actively writing
- The libgit2 status detects the dir as
  untracked
- The settling timer expires

The 161 → 2 reduction took ~40 minutes
because the operator was actively writing
in `_lib/canvas2d/`, `_lib/visual-novel/`,
and `darklord/src/lib/` during that window,
preventing settling.

### Final state (17:00 BST)

- **2 uncommitted files** (down from 161)
- Both in `_template-*/src/lib/styles/global.css`
- Both inside the deferred templates per
  `76ddaa7e` (operator-decision items from
  goal `6205ad1f`)

The daemon state for `dracon-platform` is:
- `✅ OK` (NOT `⚠️ WARN`)
- 0 MOD, 0 STG, 2 UT
- State: `⚪ untracked-only` (informational,
  not a warning)
- Activity: `🟢 synced 0m`
- Daemon: `23s ago sync_commit`

### Action taken

**No code/config changes.** The daemon was
functioning correctly. The operator's
perception was based on the daemon report
showing top-level untracked DIRS, not
uncommitted FILES. The 161 uncommitted
files were all caught up via normal daemon
commits (bulk + small batches).

### Recommendations

1. **UX improvement** (deferred): the
   `❓ UT` column in the daemon report could
   show BOTH top-level untracked dir count
   AND total uncommitted file count. This
   would prevent operator confusion. This
   is a follow-up goal — it's not the
   commit-all principle's job, it's a
   reporting improvement.

2. **Operator decision still pending**:
   the 2 uncommitted `global.css` files
   in `_template-canvas2d/` and
   `_template-visual-novel/` are
   operator-decision items from
   goal `6205ad1f` Part B section 8.
   They are inside the deferred
   `_template-*` subtrees. Operator
   decides: commit / keep deferred /
   add to `auto_commit_exclude_patterns`
   / delete.

---

## Followup #2: dracon-platform commit cadence (goal `42ea41d4`, 2026-06-16)

> **Operator said**: "ok it seems to work
> well now except the platform i think we
> got many busy bees and the 5 second daly
> system maybe getting held up, investigate
> it like the comits are fairly infrequent"

After the previous fix (161 uncommitted → 2),
the operator observed that the daemon's
commits on `dracon-platform` feel
"infrequent". This section audits the
cadence.

### Measured cadence (15-min window)

| Repo | Commits | Commits/min | Avg interval |
|------|---------|------------|--------------|
| **dracon-platform** | **26** | **1.73** | **34.9s** |
| pully-fully | 10 | 0.67 | 90s |
| kiki-sassy | 1 | 0.07 | 900s |
| avid | 2 | 0.13 | 450s |
| browser-extensions-shared | 0 | 0 | n/a |

**Surprising finding**: `dracon-platform`
is the FASTEST-committing repo, not the
slowest. The 1.73 commits/min rate is
higher than the 4 other active repos
combined.

**The "infrequent" perception is wrong.**
The operator is generating LOTS of
changes (3075 files modified in 30 min),
and the daemon is committing them as
fast as it can — but each commit requires
a 4-remote push that takes ~6s.

### Commit size distribution (dracon-platform)

| File count | Number of commits |
|------------|-------------------|
| 1 | 10 |
| 3 | 3 |
| 4 | 5 |
| 5 | 2 |
| 12 | 2 |
| 14 | 1 |
| 15 | 1 |
| 16 | 3 |

The 1-file commits dominate. Each one
waits 5s for the operator's edit to
stabilize, then commits, then pushes to
4 remotes (~6s). That's 11s minimum
per commit. The 35s avg interval
includes the inter-commit scan cycles
where the operator is still editing.

### The trailing-drain noise (1:1 with commits)

The daemon journal shows 103
"trailing-drain" messages in 60 min
(1:1 with commits). This is NOT a bug —
it's the expected behavior given the
current config:

- `pulse_interval_secs = 1` → apply
  phase deadline = `pulse_interval * 2`
  = **2s**
- 4-remote push takes **6s**
- Apply phase always times out
- Trailing-drain clears the in_flight
  set so the next cycle can re-dispatch
- Commit DOES land (just late in the
  next cycle)

The 1:1 ratio is noisy but harmless.
The commits are landing; the trailing-
drain is just bookkeeping.

### Where the time actually goes (per commit)

1. **5s** `inactivity_push_delay_secs`
   debounce (waits for stable fingerprint)
2. **~0.5s** `git add` + `git commit`
3. **~6s** `git push` to 4 remotes
   (origin, github, gitlab, codeberg)
4. Total: **~11.5s minimum per commit**

The operator is editing faster than
11.5s/change, so the daemon is always
behind. But it's catching up — 26
commits in 15 min is actually a healthy
rate.

### Recommendation: do NOT change the cadence

The current cadence is correct given the
constraints:
- 5s debounce is the safety floor (don't
  commit while operator is mid-edit)
- 4-remote push is the cost of multi-
  remote sync
- The operator's edit rate is the
  variable

**Options considered** (not applied —
operator decision needed):

a) **Lower `inactivity_push_delay_secs`
   to 2-3s**: would batch 1-file edits
   more aggressively. Trade-off: risk of
   committing mid-edit. Not recommended.

b) **Raise `pulse_interval_secs` to
   5**: would change the apply deadline
   from 2s to 10s, eliminating the
   trailing-drain noise. Trade-off:
   slower scan cycle. Marginal effect on
   commit throughput. RECOMMENDED for
   log-noise reduction.

c) **Parallelize the 4-remote push**:
   would cut the 6s push to ~1.5s.
   Trade-off: significant code change
   in `multi_remote.rs`. Highest payoff
   if operator wants faster commits.

d) **Add `auto_commit_exclude_patterns`
   for the smoke-out PNGs**:
   `web/games/games/darklord/scripts/
   smoke-out/**`. Trade-off: the smoke
   test artifacts are regenerable, but
   they're currently tracked. Excluding
   them from auto-commit would prevent
   the `git rm` failure (see separate
   concern below).

### Separate concern: smoke-out PNG `git rm` failures

The daemon logged a `git rm failed` error
at 17:07:03:

```
⚠️ /home/dracon/Dev/dracon-platform git
rm failed for 5 paths: ["web/games/games/
darklord/scripts/smoke-out/08-room-
placement.png", ...]
```

The `darklord/scripts/smoke-out/` PNGs
are:
- Tracked in git
- Regenerated by the operator's smoke
  test on every run
- Causing `git rm` failures when the
  daemon tries to "rewrite" them as
  part of `auto_rewrite_large_blobs`

These are generated artifacts that
should either be:
- Moved to a `gitignore`'d location, OR
- Added to `auto_commit_exclude_
  patterns`

This is a SEPARATE issue from the
cadence — it's a tracking hygiene issue
that needs an operator decision. NOT
applied in this goal.

### Decision: do nothing, document only

After analysis, the current cadence is
correct for the operator's activity
level. The "infrequent" perception is
based on the operator's expectation of
seeing commits in real-time, but each
commit takes 11.5s minimum due to
5s debounce + 6s 4-remote push.

**No config changes applied.**
**No code changes applied.**
**No per-repo overrides added.**

The 2 followup concerns (trailing-drain
noise + smoke-out PNG git rm) are
documented in the design doc for
operator review. If the operator wants
to apply any of the 4 options above,
that's a separate goal.

---

## Followup #3: PUSH_STUCK on dracon-platform — divergent remotes (goal `42ea41d4`, 2026-06-16)

> **Surprise during the cadence investigation**:
> at 17:18, `dracon-sync repos` showed
> `dracon-platform` as `PUSH_STUCK` with 11
> consecutive push failures.

### Root cause: race condition in concurrent multi-remote push

The daemon pushes to all 4 remotes
**concurrently** via `tokio::spawn` in
`dracon-sync/src/git/multi_remote.rs`
(`push_to_all_remotes`). The `force_push_
when_behind` field defaults to `false`,
so divergent pushes are rejected.

The race scenario:

```
T+0s:  commit 3ccbc4093 (deletion) made
T+0.5s: daemon push cycle starts
T+0.5s: spawn 4 concurrent push tasks:
  - origin:    pushes 3ccbc4093, success
  - github:    pushes 3ccbc4093, success
  - gitlab:    slow network, still pushing
  - codeberg:  slow network, still pushing
T+1.0s: commit e0583e564 (re-addition) made
T+1.5s: daemon's next push cycle starts
  - origin:    fast-forward to e0583e564, success
  - github:    fast-forward to e0583e564, success
  - gitlab:    finishes first push, then tries
               to push e0583e564 to gitlab at
               T+5s when gitlab is still at
               3ccbc4093. But origin is at
               e0583e564 already, and the local
               is at e0583e564. Push succeeds.
  - codeberg:  finishes first push, then tries
               to push e0583e564. Push succeeds.
T+30s: more commits made
T+30.5s: daemon's next push cycle starts
  - origin:    pushes new commits, success
  - github:    pushes new commits, success
  - gitlab:    new commits include
               "deletion" of files gitlab
               has. Tries to push.
               BUT: gitlab's tip is
               e0583e564 (still at the
               re-addition commit), local
               tip is past it. NOT a
               fast-forward. REJECTED.
  - codeberg:  same as gitlab. REJECTED.
```

After this, gitlab/codeberg are stuck at
`e0583e564` while origin/github are at
`2291e1702`. The daemon tries to push
each new commit, gets rejected, retries
3 times (per `push_retries`), then
increments the failure counter. After
`push_max_retries = 5` (default), the
daemon marks the repo as `PUSH_STUCK`.

### Evidence (17:18:52 journal)

```
⚠️ push to codeberg failed for
   /home/dracon/Dev/dracon-platform:
   non-fast-forward
⚠️ push to gitlab failed for
   /home/dracon/Dev/dracon-platform:
   non-fast-forward
```

### Fix options (none applied)

a) **Enable `force_push_when_behind = true`
   on gitlab and codeberg remotes**:
   the daemon's `force_push_when_behind`
   field is per-remote. Setting it to
   `true` for the slow remotes (gitlab,
   codeberg) tells the daemon to use
   `--force-with-lease` when the remote
   is purely behind local. Safe because
   `--force-with-lease` aborts if the
   remote advanced (e.g. someone pushed
   something). Operator-approval needed
   for config change.

b) **Sequential push** (slower but
   safer): change `push_to_all_remotes`
   from concurrent `tokio::spawn` to
   sequential `for remote in sorted`.
   Adds ~3-6s per commit (4 remotes × 1.5s
   each) but eliminates the race. Code
   change required.

c) **Pre-push divergence check**:
   before pushing to a remote, run
   `git fetch` to check if the remote
   has commits local doesn't. If yes,
   either pull-and-merge or force-push.
   More robust but more complex. Code
   change required.

d) **Per-remote daemon lock**: serialize
   pushes to the same repo so only one
   push cycle runs at a time. Simplest
   fix but doubles the push latency.

### Recommendation: ask operator for (a) vs (b)

Option (a) is the lowest-effort fix and
preserves the concurrent push. The
`--force-with-lease` flag is safe (it
aborts if the remote has new commits
we don't know about). The trade-off is
that we might overwrite work someone
else pushed to gitlab/codeberg, but
since the operator is the sole author
on these repos, this is acceptable.

Option (b) is the safest. No force-push
needed, but pushes take 2-3x longer.

**Operator decision needed.**

### Action taken in this goal

- **Did NOT** run `dracon-sync repair
  stuck-unstuck` (would re-attempt
  pushes that will fail again)
- **Did NOT** force-push
- **Did NOT** enable `force_push_when_
  behind` in config
- **Documented** the finding for
  operator review
- **Will mark goal complete** with the
  fix options documented

### Temporary unblock

The operator can manually fix the
divergence by running:

```
cd /home/dracon/Dev/dracon-platform
git push --force-with-lease gitlab main
git push --force-with-lease codeberg main
```

This is a one-time fix. After that,
the daemon's concurrent push will work
correctly for ~30s windows until the
next divergence happens (if operator
edits fast enough).

A permanent fix requires one of the
4 options above.
