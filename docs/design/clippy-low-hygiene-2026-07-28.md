# LOW-hygiene clippy `--all-targets` cleanup — design doc

**Date**: 2026-07-28
**Scope**: pre-existing test-code lints in `dracon-sync` flagged by
`cargo clippy --workspace --locked --all-targets -- -D warnings 2>&1`
accumulated across the v0.112.20→v0.113.4 line.

**Status**: closed in commit `93790a1` (Mon 2026-07-27 23:20:24,
"6 file(s) in src … DELTA:+14/-18 | TEST:2"), the dedicated clippy
cleanup commit that landed immediately before the v0.113.5 M1-M4
MEDIUM-finding batch. This doc produces the per-lint summary
required by the LOW-hygiene clippy goal (`20260728001443-t1ckfc`).

## Actual baseline at v0.113.4 (the pre-fix state)

Run with the current toolchain (verified by checking out
`v0.113.4` and running `cargo clippy --workspace --locked
--all-targets -- -D warnings`):

```
$ git checkout v0.113.4
$ cargo clippy --workspace --locked --all-targets -- -D warnings
error: unnecessary `>= y + 1` or `x - 1 >=`              (test_helpers.rs:258)
error: unused variable: `remotes`                        (git/mod.rs:1128)
error: unnecessary use of `get(repo).is_none()`         (daemon.rs:1825)
error: this call to `clone` can be replaced with …     (git/branch.rs:475)
error: this creates an owned instance just for comparison (policy.rs:1966)
error: used `assert_eq!` with a literal bool            (sync.rs:4974)
error: used `assert_eq!` with a literal bool            (sync.rs:4997)
error: used `assert_eq!` with a literal bool            (sync.rs:5023)
error: used `assert_eq!` with a literal bool            (sync.rs:5046)
error: used `assert_eq!` with a literal bool            (sync.rs:5075)
error: used `assert_eq!` with a literal bool            (sync.rs:5096)
error: used `assert_eq!` with a literal bool            (sync.rs:5113)
error: useless use of `vec!`                            (git/mod.rs:1128)
error: could not compile `dracon-sync` (bin "dracon-sync" test) due to 13 previous errors
```

**Total: 13 lints across 7 categories** (NOT 17 across 8 as an
earlier draft of this doc claimed; that draft was produced before
the actual baseline was re-measured). The v0.113.5 release notes'
"14 unrelated pre-existing baseline clippy warnings" figure was
the CI-visible count including the trailing
`could not compile … due to 13 previous errors` summary line; the
actual lint count is 13, and 7 of those 13 are the
`bool_assert_comparison` instances (5 single-line + 2 multi-line
macros).

## Per-lint summary (N fixed vs M allowed)

| # | Lint category | Count | Status | Fix commit(s) |
|---|---|---|---|---|
| 1 | `int_plus_one` | 1 | FIXED | `test_helpers.rs:255` — `after_count >= initial_count + 1` → `after_count > initial_count` |
| 2 | `unused_variables` | 1 | FIXED | `git/mod.rs:1128` — `let remotes = [...]` (in `93790a1`) was renamed to `let _remotes = [...]` in commit `5557f44`; the binding is now `#[allow(unused)]`-style (via the leading-underscore convention) and is left in place as documentation of the auto-create-remotes contract |
| 3 | `unnecessary_get_then_check` | 1 | FIXED | `daemon.rs:1825` — `assert!(map.get(repo).is_none())` → `assert!(!map.contains_key(repo))` |
| 4 | `cloned_ref_to_slice_refs` | 1 | FIXED | `git/branch.rs:475` — `repair_broken_tracking(&[repo.clone()])` → `repair_broken_tracking(std::slice::from_ref(&repo))` |
| 5 | `cmp_owned` | 1 | FIXED | `policy.rs:1966` — `tx == std::path::PathBuf::from("~templates/x")` → `tx == std::path::Path::new("~templates/x")` |
| 6 | `bool_assert_comparison` | 7 | FIXED | `sync.rs:4974, 4997, 5023, 5046, 5075, 5096, 5113` — all `assert_eq!(result.unwrap(), true/false)` rewritten to `assert!(result.unwrap())` / `assert!(!result.unwrap())` |
| 7 | `useless_vec` | 1 | FIXED | `git/mod.rs:1128` — `let remotes = vec![...]` → `let remotes = [...]` |
| **TOTAL** | 7 categories | **13** | **13 FIXED, 0 ALLOWED** | commits `93790a1` + `5557f44` |

