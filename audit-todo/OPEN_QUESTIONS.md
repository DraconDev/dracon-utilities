# Open Questions — Dracon Utilities Audit

## P0 — Must Answer Before Finish

- [x] **Periodic incident ledger pruning**: Implemented in Iteration 5. Extracted shared `enforce_retention()` function, runs every 1800 cycles (~30 min) in the daemon main loop.

## P1 — Should Answer

- [x] **Scribe prompt injection sanitization**: Replaced blocklist `sanitize_for_prompt()` with structural separation (system message for instructions, user message for untrusted data). Added `ChatMessage::system()` constructor. Strengthened post-processing output validation.
- [ ] **Release profile optimization**: What's the expected binary size reduction from `strip = true` + `lto = "thin"`? Worth doing even for debug builds?
- [ ] **nix_auto_update end-to-end test**: Need to verify the feature works with the unusual merged-src layout in flake.nix.

## P2 — Consider

- [ ] **EnvRestorer docstring freshness**: Docstring mentions "334/334 pass" — update to current test count.
- [ ] **Restart=always behavior**: Should we document that daemons auto-restart even after manual `systemctl stop`? This is surprising behavior.
- [ ] **cargo update timing**: Run after billing resolved to avoid pulling broken deps.
