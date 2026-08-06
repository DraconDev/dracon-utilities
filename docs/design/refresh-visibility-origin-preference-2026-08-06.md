# refresh-visibility: prefer `github` remote over `origin` (v0.113.43)

**Date**: 2026-08-06
**Author**: pi-agent + operator
**Release**: v0.113.43
**Severity**: medium (cache poisoning, visible UX bug)
**Symptom**: `dracon-sync repos` rendered nearly every row with a blank
visibility column (no 🔒), and the user reasonably asked "did we public
them?" — no repos were actually published, but the visibility cache was
poisoned for two GitLab mirrors and one GitHub query path.

## What happened

On 2026-08-06 the operator ran `dracon-sync repos` and noticed the
visibility column was missing 🔒 icons for nearly every repo. The
legend says "no icon = public/unknown", so the natural reading was
"everything is public, did we publish them?" — which would be a
serious regression.

Investigation showed:

1. **Nothing was published on GitHub.** `gh repo list DraconDev`
   confirmed every fleet repo is still private. The authors of the
   "codeberg public-only" policy (2026-07-17) and the visibility
   primitives (v0.112.28 SYNC-H4) treat GitHub as the source of
   truth, and the GitHub state is correct.

2. **The visibility cache was stale.** The cache is only fresh for
   24h, and `cached_repo_visibility` returns `None` for stale
   entries. `report.rs` renders `None` as blank (same as public).
   The "missing lock" was a display artifact, not a state change.

3. **Two GitLab mirrors were genuinely public**, however:
   - `convos` (GitHub: private · GitLab: **public**)
   - `folder-auto-banner-fab` (GitHub: private · GitLab: **public**)

   These were the only two GitLab mirrors that had drifted from the
   operator's "private by default" policy. All other GitLab mirrors
   were correctly private.

4. **`folder-auto-banner-fab` had a mispointed `origin` remote:**
   - `origin`  → `https://github.com/DraconDev/folder-auto-banner` (a *different*, intentionally-public repo, no `-fab` suffix)
   - `github`  → `git@github.com:DraconDev/folder-auto-banner-fab.git` (the correct repo)

   The `refresh-visibility` subcommand tried `origin` first, then
   `github`, so it queried `DraconDev/folder-auto-banner` (public!)
   and cached `false` (public) for the local repo, which is
   actually private.

## Why the bug existed

The `refresh-visibility` subcommand (implemented in v0.112.33 for the
audit M25/F3.7 fix) was written to prefer `origin` because "that is
the most common remote name". This is true for single-remote repos,
but the daemon's multi-remote push path uses the named `github` and
`gitlab` remotes. When a repo has both remotes (as the multiremote
fleet does), the `github` remote is authoritative for the GitHub-side
namespace, and `origin` is just a legacy convenience alias — usually
pointing at the same GitHub URL, but in this case mispointed.

The Github repo DraconDev/folder-auto-banner (no `-fab`) is a
real, intentionally-public legacy repo (the `folder-auto-banner-fab`
variant was carved out later). The mispointed origin came from the
local repo's git config and was never caught because `refresh-visibility`
never queried the GitHub remote in preference.

## The fix (v0.113.43)

`refresh-visibility` now uses a new helper
`visibility::select_github_remote_url` that prefers the `github`
remote over `origin`. The `origin` fallback is preserved for repos
that only have `origin` (the common case).

```rust
// Before v0.113.43
for remote_name in ["origin", "github"] { ... }

// After v0.113.43
for remote_name in ["github", "origin"] { ... }
```

The helper is extracted into `visibility::select_github_remote_url` so
the preference order is unit-testable without spawning `git`. Four
tests cover: the regression (github + origin both present, origin
mispointed), the common case (origin only), the defensive empty-github
case, and the nothing-present case.

## Operational remediation (2026-08-06)

Before the fix was released, the following operational changes were
made to bring the fleet back to the documented private-default state:

1. `convos` — flipped from public → private on GitLab via
   `dracon-sync make-private convos`. GitHub was already private.
2. `folder-auto-banner-fab` — `origin` remote corrected to
   `git@github.com:DraconDev/folder-auto-banner-fab.git`, then flipped
   from public → private on GitLab via
   `dracon-sync make-private folder-auto-banner-fab`. GitHub was
   already private.
3. The legacy `DraconDev/folder-auto-banner` repo (intentionally
   public) was deliberately NOT touched.

After the fix, `dracon-sync refresh-visibility` writes the cache
against the correct GitHub repo for every fleet repo, and the table
renders the proper 🔒 / blank icon.

## What this design does NOT change

- **GitLab visibility drift detection is still absent.** The daemon
  reads GitHub visibility as the source of truth and propagates to
  GitLab via `sync_mirror_visibility` (which writes metadata, not
  visibility). It does NOT proactively audit whether a GitLab mirror
  has drifted from its GitHub "private" state. A future improvement
  would be a `dracon-sync audit-gitlab-visibility` subcommand that
  scans all GitLab mirrors and reports any whose visibility differs
  from GitHub. **Not implemented in v0.113.43** — out of scope.
- **Path ownership / multi-remote push behavior is unchanged.** The
  daemon still pushes to the `github` and `gitlab` remotes; origin is
  only consulted by `refresh-visibility` and by VSCode's upstream
  tracking.
- **The 24h cache TTL is unchanged.** A future improvement could
  shorten the TTL or add a "stale" icon in the table, but the
  current behaviour (stale → unknown → blank) is documented and
  matches the design policy.

## Lessons learned

1. **Subcommand preferences should match the daemon's primary path.**
   The push path uses `github` first; the refresh path should too.
2. **Mispointed `origin` remotes are a real hazard** — they can mask
   correct `github` remote URLs and cause operations to act on the
   wrong repo. A future janitor could detect `origin` ↔ `github`
   mismatches and warn the operator.
3. **Cache TTL + rendered "unknown" can be alarming.** The legend
   says "no icon = public/unknown", but a flat blank is easy to
   misread as "everything is public". A future improvement could
   render `None` as a distinct icon (e.g. ❓) to make it visually
   different from `Some(false)`. Deferred.
4. **GitLab mirror visibility drift is a real failure mode.** The
   `make-public` / `make-private` subcommands address it on demand,
   but no proactive audit exists. Worth tracking as a follow-up.

## Verification

- All 1232 unit tests pass (4 new tests for `select_github_remote_url`).
- `cargo clippy --workspace --locked -- -D warnings` clean.
- `cargo deny check` clean.
- `cargo build --release --locked` clean.
- After install + restart, `dracon-sync refresh-visibility` queries
  the correct GitHub repo for `folder-auto-banner-fab` (caches
  `private`).
- `dracon-sync repos` renders 🔒 for all private repos and blank for
  the intentionally-public ones.
- `convos` and `folder-auto-banner-fab` are now `private` on both
  GitHub and GitLab.
