# Untracked audit/research artifact policy

Goal: determine and implement the best Dracon approach for untracked audit/research artifacts such as `one-mil-girls` evidence.

## Decision

Keep untracked audit/research artifacts **visible** in `dracon-sync repos`, but do **not** let untracked-only files trigger WARN/CONCERN status or sync commits.

This means:
- The `UT` column continues to show untracked files/directories.
- Untracked audit evidence, screenshots, research reports, smoke-test artifacts, and validation logs remain visible for operator awareness.
- These files are not deleted, ignored, hidden, or committed without explicit approval.
- They stay `✅ OK` when there is no tracked modification, staged content, ahead/behind state, or missing remote/upstream state.
- Modified tracked files and staged content still trigger WARN/sync behavior as before.

## Rationale

Untracked files have two very different meanings in Dracon repos:

1. **Build/cache artifacts**: `target/`, `node_modules/`, generated data, browser outputs.
2. **Audit/research evidence**: screenshots, smoke-test summaries, research reports, validation logs.

Both should be visible in the report so the operator knows they exist, but neither should automatically become sync work. Sync commits should be driven by tracked modifications/staged changes, not by untracked-only evidence.

## Implementation

### Code

Updated `dracon-sync/src/report.rs`:
- Clarified `repo_is_warn()` comments to state that untracked files remain visible in the `UT` column but are not sync-relevant by themselves.
- Added regression test `report::tests::test_repo_is_warn_untracked_only_is_not_warn`.

The new test verifies:
- A repo with `untracked_files = 5`, `modified_files = 0`, and `staged_files = 0` is **not WARN**.
- Its flags are `DIRTY`, which allows the report to show the untracked state without classifying it as WARN.

### Documentation

Updated `AGENTS.md` under the untracked vs modified distinction:
- `UT` column is explicitly visible, not filtered out.
- Untracked audit/research artifacts are examples of visible untracked evidence.
- Do not delete, ignore, hide, or commit such artifacts without explicit approval.
- They remain `OK` unless there is tracked modification, staged content, ahead/behind state, or missing remote/upstream state.

## Evidence

Evidence directory:

`docs/audit/2026-06-11-full-repo-audit/untracked-artifacts-policy/evidence/`

Key files:
- `policy-diff.patch`
- `focused-repos.md`
- `dracon-sync-repos.json`
- `one-mil-girls-status.txt`
- `browser-extensions-shared-status.txt`
- `cargo-fmt-check.log`
- `cargo-fmt-exit.txt`
- `cargo-test.log`

## Focused repo evidence

`dracon-sync repos --json --full-path` showed:
- `one-mil-girls`: `untracked=5`, `warn=false`, `concern=false`, `hint=healthy`.
- `browser-extensions-shared`: `untracked=2`, `warn=false`, `concern=false`, `hint=healthy`.

`git status --short --branch --untracked-files=all` confirmed:
- `one-mil-girls` has untracked audit/research files.
- `browser-extensions-shared` has no visible untracked files after its last commit; the `UT=2` count is stale daemon/report cache.

## Validation

- Focused test: `cargo test -p dracon-sync test_repo_is_warn_untracked_only_is_not_warn -- --test-threads=1` → passed.
- Formatting: `cargo fmt --check` → passed.
- Full workspace: `cargo test --workspace -- --test-threads=1` → passed.

## Constraints respected

- No user-owned notes, screenshots, pasted images, audit evidence, research docs, or local task/session state were deleted, renamed, ignored, hidden, or committed.
- Existing tracked-file sync behavior is preserved.
- No force-push, rebase, history rewrite, visibility change, publish, secret rotation, or branch deletion was performed.
- No compatibility shims, TODO placeholders, dead code, duplicated logic, or hidden assumptions were added.

## Final state

The best approach is implemented and documented: untracked evidence remains visible, untracked-only repos remain OK, and tracked modifications remain the sync trigger.
