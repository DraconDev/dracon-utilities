# Dracon Utilities — TODO

Audit date: 2026-05-23

---

## 🔴 Must Fix (AI-to-AI Version Control)

- [ ] **Complete full audit of repo state**
  - Review all 26 repos in ~/Dev
  - Check which ones have auto GitHub private repo creation
  - Document findings in repo-specific TODOs
  - Fix any repos missing remotes

- [ ] **Verify auto_create = true is working**
  - Test with a fresh repo clone
  - Confirm GitHub remote is created automatically
  - Check incident ledger for any auto_create errors

- [ ] **Fix `auto-ai-video-processor-folder-watcher-daemon-cli`**
  - This repo has no remote (CONCERN status)
  - Auto-create should have created GitHub remote
  - Need to investigate why it didn't

## 🟡 Should Fix

- [ ] **Monitor `proc-macro-error` removal** (yanked from crates.io)
  - Chain: `age 0.10.1 → i18n-embed-fl 0.7.0 → proc-macro-error 1.0.4`
  - Affects dracon-warden + dracon-security (encryption)
  - Will break `cargo update` if transitive dep falls out of resolution

- [ ] **Add periodic incident ledger pruning**
  - Currently only prunes at daemon startup
  - Append-only at runtime → unbounded growth on long-running daemon
  - Consider pruning every N entries or every 24h in the main loop

- [ ] **Review scribe prompt injection sanitization**
  - Blocklist approach (`sanitize_for_prompt`) is fragile
  - Consider whitelist-based structured prompt delivery instead

- [ ] **Enable release profile optimizations**
  - `dracon-sync` is 13.5 MiB (3x `dracon-system`)
  - Add `strip = true` and/or `lto = "thin"` to release profile
  - Not urgent, frees ~2-4 MiB per binary

- [ ] **Test `nix_auto_update` end-to-end**
  - Feature exists in dracon-sync but flake.nix has unusual merged-src layout
  - Untested; CI nix job also blocked by billing

## 🔵 Consider

- [ ] **Update `EnvRestorer` docstring** — mentions "334/334 pass" which may be stale
- [ ] **Document `Restart=always` behavior** — daemons auto-restart even after manual `systemctl stop`
- [ ] **Run `cargo update` after billing resolved** — catches stale deps before they break

## ✅ Done (from audit)

- Cargo check passes clean
- Clippy passes with `-D clippy::all`
- No missing-docs warnings
- Working tree clean
- Mass-deletion safety guard working (proptest-verified)
- `git ls-files --count` bug already fixed
- All 3 service files installed and hardened
- `deny.toml` configured with advisories/licenses/bans
- Policy example configs present
- Architecture spec document created and committed
- GitHub auto_create enabled in dracon-sync.toml
