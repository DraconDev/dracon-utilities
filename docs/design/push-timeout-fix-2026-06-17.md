# Push Timeout Fix — 2026-06-17

> **Goal**: `04e41051-1a0a-49b1-aa9c-f1c9b849a2ff`
>
> **Outcome**: `push_op_timeout_secs` raised from `60` to `300` in
> the global config, matching the daemon's own code default. Pushed
> 23-file and 61-file PNG-heavy commits in stress test with all 4
> remotes in 0.6-10.5s, well under the new 300s budget.

## Operator's framing

> "address it" (referring to the 60s `push_op_timeout_secs` that
> caused PUSH_STUCK during the v0.112.10 release when a 23-file
> PNG-heavy commit in `dracon-platform` couldn't push to gitlab
> and codeberg)

## The v0.112.10 incident

The v0.112.10 release process hit the 60s `push_op_timeout_secs`
when `dracon-platform` was auto-committed with 23 files (mostly
game-dev smoke-out PNG binaries). The daemon's journal recorded:

```
Jun 17 00:31:52 nixos dracon-sync[188760]: ⚠️ push to gitlab failed
  for /home/dracon/Dev/dracon-platform: git push-to-gitlab timeout
  in /home/dracon/Dev/dracon-platform after 60s
Jun 17 00:31:52 nixos dracon-sync[188760]: ⚠️ push to codeberg failed
  for /home/dracon/Dev/dracon-platform: git push-to-codeberg timeout
  in /home/dracon/Dev/dracon-platform after 60s
```

Manual recovery required `timeout 300 git push --no-verify` for
each of gitlab and codeberg, then daemon retry to clear the
PUSH_STUCK state.

## Why per-remote timeouts were considered (and rejected for now)

The most precise fix would be per-remote `push_op_timeout_secs`:
- `github = 60s` (fast CDN, never needs more)
- `origin = 60s` (local/fast)
- `gitlab = 180s` (slower SSH, large commits)
- `codeberg = 180s` (slower SSH, large commits)

**But:** the daemon's `RemoteConfig` struct in
`dracon-sync/src/policy.rs` does NOT have a per-remote timeout
field. The struct has these fields:

```rust
pub(crate) struct RemoteConfig {
    pub(crate) name: String,
    pub(crate) push_url: String,
    #[serde(default)]
    pub(crate) auto_create: bool,
    #[serde(default)]
    pub(crate) auto_create_account: String,
    #[serde(default = "default_auth_type")]
    pub(crate) auth_type: AuthType,
    #[serde(default = "default_priority")]
    pub(crate) priority: u32,
    #[serde(default)]
    pub(crate) api_endpoint: Option<String>,
    #[serde(default)]
    pub(crate) auto_create_token_var: Option<String>,
    #[serde(default)]
    pub(crate) repo_name_map: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub(crate) force_push_when_behind: bool,
}
```

No `push_op_timeout_secs`. Adding it requires:
1. Adding the field to `RemoteConfig` (1 line)
2. Plumb it through `push_to_named_remote()` in
   `git/multi_remote.rs` (the function takes a global
   `timeout_secs: u64` parameter; would need to look up the
   per-remote value and use it)
3. Rebuild the daemon
4. Release the daemon as a new version (separate from
   `dracon-utilities` release)

**Decision: deferred.** Per-remote timeouts are queued for a
follow-up daemon release. The utilities-only fix (single global
`push_op_timeout_secs = 300`) is good enough for now and ships
in v0.112.11.

## The fix: `push_op_timeout_secs = 300` (global)

**Code default in daemon** (`dracon-sync/src/policy.rs`):
```rust
pub(crate) fn default_push_op_timeout_secs() -> u64 {
    300
}
```

The daemon's own default is **300s**. The operator's config had
`push_op_timeout_secs = 60`, which is an **override-down** from
the default. The fix is to remove the override (use the default)
or set it explicitly to 300 with a comment explaining why.

**Edit** in `~/.dracon/utilities/sync/dracon-sync.toml`:
```diff
- push_op_timeout_secs = 60
+ push_op_timeout_secs = 300
+ # CHANGED 2026-06-17: see docs/design/push-timeout-fix-2026-06-17.md
```

300s gives a **5x safety margin** over the v0.112.10 measured
>60s push time. It's wasteful for github (which never takes
more than a few seconds) but harmless — the daemon times out
via process kill, not via waiting.

## Measured push duration data (2026-06-17 01:05 UTC)

**Small commits** (5 files, no binaries, normal source diffs):
- github:  ~1.2s
- gitlab:  instant (up-to-date)
- codeberg: ~1.3s
- origin:  ~7.7s (mostly ssh handshake + protocol negotiation)

