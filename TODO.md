# Dracon Utilities — TODO

Audit date: 2026-05-22

---

## 🔴 Must Fix

- [ ] **Settle GitHub Actions billing**
  - All CI jobs blocked: *"Recent account payments have failed or your spending limit needs to be increased."*
  - 10+ consecutive failures, every job silently skipped
  - → `https://github.com/settings/billing`

- [ ] **Bump `git2` in dracon-libs/dracon-git** (RUSTSEC-2026-0008)
  - Unsound: *"Potential undefined behavior when dereferencing Buf struct"*
  - Currently pinned to 0.18.3; acknowledged in `deny.toml` but fix pending
  - Blocks clearing an open advisory

- [ ] **Investigate `wal-backup` daemon sync loop**
  - Incident ledger shows 12+ rapid triage entries in tight succession
  - Stale `index.lock` failure at least once
  - Daemon cycling rapidly on this repo — wasting CPU and ledger space
  - Check for git hooks, filesystem events, or fingerprint debounce gaps

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
