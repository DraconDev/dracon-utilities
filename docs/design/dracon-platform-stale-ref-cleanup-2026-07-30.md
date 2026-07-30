# dracon-platform stale-ref cleanup (2026-07-30)

**Trigger**: operator asked why dracon-platform's own pack was 12 GiB
(v0.113.20 SIZE cell `12G+7.3G`) when the games had been split into
submodules precisely to avoid one huge repo.

## Findings

- main alone: 71,864 objects; ALL refs: 349,534 → ~80% pinned by
  dead refs, not main.
- **13 stale branches** (phase-0..4 workstreams, archive/azumi-final,
  auth-email-otp, cline/checkpoints/*, refs/annex/last-index),
  Mar–Jun 2026, pre-submodule-split asset-heavy history, present on
  GitHub too (gitlab had only main).
- **Hidden pinner after first gc**: HEAD/worktree reflogs (90-day
  expiry) + a stale `/tmp/prepolish` worktree (detached HEAD).
  `git gc --prune=now` alone was a no-op (345k objects retained).
- **332 pre-split version tags** (v0.7.1241 … v47.261.0, incl. ~120
  auto-incremented v47.139.x noise tags) pinned another 144,509
  objects / ~5 GiB after the branches were gone.

## Actions (operator-approved, janitor pattern)

1. Bundled all stale refs →
   `~/dracon/backups/stale-branch-bundles-20260730/`
   (`dracon-platform-stale-refs.bundle` 11G, `-tags.bundle` 6.1G,
   both `git bundle verify` = complete history).
2. Deleted branches + tags locally and on GitHub/gitlab via
   `DRACON_ALLOW_REWRITE=1 git push <remote> --delete` (warden
   pre-push escape hatch; dead-ref deletion, main never touched).
3. Removed `/tmp/prepolish` worktree; `git reflog expire
   --expire=now --all`; two `git gc --prune=now` passes.

## Result

- in-pack 345,148 → **71,864** (exactly main's object count)
- size-pack 12,071,073 KiB → **321,863 KiB (314 MiB)**, −97.4%
- SIZE cell: `12G+7.3G` → `314M+7.3G`
- main's ~1.5 GiB blob churn (1.24 GiB JSON + 297 MiB PNG + 4×22 MB
  FBX) delta-compresses to 314 MiB.

## Lessons

- `git gc` is NOT sufficient for dead-history cleanup: reflogs and
  stale worktrees pin everything. Sequence must be: delete refs →
  remove stale worktrees → `reflog expire --expire=now --all` → gc.
- Old version tags are as heavy as dead branches; check
  `git rev-list --objects --tags --not main | wc -l` when a repo's
  pack seems oversized.
- The daemon's auto-gc (`auto_gc_garbage_threshold_bytes`) only
  handles DANGLING garbage (tmp_pack_*); it cannot see
  ref-pinned dead history. Manual/operator-approved cleanup remains
  the tool for this class.
