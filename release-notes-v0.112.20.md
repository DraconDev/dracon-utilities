# dracon-sync v0.112.20 — CONCERN fix via dracon-git v94.7.1 patch

**Date:** 2026-07-18
**Scope:** workspace `Cargo.toml` (added `[patch.crates-io]` for dracon-git), `deny.toml` (`allow-git` updated), removed local `/home/dracon/Dev/dracon-libs` clone.
**Motivation:** `dracon-sync repos` showed `❌ CONCERN 2` for endless-td (53-ahead push-stuck) and neonbreak (4-minute PENDING)

---

## TL;DR

The 2 CONCERNs were caused by a libgit2 fetch bug in the external
`dracon-git` crate v94.7.0. The daemon's `fetch()` function used
`git2::Cred::ssh_key_from_agent`, which requires a running ssh-agent.
The operator's wezterm/NixOS session has no ssh-agent (only a wezterm
socket), so every libgit2 fetch failed with
`unsupported URL protocol; class=Net (12)`.

This release **doesn't change any daemon source code**. Instead, it
patches the workspace `Cargo.toml` to use a locally-built
`dracon-git v94.7.1` (from `DraconDev/dracon-libs`) where `fetch()` is
rewritten to use `git fetch` CLI as the primary path (which respects
`~/.ssh/config` and `IdentityFile ~/.ssh/id_ed25519`).

The daemon's push code path was never broken — std::process `git push`
always worked because SSH config has `IdentitiesOnly yes`. Only the
fetch code path was affected.

---

## What's in this release

1. **New version 0.112.20** (patch bump from 0.112.19 — purely a
   dependency change, no daemon behavior changes).
2. **Workspace `Cargo.toml`**: added `[patch.crates-io]` for dracon-git
   (initially path-based; later transitioned to git-tag — see below).
3. **dracon-git v94.7.1** (tagged release of `DraconDev/dracon-libs`):
   - `fetch()` rewritten: CLI primary, libgit2 fallback
   - 1 new regression test `test_fetch_uses_cli_path_successfully`
   - 33 tests pass (was 32)
   - clippy clean, deny clean

## Patch source transition (same release, follow-up commit)

The patch initially used `path = ".../dracon-libs/tools/sync/dracon-git"`
(operator's local clone). That was fragile: required the clone at a
fixed absolute path. Switched to:
```toml
[patch.crates-io]
dracon-git = { git = "https://github.com/DraconDev/dracon-libs", tag = "v94.7.1" }
```
which resolves from the github tag. Same daemon binary, same `dracon-git`
source commit (`04ef4427`). The local clone was then removed (2.0 GB).

See `docs/design/patch-to-git-tag-2026-07-18.md` for the full chain.

---

## Files changed

- `Cargo.toml` (workspace) — added `[patch.crates-io]`
- `Cargo.lock` — `dracon-git v94.7.0 (registry)` → `v94.7.1 (path)`
- `docs/design/concerns-investigation-2026-07-18.md` — full
  root-cause analysis + resolution
- `AUDIT_FULL_2026-07-18.md` — §F5 resolution appendix (TBD)
- `CHANGELOG.md` — this entry

## No changes to

- `dracon-sync/src/*.rs` — daemon source unchanged
- `dracon-sync/src/main.rs` — CLI args unchanged
- `dracon-sync/src/report.rs` — table rendering unchanged (v0.112.19)
- Configuration file format
- Policy semantics

---

## Verification

```
$ dracon-sync repos | head -3
📜 /home/dracon/.dracon/utilities/sync/dracon-sync.toml
📦 32 repos  ✅ CLEAN 28  🔄 ACTIVE 4  ⚠️  WARN 0  ❌ CONCERN 0  ⛔ init/status failed: 0
```

`32 repos` = 31 original + 1 new (`dracon-libs`, auto-discovered).

### Endless-td specific

- Pre-fix: `❌ CONCERN · 🛑 STUCK · push-stuck (35 consecutive failures) · 53 ahead / 4 behind`
- Post-fix: `✅ CLEAN · 🟢 synced 8m · healthy · 0 ahead / 0 behind`
- Manual intervention required: yes (operator chose reset+replay strategy).
  See design doc §"Step 1: Endless-td reset+replay".

### Neonbreak specific

- Pre-fix: `❌ CONCERN · 🟣 PENDING · pushing 4m · 6 ahead / 4 behind`
- Post-fix: `✅ CLEAN · 🟢 synced 15m · healthy · 0 ahead / 0 behind`
- Manual intervention required: none (auto-recovered once `git fetch
  origin` updated the tracking ref).

---

## Build / test / lint / deny

- `cargo build --release --locked` — succeeds (53s)
- `cargo test --workspace --locked` — 890 tests pass
- `cargo clippy --workspace --locked --all-targets -- -D warnings` — clean
- `cargo deny check` — clean (advisories, bans, licenses, sources)

Plus on the `dracon-libs` side:

- `cargo test -p dracon-git --lib` — 33 tests pass (was 32)
- `cargo clippy -p dracon-git --lib --all-targets -- -D warnings` — clean

---

## Required follow-up actions

1. **Publish dracon-git v94.7.1 to crates.io** so the `[patch.crates-io]`
   can be removed. Requires the operator's crates.io API token:
   ```bash
   cd /home/dracon/Dev/dracon-libs
   cargo publish -p dracon-git
   ```

2. **Update `AUDIT_FULL_2026-07-18.md` §F5** with the resolution
   appendix pointing at this design doc.

3. **Verify dracon-libs auto-pushed commits** on all 3 mirrors:
   - Commit `659f4453` — version bump + lib.rs fetch() rewrite
   - Commit `04ef4427` — new regression test
   Daemon auto-committed both; should be on github/gitlab/codeberg.