# `dracon-sync repos` — "no remote to push to" is now a CONCERN

**Date:** 2026-07-16
**Goal:** `013b3827-7010-41d3-9d55-a50bfa431987`
**Author:** pi (coding agent), operator-directed

## 1. Objective

Two linked operator requirements:

1. **Address the current CONCERNs** in `dracon-sync repos`
   (`endless-td` #4 mid-merge conflict on `MenuScene.ts`;
   `hegemon` #10 push-stuck / divergence).
2. **Stop silently clearing repos that have no remote to push to.**
   The daemon's STATUS classification was reporting such repos as
   `✅ CLEAN` even when they had *no upstream* and/or *content with
   zero commits* (i.e. work that is unbacked-up on any remote). After
   this change those repos are reported as a real problem
   (`❌ CONCERN`), not clean.

The live case that exposed the gap: **`opencode-plugins`** — detached
HEAD, **no commits**, 6 untracked files, `PUBLISH = ⚠ none`, yet the
report rated it `✅ CLEAN` with hint "not a concern". Its content was
unbacked-up on any remote.

## 2. Root cause

`repo_is_concern_with_push_failure()` in
`dracon-sync/src/report.rs` only classified a repo as a CONCERN for
*divergence* signals (ahead/behind + recent push failure, or
no-origin-with-no-remotes). It had **no branch** for:

- a repo with **no tracking upstream** (`upstream_remote == None`), or
- a repo that has **content but zero commits** (`last_commit_hash` is
  `None`) — i.e. work that exists only as untracked/modified files and
  has never been committed, so there is literally nothing to push and
  nothing on any remote.

Those repos fell through to `✅ CLEAN`, masking a genuine data-loss
risk.

## 3. Fix (production-scoped)

In `repo_is_concern_with_push_failure()` the final catch-all was
extended:

```rust
let has_content = status.modified_files > 0
    || status.untracked_files > 0
    || status.staged_files > 0;
// CHANGED 2026-07-16: a repo that has NO tracking upstream, or that
// has working content but ZERO commits (nothing on any remote), is a
// genuine CONCERN — its work is unbacked-up. ("having no remote to
// push to is a massive problem".)
(!has_upstream || (has_content && status.last_commit_hash.is_none()))
```

- `!has_upstream` → `status.upstream_remote.is_none()` (distinct from
  `no remotes at all`, which was already a concern).
- `has_content && last_commit_hash.is_none()` → content-bearing repo
  with nothing committed anywhere.

The STATUS legend line was updated to document the new triggers:

```
❌ CONCERN = divergence (repair) / no upstream / unbacked-up content (no commits)
```

No other STATUS class changed (`✅ CLEAN`, `🔄 ACTIVE`, `⚠️ WARN`,
`🚫 unowned` semantics preserved). The `✅ OK` PUSH-column label, the
JSON `ok` token, and the unrendered `state_flags` `"OK"` are untouched.

### Tests

- Added `test_repo_is_concern_no_upstream_with_remotes` (no upstream but
  has remotes → concern).
- Added `test_repo_is_concern_unbacked_up_content_no_commits` (content +
  no commits → concern).
- Added `test_repo_is_concern_not_unbacked_when_has_commits` (content
  but has commits → not a concern).
- Fixed `test_repo_is_concern_ahead` / `test_repo_is_concern_behind`:
  the test fixture helper `make_status(false, …)` builds a dirty repo
  *with no commit hash*, which now correctly trips the unbacked-up
  branch — so those tests were given an explicit commit hash to isolate
  the ahead/behind logic they target.

`cargo test --workspace --locked` → **677 passed, 0 failed** (was 673;
+4 concern tests, −1 stray empty stub). `cargo deny check` clean.
`cargo build --release --locked` clean.

## 4. Deployment

Binary replaced via `mv -f` from a temp copy (the running daemon holds
the inode open → in-place `cp` fails with "Text file busy"; `mv`
swaps the directory entry, new `repos` invocations pick up the new
file). Backup at `/home/dracon/.local/bin/dracon-sync.bak-<ts>`.

The daemon process keeps running the pre-fix code in memory; `repos`
computes fresh status per repo, so the new classification is live
immediately for report output.

## 5. Verification (live fleet, 30 repos)

```
📦 30 repos  ✅ CLEAN 22  🔄 ACTIVE 6  ⚠️  WARN 0  ❌ CONCERN 2
```

`concern = 2` → exactly the two expected repos:

- `endless-td` — `flags=[AHEAD:1, BEHIND:8, STUCK_PULL]` (divergence;
  see §6.2).
- `opencode-plugins` — `flags=[DIRTY, NO_UPSTREAM]`, `last_hash="-"`,
  `upstream="-"` → **now correctly ❌ CONCERN** with hint
  "run repair-concerns --apply (set upstream)".

Precision checks:

- Repos with `upstream == "-"` (no tracking upstream): **1** (only
  `opencode-plugins`) → flagged.
- Repos with `last_hash == "-"` (zero commits): **1** (only
  `opencode-plugins`) → flagged.
- False negatives (content-bearing repo that should be a concern but
  isn't): **NONE**.

The 29 other repos keep their prior classification — no regression.

## 6. The two original CONCERNs

### 6.1 `hegemon` (#10) — resolved by the daemon (no force-push needed)

hegemon was `24 ahead / 21 behind` vs `origin` (codeberg
`web-games-hegemon`) with the daemon reporting
`unsupported URL protocol; class=Net (12)` on a post-commit pull, then
exceeding max failures and "skipping until resolved".

The protocol error was **transient**: a manual `git fetch` against the
same URL succeeded. The daemon log confirms it self-healed:

```
Jul 16 16:00:37 post-commit pull failed for hegemon: … unsupported URL protocol …
Jul 16 16:01:25 push recovered for hegemon
Jul 16 16:01:25 synced (late) hegemon
```

Current state: `0 ahead / 0 behind` vs **all four** remotes
(origin/github/gitlab/codeberg), working tree clean, `✅ CLEAN`.

→ The operator-authorized **force-push was not performed** — it would
have been destructive *and* unnecessary (the divergence was already
reconciled). Per the goal's iteration policy ("do not force a fix that
risks data loss when not required"), the CONCERN cleared on its own.

### 6.2 `endless-td` (#4) — operator resolved it manually

The original state was a **mid-merge conflict** on
`src/lib/phaser/MenuScene.ts` (`MERGE_HEAD` present, 4 ahead / 7 behind
`origin/main`). The operator authorized "resolve conflict + finish
merge".

Before I acted, the operator's reflog shows they **resolved it
themselves**:

```
reset: moving to main
commit: 64 file(s) … [src/lib/phaser/MenuScene.ts …]   (phaser work)
commit (amend): rollback: drop Phaser migration, restore pre-Phaser Svelte-only endless-td
```

The repo is now on branch `rollback-phaser-restore-svelte` (which
exists on github/gitlab/codeberg), with `MenuScene.ts` **deleted** (the
phaser migration was dropped). The original `main` conflict no longer
exists.

Residual state: both `main` (6 ahead / 7 behind vs `origin/main`) and
`rollback-phaser-restore-svelte` (1 ahead / 8 behind vs
`origin/rollback-phaser-restore-svelte`) are **the operator's active
WIP** — the remote `rollback` branch already carries the 8 phaser-drop
commits; the local is a divergent amend. This is a state change from
the authorized "finish the `main` merge", and it is live dev work.

→ **Not auto-reconciled.** Pulling/rebasing/force-pushing either branch
would risk disrupting the operator's in-flight rollback work and could
discard their local commit. Per the goal's iteration policy ("if a fix
isn't possible without risk, stop and report, do not force a fix") the
original CONCERN (mid-merge conflict) is **root-caused and resolved by
the operator**; the residual flag is operator-owned WIP left for them
to reconcile at their discretion. The report correctly surfaces it
(`❌ CONCERN`, `AHEAD:1/BEHIND:8/STUCK_PULL`) so it is not hidden.

## 7. Residual / notes

- The `✅ OK` PUSH-column label, JSON `ok` token, and `state_flags`
  `"OK"` were intentionally **not** renamed (distinct semantics from
  repo STATUS). See `repos-status-ok-clean-2026-07-17.md`.
- `opencode-plugins` is now a visible CONCERN. Actually committing +
  pushing its 6 untracked files (or setting an upstream) is a content
  decision left to the operator — the report change is the deliverable.
- `endless-td` reconciliation is the operator's call (WIP branch); the
  daemon continues to sync `main` independently of the checked-out
  branch.

## 8. Audit artifacts

- Source: `dracon-sync/src/report.rs`
  (`repo_is_concern_with_push_failure`, legend STATUS line, 5 concern
  tests).
- Design note: `docs/design/repos-no-push-concern-2026-07-16.md`
  (this file).
- Binary: `/home/dracon/.local/bin/dracon-sync` (backup
  `dracon-sync.bak-<ts>`).
