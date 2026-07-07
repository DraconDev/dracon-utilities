# dracon-platform pack-size hint + endless-td concern fix (2026-07-07)

## Goal
`dracon-sync repos` showed "all green except two concerning":
1. **dracon-platform** (parent monorepo): `💡 HINT: .git exceeds 2 GB (github limit)`
2. **endless-td** (submodule): `❌ CONCERN`, `🟣 pushing 20m (1 ahead, 1 behind)`

Outcome: **26 OK / 0 WARN / 0 CONCERN**, no pack-size hint.

## 1. endless-td — RESOLVED (legit divergence, normal merge)
- Local-only `74da822 "v54.4: affix coverage audit"` (operator work, pushed to
  github/gitlab/codeberg) was 1 ahead.
- Remote-only `85ec3a6 "Merge gitlab.com:…"` was 1 behind on `origin/main`.
- Both are legitimate (operator commit + gitlab merge). Stopped the daemon,
  `git merge origin/main` (ort strategy, no force) → `f6a7bc5`. Pushed to
  github/gitlab/codeberg. Verified 0 ahead / 0 behind.
- **Lesson**: a `CONCERN` with 1 ahead + 1 behind on a submodule is usually a
  benign local-vs-remote split, not a real problem. Merge (no force) and push.

## 2. dracon-platform — RESOLVED (exclude github; cannot shrink)
### Why it can't be shrunk
- `.git` = 17 GiB; `git count-objects -vH` → `size-pack = 12.61 GiB`.
- 461,357 pack objects but only **13,157 reachable** — the rest is unreachable
  cruft. `git gc --prune=now` **FAILS**:
  ```
  error: Could not read c2eb911c... (parent of commit 93ee4b46)
  fatal: bad tree object ...
  ```
- `93ee4b46` is on a legacy/tool ref (`main-temp`, old `codeberg/*` branches),
  **NOT on `main`** (`git merge-base --is-ancestor 93ee4b46 main` → false;
  `main` = `d0ff324`, synced to gitlab/codeberg). Its parent `c2eb911c` is
  unrecoverable (not on origin/github/gitlab/codeberg).
- Any `git gc` / `filter-repo` traversal hits the missing object → shrink is
  infeasible. The corruption must be repaired (fetch the missing object from a
  remote that still has it) before the repo can be repacked below 2 GiB.

### Why excluding github is the safe path
- GitHub's hard limit is **2 GiB/pack**. dracon-platform is 12.61 GiB → can
  never push to github. The daemon already proactively skips the github push
  on size (Part-1 fix from goal `a8d6990e`).
- gitlab + codeberg accept the full history and remain authoritative
  (verified ✅ OK / synced; `main` = `d0ff324` on both, matching local).

### The fix (two parts)
1. **Daemon code change** (`dracon-sync/src/report.rs`, commit `f39a7cf`,
   pushed to all 4 remotes): the `PACK_SIZE_WARNING` hint was set purely on
   `.git` size ≥ 2 GiB and did NOT check `exclude_remotes`. Now it is gated:
   ```rust
   let github_excluded = repo_override.exclude_remotes
       .iter().any(|r| r.eq_ignore_ascii_case("github"));
   if size >= GITHUB_PACK_LIMIT_BYTES && !github_excluded {
       flags.push("PACK_SIZE_WARNING".to_string());
   }
   ```
   So the hint only appears when GitHub is actually a push target.
2. **dracon-platform config** (`.dracon/dracon-sync.toml`, commit
   `11d763832e0`, pushed to gitlab + codeberg — NOT github):
   ```toml
   exclude_remotes = ["github"]
   ```
   (Previously `exclude_remotes = []`.) This (a) makes the github skip explicit
   and (b) clears the hint via the code change above.

### If you ever want github back
- Repair the corruption first: find a remote/repo that still has object
  `c2eb911c`, fetch it, then `git gc --prune=now` to drop the ~448k unreachable
  objects (may bring the pack under 2 GiB).
- OR finish the OVH-bucket / git-annex migration so the packable size drops
  below 2 GiB, then remove `exclude_remotes = ["github"]`.
- Do NOT force-push github in the meantime (it rejects 17 GiB anyway).

## Verification (2026-07-07, goal `d4fbbcb3`)
```
📦 26 repos  ✅ OK 26  ⚠️ WARN 0  ❌ CONCERN 0  ⛔ init/status failed: 0
```
- dracon-platform: `✅ OK`, publish `gitlab,codeberg [excl:github]`, 0 ahead /
  0 behind, HINT = healthy (no pack-size warning).
- endless-td: `✅ OK`, 0 ahead / 0 behind, no concern.
- 0 rows carry `PACK_SIZE_WARNING` in the HINT column.
- dracon-sync daemon binary = `f39a7cf` (running, installed at
  `~/.local/bin/dracon-sync`).
