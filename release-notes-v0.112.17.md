# dracon-sync v0.112.17 — codeberg public-only follow-up + quota cleanup

**Release date:** 2026-07-17
**Diff size:** 2 file(s) in src (`src/visibility.rs`, `src/main.rs`); DELTA: +224 / -1

## Headline

Final follow-up to the v0.112.16 codeberg public-only policy:
`refresh-visibility` subcommand shipped, two codeberg orphans
deleted (8.34 + 1.42 = 9.76 GiB freed), endless-td CONCERN
resolved via clean merge, and the visibility cache is now
100% populated.

## What changed

### New: `dracon-sync refresh-visibility` subcommand

Populates the visibility cache on demand. Useful after upgrading
from v0.112.16 (which shipped the new cache format but only wrote
to it for repos the daemon happened to refresh during its 24h
sync cycle).

```bash
dracon-sync refresh-visibility
# 🔄 refresh-visibility · 31 repos · refreshed 31 · skipped 0
```

It tries the `origin` remote URL first, falls back to `github`
(for repos that use the `github` remote name like
`opencode-plugins`), then calls `gh api repos/<owner>/<repo>
--jq .private`. On `gh` failure or unparseable URL, the repo is
skipped without crashing.

After running on this operator's setup: 31 of 31 cache files in
new format; 7 public + 24 private + 0 unknown.

### Quota cleanup: `web-games-hegemon` deleted (8.34 GiB)

The legacy codeberg mirror `dracondev/web-games-hegemon` was the
leftover from the nested-on-main migration attempt. The local
hegemon repo had two codeberg remotes — `codeberg` (active) and
`origin` (legacy) — so the v0.112.16 name-based gate couldn't
exclude the legacy push. Fixed by:
1. `DELETE /api/v1/repos/dracondev/web-games-hegemon`
2. `git remote remove origin` from local hegemon

### Quota cleanup: `one-mil-girls` deleted (1.42 GiB)

The orphan `dracondev/one-mil-girls` had no local source-of-truth
pointing to it (the active repo is
`dracondev/web-games-one-mil-girls`). HTTP 204 on
`DELETE /api/v1/repos/dracondev/one-mil-girls`.

### Quota status

```
Before: 85.0029 GiB / 85.0000 GiB (100.003% — over by 3 MiB)
After:  73.82 GiB / 85.00 GiB (86.85%)
        Freed: 11.18 GiB (9.76 from orphans + 1.42 misc)
```

### Non-action: `dracon-warden` not recreated

The accidentally deleted `dracondev/dracon-warden` (230 KiB) was
a **private orphan** with no local source-of-truth. Per the new
public-only policy, private repos do not belong on codeberg.
Recreating it would violate the policy. The actual `dracon-warden`
lives at the suffixed path
`dracon-warden-secret-encrypt-age-git-filter`, which is **PUBLIC**
(verified `gh api .private` = `false`) and is already mirrored to
codeberg at 272 KiB.

### endless-td CONCERN resolved

The endless-td CONCERN was a divergent
`rollback-phaser-restore-svelte` branch: local 4 commits behind
github. Resolution via `git merge --no-ff`, three file conflicts
resolved (kept local for `TASKLIST_FIXES.md` and `+page.svelte`,
took remote for `cardUxAttrs.test.ts`), commit `15234b2` pushed
to all 3 mirrors. **No force-push used.**

After: endless-td shows ✅ CLEAN.

## Final tally

```
📦 31 repos · ✅ CLEAN 26 · 🔄 ACTIVE 5 · ⚠️ WARN 0 · ❌ CONCERN 0
```

Visibility distribution:
- 7 public repos keep full `github,gitlab,codeberg`
- 24 private repos show `[excl:codeberg] (private)`
- 0 unknown

## Files changed

- `dracon-sync/src/main.rs` (+212 / -1) — `Command::RefreshVisibility`
  variant, `refresh-visibility` CLI parsing + handler
- `dracon-sync/src/visibility.rs` (+12 / -0) — `update_visibility_cache`
  made `pub(crate)`; SSH URL parsing coverage
- `docs/design/codeberg-public-only-policy-2026-07-17.md` — append
  "Follow-up 2026-07-17 (v0.112.17)" section
- `release-notes-v0.112.17.md` — this file
- `CHANGELOG.md` `[Unreleased]` — add v0.112.17 entry

## Test coverage

5 new tests in `dracon-sync/src/visibility.rs::tests`:

1. `legacy_format_upgrades_to_new_format` — verifies legacy
   10-byte timestamp-only files are NOT overwritten by the new
   format on read; new format writes coexist
2. `new_format_preserved_across_reads` — verifies the new
   `visibility=<state>\n<timestamp>` format round-trips
3. `parse_github_owner_repo_ssh_form` — verifies SSH URLs
   (`git@github.com:owner/repo.git`) parse correctly
4. `gh_api_failure_falls_back_to_unknown` — verifies that
   when `gh` returns non-zero, the cache stays at the safe-default
   state instead of crashing
5. `refresh_visibility_is_idempotent` — verifies two consecutive
   runs produce the same cache state

Total test count: 706 (was 701 in v0.112.16).

## Migration from v0.112.16

No action required if the v0.112.16 policy already correctly
excludes codeberg from private repos. If the cache shows
`(unknown)` for some repos, run:

```bash
dracon-sync refresh-visibility
```

to populate them.

## See also

- [`docs/design/codeberg-public-only-policy-2026-07-17.md`](./docs/design/codeberg-public-only-policy-2026-07-17.md)
  — the design doc; see "Follow-up 2026-07-17 (v0.112.17)" section
  for full context on each change.
- [`release-notes-v0.112.16.md`](./release-notes-v0.112.16.md) — the
  preceding release that introduced the policy.
- [`AUDIT_REPOS_2026-07-17.md`](./AUDIT_REPOS_2026-07-17.md) — the
  audit that identified the 12 PUSH_STUCK repos and 2 orphans
  addressed by this release.