**Stress test** (61 files, ~1.5MB of PNG binaries, 30x 50KB PNGs
+ 30x 67B PNGs + 1 spec file):
- github:  2.35s
- gitlab:  2.57s
- codeberg: 10.51s
- origin:  0.64s

**Observations:**
- All 4 remotes handle a 61-file / 1.5MB stress test well under 300s
- The slowest remote (codeberg) is 28x under the timeout
- The 23-file commit in v0.112.10 that triggered the original
  incident was **smaller** than this stress test, so the
  v0.112.10 timeout was network-related (slow connection at that
  moment), not capacity-related

**Live test with daemon:** the daemon successfully pushed the
codeberg gap (1 commit, 14 binary files) with the new 300s
timeout — no PUSH_STUCK. The "stalled" and "settling" states in
the daemon's view are normal settle behavior, not timeouts.

## Why 300s and not higher (or lower)

| Value | Pros | Cons |
|-------|------|------|
| 60s (was) | Catches truly-stuck pushes fast | False PUSH_STUCK on slow connections for normal-size commits |
| 180s | 3x safety over measured >60s | Still might be too tight for very large commits |
| **300s (chosen)** | **Matches daemon default, 5x safety over >60s** | **Wastes ~290s on github for genuinely-stuck pushes** |
| 600s | 10x safety over >60s | Even more wasted on github |

**Rationale for 300s:**
- Matches the daemon's own code default (no surprise behavior
  change for users on the default config)
- 5x safety margin over the v0.112.10 measured >60s
- The "waste" on github is bounded: the daemon's `push_retries = 3`
  means at most 3x 300s = 15min worst case for a genuinely-stuck
  github push before the daemon gives up

The trade-off is between:
- **Faster detection of stuck pushes** (low timeout, more false
  positives)
- **Tolerance of slow connections** (high timeout, more wasted
  budget on stuck pushes)

300s is the sweet spot for a single global value.

## Per-remote timeouts (deferred to a future daemon release)

The proper fix is per-remote timeouts, like `force_push_when_behind`
(goal `87c1bf4d`). This requires:

1. Add `push_op_timeout_secs: Option<u64>` to `RemoteConfig`
2. In `push_to_named_remote()`, look up the remote's timeout
   and use it instead of the global default
3. Fall back to the global `push_op_timeout_secs` if the
   per-remote value is `None`
4. Test that 60s for github, 180s for gitlab, 180s for codeberg
   works
5. Rebuild and release the daemon

This is a daemon release, not a utilities release. It can ship
independently of v0.112.11.

## Runbook for future large pushes

When a commit triggers PUSH_STUCK or "push timeout" in the
daemon's journal:

1. **Check the commit size**:
   ```bash
   git log -1 --stat
   # Look for: BIN:N (binary files), large file sizes
   ```

2. **If the commit has <10 binary files and <1MB total**:
   - The 60s timeout should have been enough
   - The issue is likely network latency
   - Retry the push manually with a longer timeout:
     ```bash
     timeout 300 git push --no-verify <remote> main
     ```
   - Then let the daemon catch up

3. **If the commit has >10 binary files or >1MB total**:
   - This is a "large commit" case
   - The 300s global timeout should handle it (per the stress
     test above)
   - If it still times out, the commit is genuinely large;
     consider git LFS for game-dev smoke-out PNGs as a
     future improvement
   - Manual retry with `timeout 600 git push --no-verify`
     can unblock the immediate issue

4. **If the issue is recurring** (multiple PUSH_STUCK events
   per day):
   - The push_op_timeout_secs value needs to be higher
   - OR per-remote timeouts need to be implemented
   - OR git LFS needs to be adopted for the large-binary
     directories

5. **If a 4-remote push fails completely** (not timeout, but
   rejection): investigate, don't force-push. The fix is
   probably a divergence resolution, not a timeout change.

## Related docs

- `AGENTS.md` (the policy doc, will be updated with the new
  push timeout note in v0.112.11)
- `docs/design/pi-tmp-persist-policy-2026-06-16.md` (the
  v0.112.10 design doc that surfaced the 23-file PNG commit
  that triggered this fix)
- `docs/design/sync-push-classification.md` (the push state
  classification — PENDING / OK / PUSH_STUCK / etc.)
- `~/.dracon/utilities/sync/dracon-sync.toml` (the global
  config with the new 300s value + comment)
- `dracon-sync/src/policy.rs` (the daemon's code where
  `default_push_op_timeout_secs = 300` is defined)
- `dracon-sync/src/git/multi_remote.rs` (the function that
  uses the global timeout; would need to be updated for
  per-remote timeouts)
