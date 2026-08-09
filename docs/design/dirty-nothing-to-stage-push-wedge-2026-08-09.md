# Dirty-but-nothing-to-stage repos never pushed ahead commits (2026-08-09)

**Severity**: HIGH (silent push wedge — no error, no retry, permanent false "pushing" display)
**Affected**: dracon-sync ≤ v0.113.46 (fixed in v0.113.47)
**Live incident**: dracon-platform, 2026-08-09 00:24 → 00:52 (40+ minutes, then remediated by the fix)
**Related bug family**: libgit2 1.9.x ignore mis-evaluation (see
`installed-binary-drops-patch-dracon-git-2026-08-08.md` — same underlying library bug, different symptom)

## Symptom

`dracon-sync repos` showed dracon-platform as `🔄 ACTIVE`, `🟣 pushing 14m`
(later `24m`), `↑1`, `push: 🟣 PENDING` while the daemon journal logged
`🔁 synced /home/dracon/Dev/dracon-platform` every ~40s. The repo was
`1 ahead` (commit `690d39180`, authored by the `dracon` agent at 00:24:08)
and the upstream tracking ref `refs/remotes/origin/main` was permanently
stale (last successful push 00:17:04, `dcb0ff497`).

Diagnostic facts that narrowed it down:

- `ps` showed zero git/ssh push processes; the daemon never *attempted* the push.
- No `⏫` push-start log for dracon-platform since 00:17:04.
- The stuck-push ledger was empty; the upstream ref was stale, not missing.
- A raw git2 0.21 probe agreed with git CLI: `AHEAD=1 BEHIND=0` — so the
  daemon's *own* status code *should* have seen the ahead commit.
- `DRACON_SYNC_DEBUG=1 dracon-sync once` for dracon-platform:

  ```
  🐛 /home/dracon/Dev/dracon-platform status: clean=false modified=4 staged=0 entries(libgit2)=0
  🐛 /home/dracon/Dev/dracon-platform to_stage=0 to_restore=0
  🔁 synced /home/dracon/Dev/dracon-platform
  ```

  `clean=false` (4 modified) came from **libgit2's `statuses()`**; `to_stage=0`
  came from the **git CLI** (`git diff --name-status -z HEAD` was empty). The
  two disagree — libgit2 reports phantom changes that git CLI does not.

## Root cause (two layers)

### Layer 1 — libgit2 1.9.x ignore mis-evaluation (trigger)

`get_status` counts `modified_files` from libgit2 `repo.statuses()`. On
dracon-platform, libgit2 reports **48 phantom untracked PNGs** +
**4 phantom modified gitlinks** while git CLI reports a clean tree:

- The 48 PNGs live under `web/screenshots/ai-hub-current/`, ignored by
  `web/.gitignore:11: screenshots/`. libgit2 fails to apply that
  directory-ignore rule to new files inside a **tracked subdirectory** of
  the ignored dir (`ai-hub-current/` contains tracked `*.png`). git CLI
  ignores them correctly. Minimal repro (git2 0.21 / libgit2 1.9.x):
  parent with tracked `web/screenshots/ai-hub-current/affiliates.png`
  (added with `-f`), new `web/screenshots/ai-hub-current/1280-keys.png`
  → CLI clean, libgit2 `WT_NEW`.
- The 4 gitlinks (endless-td, polis, deathrun, hellhunter) are flagged
  `WT_MODIFIED` because their nested worktrees contain gitignored content
  (per their own `.gitignore` rules) that libgit2's submodule-status walk
  mis-evaluates — same ignore bug, different shape. git CLI sees the
  gitlink as unchanged (nested HEAD == index gitlink).

The v0.113.46 fix (the `git ls-files --others --exclude-standard` CLI
override) neutralized the phantom **untracked** counts (untracked_files is
replaced by the CLI count and `is_clean` is recomputed) — but the phantom
**WT_MODIFIED** gitlinks still poison `modified_files` → `is_clean=false`.

### Layer 2 — `sync_repo` early return bypassed the push gate (the wedge)

In `sync_repo` (src/sync.rs), the auto-commit block ended with:

```rust
if !to_stage.is_empty() {
    if let Some(outcome) = stage_commit_and_push(...).await? {
        return Ok(outcome);
    }
} else if ... { ... }
return Ok(SyncOutcome::Synced);   // ← early return — handle_ahead_push NEVER reached
```

