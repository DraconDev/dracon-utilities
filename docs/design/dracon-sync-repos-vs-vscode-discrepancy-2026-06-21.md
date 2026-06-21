# `dracon-sync repos` vs. VS Code "looks fine" discrepancy — 2026-06-21

## Summary

After the v0.112.12 release cut, `dracon-sync repos` showed
`dracon-utilities` as `⚠ WARN` with `PUSH_STUCK (80 failures)` and
the hint `run repair-concerns --apply`. Opening the same repo in VS
Code Source Control "looked fine" — clean tree, in sync with
`github/main`, no errors. The discrepancy is real, not a bug: VS
Code's view and `dracon-sync`'s view are answering different
questions.

## Root cause

The `PUSH_STUCK` counter is **stale**. It accumulated over a
~30-minute window during the v0.112.12 cut when pushes failed
repeatedly for two reasons:

1. **GitHub push protection rejected pushes that contained the
   literal crates.io token.** This was a transient state — the
   token was redacted from local files, but the daemon kept
   trying to push the historical commits to github (and
   sometimes succeeded after the local tree caught up).

2. **`gitlab` and `codeberg` mirrors are 14 commits behind
   `github/main`.** Those mirrors carry a side-branch from
   earlier in the release-flow work that included the literal
   token in `.pi-tmp/release-goal-blocker-questions.md`. The
   daemon tried to push and got `non-fast-forward` every time,
   incrementing the failure counter.

After the release completed and local caught up with `github/main`,
the counter did NOT clear. The daemon only resets the counter on
a *successful* push, and there was nothing left to push
(`ahead = 0` on `github/main`). But the daemon kept
retry-pushing and kept failing on gitlab/codeberg, accumulating
more failures.

The state at the time of the discrepancy:

```
$ git status --short
(empty)

$ git log github/main..HEAD --oneline
(empty)

$ git log HEAD..gitlab/main --oneline | wc -l
14

$ dracon-sync repos
... ⚠ WARN  | dracon-utilities | PUSH_STUCK | push-stuck 16m
...            80 failures: git push returned non-zero
...            → run repair-concerns --apply
```

## Why VS Code "looks fine"

VS Code's Source Control view calls `git status` and checks the
configured upstream (`branch.<name>.remote` /
`branch.<name>.merge`). It does not run the daemon's full push
pipeline, does not check the daemon's push-attempt counter, and
does not see the per-remote state. From VS Code's perspective,
the repo is clean and `main` is in sync with `github/main` —
everything is fine.

The daemon, by contrast, tracks:

- Per-remote push status (gitlab, codeberg, github) as
  *separate* concerns, not aggregated.
- A rolling push-failure counter per repo. The counter is
  reset only when a successful push is recorded.
- A `stuck-repos` registry (24h expiry) for repos that have
  exceeded a failure threshold.

VS Code is reporting git-level health. The daemon is reporting
daemon-level health. Both are "correct" for what they measure;
the difference is that the daemon's view carries state from
earlier failures that git itself no longer reflects.

## The fix

`dracon-sync repair stuck-unstuck <repo-path>` — the unstuck
command clears the stale entry from the daemon's stuck-repos
registry. After running it:

```
$ dracon-sync repair stuck-unstuck /home/dracon/Dev/dracon-utilities
🔓 unstuck: /home/dracon/Dev/dracon-utilities

$ dracon-sync repair stuck-list
✅ no stuck repos

# (after one daemon cycle)

$ dracon-sync repos
📦 12 repos ✅ OK 12 ⚠️ WARN 0 ❌ CONCERN 0
   ✅ OK  dracon-utilities   PUSH=OK  healthy
```

No daemon code change needed. The fix is an operator-actionable
command that already existed (`repair stuck-unstuck`) — the
goal-resolution path was just undiscovered.

## Why the operator hint didn't say this directly

The daemon's hint for `PUSH_STUCK` rows is generic:
`run repair-concerns --apply`. That's because the underlying
concern (`PUSH_STUCK`) is detected by a counter, and
`repair-concerns` knows how to re-attempt the failed push.
But when the failure is **non-fast-forward on a divergent
mirror**, re-attempting the push doesn't help — the push will
keep failing until the mirror is brought back into sync.

The hint should ideally distinguish:
- "real push failure, retry might succeed" → `repair-concerns --apply`
- "mirror divergence, push will keep failing" → `git pull` the
  divergent remote, or use `stuck-unstuck` after the divergence
  is resolved

This is a daemon UX improvement, not a bug. The current
behavior is correct (don't push to a divergent mirror) but the
hint is too generic.

## Preventing recurrence in future release cuts

Two concrete suggestions:

### A. Make `scripts/release.sh` self-verify after the cut

After step 6d (create GitHub release), the script could run:

```bash
dracon-sync repair stuck-unstuck "$MONOREPO_ROOT" 2>/dev/null || true
```

This is harmless if there's no stuck state (it just prints
`✅ no stuck repos`), and it clears a stale counter that the
release-cut window inevitably produces. The script's job is to
leave the workspace in a known-good state; the release cut
itself is what creates the stuck-repos entry.

### B. Detect mirror divergence as a separate concern

The daemon already has `mirror-only` repos as a recognized
category (per `mirror-only-push-and-empty-repo-remotes-2026-06-20.md`).
For non-mirror-only repos where a mirror is divergent, the
daemon could:

1. Detect divergence via `git fetch <remote>` + `git log
   HEAD..<remote>/<branch>`.
2. Report it as a separate `MIRROR_DIVERGED` concern (not a
   `PUSH_STUCK`).
3. Hint: `git pull <remote>` (or the operator's preferred
   resolution per `mirror-divergence-and-secret-remediation-2026-06-21.md`).

This would prevent the stuck-repos counter from accumulating
failures for divergence reasons that aren't going to be
resolved by retrying the push.

## Current state at time of this doc

- `dracon-sync repos`: 12 OK / 0 WARN / 0 CONCERN.
- `dracon-utilities`: `PUSH=OK`, `healthy`.
- v0.112.12 still on crates.io and GitHub.
- `gitlab` and `codeberg` mirrors: still 14 commits behind
  `github/main`. This is a separate concern with its own design
  doc (`mirror-divergence-and-secret-remediation-2026-06-21.md`)
  and three operator remediation paths.

## Reference

- `docs/design/release-process-2026-06-21.md` — the release
  flow design doc that produced this state.
- `docs/design/mirror-divergence-and-secret-remediation-2026-06-21.md` —
  the gitlab/codeberg divergence runbook.
- `docs/design/mirror-only-push-and-empty-repo-remotes-2026-06-20.md` —
  the existing mirror-push classification.
- `AGENTS.md` "Commit policy" — the auto-commit behavior that
  contributed to the counter accumulating.
- `dracon-sync repair stuck-unstuck --help` — the command that
  fixed the discrepancy.