# TODO-Context Mode — AI-to-AI Implementation

Strategy: **Ratio & Fact Reporting** — dumb deterministic stenographer, no semantic scope matching.
Title is routing key for downstream AI: `sync: X checked`. Body is JSON with ledger_delta, code_delta, verification.

---

## ✅ Status

- [x] **Parser** — `parse_todo_task()` exists in `todo_parser.rs`, tested
- [x] **Config** — `todo_commit_messages: bool` in policy, defaults to `false`
- [x] **Wiring** — `sync.rs` uses `todo_context_message()` when toggle is on
- [x] **Format update** — Changed to JSON routing-key format
- [x] **Tests** — Updated integration tests

---

## Summary

| Stage | What | Files | Risk |
|-------|------|-------|------|
| 1 | Parser | `todo_parser.rs` | Low |
| 2 | Formatter | `scribe.rs` (update to JSON) | Low |
| 3 | Config | `policy.rs` + example.toml | Low |
| 4 | Wiring | `sync.rs` | Low |
| 5 | Tests | Integration tests | Low |

**Existing behavior preserved** — toggle defaults to `false`.
When `true` and no `[ ]` found, it silently falls back to `local_fallback_message`.