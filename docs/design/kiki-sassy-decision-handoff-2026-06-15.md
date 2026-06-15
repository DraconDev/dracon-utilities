# Kiki-sassy decision handoff — 2026-06-15

> **Status**: awaiting operator decision.
> **Goal**: `546d4f9c-2e35-4104-a093-55345b41eeab` (active).
> **Blocker**: divergent history on `github` remote.

## TL;DR

The kiki-sassy `github` remote is **436 commits behind** local and has **436 commits ahead** of local, with **316 merge conflicts** across 15 files if we try to merge. The push fails with "non-fast-forward". The 3 other remotes (origin, gitlab, codeberg) are aligned with local and working.

The 4 reasonable fixes all require operator input. The daemon will keep trying to push and failing until you decide.

## 5 fix options

### (a) Recreate github repo
**Action**: Delete github repo, recreate with same name, re-add remote.
**Cost**: **LOSES 436 commits** of real feature work:
- `MESSAGES.md` (600+ lines of AI message catalog)
- `.github/FUNDING.yml` (GitHub Sponsors button)
- Notification truncation code (`src/config.rs`, `src/daemon.rs`, `src/main.rs`)
- `scripts/test-messages.sh`
- `audit.md` notification truncation section
- 14 other `.rs` files, 8 `.sh` files, 6 `.nix` files
- Total: 69 files changed in 436 commits
- Author: 432 commits by `DraconDev <dracsharp@gmail.com>` (the operator)

**Verdict**: NOT recommended. Loses real work.

### (b) Pull from github and merge
**Action**: `cd /home/dracon/Dev/kiki-sassy-desktop-announcer && git merge github/main`
**Cost**: 316 merge conflicts across 15 files:
- 6 Rust source files (`config.rs`, `daemon.rs`, `ipc.rs`, `main.rs`, `tts.rs`, `cli_tests.rs`)
- 3 Nix files (`home-manager-module.nix`, `voice-notify-package.nix`, `shell.nix`)
- 2 Cargo files (`Cargo.lock`, `Cargo.toml`)
- 1 encryption key (`.dracon/data/keys/owner_nixos.pub` — **sensitive**, age key rotation)
- 2 docs/config (`NIXOS_SETUP.md`, `config.example.toml`)
- 1 tests (`tests/cli_tests.rs`)

**Estimated effort**: 1-2 hours for an operator who knows the codebase.

**Verdict**: OK if operator values the github work and wants to do the merge.

### (c) Stop pushing to github for kiki-sassy
**Action**: Edit `~/.dracon/utilities/sync/dracon-sync.toml` to remove the `github` remote entry (affects ALL 13 watched repos, not just kiki-sassy).

**Cost**:
- github stays frozen at `a80dc09` (June 10, 2026) for kiki-sassy
- 12 OTHER repos also lose github sync (significant side effect)
- No per-repo remote filter exists in the code (per goal 546d4f9c scope)

**Verdict**: OK to defer the decision, but significant side effect on other repos.

### (c') NEW: Add per-repo `skip_remotes` field to the code
**Action**: Code change in `dracon-sync/src/policy.rs` adding a `skip_remotes: Vec<String>` field to `RepoPolicyOverride`. Operator can then add `skip_remotes = ["github"]` to kiki-sassy's `.dracon/dracon-sync.toml`.

**Cost**:
- Requires a code change (scope creep, NOT in the current goal)
- Operator must apply the per-repo override themselves (operator-owned repo)
- All 13 repos continue pushing to github; only kiki-sassy skips it

**Verdict**: Cleanest solution but requires a separate code change. Not in scope for goal 546d4f9c.

### (d) Force-push local to github
**Action**: `git push --force github main`
**Cost**: **LOSES 436 commits** of real feature work (same as (a)).

**Verdict**: BLOCKED by goal's stop condition ("If pushing kiki-sassy after fix would require force-push, stop and ask the operator").

### (e) Inspect the 436 commits manually
**Action**: `cd /home/dracon/Dev/kiki-sassy-desktop-announcer && git log --pretty=format:"%ai %h %s" main..github/main | head -50`

**Cost**: Operator time, but you can see exactly what's on github before deciding.

## My recommendation

**Option (b)** if you value the github work and can do 1-2h of merge work.
**Option (e)** first if you want to make an informed decision.

If you want to defer without making a decision: **option (c)** (remove github from daemon config) is acceptable but affects 12 other repos.

## What I've already done

- Durable code defaults in `dracon-sync/src/policy.rs` (commit-all policy)
- 2 new tests + 1 updated test (851 tests pass, was 849 + 2 new)
- `dracon-sync.example.toml` updated + drift fixed
- `AGENTS.md` created
- Operator's daemon config drift fixed (added missing `**/research/scratch/**`)
- Daemon restarted
- Design doc and CHANGELOG updated
- 4-remote alignment for `dracon-utilities` at `343c89d49261`
- 2 of 3 concerns resolved:
  - dracon-platform 5 MOD: resolved (transient, daemon auto-committed)
  - Junk-Runner-bevy 90 MOD: per-repo policy working as designed
  - kiki-sassy push-stuck: THIS DOCUMENT, awaiting your decision

## Tell me

Reply with (a), (b), (c), (d), (e), or describe what you want, and I'll execute.
