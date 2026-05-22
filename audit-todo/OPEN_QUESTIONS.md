# Open Questions — Dracon Utilities Audit

## P0 — Must Answer Before Finish

- [ ] **Periodic incident ledger pruning**: Currently only prunes at daemon startup. Need to decide on approach: prune every N entries, every 24h, or at a configurable interval in the main loop.

## P1 — Should Answer

- [ ] **Scribe prompt injection sanitization**: Blocklist (`sanitize_for_prompt`) is fragile. Investigate whitelist-based structured prompt delivery.
- [ ] **Release profile optimization**: What's the expected binary size reduction from `strip = true` + `lto = "thin"`? Worth doing even for debug builds?
- [ ] **nix_auto_update end-to-end test**: Need to verify the feature works with the unusual merged-src layout in flake.nix.

## P2 — Consider

- [ ] **EnvRestorer docstring freshness**: Docstring mentions "334/334 pass" — update to current test count.
- [ ] **Restart=always behavior**: Should we document that daemons auto-restart even after manual `systemctl stop`? This is surprising behavior.
- [ ] **cargo update timing**: Run after billing resolved to avoid pulling broken deps.
