# dracon-sync v0.112.18 — Full audit + clippy cleanup + private-orphan purge

**Released:** 2026-07-18
**Goal:** `e6c92613-e663-410c-b4f1-f876acb0f876` — "looking good but are?"
**Type:** Internal audit + clippy cleanup + codeberg hygiene

---

## TL;DR

Three changes in one release:

1. **23 clippy errors fixed** (1 substantive bug + 22 stylistic warnings). The substantive bug was a tautological test assertion in `sync.rs:6787` that was masking a real test invariant mistake — fixed.
2. **21 private orphan repos deleted from codeberg** (1.353 GiB total). Aligns codeberg with the public-only policy.
3. **Stale CHANGELOG references fixed** for v0.112.13/14 release notes.

Plus a **full audit report** at `AUDIT_FULL_2026-07-18.md` covering every dimension of the daemon + all 31 watched repos.

---

## The substantive bug: sync.rs:6787

```rust
// BEFORE (broken — clippy::logic-bug):
assert!(
    staged_files.lines().all(|l| l != "sibling" || true),
    "staged files line listing: {}",
    staged_files
);

// The `|| true` makes the predicate always-true. This was masking the
// fact that the test was asserting the WRONG invariant: a successful
// stage_gitlink_updates() legitimately stages the gitlink name 'sibling'
// in `git diff --cached --name-only`. The test should have been
// asserting that NO files from INSIDE sibling/ (i.e. 'sibling/a.txt',
// 'sibling/b.txt') are staged — which the two preceding assertions
// already cover.
```

The previous tautology meant the test passed even when the daemon was
behaving correctly (gitlink name was correctly staged in the diff).
The fix removes the broken assertion and adds a NOTE comment explaining
the correct invariant.

**Did this affect production behavior?** No — the assertion was in
a unit test, not in the daemon's runtime path. The daemon's
`stage_gitlink_updates()` was always working correctly. The bug was
purely in the test's self-validation.

---

## The 22 stylistic clippy fixes

| File | Lint | Fix |
|---|---|---|
| daemon.rs:362 | empty line after doc comment | removed the empty line |
| daemon.rs:144 | useless `as_ref` | `repo.as_ref()` → `repo` |
| sync.rs:1509 | useless `as_ref` | `repo.as_ref()` → `repo` |
| sync.rs:6978 | useless `format!` | `format!("...")` → `"...".to_string()` |
| exclude.rs:713 | useless `format!` | inlined literal string |
| exclude.rs:699 | useless `vec!` | `[String; 4]` array |
| report.rs:7877 | useless `vec!` | `[String; 3]` array |
| report.rs:6136 | duplicated `#[test]` | removed duplicate |
| report.rs:3314 | `sort_by` → `sort_by_key` | `sort_by_key(\|f\| Reverse(f.1.total_size_bytes))` |
| report.rs:3375 | `print_literal` | inlined literal into format string |
| report.rs:3383, 3387 | needless borrow | `&leaf` → `leaf` |
| report.rs:7253, 7275, 7296, 7321 | `let_unit_value` | `let _ = ...` → bare call |
| policy.rs:946, 955, 956 | doc list overindented | 7-space → 4-space continuation |
| policy.rs:1875 | doc list without indentation | added empty separator line |
| policy.rs:2134 | unnecessary cast u64→u64 | removed `as u64` |
| report.rs:854 | doc list without indentation | added empty separator line |

---

## Codeberg private orphan purge

21 private orphan repos deleted via the codeberg API. Total size
1.353 GiB. Each repo was verified as 404 after deletion.

```
SamAI                          0.392 GiB
dracon-demons                  0.205 GiB
live                           0.190 GiB
dracon-rust-ui                 0.139 GiB
dracon-voice-notifications     0.106 GiB
dracon-spark-and-director      0.084 GiB
.dracon                        0.077 GiB
kiki-sassy-desktop-announcer   0.056 GiB
dracon-utilities-legacy        0.038 GiB
shared-config                  0.025 GiB
cli-file-manager               0.025 GiB
video-factory                  0.008 GiB
wal-backup                     0.004 GiB
video-uploader                 0.001 GiB
quick-draw-screenshot-clipboard 0.001 GiB
dracon-sync                    0.001 GiB
DraconDev-private              0.001 GiB
todo-addict                    0.000 GiB
test_banner                    0.000 GiB
test-auto-create               0.000 GiB
pi-global-context-limit        0.000 GiB
TOTAL                          1.353 GiB
```

These were all orphaned (no local source-of-truth pointed to them) and
violated the public-only policy (codeberg is now a public-only marketing
surface per goal `219d97db` from 2026-07-17).

**Quota impact**: codeberg quota dropped from 75.2491 GiB → 73.8961 GiB
(after the 1.353 GiB deletion). Now at 86.94% of the 85 GiB limit.

---

## 33 public orphans: review pending

33 public orphan repos (1.378 GiB total) were identified but NOT
deleted in this release. The operator asked to review the list before
deciding. The list is available in the audit report
`AUDIT_FULL_2026-07-18.md` (AC #3 section). Top offenders:
- ai-vid-editor (0.353 GiB)
- ai-gui-auto-video-editor (0.350 GiB)
- brics (0.185 GiB)
- kittentts-showcase (0.096 GiB)
- dracon-libs (0.093 GiB)
- ... and 28 more.

---

## Stale CHANGELOG references

The meta-repo `CHANGELOG.md` referenced `release-notes-v0.112.13.md`
and `release-notes-v0.112.14.md` at the root, but those files only
existed at `dracon-sync/release-notes-v0.112.1[34].md` (inside the
nested standalone repo). Fixed by updating the references to point
to the correct nested path.

---

## Audit report

Full audit at `AUDIT_FULL_2026-07-18.md` (15.2 KiB). Covers:

- AC #1: daemon code (build, test, deny, clippy, eprintln/unwrap patterns)
- AC #2: each of 31 watched repos individually
- AC #3: PUSH_STUCK, codeberg quota, orphan analysis, scan-bloat
- AC #4: daemon health (uptime, memory, CPU, errors)
- AC #5: meta-repo consistency (CHANGELOG, AGENTS.md, mirror sync)
- AC #6: findings with verdicts
- AC #7: operator sign-off decisions
- AC #8: final tally verification

---

## Verification

```bash
$ cargo build --release --locked
   Compiling dracon-sync v0.112.18
    Finished `release` profile [optimized] target(s) in 28.24s

$ cargo test --workspace --locked
test result: ok. 705 passed; 0 failed; 3 ignored
test result: ok. 10 passed; 0 failed
test result: ok. 86 passed; 0 failed
test result: ok. 76 passed; 0 failed
test result: ok. 10 passed; 0 failed
TOTAL: 887 tests passing, 0 failed

$ cargo clippy --workspace --locked --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.79s

$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
```