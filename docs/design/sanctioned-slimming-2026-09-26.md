# Sanctioned slimming — operator history rewrites for size (2026-09-26)

## Why this exists

Under commit-all + never-rewrite, reachable history is monotonically
non-decreasing: deleting a huge file from the worktree stops new growth
but leaves every committed byte counting against the 2 GiB guard
forever. Worktree deletion without history rewrite is not a size fix.
The 2026-07-25 no-rewrite rule was written about *coordination* (loops
racing the daemon's seconds-fast pushes), not size — this document adds
the missing third path: operator-executed, fully coordinated slimming.

## What it is not

- Loops/agents working in watched repos MUST still never rewrite
  history (see AGENTS.md "Agent loops MUST NOT rewrite history").
  They cannot coordinate, hold a pause, or verify a cutover.
- It is not a substitute for bucket-overflow: slimming fixes the
  past, overflow keeps bulky regenerable content out of history in
  the future. The `*.sqlite`/`*.db` class (auto-committed up to
  100 MB by policy) is overflow's first bite point.

## When it applies

A repo breaches (or is trending into) the 2 GiB bucket-guard policy
estimate AND the bloat is an excisable class (regenerable dumps,
superseded media, loop scratch) — never shipped product assets,
unless the operator explicitly reclassifies them.

## Procedure (all steps, in order)

1. **Bundle backup**: `git bundle create
   ~/dracon/backups/<repo>-pre-slim-YYYYMMDD.bundle --all`, then
   `git bundle verify`. No rewrite without a verified bundle.
2. **Scratch rewrite**: clone to `~/dracon/conv-work/<repo>`
   (outside daemon watch roots), run
   `git filter-repo --invert-paths --path <class>... --force`.
   Dry-run first on unfamiliar specs.
3. **Guard-verify the rewritten history**: run
   `web/scripts/bucket-strategy-guard.sh` against the scratch tip.
   Policy estimate must read under 2 GiB or stop here.
4. **Cutover in one maintenance window**:
   `dracon-sync maintenance --` push the rewritten tip with
   `--force-with-lease=<old>:<new>` to github, gitlab, and origin
   in the same window. Preconditions: GitLab `main` unprotected
   for the window (re-protect after), `DRACON_ALLOW_REWRITE=1`
   past the warden hooks.
5. **Gitignore excised paths** in the live worktree BEFORE resume,
   so the daemon does not recommit the bloat on its next cycle.
6. **Re-verify**: guard + `repos` green on the live checkout, all
   three remotes agreeing. Every other clone of the repo must be
   re-cloned — a missed clone reintroduces old objects on push.
7. **Record**: one short note (what was excised, old→new tip SHAs,
   bundle path) appended below.

## Log

- (none yet — CAG 4-class slim is the first candidate, pending
  operator spec confirmation as of 2026-09-26.)
