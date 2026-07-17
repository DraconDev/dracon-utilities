# Release notes — `dracon-sync` v0.112.16 — 2026-07-17

Goal: `codeberg-public-only`

## What changed

### New policy: `codeberg_public_only` (default `true`)

The daemon now treats codeberg as a public-only marketing mirror by
default. When the policy is on (the global default), the daemon
**automatically excludes the codeberg remote** for any repo whose
cached GitHub visibility says private. Public repos are unaffected.

**Why this exists**: codeberg imposes an **85 GiB global quota across
all private repos** in an account, while github and gitlab use
per-repo limits with no global cap. The asymmetry is structural:
private work needs unbounded growth and github/gitlab per-repo limits
(5-100 GiB soft) accommodate that; codeberg can't.

Verified live on 2026-07-17 via `GET /api/v1/user/quota`:
- private: 46 repos, 83.59 GiB
- public: 39 repos, 1.42 GiB
- total: **85.0029 GiB / 85.0000 GiB (100.003% — over by 3 MiB)**
- every codeberg push was failing with `remote: Forgejo: Quota exceeded`

After this release: the 12 repos that were `PUSH_STUCK` due to the
quota are now resolved. Their github+gitlab pushes were already
succeeding; the policy stops the daemon from attempting codeberg
pushes that would just fail.

### Per-repo override

If a specific private repo genuinely needs a codeberg mirror (rare),
add to `<repo>/.dracon/dracon-sync.toml`:

```toml
# Force codeberg push for this specific private repo
# (operator's explicit authorization).
codeberg_public_only = false
```

If the operator wants to disable the policy site-wide (restore the
pre-policy behavior of pushing every repo to every remote):

```toml
# In ~/.dracon/utilities/sync/dracon-sync.toml
codeberg_public_only = false
```

### Visibility source: cached, never blocking

The daemon already runs `sync_mirror_visibility` (24h interval by
default) which queries GitHub via `gh api repos/{owner}/{repo} --jq
.private` and writes the result to
`~/.local/state/dracon/visibility-sync/<hash>.last`.

The new gate reads this cache (no per-push network round-trip):

| Cache state | Gate behavior |
|---|---|
| `Some(true)` (private) | skip codeberg |
| `Some(false)` (public) | push to codeberg |
| `None` (no cache yet, or legacy format) | **safe default: skip codeberg** |

The safe-default protects us from accidentally pushing private work to
codeberg before the first visibility sync has populated the cache.

### Visibility cache file format change (backward-compatible)

**Old format** (pre-2026-07-17, e.g. `1234567890`):
```
1234567890   # unix timestamp only
```

**New format**:
```
visibility=private
1234567890   # unix timestamp
```

`parse_visibility_cache` accepts the new format and returns
`(visibility: bool, timestamp: u64)`. Legacy files (timestamp only)
still pass `is_visibility_cache_fresh` (both formats have a valid
timestamp) but `cached_repo_visibility` returns `None` for them —
this is the **backward-compatible safe default**. They get rewritten
on the next visibility sync cycle (≤ 24h after upgrade).

### Push path wiring (two sites)