When the repo was dirty (`is_clean=false`) but nothing was committable
(`to_stage` empty — exactly the phantom-gitlink case; or every dirty file
excluded by `auto_commit_exclude_patterns`), the function returned
`Synced` **without calling `handle_ahead_push`** — so a genuine `ahead > 0`
commit was never pushed. Because the repo stayed phantom-dirty, it was
re-dispatched every cycle, re-entered the same branch, and returned
`Synced` again: a silent, self-perpetuating wedge with a false "synced"
log and a false "pushing Xm" report (`report.rs:3819` derives
`push_status="PENDING"` purely from `ahead > 0` — no push in flight).

`handle_ahead_push` itself was correct (`should_push = ahead > 0 ||
upstream_ref_missing`, v0.113.5 gate); it was simply unreachable on this
path.

## Fix (v0.113.47)

`sync_repo`'s dirty path now falls through to the `handle_ahead_push` gate
instead of returning `Synced` early:

- `to_stage` empty → fall through → `handle_ahead_push` → push when
  `ahead > 0` (or upstream ref missing). Outcome becomes `NothingToDo`
  when there is nothing to push (both map to `ApplyOutcome::Success`; the
  `🔁 synced` log line is intentionally lost for the nothing-to-do case —
  it was a lie anyway).
- `to_stage` non-empty and `stage_commit_and_push` succeeded (`Ok(None)`)
  → still returns `Synced` (the honest "this cycle did work" outcome; the
  fresh status after the push has `ahead=0`, so the extra gate would find
  nothing to do).
- `Blocked` / `PushFailed` / commit-skip `NothingToDo` outcomes unchanged.

Regression test `test_sync_repo_dirty_nothing_to_stage_still_pushes_ahead`
(repo with a tracked file excluded via `auto_commit_exclude_patterns` +
an unpushed commit ahead → asserts the commit is pushed and the excluded
file stays modified-unstaged). Verified: fails on the pre-fix code,
passes post-fix. Workspace: **1238 passed / 9 ignored**, clippy clean,
deny clean.

## Residual issue (follow-up, cosmetic)

The phantom libgit2 counts themselves are NOT fixed in dracon-git: a repo
with tracked files inside an ignored subdir (or gitignored content inside
a submodule worktree) still reports `is_clean=false` to the daemon. After
v0.113.47 the only cost is the repo being dispatched every pulse and a
harmless no-op cycle (the staging path already uses CLI truth). The
user-facing `repos` report was always truthful (CLI-derived). A dracon-git
fix mirroring the ls-files override pattern (override `modified_files`
with the CLI `git diff --name-only HEAD` count when they disagree) is a
candidate follow-up.

## Timeline (live)

| Time (2026-08-09) | Event |
|---|---|
| 00:17:04 | Last successful dracon-platform push (`dcb0ff497`) |
| 00:24:08 | `690d39180` committed by the `dracon` agent (never pushed) |
| 00:24–00:41 | Daemon logs `🔁 synced` every ~40s; no push attempt; report shows "pushing 14m→24m" |
| 00:41 | `DRACON_SYNC_DEBUG=1 once` reveals `clean=false modified=4 entries(libgit2)=0` / `to_stage=0` |
| 00:44 | git2 probe: libgit2 sees 48 WT_NEW + 4 WT_MODIFIED; CLI sees clean |
| 00:46 | Nested submodule sessions reset their HEADs to the parent-tracked SHAs; CLI stays clean, libgit2 still phantom-dirty |
| 00:52 | Root cause pinned (early return bypasses `handle_ahead_push`); fix + regression test land |
| 00:59 | v0.113.47 released; installed; daemon restarted; dracon-platform's `690d39180` pushed within one cycle |

## Runbook

If a repo shows `pushing Xm` with `↑N` but `last: pushed Ym` that keeps
growing while the journal shows `🔁 synced`:

1. `ps aux | grep -E 'git push|ssh'` — if zero push processes, no push is in flight.
2. `git -C <repo> status --porcelain` — if the CLI is clean but the daemon
   still dispatches the repo, suspect phantom libgit2 status (ignore bug).
3. `DRACON_SYNC_DEBUG=1 dracon-sync once` and look for
   `clean=false ... entries(libgit2)=0` + `to_stage=0` — the wedge signature.
4. Update to ≥ v0.113.47, or manually `git push` (the daemon will pick up
   the ref move; `refresh_stale_upstream_ref` self-heals the tracking ref).
