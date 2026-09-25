# Internal high-water blockers

Observed while the daemon remained frozen on 2026-09-25. These are internal policy/structural blockers, not external provider outages.

The bucket policy in `web/config/bucket-strategy.json` is fail-closed and sets the governing GitHub ceiling to `2,147,483,648` bytes. The linked-worktree `GIT_DIR` defect was fixed and covered by 22 passing hook/strategy tests, but normal hooks then returned `BUCKET_STRATEGY_HIGH_WATER` with `forwardOnly.ok = true` for these repositories:

| Repository | Exact projected bytes | Limit | Remote main before/after | Result |
|---|---:|---:|---|---|
| `/home/dracon/Dev/dracon-platform` | `28,594,143,289` | `2,147,483,648` | `a3420c4f5bb935d9c4fbd98b0ccad0e21fe16b51` / unchanged | blocked on GitHub and GitLab |
| `/home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls` | `3,509,532,095` | `2,147,483,648` | `6721c2ef05093ca5321b03447cd1bfdaf4570ff4` / unchanged | blocked on GitHub and GitLab |
| `/home/dracon/Dev/dracon-platform/web/games/wip/deathrun` | `2,367,499,965` | `2,147,483,648` | `c1a1fb22530c7e9e32a3c9b396712be433ff5eb6` / unchanged | blocked on GitHub and GitLab |

The parent `main` alone has roughly `25.96 GB` of reachable raw object data. Deleting large paths in a normal forward commit would not remove their historical blobs from reachable history, so it cannot bring the complete reachable graph below the configured ceiling. Convergence would therefore require a prohibited history rewrite/force-update or a policy change that weakens the guard. Neither was attempted.

Evidence:

- `commands/push-snapshot-committed-heads.log`
- `commands/push-linked-worktree-heads.log`
- `commands/parent-linked-hook-env-fix.patch`
- `commands/bucket-hook-regression.log`

GitLab printed a free-storage-limit notice during some successful pushes, but those pushes succeeded and the advertised refs changed as expected; storage notice text is not classified as an external blocker here.
