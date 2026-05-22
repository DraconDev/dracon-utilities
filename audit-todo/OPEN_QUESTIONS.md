# Open Questions — Dracon Utilities Audit

## P0 — Must Answer Before Finish

- [x] **Periodic incident ledger pruning**: Implemented in Iteration 5. Extracted shared `enforce_retention()` function, runs every 1800 cycles (~30 min) in the daemon main loop.

## P1 — Should Answer

- [x] **Scribe prompt injection sanitization**: Replaced blocklist `sanitize_for_prompt()` with structural separation (system message for instructions, user message for untrusted data). Added `ChatMessage::system()` constructor. Strengthened post-processing output validation.
- [x] **Release profile optimization**: Done. `strip = true` + `lto = "thin"` added to workspace Cargo.toml. Measured savings: dracon-sync 13M→10M (-23%), dracon-system 4.0M→3.2M (-20%), dracon-warden 6.2M→4.8M (-23%).
- [x] **nix_auto_update end-to-end test**: Done. Found and fixed `update_version_in_flake_nix` parser bug — didn't handle `buildRustPackage (commonArgs // {` pattern. Added `});` as block-end marker. Tests added for merged-src layout.

## P2 — Consider

- [x] **EnvRestorer docstring freshness**: Updated from "334/334 pass" to "458/458 pass as of 2026-05-22".
- [x] **Restart=always behavior**: Documented in AGENTS.md with note explaining that `systemctl stop` alone won't keep the daemon stopped — use `systemctl --user restart` for restarts, disable first to permanently stop.
- [x] **cargo update timing**: Ran `cargo update` — 11 packages updated, all 458 tests pass, cargo check clean.
