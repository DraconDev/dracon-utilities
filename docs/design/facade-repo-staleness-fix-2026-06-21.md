# Façade repo staleness fix — 2026-06-21

## Summary

The 3 github façade repos (`dracon-sync-background-auto-commit-multi-remote`,
`dracon-warden-secret-encrypt-age-git-filter`, `dracon-system-disk-process-guard-doctor`)
were 5 days stale (last push 2026-06-16) while the monorepo was actively
updated through the v0.112.12 release cut (2026-06-21). They now update
individually, each on its own daemon-driven push cycle.

## Root cause: 3 missing layers

The façade-update pipeline had three independent gaps, each of which
was sufficient to break propagation:

1. **No local working trees.** `/home/dracon/Dev/facade-repos/`
   did not exist. The facades were public github/gitlab/codeberg
   repos but had no on-disk clones. So `regenerate_facade_repos.py`
   couldn't write to them and the daemon couldn't watch them.

2. **No post-commit hook.** `dracon-utilities/.git/hooks/post-commit`
   did not exist. So monorepo commits never triggered the regen
   script. (The hook *specification* existed in
   `scripts/release.sh --install-hook` and in the regen script's
   docstring, but was never actually installed.)

3. **The daemon wasn't watching the facade-repos root.** Even if
   the regen script wrote new content, the daemon's `watch_roots`
   in `~/.dracon/utilities/sync/dracon-sync.toml` didn't include
   `/home/dracon/Dev/facade-repos/`, so the daemon wouldn't
   auto-commit or auto-push the regenerated trees.

Additionally, two latent script bugs surfaced during the fix:

4. **`scripts/scaffold_feature_repos.py` had a corrupted
   `dracon-system` long-name.** The `UTILITIES` dict entry for
   `dracon-system` had its `name` field set to
   `dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+...]`
   — a 926-character string with an inline warden secret
   annotation. This was a self-referential warden false-positive
   where the long repo name (which contains the substring "di")
   tripped the filter and got a "secret" annotation injected. The
   correct name is `dracon-system-disk-process-guard-doctor`.

5. **`scripts/regenerate_facade_repos.py` had the same corrupted
   string in its own `UTILITY_LONG_NAMES` lookup.** It would have
   crashed with `OSError: [Errno 36] File name too long` on any
   regen attempt for the system facade.

Both bugs were fixed by replacing the corrupted strings with
the canonical 3-keyword names: `dracon-sync-background-auto-commit-multi-remote`,
`dracon-warden-secret-encrypt-age-git-filter`,
`dracon-system-disk-process-guard-doctor`.

## The fix

```
# 1. Scaffold the 3 working trees
python3 scripts/scaffold_feature_repos.py \
    --apply --init-git \
    --target-root /home/dracon/Dev/facade-repos

# 2. Reset local to remote (since the public mirrors have 4 commits
#    of legitimate history that the fresh scaffold would have duplicated)
for facade in /home/dracon/Dev/facade-repos/*/; do
    cd "$facade"
    git fetch github
    git reset --hard github/main
done

# 3. Install the post-commit hook
cat > /home/dracon/Dev/dracon-utilities/.git/hooks/post-commit << 'HOOK'
#!/bin/sh
exec python3 "$(git rev-parse --show-toplevel)/scripts/regenerate_facade_repos.py"
HOOK
chmod +x /home/dracon/Dev/dracon-utilities/.git/hooks/post-commit

# 4. The daemon auto-discovers the new repos under /home/dracon/Dev
#    (the daemon's watch_roots includes /home/dracon/Dev).
#    No daemon config change needed.
```

After running step 1-3, the daemon's `repos` output goes from
12 repos to 15 repos (12 existing + 3 façade). After step 4
plus a `regenerate_facade_repos.py --all` trigger, all 3 façade
github repos have `pushed_at` within minutes (verified at
2026-06-21T09:56:19Z, 2026-06-21T09:56:50Z, 2026-06-21T09:56:49Z).

## Verification

