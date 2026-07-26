# FilterOnly push starvation + stale upstream display (v0.113.1, 2026-07-26)

## Symptom

`dracon-sync repos` showed junk-runner as `🟣 pushing 240m · ↑19 · PENDING`
for 4+ hours while the daemon demonstrably committed/pushed every other
repo. A manual `git push gitlab main` succeeded instantly — remotes,
ssh, and branch protection were all fine.

## Root cause (two interlocking bugs)

1. **`filter_only_cleared` early-returned before the push phase**
   (`sync.rs`). junk-runner's loop agent rewrites
   `.pi-glla/active.jsonl` every ~15s; the file is tracked + gitignored
   and always diffs as filter-only noise. Every daemon cycle:
   dispatch → FilterOnly → 300s stage cooldown (silent at default log
   level) → push phase never reached. Already-committed work piled up
   unpushed **indefinitely** — 19 commits over 10h, while `sync-now`
   printed only "filter-only dirty (nothing real to commit)".

2. **Stale `refs/remotes/origin/main` made the report lie.** The
   daemon pushes to named mirror remotes (github/gitlab); junk-runner's
   `origin` shares the gitlab URL, but pushing `gitlab` does not update
   the `origin` tracking ref. libgit2 computed ahead=19 against the
   stale ref forever, so the report showed "pushing 240m" long after
   gitlab was actually current — the "240m" was just the last-commit
   timestamp, not an in-flight push.

## Fix (v0.113.1)

- The FilterOnly path now runs `handle_ahead_push` before returning
  (cheap local no-op when nothing is pending; the 300s cooldown still
  bounds never-converging repos to one push attempt per 5 min).
- New `refresh_stale_upstream_ref`: after a successful push, bounded
  `git fetch <upstream-remote>` — but only when the tracking ref
  actually disagrees with HEAD (zero network cost when converged).

## Verification

Post-deploy: daemon committed + pushed junk-runner's 20 commits to
github + gitlab (both at HEAD `874196b6`), the origin tracking ref
converged to HEAD, and the report row flipped from `pushing 240m ↑19
PENDING` to `dirty 0m ✅ OK`. Regression test:
`test_refresh_stale_upstream_ref_converges`.

## Lesson for future "pushing Nm" reports

`pushing Nm` = PENDING status + last-commit age. It does NOT prove a
push is in flight. The diagnostic sequence that worked: journal grep
for the repo (silence = daemon not processing it) → `sync-now`
(filter-only reveal) → compare `refs/remotes/<named-mirror>` vs
`refs/remotes/origin` (staleness reveal).
