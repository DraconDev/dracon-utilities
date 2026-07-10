# Audit — 3 dracon-utilities projects (2026-07-10)

**Goal:** `mrfgzxre-n5fqe6` — "audit what we have all 3 projects" → the 3 utilities in
`dracon-utilities` (confirmed via README + user clarification): **dracon-sync,
dracon-system, dracon-warden**.

**Environment:** cargo 1.95.0 · git 2.51.2 · cargo-deny 0.19.9 · Linux (nixos).

## Scope note
`dracon-utilities` is a monorepo of **3 independent crates — there is no workspace-root
`Cargo.toml`**. Each was built / tested / denied individually under its own directory.
`dracon-sync` depends on the published `dracon-git` 94.7.0 (crates.io), so the sibling
`dracon-libs` repo is NOT required to build. The README's "build from the monorepo root"
instruction does not work as written (no root manifest) — build each crate under its dir.

## 1. Build health — `cargo build --release --locked`
| Crate | Result | Errors | Warnings |
|-------|--------|--------|----------|
| dracon-sync | PASS (exit 0) | 0 | 17 (6 unused, 3 function, 2 variable, 2 fields, 1 value, 1 methods, 1 method) |
| dracon-system | PASS (exit 0) | 0 | 0 |
| dracon-warden | PASS (exit 0) | 0 | 0 |

All 3 compile cleanly. dracon-sync carries 17 dead-code/unused warnings (minor).

## 2. Test health — `cargo test --locked`
| Crate | Result | Passed | Failed | Ignored |
|-------|--------|--------|--------|----------|
| dracon-sync | **FAIL (exit 101)** | 647 | **18** | 3 |
| dracon-system | PASS (exit 0) | 86 | 0 | 0 |
| dracon-warden | PASS (exit 0) | 76 + 10 doc-tests | 0 | 0 |

### dracon-sync — 18 failures: root cause
All 18 are in submodule/gitlink/discovery tests (`role::tests`,
`git::discovery::submodule_tests`, `exclude::tests`,
`sync::tests::parent_gitlink_propagates_after_standalone_commit`,
`daemon::submodule_materialize_tests`).

**Cause: git 2.51.2's stricter `git update-index --cacheinfo` validation.** The tests
deliberately pass an **empty SHA** (e.g. `role.rs` `stage_gitlink` comment: *"empty SHA is
fine"*), producing `--cacheinfo 160000,,<path>`, which git 2.51.2 rejects with
`option 'cacheinfo' expects <mode>,<sha1>,<path>`. That single rejection cascades: the
gitlink is never staged → `git ls-tree` returns empty (assertion failure) and the
shared-gitdir `refs/heads/main` is never written (subsequent `unwrap()` panics with
`NotFound`).

This is a **test-suite / git-version compatibility defect, NOT a daemon logic regression**:
- Every production `--cacheinfo` call site already uses the correct comma form
  `160000,<sha>,<path>` (discovery.rs:886/1006/1126, daemon.rs:3195, sync.rs:1017/7654,
  exclude.rs:689, role.rs:223).
- The daemon's real gitlink commits worked during this session (darklord + polis), proving
  production is unaffected.

**Impact:** violates AGENTS.md "`cargo test --workspace --locked` must pass" for
dracon-sync. Pre-existing / environmental, not a recent code regression. Fix: update the
test helpers to use a valid (non-empty) SHA or a git-2.51.2-compatible cacheinfo invocation.

## 3. Dependency / license health — `cargo deny check`
| Crate | Result | Advisories | Notes |
|-------|--------|-----------|-------|
| dracon-sync | PASS (exit 0) | ok | warning: "unmatched skip configuration" |
| dracon-system | **FAIL (exit 1)** | FAILED | RUSTSEC-2026-0190 (unsound `Error::downcast_mut()` in anyhow, via the dracon-system ↔ dracon-system-lib path) |
| dracon-warden | **FAIL (exit 1)** | FAILED | RUSTSEC-2026-0190 (anyhow unsound) + **RUSTSEC-2026-0204 (VULNERABILITY: invalid pointer dereference in `fmt::Pointer` impl for `Atomic`/`Shared` in `crossbeam-deque v0.8.6` — reached via rayon-core → rayon → dracon-security → dracon-warden)** |

Both dracon-system and dracon-warden fail `cargo deny check` on advisories.
RUSTSEC-2026-0204 (crossbeam-deque) is a **security vulnerability** — highest-priority item.
(The `Atomic`/`Shared` `fmt::Pointer` types belong to the crossbeam family; `triomphe` is
NOT a dependency of dracon-warden — the earlier triomphe attribution was incorrect and has
been corrected to `crossbeam-deque v0.8.6`.)

## 4. CONCERNs (priority order)
1. 🔴 **dracon-warden — RUSTSEC-2026-0204** (security vuln, `crossbeam-deque v0.8.6`
   `Atomic`/`Shared` `fmt::Pointer` invalid-ptr deref, via rayon-core → rayon →
   dracon-security → dracon-warden). Bump `crossbeam-deque`/`rayon` to a fixed version.
   (`triomphe` is NOT a dracon-warden dependency — corrected from earlier draft.)
2. 🟠 **dracon-system & dracon-warden — RUSTSEC-2026-0190** (anyhow `Error::downcast_mut`
   unsound). Bump `anyhow` or apply a justified `deny.toml` skip.
3. 🟡 **dracon-system — NOT a cyclic dependency (corrected).** The `cargo deny` advisory
   graph shows `dracon-system` ↔ `dracon-system-lib` paths to anyhow
   (RUSTSEC-2026-0190). `cargo tree` exits 0, so there is **no true cyclic dependency**
   in the resolved graph — the `(*)` marker is a graph back-reference (diamond path to
   anyhow), not a build cycle. No action required; noted for accuracy.
4. 🟡 **dracon-sync — 18 test failures** (git-2.51.2 `cacheinfo` test incompat). Fix test
   helpers; does not affect production.
5. 🟡 **No workspace-root `Cargo.toml`** — README "build from monorepo root" is inaccurate;
   build per-crate.
6. 🟢 **dracon-sync — 17 warnings** (unused/dead_code). Minor; `cargo fix` candidate.

## 5. What's healthy
- All 3 crates compile cleanly (release, locked).
- dracon-system and dracon-warden: 0 test failures; clean licenses / bans / sources.
- dracon-sync: 647/665 tests pass; production gitlink logic verified working this session.
- dracon-sync `cargo deny check` passes (minor skip-config warning).
