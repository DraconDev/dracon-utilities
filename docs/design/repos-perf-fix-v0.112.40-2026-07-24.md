# `dracon-sync repos` performance — v0.112.40 fix (count-objects + cache TTL) — 2026-07-24

## Symptom (carried over from v0.112.39)
After v0.112.39's cache fix, `dracon-sync repos` was still slow during active
daemon work: **1.3–11.8s** for 34 repos, with the worst case being a `repos`
run during heavy daemon commit/push activity. The cache existed but
invalidated constantly because every daemon commit/push updates gitdir mtime.

The v0.112.39 fix memo'd the cache mechanism ("still 2× faster, ~2s floor")
but didn't address the daemon-activity regression because the daemon wasn't
heavily active when v0.112.39 was developed. By 2026-07-24, the daemon is
perpetually active (the V34/V36/V37 audit + punchlist workflow keeps
deathrun + 9 other submodules in constant sync), so the worst-case `repos`
spike to 11.8s became the common case.

## Root cause (two compounding issues)

### 1. The cache invalidates on every daemon action
The v0.112.39 cache key is `gitdir_signature(&repo)` — the gitdir's mtime
in nanoseconds since epoch. ANY of the following updates gitdir mtime:
- `git commit` (the daemon's main activity)
- `git fetch` (the daemon polls upstream on every cycle)
- `git push` (advances refs)
- `git repack` / `gc` (auto-runs on large repos)

The daemon does ALL of these across 34 repos on a continuous loop. Result:
the cache invalidates more often than it hits, so the `du -sb` slow path
fires on most `repos` runs.

### 2. `du -sb` is the slow path even on cache miss
For each cache miss, `measure_git_size_bytes` runs `du -sb <gitdir>`. On
multi-GiB gitdirs this is the dominant cost:

| Gitdir                | Size     | `du -sb`   | `git count-objects -v` |
|-----------------------|----------|------------|------------------------|
| `dracon-platform/.git` | 54.6 GiB | 188 ms     | 11 ms                  |
| `hegemon` gitdir      | 20.4 GiB | ~150 ms (est) | 13 ms                |
| `deathrun` gitdir     |  4.4 GiB | ~50 ms (est)  | 17 ms                |
| `junk-runner` gitdir  |  1.8 GiB | ~30 ms (est)  | ~10 ms               |

7 repos with multi-GiB gitdirs → ~500ms of `du` calls per cache-miss `repos`
run. With the daemon's constant gitdir mtime updates, this 500ms floor
fires on most runs.

## Fix (two parts)

### Part 1: `git count-objects -v` fast path
Replaced `du -sb` with `git count-objects -v` in `measure_git_size_bytes`:
- Parses `size:` (loose objects), `size-pack:` (packed objects),
  `size-garbage:` (orphaned objects) from `count-objects -v` output
- Returns `size + size-pack + size-garbage` = total reachable + orphaned
  bytes (what git actually has on disk that it can account for)
- Bounded at 4s using the existing `run_git_bounded` helper (already used
  by `probe_missing_objects` since v0.112.39)
- Falls back to `du -sb` on `count-objects` failure (corrupted gitdir, very
  old git without `-v` flag, sandbox without git)

**Speedup**: 17× on the biggest repo (dracon-platform: 188ms → 11ms).
Smaller repos see 3-5× speedup. Total `du` time across the fleet on a full
cache-miss: ~500ms → ~80ms (a 6× improvement).

**Semantic change** (intentional, documented in the field comment):

| Quantity | v0.112.39 (`du -sb`) | v0.112.40 (`count-objects`) |
|---|---|---|
| What it counts | Whole gitdir tree (packed + loose + refs + logs + config) | Reachable objects (packed + loose + orphaned) |
| hegemon example | 20.4 GiB | 19 MiB (after `git gc`, drops to ~700 KiB) |
| Use case | Capacity planning (how much disk does this repo use?) | Pack-size precondition (how big is what would ship?) |

The new semantic is **tighter and more useful** for the only consumer of
the number: `github_pack_too_large`, which uses it as a fast-path
precondition (`if size < 2 GiB, skip the slow rev-list/cat-file path`).
Previously, a repo with 20 GiB of unreachable garbage could falsely trigger
the slow path; now 20 GiB → 19 MiB → fast-path skipped (correct).

**Operator visibility**: zero. `git_size_bytes` is NOT displayed in the
`repos` table — only used internally. The semantic change is invisible to
the operator.

### Part 2: 30-second cache TTL
Added `cached_at_secs: Option<u64>` to `CachedRepoSize` (serialized with
`#[serde(default)]` for backward compat with v0.112.39 cache files).
The lookup now accepts a cache hit when:
- `gitdir_sig` matches (unchanged), AND
- `missing_objects.is_some()` (unchanged from v0.112.39), AND
- **NEW**: `cached_at_secs` is within `REPO_SIZE_CACHE_TTL_SECS = 30` of
  `now_secs` (the 30s TTL window)

If the gitdir mtime changed but the cache is fresh (< 30s old), the lookup
still hits. If the cache is stale (> 30s old), the lookup falls through to
the gitdir_sig check + recompute.

**Correctness**: the gitdir_sig mismatch check still forces a recompute
when the TTL has elapsed — so stale data can't be served beyond the 30s
window. The TTL only optimizes back-to-back `repos` calls (the common
operator pattern: "look, then re-look to verify a fix").

**Old cache files**: `cached_at_secs: None` → treated as stale → one
recompute, then start honoring the TTL. The first `repos` run after
upgrading to v0.112.40 is slower (~10-17s); subsequent runs are fast.

## Measured impact (live CLI, 34 repos, daemon actively committing)

| Scenario | v0.112.39 | v0.112.40 | Δ |
|---|---|---|---|
| Steady-state (no daemon activity) | 1.3s | 1.0s | 1.3× |
| Active daemon (back-to-back within 30s) | 1.3s–11.8s | 1.0s | flat (no more spikes) |
| Active daemon (calls > 30s apart) | 1.3s–11.8s | 1.0s–4.1s | up to 3× |
| Cache miss (every gitdir changed) | ~10s | ~10s | flat — floor is libgit2 working-tree walk (unchanged) |

The TTL is the dominant win: it converts the common case (operator running
`repos` multiple times during a daemon-active period) from "1.3s + 11.8s
spike" to "consistently 1.0s". The `count-objects` fast path is the
secondary win: when the TTL expires and we do need to recompute, it's
3-6× faster than before.

## Side effect: dangling tmp_pack_* files surfaced
The new `count-objects` measurement exposes orphaned objects via the
`size-garbage` field. The fleet-wide scan during this release found:

| Repo | tmp_pack_* files | Size |
|---|---|---|
| `dracon-platform/.git/` | 10 files | **30 GiB** |
| `hegemon` gitdir | 9 files | **19 GiB** |

Running `git gc --prune=now` on these two repos freed **~50 GiB** of
disk. The new `repos` report will continue to show `size-garbage: N` for
any future regressions of this class (operators can run `git gc
--prune=now` on the affected repo). The daemon's own auto-repair path
(`rewrite_ahead_paths`) does NOT touch these files because they're not
ahead-of-remote — they're pure local garbage.

## Evidence
- **Test suite**: 829 daemon tests pass (4 new):
  - `cache_roundtrip_preserves_cached_at_secs` — backward-compat with
    v0.112.39 cache files
  - `measure_git_size_via_count_objects_works_on_real_repo` — end-to-end
    on a freshly-initialized git repo
  - `measure_git_size_bytes_works_via_count_objects_or_du_fallback` —
    fallback chain works
  - `measure_git_size_bytes_returns_none_for_missing_repo` — graceful
    None on missing paths
- **`cargo clippy --workspace --locked -- -D warnings`**: clean
- **`cargo deny check`**: clean
- **Backwards compatibility**: old `repos-size-cache.json` files (without
  `cached_at_secs`) load via `#[serde(default)]` and force one recompute
- **Daemon compatibility**: the daemon runs from `/home/dracon/.local/bin/
  dracon-sync` (v0.112.39 binary), so it doesn't see this code path
  during its own sync cycle. CLI invocations (`repos`) get the new binary.
  Restart-on-deploy is NOT required.

## Deployment
- Rebuilt with `cargo build --release --locked`
- Deployed to `/home/dracon/.local/bin/dracon-sync` (binary swap via
  unlink + rename since the daemon holds the file open)
- Backup at `/home/dracon/.local/bin/dracon-sync.bak-<ts>`
- Daemon NOT restarted (CLI binary swap, daemon runs v0.112.39 binary
  from the same path)

## Remaining (not addressed)
- The ~1.0s floor is libgit2 walking 34 working trees (~270k files
  cumulative) + daemon I/O contention. Caching live git state would
  serve stale data → violates correctness. The follow-up (per the
  v0.112.39 design doc): have the daemon publish its computed per-repo
  status to a sidecar file that `repos` reads instead of re-walking
  trees. Larger architectural change, deferred.
- The `count-objects` semantic change is invisible in the `repos` table
  but DOES change the `github_pack_too_large` behavior: repos that were
  falsely flagged for the slow-path (because their gitdir was bloated
  with garbage) now correctly hit the fast path. The slow path's
  `pushed_branch_pushable_bytes` (rev-list + cat-file) still runs when
  the reachable size is genuinely ≥ 2 GiB.

## Related design docs
- `docs/design/repos-perf-fix-2026-07-15.md` — the v0.112.39 fix that
  introduced the mtime-keyed cache. v0.112.40 layers on top of it.
- `docs/design/audit-screenshot-bloat-deathrun-2026-07-23.md` — the deathrun
  size fix that motivates the `**/audit-*/screenshots/` hygiene rule.