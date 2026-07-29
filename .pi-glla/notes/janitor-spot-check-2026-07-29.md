# Janitor production spot-check — v0.113.10 `auto_prune_stale_backup_branches`

> **Date**: 2026-07-29 18:10 UTC (goal-list item)
> **Feature**: opt-in janitor enabled fleet-wide 2026-07-29 ~06:39 UTC.
> Reaps stale `backup/*` branches + orphaned remote-tracking refs daily,
> bundle-first into `~/dracon/backups/auto-prune/`, every deletion
> `log_warn!`'d with repo/ref/tip/bundle.
> **Observation window**: ~11.5h since enablement (operator resumed the
> goal early — contract said ≥24h / 06:40 UTC 2026-07-30). Daemon
> restarted 11:07 UTC for v0.113.13, so ~7h of post-restart coverage.
> The janitor's per-repo 24h attempts-map resets on restart, and the
> sync loop ticks every ~60s, so **every watched repo has had ≥1
> janitor pass** since the restart — coverage is real despite the
> shorter window.

## Check (1) — journal for janitor deletions

```
journalctl --user -u dracon-sync.service --since 2026-07-29T06:39 \
  | grep -E "janitor|stale daemon branch|auto-prune|bundled"
→ zero matches
```

The 133 bare-🧹 lines in the journal are the **unrelated** "unstaging
N excluded path(s) after commit (edits preserved)" hygiene logs
(junk-runner's `.pi-glla/active.jsonl`) — same emoji, different
subsystem. The janitor's actual log signatures (`janitor: deleted …`,
`stale daemon branch(es)`, bundle paths) appear **zero times**.

## Check (2) — auto-prune bundle dir

```
ls ~/dracon/backups/auto-prune/
→ No such file or directory
```

The dir is only created when candidates exist (first bundling pass),
so its absence independently confirms **zero deletions ever fired**.

## Check (3) — fleet-wide candidate sweep

All **36** daemon-watched repos checked via
`git for-each-ref refs/heads/backup/ refs/remotes/` (orphaned =
remote-tracking ref whose remote is no longer configured):

```
repos checked: 36, candidates: 0
```

Zero `backup/*` branches, zero orphaned remote-tracking refs — the
2026-07-29 fleet cleanup (bundles in
`~/dracon/backups/stale-branch-bundles-20260729/`) holds, and nothing
new has been created by the `rewrite_ahead_paths` repair path since.

## Verdict

**Correct silence.** The janitor is enabled (config check: sync.rs
calls it with `policy.auto_prune_stale_backup_branches`, enabled
fleet-wide), running (per-repo pass executed post-restart), and has
correctly found nothing to do. The operator-review signal is armed:
any future deletion will log_warn with repo/ref/tip/bundle and create
`~/dracon/backups/auto-prune/<slug>-<ts>.bundle` before deleting.

Follow-up: none required. If a `backup/*` branch ever appears, expect
it gone within 24h with a bundle + journal line — investigate the
journal, not the branch list.
