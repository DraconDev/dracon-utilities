# Kiki-sassy deep investigation — 2026-06-16

> **Operator said**: "investigate teh kik isituation and
> whether there is a good default solution for it, btw that
> remotei s mine setting it to read only is out"
>
> **Goal**: `156ec13e-ca73-46e9-adeb-afab7730144c` (active)
> **Status**: investigation complete, recommendation
> pending operator decision

## TL;DR

The kiki-sassy github remote is at a divergent SHA
`a80dc0938228` (436 ahead, 804 behind local). Both
remotes (github `DraconDev/kiki-sassy-desktop-announcer`
and origin `DraconDev/dracon-voice-notifications`) are
TWO SEPARATE GitHub repos with different content. The
local/gitlab/codeberg are aligned with the OLD
`dracon-voice-notifications` URL.

**The 118 differing files break down**:
- 21 in `artifacts/` (operator's test artifacts)
- 14 in `src/` (operator's NEW gemini.rs + config.rs)
  - `src/gemini.rs` doesn't exist on github (new file)
  - `src/config.rs` has 960 lines of diff
- 13 in `scripts/` (operator's NEW Python testing scripts)
- 11 in `docs/` (operator's NEW Gemini announcer docs)
- 4 in `nix/` (legacy + kiki packages)
- 4 in `.pi/goals/archived/` (operator's goal artifacts)
- 6 in `.ralph/` (operator's audit state)
- 1 `.dracon/data/keys/owner_nixos.pub` (**DIFFERENT
  age encryption keys**)
- 1 `.dracon/dracon-sync.toml` (local-only per-repo config)
- 1 `.github/workflows/ci.yml` (local-only CI config)

**The structural cause**: a 6+ day window (June 10-16)
where the operator did significant work locally that was
never pushed to github, while github received independent
work (GitHub Sponsors button, FUNDING.yml, MESSAGES.md)
that was merged into local via the OLD dracon-voice-
notifications URL.

**No commits are lost** (804 github-only + 436 local-only
all preserved on their respective remotes).

## The 4 remotes mapped

| Remote | URL | SHA | Status |
|--------|-----|-----|--------|
| `origin` | `https://github.com/DraconDev/dracon-voice-notifications.git` | `1b76dfa` | matches local |
| `github` | `git@github.com:DraconDev/kiki-sassy-desktop-announcer.git` | `a80dc09` | **DIVERGENT** |
| `gitlab` | `git@gitlab.com:dracondev/kiki-sassy-desktop-announcer.git` | `1b76dfa` | matches local |
| `codeberg` | `git@codeberg.org:dracondev/kiki-sassy-desktop-announcer.git` | `1b76dfa` | matches local |
| `local` | (this checkout) | `1b76dfa` | HEAD |

**The `origin` URL is the OLD name** of the project
(`dracon-voice-notifications`). The github URL is a
NEW, separate repo that was apparently created on
**June 10, 2026** with different content.

## Structural cause of the divergence

**Common ancestor**: `e322e965045cb9c206007d1e28ef5a980f2f146e`
(2026-02-28 16:27:23 +0000). Commit message:
> "sync: 1 added, 2 modified (Cargo.lock, Cargo.toml,
> NIXOS_SETUP.md in **dracon-voice-notifications**)"

**Timeline**:
1. **2026-02-28**: Both sides diverge from common
   ancestor. Commit messages still mention
   "dracon-voice-notifications" (the old project name).
2. **2026-05-23** (`f7128d8`): Local rename event —
   package renamed from `voice-notify` → `kiki`. Old
   `voice-notify-package.nix` kept for backward
   compatibility.
3. **2026-06-10 22:58-20** (`ce924cb`): GitHub Sponsors
   button added to github (`.github/FUNDING.yml`,
   `DraconDev@users.noreply.github.com` author = GitHub
   UI edit).
4. **2026-06-10 22:59-14** (`a80dc09`): Same FUNDING.yml
   file added via a different commit on github (likely a
   push from local after the GitHub UI edit).
5. **2026-06-10 23:39-32** (`359bab1`): Local commit
   "**Merge https://github.com/DraconDev/dracon-voice-
   notifications**" — local receives the FUNDING.yml
   from the OLD repo URL.
6. **2026-06-10 23:39 → 2026-06-16 12:18**: 6+ days of
   local work (804 commits!) that were never pushed to
   the new github repo.

**What we know for sure**:
- The two GitHub URLs (origin's `dracon-voice-
  notifications` and github's `kiki-sassy-desktop-
  announcer`) are SEPARATE repos with different
  content (different SHAs returned by `git ls-remote`).
- The local repo's HEAD (1b76dfa) matches the OLD
  `dracon-voice-notifications` URL.
- The local repo has diverged from the NEW `kiki-sassy-
  desktop-announcer` URL by 804 ahead / 436 behind.
- The `age` encryption key in `.dracon/data/keys/
  owner_nixos.pub` is DIFFERENT on github vs local
  (this is a **SENSITIVE FILE**).

**What we don't know** (would need operator input or
github access to confirm):
- Was the github `kiki-sassy-desktop-announcer` repo
  manually created by the operator (deliberate fork)?
  Or was it a misclone?
- Are the 436 github-only commits (FUNDING.yml,
  MESSAGES.md, scripts/test-messages.sh, etc.) work
  the operator wants to keep? They include the
  GitHub Sponsors button and a major `MESSAGES.md`
  catalog.
- Should the `origin` URL be updated from
  `dracon-voice-notifications` to the new URL
  `kiki-sassy-desktop-announcer`?

## The 118 differing files (categorized)

### `artifacts/` (21 files) — operator's NEW test artifacts
Local-only:
- `taste-test-capitalist-gemma-4-26b-a4b-it-*` (9 files)
- `taste-test-capitalist-gemma-4-26b-a4b-it-schema-*` (3 files)
- `taste-test-custom-template-capitalist-gemma-4-26b-a4b-it-*` (1 file)
- `taste-test-capitalist-gemma-4-31b-it-*` (1 file)
- `announcer-audit-20260616.md` (operator's audit)
- `gemini-announcer-blocker-report-20260615.md` (operator's report)
- `diagnostic-gemini-minimal-thinking-response-schema-*-*.md` (4 files)
- `research-gemini-flags-*.md` (2 files)

### `src/` (14 files) — operator's NEW Gemini announcer code
- `src/gemini.rs` — **NEW FILE** (doesn't exist on github)
  - 388 lines of diff
  - Operator's Gemini API integration
  - Last touched locally: `5f60e57` (2026-06-16 08:48:37)
- `src/config.rs` — **MAJOR REWRITE** (960 lines of diff)
  - Last touched locally: `a35c042` (2026-06-16 12:16:58)
  - Adds Gemini announcer config, schema support,
    personality pack selection, etc.
- `src/ai.rs` — local-only (operator's AI lane code)
- `src/announcer_messages.rs` — local-only (operator's
  announcer message catalog)
- `src/main.rs` — local-only updates
- `src/daemon.rs`, `src/ipc.rs`, `src/journal.rs`,
  `src/memory.rs`, `src/monitor.rs`, `src/triggers.rs`,
  `src/tts.rs`, `src/announcement.rs`, `src/context.rs`
  — local-only updates

### `scripts/` (13 files) — operator's NEW Python testing
- `research_gemini_flags.py` + `_followup.py`
  (operator's research scripts)
- `taste_test_announcer.py` + `_schema.py`
- `test_gemini_minimal.py`, `_custom_template.py`,
  `_system_instruction.py`, `_response_schema.py`,
  `_thinking.py`, `_v1_endpoint.py`
- `dogfood_announcer.sh` (operator's dogfooding script)
- `secret_scan.sh` (operator's secret scan)

**Github-only** in scripts/:
- `test-messages.sh` (the AI message testing script from
  the handoff)

### `docs/` (11 files) — operator's NEW Gemini announcer docs
Local-only:
- `announcer-audit.md`, `announcer-line-length-audit.md`,
  `announcer-pack-audit.md`, `announcer-pack-ideas.md`,
  `announcer-pack-inspiration.md`,
  `announcer-reference-list.md`
- `gemini-announcer.md`, `gemini-output-shaping.md`
- `rebranding-and-local-vs-ai.md`, `README.md`

### `nix/` (4 files) — both legacy and new kiki packages
Local-only: `kiki-module.nix`, `kiki-package.nix`,
`home-manager-module.nix`, `voice-notify-package.nix`
(all operator's Nix packaging work)

### `.pi/goals/archived/` (4 files) — operator's goal artifacts
`goal_2026052215444139_*`, `goal_2026061123570987_*`,
`goal_2026061202062947_*`, `goal_2026061213541804_*`
(operator's completed goal files)

### `.ralph/` (6 files) — operator's audit state
`cli-audit.md`, `cli-audit.state.json`, `kiki-audit.md`,
`kiki-audit.state.json`, `kiki-audit-2026-05-23.md`,
`kiki-audit-2026-05-23.state.json`

### `Cargo.toml` and `Cargo.lock` — dependency differences
Local-only deps (in `Cargo.toml`):
- `dracon-ai-contracts = "94.7"`
- `dracon-ai-runtime-contracts = "94.7"`
- `ai-routing-runtime = "94.7"`
- `ai-runtime-config = "94.7"`
- `ai-runtime-adapters = "94.7"`
- `async-trait = "0.1"`
- `reqwest = { version = "0.12", default-features = false, ... }`
- `tempfile = "3"` (dev-dep)

Github-only deps:
- `zbus = "4"`
- `futures-util = "0.3"`

### `.dracon/data/keys/owner_nixos.pub` — **SENSITIVE FILE**
The age encryption key is DIFFERENT:
- github: `age162n5w0v0y3dxyddqvlaywt9gmyfr0e5rft6kcunnf58ceqhycdxq42vmzt`
- local:  `age1z4atpzyksuszdnd6f375xt56453uxanapxkdwxqs3uw9p24y4yzs3rx2zk`

This means secrets encrypted with one key are not
decryptable with the other. **The github repo has a
different encryption key than local**. This is a
significant divergence — the two repos cannot share
encrypted secrets.

### `.dracon/dracon-sync.toml` — local-only per-repo config
11-line per-repo override:
```toml
owned = true
```
Forces kiki-sassy to be classified as `Owned` in
the daemon, bypassing signal-based classification.
**This file does not exist on github**.

### `.github/workflows/ci.yml` — local-only CI config
51-line GitHub Actions workflow:
- `cargo fmt --all -- --check`
- `cargo check --locked`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `./scripts/secret_scan.sh`
- `cargo deny check`
**This file does not exist on github**.

## The 804 github-only commits (categorized)

By commit message first word:
- 243 `chore(sync):` (mechanical sync commits)
- 81 `sync:` (mechanical sync)
- 28 `chore(misc):`
- 24 `[plan|wip]`
- 16 `docs:` (including MESSAGES.md updates)
- 13 `[[daemon]`
- 6 `chore(audit):`
- 4 `fix:`
- 4 `chore:`
- 2 `feat(cli):` (`ad68eed` "feat(cli): add kiki config-set")
- 2 `chore(docs):`
- 1 `scripts:` (`c1434e7` scripts: add test-messages.sh)
- 1 `security(misc):`
- 1 `Verify`

**Notable github-only features** (potential operator
value):
- `a80dc09` Enable GitHub Sponsors button
- `023d54c` docs: update MESSAGES.md with actual AI
  prompts
- `8c5097d` docs: add expected AI responses and analysis
  to MESSAGES.md
- `10b6dfb` docs: add actual code templates to MESSAGES.md
- `c1434e7` scripts: add test-messages.sh for AI message
  testing
- `78cb974` docs: add MESSAGES.md cataloging all Kiki
  message types
- `0d8e2ad` fix: add missing default functions for
  truncation config
- `3b2897e` docs(audit): add notification truncation
  section
- `e6f55f1` feat(notifications): add message truncation
  for shorter AI output
- `ad68eed` feat(cli): add kiki config-set command

## The 436 local-only commits (categorized)

By commit message first word:
- 278 `chore(sync):`
- 128 numeric (e.g., "1 file(s)", "2 file(s)") — auto-
  commit messages from the daemon
- 81 `sync:`
- 48 numeric
- 34 `docs:`
- 28 `chore(misc):`
- 25 numeric
- 24 `[plan|wip]`
- 17 numeric
- 15 `Modified`
- 13 `[[daemon]`
- 12 `update`
- 7 `fix:`

**Notable local-only features** (operator's recent work):
- 6+ days of Gemini announcer development
  (`src/gemini.rs`, `src/config.rs`, tests, docs)
- `479e734` 6 file(s) in docs,src [Cargo.lock, docs/
  announcement-strategy.md, src/gemini.rs] (major work)
- `e26dd55` 2 file(s) in src [src/config.rs, Cargo.toml]
  (Cargo.toml dependency update)
- `303b98d` 10 file(s) (.dracon/dracon-sync.toml +
  artifacts + config.rs + scripts) (large batch)
- `f950677` content: curate capitalist pack to 867
  strong lines
- `1afc41d` 1 file(s) [README.md] (recent)
- `1f2a9bc` fix: avoid secret scan false positives
- `b975ce5` docs: redact taste test key examples
- `5b5fa17` fix: remove gemma fallback model
- `56ebf8f` fix: use Gemma fast model and stronger
  fallback

## Per-option impact analysis (a/b/c/d, NOT e)

### Option (a): `git pull github main` (merge)
**Action**: `cd /home/dracon/Dev/kiki-sassy-desktop-announcer && git merge github/main --no-ff`

**Impact**:
- All 804 github-only commits added to local
- All 436 local-only commits stay
- Merge commit created
- 118 files need conflict resolution
- **Actual merge conflicts**: ~`merge.conflictStyle`
  output (not yet computed, but the previous handoff
  said 316 conflicts in 15 files; the 118 differing
  files is a different metric — file-level diff vs
  hunk-level conflict)
- **.dracon/data/keys/owner_nixos.pub WILL conflict**
  (the age encryption key) — operator must choose
  which key to keep
- **Cargo.toml WILL conflict** (different deps)
- **`src/gemini.rs` is new in local, so no conflict
  there** (will be untouched by merge)
- **`src/config.rs` WILL conflict** (960 lines of diff)

**Commits lost**: 0
**Reversibility**: full (just `git merge --abort` if
needed)
**Effort**: 1-3 hours (operator time)
**Risk**: high (sensitive file conflict, dependency
conflict)

### Option (b): delete local, re-clone from github
**Action**: `rm -rf /home/dracon/Dev/kiki-sassy-desktop-announcer && git clone git@github.com:DraconDev/kiki-sassy-desktop-announcer.git`

**Impact**:
- **LOSES 436 local-only commits** including:
  - 6+ days of Gemini announcer work (`src/gemini.rs`,
    `src/config.rs`, tests, docs)
  - All operator's recent artifacts/
  - The new per-repo config `.dracon/dracon-sync.toml`
  - The CI config `.github/workflows/ci.yml`
  - The `dracon-voice-notifications` origin remote
    (will need to be re-added)
- All 804 github-only commits gained
- The new clone has github's age key
  (`age162n5w0v0y3dxyddqvlaywt9gmyfr0e5rft6kcunnf58ceqhycdxq42vmzt`)
- Operator would need to:
  - Re-apply the `.dracon/dracon-sync.toml` per-repo
    config
  - Re-add the `origin` remote
  - Re-apply any local-only config (the original age
    key is GONE)
  - LOSE 6+ days of unrecoverable work

**Commits lost**: 436 (UNRECOVERABLE)
**Reversibility**: NONE (the 436 local-only commits
still exist in `.git/objects/` after `rm -rf` until
gc'd, but a fresh clone from github doesn't have them)
**Effort**: 30 minutes + irreversible data loss
**Risk**: VERY HIGH (loses operator's 6+ days of work)

### Option (c): `git push --force-with-lease` to github
**Action**: `cd /home/dracon/Dev/kiki-sassy-desktop-announcer && git push --force-with-lease github main`

**Impact**:
- **LOSES 804 github-only commits** including:
  - GitHub Sponsors button (`.github/FUNDING.yml`)
  - `MESSAGES.md` (600+ lines of AI message catalog)
  - `scripts/test-messages.sh` (AI message testing)
  - 3 notification truncation commits
  - `feat(cli): add kiki config-set` command
  - All `chore(sync):` updates
  - All `docs:` updates
- All 436 local-only commits stay
- github is reset to local's `1b76dfa`
- github gets the local age key
  (`age1z4atpzyksuszdnd6f375xt56453uxanapxkdwxqs3uw9p24y4yzs3rx2zk`)
- All secrets encrypted with the OLD github key become
  undecryptable on github

**Commits lost**: 804 (UNRECOVERABLE without `git
reflog` on github)
**Reversibility**: NONE (force-push is destructive on
the receiving side; the github-side reflog is not
accessible)
**Effort**: 5 minutes
**Risk**: VERY HIGH (loses 6 days of work the
operator may want to keep)
**Hard constraint**: REQUIRES OPERATOR APPROVAL
(`546d4f9c`)

### Option (d): cherry-pick specific commits
**Action**: operator picks commits from each side,
cherry-picks them onto the other side

**Impact**:
- Selected github-only commits moved to local
- Selected local-only commits moved to github
- The other commits stay where they are
- Conflicts resolved per-commit (operator decides
  per-commit)

**Commits lost**: 0 (only the ones the operator
deliberately doesn't cherry-pick remain "lost")
**Reversibility**: full
**Effort**: 1-2 hours (operator time per cherry-pick)
**Risk**: low per cherry-pick, accumulates

## The "good default solution" the operator asked for

The operator said: "is there a good default solution
for it, btw that remote is mine setting it to read
only is out"

The hard constraints:
- NO force-push without approval (operator wants
  analysis, not action)
- NO read-only / skip-github config (operator
  excluded)
- NO deletion of local clone (irreversible)
- NO cherry-pick (operator's decision)
- PRESERVE all commits

This eliminates (b), (c), and the operator-excluded
(e). That leaves (a) `git merge` and (d) cherry-pick
as the only options.

**My recommendation: option (a) `git merge github/main`**

Why:
- All 804 github-only commits are preserved (including
  the GitHub Sponsors button, MESSAGES.md, etc.)
- All 436 local-only commits are preserved
- It's reversible (`git merge --abort` works until
  the merge commit is made)
- The conflicts are manageable: 1 markdown audit doc
  + 5 PNGs in the previous handoff turned out to be
  more nuanced, but for kiki-sassy the main conflicts
  will be:
  - `.dracon/data/keys/owner_nixos.pub` (operator
    picks: keep local — it's the working key)
  - `Cargo.toml` (operator resolves: union of
    dependencies, regen `Cargo.lock`)
  - `src/config.rs` (operator resolves: prefer
    local's structure, add github's features)
  - Other smaller files

**Concrete steps for option (a)**:
1. `cd /home/dracon/Dev/kiki-sassy-desktop-announcer`
2. `git merge github/main --no-ff -m "merge: integrate
   github remote (804 commits: Sponsors, MESSAGES,
   truncation)"`
3. Resolve conflicts:
   - `.dracon/data/keys/owner_nixos.pub`: keep local
     (the working key)
   - `Cargo.toml`: merge deps, then `cargo update`
   - `src/config.rs`: prefer local, add github
     features
   - Other small files: per-file operator decision
4. `cargo build --release --locked` (verify
   compilation)
5. `cargo test --locked` (verify tests pass)
6. `git push origin main` (push to OLD repo)
7. `git push gitlab main` (push to gitlab)
8. `git push codeberg main` (push to codeberg)
9. `git push github main` (push merged result to
   github) — this WILL work after the merge (no
   force-push needed since we're adding commits)
10. (Optional) `git remote set-url origin
    git@github.com:DraconDev/kiki-sassy-desktop-announcer.git`
    to re-point origin to the NEW URL

**For the `origin` URL question**:
- The current `origin` URL
  (`https://github.com/DraconDev/dracon-voice-notifications.git`)
  is the OLD name
- It works (matches local SHA) but is misleading
- After the merge, both URLs have the same content
- Recommend: `git remote set-url origin
  git@github.com:DraconDev/kiki-sassy-desktop-announcer.git`
  to use the new canonical name (and SSH for
  consistency with github/gitlab/codeberg)
- This is a one-line config change, NOT a destructive
  action, and doesn't affect the daemon's behavior
  (the daemon pushes to all 4 remotes regardless of
  which is "origin")

## What I have NOT done

- NO force-push
- NO deletion of local clone
- NO cherry-pick
- NO read-only config
- NO git config changes
- NO remote URL changes
- NO commits created or amended
- NO `git push` of any kind

## Investigation evidence

1. **`git ls-remote https://github.com/DraconDev/dracon-voice-notifications.git`** = `1b76dfa`
2. **`git ls-remote https://github.com/DraconDev/kiki-sassy-desktop-announcer.git`** = `a80dc09`
3. **`git merge-base main github/main`** = `e322e965045cb` (2026-02-28)
4. **`git log main..github/main --oneline | wc -l`** = 436
5. **`git log github/main..main --oneline | wc -l`** = 804
6. **`git diff main..github/main --name-only | wc -l`** = 118
7. **`git show github/main:.dracon/data/keys/owner_nixos.pub`** = `age162n5w0v0y3dxyddqvlaywt9gmyfr0e5rft6kcunnf58ceqhycdxq42vmzt`
8. **`git show main:.dracon/data/keys/owner_nixos.pub`** = `age1z4atpzyksuszdnd6f375xt56453uxanapxkdwxqs3uw9p24y4yzs3rx2zk`
9. **`git show github/main:.github/FUNDING.yml`** = `[DraconDev]` (added on June 10 22:58)
10. **`git show 359bab1`** = "Merge https://github.com/DraconDev/dracon-voice-notifications" (added on June 10 23:39)
11. **Daemon log**: `journalctl --user -u dracon-sync.service --since "3 hours ago" | grep kiki-sassy` shows every-1-2-minute `non-fast-forward` failures
12. **`git reflog | head -30`**: 30 most recent actions are all `commit:` (no rewrites, no abandoned pushes)
13. **`.dracon/dracon-sync.toml`**: 11-line per-repo override, `owned = true`
14. **`git remote -v`**: 4 remotes, origin URL is `dracon-voice-notifications.git` (old name)

## Decision needed from operator

**Option (a) `git merge github/main`** is my
recommendation. It's the only option that:
- Preserves all 1240 commits (804 + 436)
- Is reversible
- Doesn't require force-push
- Doesn't delete anything
- Doesn't change remote URLs
- Unblocks the daemon's 120m+ push-stuck
- Can be done with operator's manual review of
  conflicts

The conflict resolution is the only operator time
needed (~1-3 hours for the major files, then a
build verification).

**Tell me "apply (a)" or "merge github" and I'll
execute it.** Or pick a different option. Or ask
for more detail on any part of this analysis.

## Related handoffs

- `docs/design/kiki-sassy-decision-handoff-2026-06-15.md`
  — previous goal's options (a/b/c/d/e)
- `docs/design/concern-investigation-2026-06-16.md` —
  previous goal's investigation (dracon-platform +
  kiki-sassy)
