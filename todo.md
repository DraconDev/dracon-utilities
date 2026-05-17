# CLI Output Polish

## Findings & Fixes

### F1 — sync status: SCREAMING_SNAKE → Title Case ✅
`📜 POLICY:` / `📦 REPOS_DISCOVERED:` → `📜 Policy:` / `📦 Repos:`
Also applied to warden status (`WATCH_ROOTS` → `Watch roots`, `PUBKEY_SOURCE` → `Pubkey source`).
All three tools now use Title Case key names.

### F3 — Daemon cycle noise: policy/mode/push every cycle ✅
Already gated by `out!` macro (only prints when `human=true` for CLI commands).
In daemon mode (`human=false`), these lines are suppressed. No change needed.

### F4 — Zero-count concern/warn summary every cycle ✅
Summary block now only prints when `found > 0`.
When all repos are healthy, the daemon is silent.

### F5 — AI provider readiness every cycle ✅
Added `AtomicBool` gate — AI provider status logs only on first call per process.
Subsequent `SimpleAiService::new()` calls are silent.

### F8 — sync status keys don't match JSON ✅
Fixed together with F1 — Title Case keys now match JSON key style.

### F10 — JSON incident lines in stderr ✅
`log.rs` now prints human-readable format (`⚠️ message`) instead of raw JSON.
JSON incident records still go to the incident ledger file via `append_incident_record`.

### F9 — Empty link table ✅
When 0 links exist, prints "No configured links" instead of empty table border.

## Non-issues (no change)
- F2: Emoji per-concept already consistent
- F6: Repair subcommand naming is functional, not worth a breaking rename
- F7: `--apply` / `--dry-run` already consistent across tools