End-to-end pipeline confirmed working at 2026-06-21T09:56:

```
$ for repo in dracon-sync-background-auto-commit-multi-remote \
              dracon-system-disk-process-guard-doctor \
              dracon-warden-secret-encrypt-age-git-filter; do
    d="/home/dracon/Dev/facade-repos/$repo"
    for remote in github gitlab codeberg; do
        local_sha=$(cd "$d" && git rev-parse HEAD)
        remote_sha=$(cd "$d" && git ls-remote $remote | grep refs/heads/main | awk '{print $1}')
        echo "$repo @ $remote: $([ "$local_sha" = "$remote_sha" ] && echo SYNCED || echo DIVERGED)"
    done
done

dracon-sync-background-auto-commit-multi-remote @ github: SYNCED
dracon-sync-background-auto-commit-multi-remote @ gitlab: SYNCED
dracon-sync-background-auto-commit-multi-remote @ codeberg: SYNCED
dracon-system-disk-process-guard-doctor @ github: SYNCED
dracon-system-disk-process-guard-doctor @ gitlab: SYNCED
dracon-system-disk-process-guard-doctor @ codeberg: SYNCED
dracon-warden-secret-encrypt-age-git-filter @ github: SYNCED
dracon-warden-secret-encrypt-age-git-filter @ gitlab: SYNCED
dracon-warden-secret-encrypt-age-git-filter @ codeberg: SYNCED

$ dracon-sync repos
📦 15 repos  ✅ OK 13  ⚠️ WARN 2  ❌ CONCERN 0
   (the 3 facade repos each appear as their own ✅ OK row,
    with PUSH=OK, healthy)
```

The 2 WARN rows in the final state are pre-existing and unrelated
to this fix: `browser-extensions-shared` (transient dirty) and
`dracon-strategy` (pre-existing stalled state from goal `83879dd1`).

## How to prevent recurrence

1. **The post-commit hook is now installed.** Every future
   monorepo commit will trigger `regenerate_facade_repos.py`,
   which writes any changed files to the matching façade working
   trees. The daemon (watching `/home/dracon/Dev`) auto-commits
   and auto-pushes each façade independently.

2. **Future releases should use `--install-hook`.** The
   `scripts/release.sh --install-hook` flag was already implemented
   and documented but never run during the v0.112.12 cut. Running
   `scripts/release.sh <next-version> --yes --install-hook` would
   have prevented this staleness entirely.

3. **The script-level corruption bug (root cause #4 and #5 above)
   is now fixed.** The UTILITIES dicts in both
   `scaffold_feature_repos.py` and `regenerate_facade_repos.py`
   have the correct 3-keyword names. A regression here would
   surface immediately because the regen script crashes loudly
   on a too-long path. (No regression test exists; consider
   adding one as a follow-up.)

4. **The `mirror-divergence-and-secret-remediation-2026-06-21.md`
   runbook still applies** to the monorepo's gitlab/codeberg
   mirrors (which are still 14 commits behind github due to the
   literal-token incident). The façade repos are NOT affected by
   that divergence — they synced cleanly because their
   working-tree-to-remote path is the standard
   local-commit-then-push pipeline, which the daemon runs
   independently of the monorepo.

## Reference

- `docs/design/release-process-2026-06-21.md` — the release flow
  that produced the v0.112.12 release and the regen trigger.
- `docs/design/mirror-divergence-and-secret-remediation-2026-06-21.md` —
  the monorepo gitlab/codeberg divergence runbook (separate
  concern).
- `docs/design/dracon-sync-repos-vs-vscode-discrepancy-2026-06-21.md` —
  the PUSH_STUCK discrepancy that was also resolved during this
  session.
- `scripts/scaffold_feature_repos.py` `--apply --init-git` —
  the scaffold command that creates the working trees.
- `scripts/regenerate_facade_repos.py` — the regen script that
  copies changed monorepo files to the matching façade trees.
- `scripts/release.sh --install-hook` — installs the
  `.git/hooks/post-commit` hook that triggers regen.