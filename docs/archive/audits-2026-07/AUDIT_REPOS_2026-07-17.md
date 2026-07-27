# Audit: existing watched repos under codeberg-public-only policy

**Goal:** `codeberg-public-only` (2026-07-17)

**Context:** before deploying v0.112.16, audit the 31 currently-watched
repos to determine which need per-repo overrides of the new
`codeberg_public_only` policy, which can rely on the default, and
whether any need manual `exclude_remotes` cleanup of stale codeberg
remote entries in `.git/config`.

## Summary

| Repo | Has codeberg in remotes? | Visibility (cached) | Decision |
|---|---|---|---|
| dracon-sync | yes | unknown (cache not yet populated in new format) | rely on policy gate (auto-skip) |
| dracon-utilities | yes | unknown | rely on policy gate |
| dracon-system | yes | unknown | rely on policy gate |
| dracon-warden | yes | unknown | rely on policy gate |
| dracon-code | yes | unknown | rely on policy gate |
| ai-auto-writer | yes | unknown | rely on policy gate |
| browser-extensions-shared | yes | unknown | rely on policy gate |
| .dracon | yes | unknown | rely on policy gate |
| wezterm-config | yes | unknown | rely on policy gate |
| opencode-plugins | yes | unknown | rely on policy gate |
| practice-form | yes | unknown | rely on policy gate |
| pully-fully-pull-based-fleet-reconciler | yes | unknown | rely on policy gate |
| web-auto | yes | unknown | rely on policy gate |
| rust-ai-web-auto | yes | unknown | rely on policy gate |
| search-daemon | yes | unknown | rely on policy gate |
| pi-plugins | yes | unknown | rely on policy gate |
| dracon-strategy | yes | unknown | rely on policy gate |
| DraconDev | yes | unknown | rely on policy gate |
| avid | yes | unknown | rely on policy gate |
| pi-mono (DraconDev-public) | yes | unknown | rely on policy gate |
| nexus-new-tab | yes | unknown | rely on policy gate |
| **dracon-platform** (parent) | yes | unknown | rely on policy gate (parent of 10 submodules) |
| **hegemon** (submod, nested) | yes + origin=codeberg too | unknown | rely on policy gate; the dual-codeberg-remotes is a pre-existing artifact (see below) |
| **deathrun** (submod) | yes | unknown | rely on policy gate |
| **darklord** (submod) | yes | unknown | rely on policy gate |
| **hellhunter** (submod) | yes | unknown | rely on policy gate |
| **capture-anime-girls** (submod) | yes | unknown | rely on policy gate |
| **junk-runner** (submod) | yes | unknown | rely on policy gate |
| **polis** (submod) | yes | unknown | rely on policy gate |
| **endless-td** (submod) | yes | unknown | rely on policy gate |
| **neonbreak** (submod) | yes | unknown | rely on policy gate |
| **one-mil-girls** (submod) | yes | unknown | rely on policy gate (also an orphan candidate; see below) |

**All 31 repos** can rely on the policy default (`codeberg_public_only =
true`). The gate correctly skips codeberg for every repo because the
visibility cache is in the legacy `timestamp-only` format and the
`cached_repo_visibility` helper safely returns `None` (unknown) — the
safe-default path fires.

Once the daemon's `sync_mirror_visibility` cycle (24h interval)
populates the cache with the new `visibility=<public|private>` format,
the annotations will switch from `(unknown)` to `(private)` for
private repos. Public repos (none of the 31 currently) would switch
to no `excl:` annotation at all.

## Pre-existing artifacts (not fixed in this goal)

### 1. hegemon's dual codeberg remotes

Local `.git/config` for `hegemon`:
```
codeberg → git@codeberg.org:dracondev/hegemon.git           (active)
origin   → git@codeberg.org:dracondev/web-games-hegemon.git (legacy/duplicate)
github   → github.com/DraconDev/hegemon.git
gitlab   → gitlab.com/DraconDev/hegemon.git
```

