# Patch transition: local-path → github-tag (2026-07-18)

## Context

After deploying v0.112.20 (with the libgit2 ssh-agent fix in
`dracon-git` v94.7.1), the workspace `Cargo.toml` initially used a
`path = ".../dracon-libs/tools/sync/dracon-git"` patch. That required
a local clone of `DraconDev/dracon-libs` to be present at a fixed
absolute path — fragile, and would not work on other operator machines.

We now resolve `dracon-git` from a tagged release on github:
```toml
[patch.crates-io]
dracon-git = { git = "https://github.com/DraconDev/dracon-libs", tag = "v94.7.1" }
```

This is a refactor of the patch SOURCE, not of the daemon. The daemon
binary is bit-equivalent (same `dracon-git v94.7.1` source, just
fetched from a github tag instead of a local clone).

## Verification chain (2026-07-18, BST)

1. **Tag is correct on all 3 mirrors** (verified before switch):
   - `github`: `v94.7.1 -> d42f23ce9088` (annotated, derefs to `04ef4427`)
   - `gitlab`: same
   - `codeberg`: same
2. **Build resolves from github**:
   ```
   Adding dracon-git v94.7.1 (https://github.com/DraconDev/dracon-libs?tag=v94.7.1#04ef4427)
   ```
3. **Lockfile pins the commit**: `Cargo.lock` line 72 shows
   `source = "git+https://github.com/DraconDev/dracon-libs?tag=v94.7.1#04ef4427602b88e9805bb74b0a52e2f2f3ee75ff"`
4. **890 tests pass** (`cargo test --workspace --locked`)
5. **Clippy clean** (`cargo clippy --workspace --locked -- -D warnings`)
6. **`cargo deny check` clean** — required adding
   `https://github.com/DraconDev/dracon-libs` to `deny.toml [sources].allow-git`
7. **Daemon deployed** (v0.112.20, PID 1273667 since 19:25 BST), tally:
   `📦 31 repos · ✅ CLEAN 28 · 🔄 ACTIVE 3 · ⚠️ WARN 0 · ❌ CONCERN 0`
8. **Local clone `/home/dracon/Dev/dracon-libs` removed** — was 2.0 GB,
   safe to remove because (a) github tag has the same commit, (b)
   daemon has no open fds to the clone, (c) new build's rmeta
   references cargo's own git checkout dir, not the operator's clone.

## Why 31 repos (was 32)

Removing the local `/home/dracon/Dev/dracon-libs` checkout causes the
daemon to auto-unregister it. Daemon `repos` tally drops 32 → 31.
The 32 was inflated by the dev-time local clone; the production tally
of 31 is correct.

## `deny.toml` change

Old `allow-git = []` blocked any git-sourced crate. Required adding
`https://github.com/DraconDev/dracon-libs` to the allow list.

```toml
[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-git = [
    # 2026-07-18: workspace [patch.crates-io] pulls dracon-git v94.7.1 from
    # the tagged release on github. Required until v94.7.1 is published to
    # crates.io (then revert to allow-git = []).
    "https://github.com/DraconDev/dracon-libs",
]
```

A previous version of this section had a stale comment "workspace
depends on it via a local path, not git. Removed as dead config." That
comment is now wrong and has been replaced.

## Follow-up (operator action)

1. `cd /home/dracon/Dev/dracon-libs && cargo publish -p dracon-git`
   (requires `CARGO_REGISTRY_TOKEN`). After publishing:
2. Bump `dracon-sync/Cargo.toml` `dracon-git = "94.7.0"` → `"94.7.1"`.
3. Remove the `[patch.crates-io]` block from `/home/dracon/Dev/dracon-utilities/Cargo.toml`.
4. Reset `deny.toml [sources].allow-git` back to `[]`.

## Files changed (this refactor)

- `/home/dracon/Dev/dracon-utilities/Cargo.toml` — patch source: `path = ...` → `git = "...", tag = "v94.7.1"`
- `/home/dracon/Dev/dracon-utilities/Cargo.lock` — regenerated, now references the github commit
- `/home/dracon/Dev/dracon-utilities/deny.toml` — `[sources].allow-git` now allows the github URL
- `/home/dracon/Dev/dracon-utilities/docs/design/patch-to-git-tag-2026-07-18.md` — this doc

## Files REMOVED

- `/home/dracon/Dev/dracon-libs` (2.0 GB, no longer needed)
