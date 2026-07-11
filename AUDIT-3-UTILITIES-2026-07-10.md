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
| dracon-sync | PASS (exit 0) | 0 | 16 (6 unused, 3 function, 2 variable, 2 fields, 1 value, 1 methods, 1 method) |
| dracon-system | PASS (exit 0) | 0 | 0 |
| dracon-warden | PASS (exit 0) | 0 | 0 |

All 3 compile cleanly. dracon-sync carries 16 dead-code/unused warnings (minor).

## 2. Test health — `cargo test --locked`
| Crate | Result | Passed | Failed | Ignored |
|-------|--------|--------|--------|----------|
| dracon-sync | **FAIL (exit 101)** | 647 | **18** | 3 |
| dracon-system | PASS (exit 0) | 86 | 0 | 0 |
| dracon-warden | PASS (exit 0) | 76 + 10 doc-tests | 0 | 0 |

### dracon-sync — 18 failures: root cause
All 18 failures are inside `#[cfg(test)]` test modules: `role::tests`,
`git::discovery::submodule_tests`, `exclude::tests`,
`sync::tests::parent_gitlink_propagates_after_standalone_commit`, and
`daemon::submodule_materialize_tests`.

**Root cause: the globally-installed `dracon-warden` pre-commit hook at
`/home/dracon/.config/git/hooks/pre-commit` blocks `git commit` in any repo
that lacks a `.gitattributes` containing `filter=dracon`.** Because
`core.hooksPath` is set globally, every repo inherits it. The test helpers'
temp repos (created with bare `git init`) have no warden configuration, so the
hook fires and makes `git commit -q -m "init"` exit non-zero with:

    ❌ Warden filter missing from .gitattributes.
       Run: dracon-warden once /tmp/.tmpXXXX

Test-log evidence: lines 1276, 1813–1824 of the `cargo test` output show this
exact error blocking commits in `/tmp/.tmp*` temp repos. The failure cascade:

1. **9 tests** fail at `assertion failed: run(&["commit", "-q", "-m", "init"]).status.success()`
   (the hook made commit exit non-zero).
2. **Discovery / exclude / daemon-materialize tests** then panic downstream
   (`src/git/discovery.rs:840:9`, `src/exclude.rs:733:14`/`796:14`,
   `src/daemon.rs:3159:9`) on assertions that depend on the now-failed commit
   (empty `git ls-tree`, `unwrap()` on missing shared-gitdir `refs/heads/main`,
   `sibling/` not checked out).
3. **Role tests** (`role.rs:227`) pass the now-invalid `head` into
   `git update-index --cacheinfo 160000,,<path>`, which git 2.51.2 rejects with
   `option 'cacheinfo' expects <mode>,<sha1>,<path>`. The empty/bad SHA is a
   *consequence* of the blocked commit, **not** a deliberately-passed empty
   SHA from the test author.

The earlier draft attribution ("tests pass empty SHA, git 2.51.2 rejects") was
incomplete: the empty SHA is a downstream symptom of the hook, not the root.
Corrected 2026-07-11.

**Production is unaffected.** Mapping the 8 `--cacheinfo` call sites:
- `sync.rs:1014` — `async fn stage_gitlink_updates` (**PRODUCTION**): the
  daemon's real gitlink-staging function. Uses real SHAs from the shared
  gitdir's `main` ref.
- `discovery.rs:886/1006/1127` — inside `submodule_tests` (`#[cfg(test)]`).
- `daemon.rs:3196` — inside `submodule_materialize_tests` (`#[cfg(test)]`).
- `sync.rs:7655` — inside `sync::tests::parent_gitlink_propagates_after_standalone_commit`
  (`#[cfg(test)]`).
- `exclude.rs:685` — inside `exclude::tests::build_parent_with_standalone_submodule`
  (`#[cfg(test)]`).
- `role.rs:224` — inside `role::tests::stage_gitlink` helper (`#[cfg(test)]`).

So **only `sync.rs:1014` is production**; the other 7 are test helpers. The
daemon's real gitlink commits worked during this session (darklord + polis),
confirming production `stage_gitlink_updates` is fine — the failures are purely
in test code triggered by the global hook, not by daemon logic.

**Impact:** violates AGENTS.md "`cargo test --workspace --locked` must pass"
for dracon-sync, caused by the global hook environment, not by a code
regression. **Fix options** (any one): (a) have the test helpers write a
minimal `.gitattributes` with `filter=dracon` and configure
`filter.dracon.clean` in the temp repos before commit; (b) override
`core.hooksPath` to an empty path for test invocations, e.g.
`git -c core.hooksPath=/dev/null ...` in `git_c` / command builders; (c) run
the test suite in a sandbox that does not inherit the global hooksPath.

## 3. Dependency / license health — `cargo deny check`
| Crate | Result | Advisories | Notes |
|-------|--------|-----------|-------|
| dracon-sync | PASS (exit 0) | ok | warning: "unmatched skip configuration" |
| dracon-system | **FAIL (exit 1)** | FAILED | RUSTSEC-2026-0190 (unsound `Error::downcast_mut()` in anyhow, via the dracon-system ↔ dracon-system-lib path) |
| dracon-warden | **FAIL (exit 1)** | FAILED | RUSTSEC-2026-0190 (anyhow unsound) + **RUSTSEC-2026-0204 (VULNERABILITY: invalid pointer dereference in `fmt::Pointer` impl for `Atomic`/`Shared` in `crossbeam-epoch v0.9.18` — reached via crossbeam-deque → rayon-core → rayon → dracon-security → dracon-warden; fix: `cargo update -p crossbeam-epoch` to ≥0.9.20)** |

Both dracon-system and dracon-warden fail `cargo deny check` on advisories.
RUSTSEC-2026-0204 (crossbeam-epoch) is a **security vulnerability** — highest-priority item.
(The `Atomic`/`Shared` `fmt::Pointer` types belong to the crossbeam family — specifically
`crossbeam-epoch`. `triomphe` is NOT a dependency of dracon-warden, and
`crossbeam-deque v0.8.6` is merely a transitive dependent of the vulnerable crate. The
final, cargo-deny-attributed crate is **`crossbeam-epoch v0.9.18`**; fix:
`cargo update -p crossbeam-epoch` to ≥0.9.20. Two earlier draft attributions — `triomphe`,
then `crossbeam-deque` — were both inaccurate; corrected 2026-07-11.)

## 4. CONCERNs (priority order)
1. 🔴 **dracon-warden — RUSTSEC-2026-0204** (security vuln, `crossbeam-epoch v0.9.18`
   `Atomic`/`Shared` `fmt::Pointer` invalid-ptr deref, via crossbeam-deque → rayon-core →
   rayon → dracon-security → dracon-warden). **Fix: `cargo update -p crossbeam-epoch` to
   ≥0.9.20.** (`crossbeam-deque v0.8.6` only transitively depends on the vulnerable crate;
   `triomphe` is NOT a dependency at all — both prior attributions were inaccurate.)
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
6. 🟢 **dracon-sync — 16 warnings** (unused/dead_code). Minor; `cargo fix` candidate.

## 5. What's healthy
- All 3 crates compile cleanly (release, locked).
- dracon-system and dracon-warden: 0 test failures; clean licenses / bans / sources.
- dracon-sync: 647/665 tests pass; production gitlink logic verified working this session.
- dracon-sync `cargo deny check` passes (minor skip-config warning).