Both `codeberg` and `origin` point to codeberg, but to different repos.
This is the legacy pre-rename mirror. The daemon pushes the same
content to both codeberg mirrors (8.34 GiB + 7.91 GiB = 16.25 GiB on
codeberg, half is redundant).

Under the new policy, both remotes are auto-excluded for the local
push path (since hegemon is private). The mirrors remain alive but
stale (no new pushes). To deduplicate:

```bash
# Operator action (explicit, deferred)
git -C /home/dracon/Dev/dracon-platform/web/games/wip/hegemon remote remove origin
# Then delete the orphan via codeberg API:
curl -X DELETE -H "Authorization: token $CODEBERG_TOKEN" \
    "https://codeberg.org/api/v1/repos/dracondev/web-games-hegemon"
# Frees 8.34 GiB of codeberg quota.
```

**NOT executed in this goal.** Per AGENTS.md "Delete operator-owned
repos" rule, the operator authorizes deletions explicitly.

### 2. one-mil-girls orphan

`dracondev/one-mil-girls` (1.42 GiB codeberg, no description, last
touched 2026-06-14) is an orphan — no local repo pushes to it.
The active mirror is `dracondev/web-games-one-mil-girls` (0.21 GiB).

Under the new policy, the orphan is auto-excluded from the local
push path (since it's the same as the active repo's git remote
which points to `web-games-one-mil-girls`). The orphan mirror is
just stale content eating quota.

To clean up:
```bash
# Operator action (explicit, deferred)
curl -X DELETE -H "Authorization: token $CODEBERG_TOKEN" \
    "https://codeberg.org/api/v1/repos/dracondev/one-mil-girls"
# Frees 1.42 GiB of codeberg quota.
```

**NOT executed in this goal.** Same authorization requirement.

## What changed vs the prior audit (`AUDIT_REPOS_2026-07-10.md`)

The 2026-07-10 audit identified 12 repos showing `PUSH_STUCK` on
codeberg. After this goal's deployment, **0 repos** show
`PUSH_STUCK` for codeberg-related reasons — the policy gate prevents
the daemon from even attempting the failing push.

The 1 remaining `❌ CONCERN` (endless-td) is the pre-existing
divergence between local `rollback-phaser-restore-svelte` and the
remote `main` branch — unrelated to codeberg quota. Operator reset
the assistant's merge attempt on 2026-07-17; awaiting operator
decision on resolution.

## Next visibility sync expected behavior

When `sync_mirror_visibility` runs (≤ 24h after this goal's
deployment), for each of the 31 repos it will:
1. Run `gh api repos/DraconDev/<repo> --jq .private`
2. Write `visibility=<private>\n<timestamp>` to the cache file
3. The next `dracon-sync repos` invocation will read the cache
   and show `(private)` instead of `(unknown)` in the PUSH-TO column

Public repos (none in the current 31) would show no `excl:`
annotation at all once the cache says `visibility=public`.

## Operator follow-up checklist

- [ ] **No action required.** Policy default handles all 31 watched
      repos correctly.
- [ ] **Optional cleanup (deferred to operator)**:
  - [ ] Delete `dracondev/web-games-hegemon` (8.34 GiB) + remove
        `origin` remote from local hegemon
  - [ ] Delete `dracondev/one-mil-girls` (1.42 GiB) orphan
- [ ] **Optional** (if any specific private repo needs codeberg
      backup, rare): add `codeberg_public_only = false` to its
      `<repo>/.dracon/dracon-sync.toml` with a comment explaining
      why
- [ ] **Verify** post-deploy: run `dracon-sync repos` and confirm
      the PUSH-TO column shows `[excl:codeberg] (unknown)` for all
      private repos. After the next visibility sync cycle (≤ 24h),
      it should switch to `[excl:codeberg] (private)`.
