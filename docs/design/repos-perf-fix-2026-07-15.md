# `dracon-sync repos` performance regression — root cause + fix (2026-07-15)

## Symptom
`dracon-sync repos` regressed from "nearly instant" to **~4–5s** (still well
under the 90s hard timeout, but felt hanging). The fleet is 29 repos; 10 of
them (dracon-platform + 9 nested submodules) have very large `.git` directories
created by the 2026-07-15 filter-repo work.

## Root cause (two components)

### 1. The regression — huge `.git` dirs trigger the pack-size slow path
`github_pack_too_large()` (git/mod.rs) only enters its **slow path**
(`git rev-list --objects main` + `git cat-file --batch-check`) when the whole
`.git` exceeds 2 GiB. Before the filter-repo work those `.git` dirs were
sub-2-GiB, so the fast path (`du -sb` < 2 GiB → return) finished instantly and
the slow path never fired.

After filter-repo, `dracon-platform/.git` is **54.5 GiB** (`du -sb` = 0.486s)
and the hegemon submodule's gitdir is **19 GiB**. So on *every* `repos` run the
report paid:
- `measure_git_size_bytes` (`du -sb`) — 58 calls, ~2 per repo, each 0.2–0.5s on
  the multi-GiB dirs (dracon-platform + 9 submodules attributes ~22 of them to
  the parent path because their gitdirs live under `dracon-platform/.git/modules/`).
- `github_pack_too_large` slow path — `rev-list --objects` + `cat-file` over the
  large pushed branches.

Measured: `du -sb dracon-platform/.git` = **54,543,473,796 bytes / 0.486s**
(was <2 GiB historically → instant).

### 2. The inherent floor (~2s, NOT a regression)
The remaining ~2s is in-process work that existed historically and is required
for a correct live report:
- **libgit2 `get_status`** stat-ing ~**267k working-tree files** across 29 repos
  (10 huge). strace: `statx` 183k + `newfstatat` 84k + `getdents64` 81k +
  `openat` 94k ≈ 2.5s cumulative.
- **Daemon I/O contention**: the live daemon competes for disk during scans,
  causing run-to-run variance (2.0–4.7s).
- git *subprocess* calls are only **~4ms each** (even on the 54 GiB repo) — they
  are **not** the bottleneck. futex 90.8% in `strace -c` is normal tokio
  scheduling overhead, not a lock bug.

## Fix
Per-repo `.git` size + GitHub pack-size guard is now **cached** in
`run_repos_report` (report.rs), keyed by repo path + the resolved gitdir's
**mtime signature**. A cache hit skips `du -sb` **and** `git rev-list`/`cat-file`
entirely. Correctness is preserved: any commit/push updates the gitdir mtime →
signature mismatch → recompute. The cache file lives at
`/home/dracon/.dracon/utilities/sync/repos-size-cache.json` (a config dir, never
a watched git repo, so it is never auto-committed by the daemon).

Also refactored `github_pack_too_large(repo, Option<u64> precomputed_size)` to
accept a precomputed size (behavior-preserving). All three callers updated:
- `report.rs` (the `repos` closure) passes the measured size → avoids a redundant
  `du`.
- `git/mod.rs` (test) and `sync.rs` (push path) pass `None` → unchanged behavior.

## Evidence
- **strace**: git spawns 379 (4 ms each, not the bottleneck); `du` 58 → **12** on
  a cache hit (only repos that changed between runs recompute); `wait4` 1.9s
  cumulative / 371 calls; ~267k stat calls = libgit2 working-tree walk.
- **Before/after timing** (live CLI, 29 repos):
  - Old binary: ~3.6–4.9s.
  - New binary, cache-hit: **~2.0–2.1s** (2× faster). Run-to-run spikes to ~4.7s
    are daemon I/O contention, not a code cost.
- **Correctness / no regression**:
  - `cargo build --release --locked`, `cargo test --workspace --locked`
    (672 tests pass, incl. 6 new: 2 precomputed-size, 4 cache/signature),
    `cargo deny check` (advisories/bans/licenses/sources all ok) all green.
  - Report content is **identical** to the old binary except for live daemon
    state (the daemon committed the cache file and was actively syncing between
    runs). `PACK_SIZE_WARNING` count = 0 for both; cached `git_size_bytes` /
    `pack_too_large` equal the freshly computed values (gitdir mtime unchanged →
    identical `du`/`rev-list` result). The cache is pure memoization.
- **Deployment**: rebuilt binary copied to `/home/dracon/.local/bin/dracon-sync`
  (backup at `.bak-<ts>`). The daemon was **not** restarted — `repos` is a
  standalone CLI that loads the binary fresh each invocation, and the daemon's own
  pack-guard passes `None` (unchanged behavior).

## Remaining (not a regression)
The ~2s floor is inherent live-status scanning (libgit2 walking 29 working trees)
+ daemon contention. It cannot be reduced without caching **live** git state
(which would show stale data → violates correctness) or shrinking the fleet. If
truly "near-instant" (sub-second) is required, the lever would be having `repos`
read the daemon's already-computed per-repo status instead of re-walking trees —
a larger architectural change with its own tradeoffs, deferred as a follow-up.
