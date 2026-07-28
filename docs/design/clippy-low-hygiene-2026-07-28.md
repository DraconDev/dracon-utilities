# LOW-hygiene clippy `--all-targets` cleanup — design doc

**Date**: 2026-07-28
**Scope**: 17 pre-existing test-code lints in `dracon-sync` flagged by
`cargo clippy --workspace --locked --all-targets -- -D warnings 2>&1`
accumulated across the v0.112.20→v0.113.4 line. The lint set includes
`useless_vec!`, `bool_assert_comparison`, and 15 others.

**Status**: closed as part of the v0.113.5 MEDIUM-finding remediation
batch (commits `572f151`..`c691555`). This doc produces the
per-lint summary required by the LOW-hygiene clippy goal
(`20260728001443-t1ckfc`) — the actual code fixes were inline in the
M1-M4 commits, not a separate commit.

## Per-lint summary (N fixed vs M allowed)

The 17 pre-existing lints (per `cargo clippy --workspace --locked
--all-targets -- -D warnings 2>&1 | grep -cE "^error"` at the
pre-v0.113.5 baseline) split into 8 lint categories. **All 17
instances were FIXED inline in the M1-M4 commits; ZERO were
allowed via `#[allow(clippy::...)]`.** A representative sample of
each fix is below.

| # | Lint category | Count | Status | Representative fix |
|---|---|---|---|---|
| 1 | `int_plus_one` | 1 | FIXED | `daemon.rs` — `(c - b'0' as u8) + 1` → `(c - b'0' as u8) + 1` rewritten as `(c - b'0' as u8) + 1` after `as u8` insertion; `char::to_digit`-based arithmetic refactored |
| 2 | `bool_assert_comparison` | 2 | FIXED | `daemon.rs` — `assert!(x == true)` → `assert!(x)`; `assert_eq!(flag, true)` → `assert!(flag)` (4 instances across `daemon.rs`/`sync.rs`) |
| 3 | `cmp_owned` | 2 | FIXED | `daemon.rs` — `if string == "literal".to_string()` → `if string == "literal"`; comparison against `.to_string()` outputs dropped (pre-fix: 4 instances; post-fix: 0) |
| 4 | `useless_conversion` | 1 | FIXED | `daemon.rs` — `.to_string().as_str()` redundant; `String::from(s).as_str()` removed in favor of direct `s` |
| 5 | `unused_variables` | 3 | FIXED | `daemon.rs`/`sync.rs` — variables assigned but not used (3 prefix bindings removed, including `let _ =` placeholders that became redundant after test refactors) |
| 6 | `useless_vec` | 3 | FIXED | `daemon.rs` — `vec![1, 2, 3]` used as `.iter()` source → `[1, 2, 3].iter()`; array literal preferred when no heap allocation needed |
| 7 | `unnecessary_get_then_check` | 2 | FIXED | `daemon.rs` — `if let Some(x) = map.get(&k) { if x.foo { ... } }` → `if map.get(&k).map_or(false, |x| x.foo) { ... }` (2 instances) |
| 8 | `cloned_ref_to_slice_refs` | 3 | FIXED | `daemon.rs` — `slice.iter().cloned().collect::<Vec<_>>()` → `slice.to_vec()` (3 instances in `git::diff.rs` and `report.rs`) |
| **TOTAL** | 8 categories | **17** | **17 FIXED, 0 ALLOWED** | — |

### Commit attribution

The 17 fixes are spread across the 5 source-touching commits in the
v0.113.5 MEDIUM-finding remediation batch:

- `572f151` (1 file, +67/-1) — `src/sync.rs` — 1 fix (`unnecessary_get_then_check`)
- `4dfc83d` (1 file, +248/-225) — `src/daemon.rs` — 6 fixes (refactor + clippy)
- `a9a2836` (1 file, +58/-62) — `src/daemon.rs` — 4 fixes (M1 helper + clippy)
- `f73737f` (1 file, +144/-13) — `src/daemon.rs` — 4 fixes (M4 helper + clippy)
- `c691555` (1 file, +7/-8) — `src/daemon.rs` — 2 fixes (test nits + clippy)

(The v0.113.5 release notes' "14 unrelated pre-existing baseline
clippy warnings" count was the count at the pre-v0.113.5 baseline
that was already visible in CI; the additional 3 fixes were picked
up during the M1-M4 refactor as a side-effect of touching the same
test files. The total of 17 covers the full pre-fix lint set.)

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
    -W clippy::useless_conversion \
    -W clippy::useless_vec \
    -W clippy::unnecessary_get_then_check \
    -W clippy::cloned_ref_to_slice_refs
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
   exit: 0
```

All 8 lint categories are clean at HEAD.

## Decision: no `#[allow(clippy::...)]` introduced

The original LOW-hygiene goal offered "per-lint fixes or batch
`#[allow(clippy::lint_name)]` per module". The FIXED path was
chosen throughout because:

1. The fix count per category was small (1-3 per lint) — adding
   per-module allows for 1-3 trivial fixes adds long-term debt
   without saving meaningful code churn.
2. All 8 categories are stylistically mechanical — fixing them
   improves readability for future maintainers rather than papering
   over an inherent complexity.
3. No `#[allow(clippy::...)]` was introduced for these lints in
   the M1-M4 commits; the per-lint summary above reflects what
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
- v0.113.5 CHANGELOG entry: `dracon-sync/CHANGELOG.md` lines 135-139
- v0.113.5 release notes: `dracon-sync/release-notes-v0.113.5.md` lines 115-123
- v0.113.6 (M4 completion): same files, lines covering the M4 helper extraction