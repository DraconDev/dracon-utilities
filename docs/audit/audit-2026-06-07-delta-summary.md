# Delta Audit Summary — 2026-06-07

**Full report:** [`audit-2026-06-07-delta.md`](audit-2026-06-07-delta.md)

## TL;DR

The 2026-06-06 audit's most important findings have been addressed. **3 CI jobs (clippy, fmt, docs) flipped from RED to GREEN** in 24 hours. The `test-ai` cleanup, freeze-marker TTL, archived-goals gitignore, and dead `deny.toml` entries are all done. AGENTS.md is now correct.

## Status of prior audits

| Audit | Findings | Resolved | Still Open | Improved |
|-------|----------|----------|------------|----------|
| v2 (`audit-2026-06-06.md`) | 9 | 6 | 3 | 0 |
| full (`audit-2026-06-06-full.md`) | 30+ | 12 | 13+ | 1 |

## CI status today

| Job | 2026-06-06 | 2026-06-07 |
|-----|-----------|-----------|
| lint (fmt) | ❌ RED | ✅ **GREEN** |
| lint (clippy) | ❌ RED (1 error) | ✅ **GREEN** (0 errors, 4 warnings) |
| docs (strict) | ❌ RED (4 warnings) | ✅ **GREEN** |
| test (serial) | ✅ 575 passed | ✅ **590 passed** |
| deny | ✅ (10 dup + 7 license + 1 source) | ✅ (10 dup, 0 license, 0 source) |

## What was fixed (between yesterday and today)

- F-1.2 / F-1.3 / F-1.4: CI lint and docs jobs now green
- F-7.1.2: `test-ai` references all removed (was 6 places)
- F-7.1.1: AGENTS.md CLI table fixed (nested subcommands)
- F-7.2.1: `dracon-sync/BLUEPRINT.md` "AI Integration" section rewritten
- F-7.2.2: `- [x]` in In Progress section now uses `[~]`
- F-7.3.1: warden BLUEPRINT legend now uses all 3 markers
- F-2.3.1: freeze-marker TTL implemented (24h auto-expire)
- F-8.1 partial: Cargo.lock 20+ → 10 duplicates (v2 audit's count was correct; full audit's was wrong)
- F-8.2: dead `allow-git` in deny.toml commented out
- F-8.3: 7 unused license entries removed from deny.toml
- F-9.1: `install.sh` has `set -euo pipefail`
- F-9.7.1: `verify-spec.sh` uses `--workspace --bins` instead of `--lib`
- F-10.1: 35 archived `.pi/goals/archived/*.md` untracked (0 in git)
- F-10.5: `autoresearch.jsonl` covered by `*.jsonl` rule
- F-6.2: `EnvRestorer` adoption up from 1 to 7 files

## What remains

**Easy (≤ 30 min total):**
- Fix 7+ flat CLI paths in `dracon-sync/README.md` and 1 in `docs/OPERATIONS.md`
- Remove `dracon-sync/note.md` (113B leftover todo)
- Untrack 4 tarpaulin reports (1.6+ MB)
- Silence 8 dead-code warnings on `print.rs` helpers (new regression)
- Fix 4 remaining clippy warnings in sync (`tokio_git_command` import, dead `stop_reason`/`title` fields, dead `test_deletions_committed_when_intentional`)
- Update test counts in AGENTS.md (claims 686, real 590) and project-state.md (claims 575, real 590)

**Medium:**
- `cargo dedupe` (1-2 h, awaits `dracon-libs` pin)
- `sync.rs` modularization (4469 lines, was 4340 yesterday — same pattern that worked for `git/mod.rs`)

**Deferred:**
- Pedantic+nursery clippy gating
- `reqwest` blocking feature refactor
- `verify-spec.sh` improvements
- Re-run tarpaulin

## Top 3 next steps (in order)

1. **Fix the 2 docs with flat CLI paths** (10 min) — unblocks humans and AI agents
2. **Remove `note.md` + untrack tarpaulin** (10 min) — repo hygiene
3. **Silence 8 dead-code warnings** (5 min) — restore CI clippy to 0 warnings

Total: **~25 minutes** to clear all the remaining P3 surface.