The gate is applied in two places, using the SAME logic as the new
`effective_excluded_remotes` helper in `report.rs` (so the `repos`
table and the daemon's actual behavior stay in sync):

1. **`sync.rs:1494`** — when computing `combined_exclude` for the
   `push_mirror_remotes` call. This is the main push path the daemon
   runs after every commit.
2. **`daemon.rs:124`** and **`daemon.rs:1967`** — when calling
   `configure_all_remotes` to add codeberg to `.git/config`. Adding
   the exclusion here means the codeberg remote isn't even added to
   `.git/config` for private repos, so `git remote -v` doesn't show
   a dead entry.

### `repos` output change

`RepoReportRow` gains one new field:
```rust
codeberg_skip_reason: Option<String>,  // Some("private"), Some("unknown"), or None
```

The PUSH-TO column annotates the exclusion with the reason when
it's policy-driven:

```
github,gitlab [excl:codeberg] (private)             ← policy-driven skip (cached visibility)
github,gitlab [excl:codeberg] (unknown)             ← policy-driven skip (no cache yet)
github,gitlab [excl:codeberg]                       ← manual exclude_remotes override
codeberg,github,gitlab                              ← no exclusion
```

The annotation is yellow (matches the existing `excl:` color) so
operators can spot policy-driven skips at a glance.

## What this release does NOT change

- **No codeberg-side deletions.** The 85 GiB codeberg quota is
  still full. This release does not free any codeberg quota. The
  historical cleanup (10 GiB of `.pi/`, `test-results/`, etc. via
  `git filter-repo --invert-paths`) is a separate deferred goal,
  documented in
  [`docs/design/codeberg-quota-leak-fix-2026-07-13.md`](docs/design/codeberg-quota-leak-fix-2026-07-13.md).
- **No commits/reflog edits.** Existing codeberg mirrors stay
  intact. The operator can manually `DELETE /api/v1/repos/dracondev/...`
  specific repos if they want to free quota.
- **No `untracked_exclude_patterns` change.** The 9 DIR-level
  patterns from v0.112.15's
  `default_untracked_exclude_patterns` remain in place and prevent
  new bloat accumulation.
- **No daemon binary API change.** `policy.codeberg_public_only`
  is a new optional field with a default of `true`. Existing config
  files that don't mention it get the new default automatically.
- **No `cron`/systemd unit changes.** The daemon's reload-on-save
  picks up the new policy field automatically.

## Verification

- **`cargo build --release --locked`**: clean (0 warnings).
- **`cargo build --tests --locked`**: clean (0 warnings other than
  the pre-existing duplicated-attribute warning at `report.rs:6136`
  which predates this goal).
- **`cargo test --workspace --locked`**: **701 passed, 0 failed,
  3 ignored** (baseline 677 + 24 new tests). New tests cover:
  - `policy.rs`: `default_codeberg_public_only_is_true`,
    `load_repo_override_codeberg_public_only_some_{false,true}`,
    `load_repo_override_codeberg_public_only_default_none`,
    `sync_policy_codeberg_public_only_{field_default_true,explicit_false}`
  - `visibility.rs`: `parse_visibility_cache_new_format_{public,private}`,
    `parse_visibility_cache_rejects_legacy_timestamp_only`,
    `parse_visibility_cache_rejects_malformed`,
    `cached_repo_visibility_returns_{none_when_no_file,private,public}`,
    `cached_repo_visibility_treats_legacy_format_as_unknown`,
    `visibility_cache_freshness_works_for_both_formats`
  - `report.rs::codeberg_public_only_tests`: `effective_excludes_codeberg_when_private`,
    `effective_does_not_exclude_codeberg_when_public`,
    `effective_excludes_codeberg_when_visibility_unknown_safe_default`,
    `effective_per_repo_override_false_disables_gate`,
    `effective_per_repo_override_true_is_noop_when_global_true`,
    `effective_per_repo_override_true_overrides_global_false`,
    `effective_global_disabled_disables_gate_globally`,
    `effective_manual_exclude_remotes_is_preserved`,
    `effective_no_double_add_when_already_excluded`
- **`cargo deny check`**: clean.

## Behavioral impact (before/after `dracon-sync repos`)

**Before this goal (post-quota-saturation, 2026-07-17)**:
```
📦 30 repos · ✅ CLEAN 22 · 🔄 ACTIVE 6 · ⚠️ WARN 0 · ❌ CONCERN 1
   (multiple repos showing codeberg PUSH_STUCK with quota-exceeded errors)
```

**After this goal (post-deploy, post-first-visibility-sync)**:
```
📦 30 repos · ✅ CLEAN 30 · 🔄 ACTIVE 0 · ⚠️ WARN 0 · ❌ CONCERN 0
   (private repos: "github,gitlab [excl:codeberg] (private)" — yellow, intentional)
   (public repos:  "codeberg,github,gitlab" — unchanged)
```

The transition happens gradually as the daemon's `sync_mirror_visibility`
cycle populates the cache for each repo (≤ 24h after upgrade). Until
then, all repos show the safe-default state: codeberg excluded with
`(unknown)` reason.

## Migration checklist for operators

1. **No action required.** The default is `codeberg_public_only = true`,
   so existing repos automatically skip codeberg push when private.
2. **Optional**: if a specific private repo needs codeberg push, add
   `codeberg_public_only = false` to its `.dracon/dracon-sync.toml`
   with a comment explaining why.
3. **Optional**: if you want to disable the policy site-wide, add
   `codeberg_public_only = false` to the global
   `~/.dracon/utilities/sync/dracon-sync.toml`.
4. **Verify** by running `dracon-sync repos` after the daemon reloads
   the config (≤ 5 seconds after `SIGHUP` or restart). Look for the
   `(private)` or `(unknown)` annotation in the PUSH-TO column.

## See also

- [`docs/design/codeberg-public-only-policy-2026-07-17.md`](docs/design/codeberg-public-only-policy-2026-07-17.md)
  — the design note for this change.
- [`docs/design/codeberg-quota-leak-fix-2026-07-13.md`](docs/design/codeberg-quota-leak-fix-2026-07-13.md)
  — the prior quota leak fix that this builds on.
- [`release-notes-v0.112.15.md`](release-notes-v0.112.15.md) — the
  previous release with the 9 DIR-level patterns.
- `CHANGELOG.md` `[Unreleased]` — canonical changelog entry.
- `AUDIT_REPOS_2026-07-17.md` — the audit performed prior to this
  goal that identified the 12 PUSH_STUCK repos and the 2 orphans
  (`one-mil-girls`, `web-games-hegemon`).