### Note on earlier draft vs actual data

An earlier draft of this doc claimed the fixes were "inline in
the M1-M4 batch commits `572f151..c691555`" with a per-commit
breakdown. That claim was wrong — `93790a1` (a single dedicated
clippy cleanup commit) is where 12 of the 13 fixes landed, plus
`5557f44` for the `unused_variables` rename. The M1-M4 commits
(`572f151`, `4dfc83d`, `a9a2836`, `f73737f`, `c691555`) are the
SYNC-M1..M4 audit fixes (detached_discard, filter-only bypass,
should_push, apply_outcome) — distinct from the clippy cleanup.
The auditor's review of the earlier draft caught this
misattribution; this revision points to the correct commits.

### Verification (current state, 2026-07-28)

```
$ cargo clippy --workspace --locked --all-targets -- -D warnings
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.52s
   exit: 0
```

```
$ cargo clippy --workspace --locked --all-targets -- -D warnings \
    -W clippy::int_plus_one \
    -W clippy::bool_assert_comparison \
    -W clippy::cmp_owned \
    -W clippy::useless_vec \
    -W clippy::unnecessary_get_then_check \
    -W clippy::cloned_ref_to_slice_refs
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
   exit: 0
```

(`unused_variables` is a `rustc` lint, not a `clippy::*` lint, so
it cannot be enabled via `-W clippy::unused_variables` — it lives
at the rustc level. The `cargo clippy --workspace --locked
--all-targets -- -D warnings` command above (the project's
default gate) already covers `unused_variables`.)

All 7 categories are clean at HEAD.

## Decision: no `#[allow(clippy::...)]` introduced

The original LOW-hygiene goal offered "per-lint fixes or batch
`#[allow(clippy::lint_name)]` per module". The FIXED path was
chosen throughout because:

1. All 13 instances were mechanical — fixing them improves
   readability for future maintainers rather than papering over
   an inherent complexity.
2. No `#[allow(clippy::...)]` was introduced for these lints in
   the `93790a1` commit; the per-lint summary above reflects what
   actually shipped.

## Out-of-scope items (deferred)

The LOW-hygiene goal's hard-out-of-scope clause excludes:

- Fixing non-test-code lints (production code) — any clippy
  warning in `src/` outside the test modules remains a separate
  concern. The current `cargo clippy --workspace --locked -- -D warnings`
  gate (without `--all-targets`) already covers production code.
- Banning lints globally — would require changes to `Cargo.toml`
  `[lints]` table or `clippy.toml`; not part of this goal.
- Breaking changes to test setup — no test infrastructure changed.

## Cross-references

- Goal file: `dracon-utilities/.pi-glla/goals/20260728001443-t1ckfc.md`
- Origin goal: `20260726235933-0ik9uq.md` (paused/aborted; subsumed)
- Clippy cleanup commits: `dracon-sync` git `93790a1` (Mon 2026-07-27 23:20:24 — 12 of the 13 fixes) + `5557f44` (the `unused_variables` rename to `_remotes`)
- v0.113.5 CHANGELOG entry: `dracon-sync/CHANGELOG.md` lines 135-139
- v0.113.5 release notes: `dracon-sync/release-notes-v0.113.5.md` lines 115-123
- v0.113.6 (M4 completion): same files, lines covering the M4 helper extraction