# Project State

## Current Focus
Fix commit spam loop: bumper treats .dracon/ and .pub as meaningful, stale project-state.md drives identical commit messages

## Context
193 consecutive commits with identical messages (fix(fix multi-remote): ...) triggered by 3 interacting bugs: (1) NOISE_PATTERNS missing .dracon/ and .pub so pubkey rotation triggered version bumps, (2) stale project-state.md with no AI keys configured produced identical category/scope every cycle, (3) warden publish_repo_pubkey wrote new keys making repos dirty. Added .dracon/ and .pub to NOISE_PATTERNS, added stale focus detection in build_commit_context, added noise-only shortcut to skip scribe and use 'chore: sync metadata', changed plaintext gitattributes from -filter -diff -merge to -filter only.

## Completed
- [x] Fix A: Added .dracon/ and .pub to NOISE_PATTERNS in bump.rs
- [x] Fix B: Verified original logic already returns None for noise+version-only diffs
- [x] Fix C: Stale focus detection — if focus line appears in last commit subject, clear description/category/scope
- [x] Fix D: Noise-only shortcut — skip scribe call, use 'chore: sync metadata' commit message
- [x] Fix F: Changed plaintext gitattributes from `-filter -diff -merge` to `-filter` only (restores normal diffing for Cargo.toml/Cargo.lock etc.)

## In Progress
- Fix H: Compiler warnings (SyncContext dead_code, filter_only_cleared unused)

## Blockers
- None

## Next Steps
1. Fix compiler warnings in dracon-sync
2. Run full test suite
3. Rebuild + deploy + restart services
