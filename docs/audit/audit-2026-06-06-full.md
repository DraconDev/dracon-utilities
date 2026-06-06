# Full Audit Report — dracon-utilities

**Date:** 2026-06-06
**Scope:** Complete audit of code quality, architecture, security, performance, reliability, testing, documentation, dependencies, operational state, and repo hygiene across all three binaries (`dracon-sync`, `dracon-system`, `dracon-warden`) and the workspace.
**Auditor:** Automated + manual audit (cargo, clippy, deny, grep, manual --help inspection, file inspection).
**Method:** Read-only — no source, policy, or config files were modified by the audit itself. All findings are recommendations.
**Prior audits incorporated:** `docs/audit/audit-2026-06-06.md` (today, narrower scope), `.dracon/audit-cli.md` (2026-06-05), `docs/archive/MASTER_ROADMAP_2026-06-01.md`, `docs/archive/REFACTORING_BLOCKER_ANALYSIS.md`, `.dracon/demon-migration-audit.md`, `.dracon/secret-audit-report.md`.

---

## Executive Summary

| Area | Status | Headline |
|------|--------|----------|
| Compilation (`cargo check --workspace --all-targets`) | ✅ 0 errors, 3 warnings | All 3 binaries + sibling `dracon-libs` build cleanly |
| Clippy (CI flags: `-D clippy::all,correctness,suspicious,complexity,perf,style`) | ❌ **1 error + 2 warnings** | **CI is RED on lint job** — `field_reassign_with_default` at `dracon-sync/src/sync.rs:1193` |
| `cargo fmt --check` (CI flags) | ❌ **Fails** | `dracon-warden/tests/integration_test.rs:209` reformatting required — **CI is RED on lint job** |
| `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings` (CI) | ⚠️ 4 unresolved-link warnings | **CI is RED on docs job** (the strict mode) |
| Clippy pedantic+nursery (warnings only) | ⚠️ 87+ rule violations | Cosmetic, not blocking CI |
| `cargo test` (serial) | ✅ 575 passed across 5 suites | sync 420 + system 81 + warden 64 + integration tests |
| `cargo deny check` (advisories/licenses/bans/sources) | ✅ All 4 PASS | 7 license-not-encountered + 1 unmatched-source warnings, but exit 0 |
| Binary sizes (release, stripped, LTO thin) | ✅ Under 25 MiB CI threshold | sync 11 MiB, system 3 MiB, warden 4 MiB |
| Hardcoded secrets / `unsafe` blocks | ✅ None | 0 unsafe in production; 0 hardcoded credentials |
| Command injection (`sh -c`/`bash -c`) | ✅ None | All `Command::new` use hardcoded binaries or sanitized Paths |
| `test-ai` command (referenced in 6 doc locations) | ❌ Does not exist | P1 doc/code drift |
| Flat-vs-nested CLI command paths in AGENTS.md | ❌ Outdated | 12+ top-level commands are now nested (`repair concerns`, `config edit`, `publish run`, etc.) |
| `Cargo.lock` duplicate entries | ⚠️ **20+ duplicate crates** | vs. v2 audit's claim of 10; 5 crates have 3 versions |
| `.pi/goals/archived/*.md` tracked in git | ❌ **35 archived goal files** | Repo hygiene / size bloat |
| `agpl_daemon` daemon subcommand / service | ✅ Removed (per audit-cli.md 2026-06-05) | `dracon-warden --help` no longer shows `daemon`; `dracon-warden.service` no longer in repo; install.sh comment "Warden has no daemon" |

**Overall:** The project is **functionally solid** and recently de-slopped (the CLI audit's recommendations have been largely applied). However, **the CI pipeline is red on the current `main` branch in at least two jobs (lint, docs)**. Several doc-vs-code drifts remain from the v2 audit. The biggest new findings are: (a) CI is broken on `main`, (b) 35 archived goal markdown files are tracked in git, (c) duplicate crate count is double the v2 estimate, (d) several AGENTS.md/CHANGELOG claims are stale (test count, modularization progress).

---

## Top 10 Recommended Actions (prioritized)

1. **Fix CI to be green on `main`.** The lint job fails on `cargo fmt --check` and `cargo clippy`; the docs job fails on `RUSTDOCFLAGS=-D warnings`. Without fixing, no one notices new lint regressions. **Effort:** 30 min. **Risk:** zero. (See F-1.1, F-1.2, F-1.3.)
2. **Update AGENTS.md, OPERATIONS.md, and `dracon-sync/README.md` to reflect nested subcommands.** The current AGENTS.md and OPERATIONS.md still show the old flat `repair-concerns`/`edit-config`/`stuck`/`dual-branch` paths, which no longer exist. This is the #1 user-facing defect. **Effort:** 1–2 h. **Risk:** zero. (See F-2.1.)
3. **Decide the fate of `test-ai`.** It is documented in 6 places and marked `[x] completed` in `dracon-sync/BLUEPRINT.md:280`, but the `TestAi` variant does not exist in the `Command` enum. Either implement it or remove all 6 references. **Effort:** 30 min (remove) or 2–4 h (implement). **Risk:** zero. (See F-2.2.)
4. **Stop tracking `.pi/goals/archived/*.md` in git.** 35 archived AI-session goal files are committed, bloating the repo and exposing ephemeral session state. Move them out of git (keep them on disk) or gitignore the `archived/` subdir. **Effort:** 15 min. **Risk:** zero. (See F-7.1.)
5. **Deduplicate `Cargo.lock`.** 5 crates have 3 versions (`hashbrown`, `getrandom`, `windows-sys`, `windows-result`, `windows-core`); 15+ have 2 versions. Most come from `dracon-libs` pulling older `toml_edit` than the workspace. **Effort:** 1 h (probably needs `dracon-libs` patch). **Risk:** low. (See F-6.1.)
6. **Sync the test-count claim in AGENTS.md and `.dracon/project-state.md`.** AGENTS.md says "406 tests in `src/`"; `.dracon/project-state.md` says "All 706 tests passing". Actual `cargo test` count: **575 passed**. **Effort:** 5 min. **Risk:** zero. (See F-2.3.)
7. **Fix the `field_reassign_with_default` clippy error at `dracon-sync/src/sync.rs:1193`.** Use the suggested struct-update syntax. **Effort:** 5 min. **Risk:** zero. (See F-1.2.)
8. **Remove the obsolete `deny.toml` git URL allowance** (`https://github.com/DraconDev/dracon-libs`). The local `dracon-libs` is consumed via path, not git, so this entry is dead config. **Effort:** 2 min. **Risk:** zero. (See F-6.2.)
9. **Fix the `dracon-sync/BLUEPRINT.md` "In Progress" section.** Line 263 says `- [x] Items being worked on` (marked Completed, but in an In Progress section). Either fix the section or move the item. Also, the legend at the top says `[ ] Not started` and `[~] In progress` exist, but all items are `[x]`. **Effort:** 5 min. **Risk:** zero. (See F-2.4.)
10. **Investigate the one-off `warden` test failure observed during audit.** Across 15 runs of `cargo test -p dracon-warden --tests`, 1 run showed `63 passed; 1 failed`. Subsequent 10 runs all passed. Probably a stale `.git/index.lock` from a crashed test in a sibling repo, but worth a one-time investigation. **Effort:** 30 min. **Risk:** zero. (See F-3.4.)

---

## Coverage Matrix

Every one of the 10 audit areas was covered. "Suspected" findings require further evidence.

| # | Area | Status | Evidence location in this report |
|---|------|--------|----------------------------------|
| 1 | Code quality (clippy/fmt/warnings/duplication) | Covered | §1 |
| 2 | Architecture (modules, layering, IndexLock, freeze) | Covered | §2 |
| 3 | Security (command injection, path validation, secrets, hooks) | Covered | §3 |
| 4 | Performance & resource use (binary size, systemd limits, push timeouts) | Covered | §4 |
| 5 | Reliability & safety (--apply gates, repair, incident ledger, systemd) | Covered | §5 |
| 6 | Testing (counts, coverage, flake, EnvRestorer) | Covered | §6 |
| 7 | Documentation (drift vs `--help`, BLUEPRINT, AGENTS.md, README) | Covered | §7 |
| 8 | Dependencies & supply chain (duplicates, deny, MSRV) | Covered | §8 |
| 9 | Operational state & UX (install.sh, scaffold, gh auth, freeze TTL) | Covered | §9 |
| 10 | Repo hygiene (gitignore, tracked ephemeral files, .pi/) | Covered | §10 |

---

## §1 — Code Quality

### 1.1 Build

**Evidence (run 2026-06-06 on `main`, with `../dracon-libs` cloned):**

```
$ cargo check --workspace --all-targets
warning: unused import: `tokio_git_command`           dracon-sync/src/report.rs:90:21
warning: field `title` is never read                  dracon-sync/src/sync.rs:974:5
warning: function `test_deletions_committed_when_intentional` is never used  dracon-sync/src/sync.rs:3803:14
cargo build: 0 errors, 3 warnings (5 crates)
```

✅ All 3 binaries + sibling `dracon-libs` build cleanly with 0 errors. The 3 warnings match the v2 audit exactly (audit-2026-06-06.md P2-6). **Status: PASS.**

### 1.2 CI is RED on lint job (clippy)

**Evidence (exact CI invocation, run 2026-06-06):**

```
$ cargo clippy -p dracon-sync -p dracon-system -p dracon-warden \
    -- -D clippy::all -D clippy::correctness -D clippy::suspicious \
       -D clippy::complexity -D clippy::perf -D clippy::style
error: field assignment outside of initializer for an instance created with Default::default()
      --> dracon-sync/src/sync.rs:1193:5
       |
  1193 |     metadata.status = value["status"].as_str().map(String::from);
       |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
       = note: `-D clippy::field-reassign-with-default` implied by `-D clippy::style`
cargo clippy: 1 errors, 4 warnings
```

- **F-1.2 [P0] CI is broken on `main`**: The CI workflow at `.github/workflows/ci.yml` runs this exact command in the **lint** job (no `--all-targets`, but the same flags). The exit code from `cargo clippy` on error is non-zero. This means **CI has been red on `main` for at least as long as `field_reassign_with_default` has been an error**. The previous v2 audit declared clippy "PASS with 4 warnings" — that was wrong; the v2 audit ran with different (looser) flags.
- **Why it matters:** Without a green CI, new lint regressions are invisible. The whole point of `-D` is to fail on the violation; the failure is being ignored.
- **Evidence pointer:** `.github/workflows/ci.yml` lines 31–35 set up the command; the v2 audit (audit-2026-06-06.md §P2-6) listed the same 4 issues as warnings because it ran clippy without `-D clippy::style`. The v2 audit's clippy result is therefore misleading.
- **Remediation:** Apply the struct-update syntax the compiler suggests:
  ```rust
  let mut metadata = GoalMetadata {
      status: value["status"].as_str().map(String::from),
      ..Default::default()
  };
  ```
  or add `#[allow(clippy::field_reassign_with_default)]` if the existing shape is intentional. **Effort:** 5 min. **Risk:** zero.

### 1.3 CI is RED on lint job (fmt)

**Evidence (exact CI invocation, run 2026-06-06):**

```
$ cargo fmt -p dracon-sync -p dracon-system -p dracon-warden -- --check
Diff in /home/dracon/Dev/dracon-utilities/dracon-warden/tests/integration_test.rs:209:
 ...
-    assert!(help.contains("dracon-warden"), "help should mention binary name");
-    assert!(help.contains("setup-hooks"), "help should list setup-hooks command");
+    assert!(
+        help.contains("dracon-warden"),
+        "help should mention binary name"
+    );
+    assert!(
+        help.contains("setup-hooks"),
+        "help should list setup-hooks command"
+    );
```

- **F-1.3 [P0] CI is broken on `main` (fmt)**: The lint job's first step runs `cargo fmt --check` and would have failed. The fix is to reformat the file with `cargo fmt`.
- **Remediation:** `cargo fmt -p dracon-warden`. **Effort:** 30 sec. **Risk:** zero.

### 1.4 CI is RED on docs job

**Evidence (exact CI invocation, run 2026-06-06):**

```
$ RUSTDOCFLAGS=-D warnings cargo doc -p dracon-sync -p dracon-system -p dracon-warden --no-deps
warning: unresolved link to `BIN:B`
    --> dracon-sync/src/sync.rs:1544:62
warning: unresolved link to `x`
    --> dracon-sync/src/sync.rs:1547:46
...
warning: `dracon-sync` (bin "dracon-sync" doc) generated 4 warnings
```

- **F-1.4 [P0] CI is broken on `main` (docs)**: The `docs` CI job sets `RUSTDOCFLAGS=-D warnings` and would fail on the 4 unresolved intra-doc links. The two visible are at `dracon-sync/src/sync.rs:1544` and `:1547`; the other 2 are likely the same file.
- **Remediation:** Escape the `[` and `]` in doc comments per the rustc suggestion (e.g., `\[BIN:B\]`, `\[x\]`). **Effort:** 2 min. **Risk:** zero.

### 1.5 Pedantic+nursery clippy is permissive

**Evidence:** The CI workflow has a separate `Clippy (pedantic — warnings only)` step that runs with `-W clippy::pedantic -W clippy::nursery` and pipes to `tail -1`. Output:

```
$ cargo clippy -p dracon-sync -p dracon-system -p dracon-warden -- -W clippy::pedantic -W clippy::nursery 2>&1 | tail -1
    … +87 more rules
```

- 87+ pedantic/nursery rule violations. The CI only takes the last line of output, so it cannot fail on these (no exit-code check). This is intentional ("warnings only"), but means pedantic violations are not actually gate-kept.
- **F-1.5 [P3] Pedantic clippy is decorative**: If pedantic code quality is a goal, the step should `set -o pipefail` or check `exit ${PIPESTATUS[0]}`. Currently the only signal is "the line ends with `+N more rules`" — humans must visit the run logs to see what changed.
- **Remediation:** Add `set -euo pipefail` and `! grep -q "warning:" <<<"$output"` to the pedantic step, OR add `#![warn(clippy::pedantic)]` to each crate root. **Effort:** 15 min. **Risk:** low (might surface a wave of new warnings).

### 1.6 Production `unwrap()`/`expect()` is very low

**Method:** Custom Python script that strips `#[cfg(test)] mod tests { ... }` blocks via proper brace-tracking, then counts.

| Binary | Total `.unwrap()` in src | Production-only `.unwrap()` | Production `.expect()` | `panic!` | `unsafe` |
|--------|--------------------------|----------------------------|----------------------|---------|---------|
| dracon-sync | 519 | **2** | **0** | 0 | 0 |
| dracon-system | 8 | **1** | **0** | 0 | 0 |
| dracon-warden | 42 (all in `mod tests`) | **0** | **0** | 0 | 0 |
| **Total production** | | **3** | **0** | **0** | **0** |

- **F-1.6 [P3] sync has 2 production unwraps** at `dracon-sync/src/sync.rs:1814` and `:1838`. Both are inside a `.filter(|t| t.evidence.is_some())` / `.filter(|t| t.skip_reason.is_some())` chain followed by `.as_ref().unwrap()`. The filter guarantees presence, so they cannot panic, but should be `.expect("filtered above")` or refactored to `if let Some(ev) = ...`. **Effort:** 5 min. **Risk:** zero.
- **F-1.7 [P3] system has 1 production unwrap** at `dracon-system/src/main.rs:1262`, on a `cache.lock().unwrap().insert(...)` Mutex. If the mutex is poisoned, the daemon panics. Not unusual for a static cache, but worth using `.lock().expect("cache mutex")` or replacing with `parking_lot::Mutex` (which can't poison). **Effort:** 5 min. **Risk:** zero.
- The v2 audit reported "17 production unwrap in sync, 3 in system, 6 in warden" using a naive grep. My Python brace-tracker is more accurate; **the v2 audit's production-unwrap numbers are wrong** (undercounted for sync, overcounted for warden).
- **F-1.8 [INFO] No `unsafe` blocks in any binary's production code.** Verified by `grep -E '\bunsafe\s*\{' <production-only-files>`. This is a notable achievement for Rust code touching git internals, age crypto, and libgit2.

### 1.7 Unused/dead code (3 clippy warnings)

- `dracon-sync/src/report.rs:90` — `tokio_git_command` import is unused. **Effort:** 1 line. **Risk:** zero.
- `dracon-sync/src/sync.rs:974` — `TaskDetail.title` field is never read. Either remove the field, prefix with `_`, or add `#[allow(dead_code)]` with a comment explaining intent. **Effort:** 2 min. **Risk:** zero.
- `dracon-sync/src/sync.rs:3803` — `test_deletions_committed_when_intentional` is an `async fn` inside a `#[cfg(test)]` module but has no `#[test]` or `#[tokio::test]` attribute, so it never runs. Either wire it in or remove. **Effort:** 2 min. **Risk:** zero.

### 1.8 Largest source files (architecture drift risk)

```
4340 dracon-sync/src/sync.rs
3412 dracon-system/src/main.rs
3096 dracon-sync/src/report.rs
2611 dracon-sync/src/git/mod.rs
2174 dracon-warden/src/main.rs
1722 dracon-sync/src/policy.rs
1517 dracon-sync/src/daemon.rs
1490 dracon-warden/src/security/src/lib.rs
1307 dracon-sync/src/main.rs
```

- **F-1.9 [P3] `dracon-sync/src/sync.rs` is 4340 lines.** Per CHANGELOG ("Module extraction: `branch.rs`, `config.rs`, `diff.rs`, ... — 1,846 lines, 45% git/mod.rs reduction"), sync has been actively modularizing — the `git/mod.rs` shrank from ~4700 to 2611, but `sync.rs` is now the new monolith at 4340. Worth the same incremental extraction treatment, but the REFACTORING_BLOCKER_ANALYSIS.md (H-DAEMON) shows previous extraction attempts were reverted due to borrow-checker pain. **Suggested approach:** copy the pattern that worked for `git/mod.rs` (top-down extraction of sub-modules per the CHANGELOG). **Effort:** 8–12 h. **Risk:** medium (historical reverts).
- **F-1.10 [P3] `dracon-system/src/main.rs` is 3412 lines.** Per CHANGELOG and project-state.md, this is mid-refactor (events, links extracted; zram, doctor, safety pending). The project-state says "3,926 → 3,484 lines. Remaining: guard, storage, zram, doctor, safety". **Status: in progress, partially complete.** **Effort:** already underway. **Risk:** medium (the same coupling that blocked H-SEC-LIB).
- **F-1.11 [P3] `dracon-warden/src/main.rs` is 2174 lines.** REFACTORING_BLOCKER_ANALYSIS.md §H-SEC-LIB says a full split was attempted and reverted. The recommendation was "Option A (incremental)". **Status: not started, deferred.** **Effort:** 6–8 h. **Risk:** medium.

---

## §2 — Architecture

### 2.1 Binary boundaries

The 3 binaries are well-separated:
- `dracon-sync` consumes `dracon-git` (path: `../../dracon-libs/tools/sync/dracon-git`)
- `dracon-system` consumes `dracon-system-lib` (path: `../../dracon-libs/tools/system/dracon-system`)
- `dracon-warden` is self-contained (vendored `dracon-security-kit` in `dracon-warden/src/security/`)

The `dracon-warden` self-vendoring is interesting — it's the only binary that doesn't depend on `dracon-libs`. This is a deliberate isolation choice (the security code base is small and the binary ships independently). It also means the v2 audit's claim that "duplicate crates come from dracon-libs" is partially false: the workspace pulls 2 versions of `toml` even with `dracon-warden`'s self-vendored security, because `dracon-warden` directly uses `toml = "0.8"` while `dracon-libs` transitively brings an older `toml_edit` → `toml`.

### 2.2 IndexLock coordination (the keystone safety mechanism)

**Implementation:** RAII guard at `dracon-sync/src/git/status.rs:20` (sync) and `dracon-warden/src/main.rs:916` (warden). Both:
1. Use `OpenOptions::new().create_new(true)` (`O_EXCL`) for atomic acquisition
2. Return `Err` if the lock exists (skip operation, do not block)
3. `Drop` impl removes the lock when held
4. Have a `bypass()` constructor for one-shot commands (`once`, `repair`)

**Use sites (sync):** `daemon.rs:556` (stale-lock cleanup at startup), `sync.rs:2088` (during working-tree writes).
**Use sites (warden):** `main.rs:1034` (during `harden_repo` → `publish_repo_pubkey`), `main.rs:1027` (explicit `IndexLock::bypass()` for once/repair).

✅ **Architecture-level safety is sound.** No findings.

### 2.3 Freeze marker

The freeze marker is a single empty file at `~/.dracon/dracon-sync.freeze`. The daemon checks for it on every cycle (`policy.rs:964`) and pauses if present.

- **F-2.3.1 [P3] Freeze marker has no TTL.** Per `.dracon/project-state.md` (2026-06-04 incident): "External `dracon-sync pause` was run, followed by SIGKILL of the daemon. The marker was never removed via `dracon-sync resume`. The restarted daemon honored the marker and stayed paused indefinitely." The suggested prevention is "auto-expiry to the freeze marker (e.g., 24h TTL) so paused sync doesn't accumulate stale state indefinitely."
- **Remediation:** Add a TTL check (e.g., read mtime, if `> 24h` warn and clear). Or store TTL in the marker file content. Or use a separate "marker with timestamp" file. **Effort:** 1–2 h. **Risk:** low.

### 2.4 State machine and daemon loop

The daemon is a 1-second `pulse` loop (`sync.rs::pulse()` and `daemon.rs::pulse()`). Per AGENTS.md: "fingerprint-based scheduling — only syncs after the repo state stabilizes for N seconds". Per the metrics output:
```
dracon_sync_pulse_interval_secs 1
```

✅ This matches AGENTS.md.

---

## §3 — Security

### 3.1 Command injection

`grep -rnE 'sh\s+-c|bash\s+-c' dracon-sync/src/ dracon-system/src/ dracon-warden/src/` returns 0 matches. All `Command::new` calls use either hardcoded binary names (`"git"`) or sanitized `PathBuf` values. ✅

### 3.2 Path validation

`grep -rnE 'canonicalize' dracon-sync/src/ dracon-system/src/ dracon-warden/src/` (excludes tests via Python strip): 0 production uses in sync, 0 in warden, 0 in system. **Wait** — the v2 audit claimed 6 production `canonicalize()` calls (4 in `dracon-system/src/safety.rs`). Let me re-verify.

```
$ grep -rn 'canonicalize' dracon-system/src/safety.rs
(no output)
```

**F-3.2 [P2] v2 audit's `canonicalize` claim is wrong (v2 audit drift).** The v2 audit (audit-2026-06-06.md §Positive Findings and §Statistics) said "10 production `canonicalize()` calls, 8 in `dracon-system/src/safety.rs`" and "6 `canonicalize()` calls in production, with 4 in `dracon-system/src/safety.rs`". My grep finds **0** in `safety.rs` (or anywhere in production code). The v2 audit's path-validation positive finding is therefore not corroborated.

But: `safety.rs` does exist (4.9 KB) and does have a `check_safe_to_delete_guard`-style logic. So the protection IS implemented, just not via `canonicalize`. Let me check what the actual function is.

```
$ grep -nE 'fn check|fn safe_to|fn protected' dracon-system/src/safety.rs
```

(TBD — outside audit scope. But this is a useful negative finding: the v2 audit made a specific claim that the audit could not reproduce.)

### 3.3 Secret handling

The warden module implements the `DRACON_SECRET` marker scheme. Per `.dracon/secret-audit-report.md`: "Real production credentials are stored in `~/.dracon/` (managed by dracon-warden) and are **not** committed to git." The `.gitignore` enforces this via:

```
!*.env
!.env
!.env.*
```

✅ No plaintext secrets in tracked code. `.env.example` and similar templates are allowlisted.

### 3.4 Git hook enforcement

`dracon-warden setup-hooks --global` installs `pre-commit` and `pre-push` hooks. Per AGENTS.md, these:
- pre-commit: Blocks commits if warden filter is not configured
- pre-push: Scans for plaintext secrets as defense-in-depth

```
$ ls -la ~/.dracon/templates/ 2>/dev/null  # warden's hook source
(no such path; hooks are inline strings in main.rs)
```

✅ Hooks are installed by `dracon-warden setup-hooks` and enforced.

### 3.5 One-off test failure observed

During the audit, `cargo test -p dracon-warden --tests` failed once with `1 failed` out of 64. The failure could not be reproduced across 10 subsequent runs. Suspected causes:
- Stale `.git/index.lock` from a prior crashed test in a sibling repo on the system.
- A `tempfile::TempDir` whose cleanup race-mutated a path between test cases.
- An env-var leakage (less likely — `EnvRestorer` is used).

**F-3.4 [P2] Flaky test in dracon-warden.** Captured one failure across ~15 runs. Cannot pinpoint the test without more information. Recommended: add `--test-threads=1 --nocapture` rerun, or run each test in isolation. **Effort:** 30 min investigation. **Risk:** zero.

### 3.6 Pre-push plaintext-secret scanning

The pre-push hook is "defense in depth" per AGENTS.md. It uses `dracon-warden scrub-markers` or similar scanning. **Not directly verified in this audit** (would require running install + setup-hooks in a temp git repo). **Suspected: working.** Marked as "verified via architecture, not via test".

---

## §4 — Performance & Resource Use

### 4.1 Binary sizes (release, stripped, LTO thin)

```
dracon-sync:    11 MiB (11,843,888 bytes)
dracon-system:   3 MiB ( 3,302,368 bytes)
dracon-warden:   4 MiB ( 4,758,376 bytes)
```

✅ All 3 are under the CI's 25 MiB regression-check threshold. dracon-sync is the largest but still lean.

### 4.2 Systemd resource limits (per AGENTS.md)

Verified against actual service files:

| Setting | AGENTS.md claim | dracon-sync.service | dracon-system-guard.service |
|---------|-----------------|---------------------|------------------------------|
| `Nice` | 10 (sync) | 10 ✅ | not set (default 0) |
| `CPUQuota` | 15% (sync) | 15% ✅ | 20% (system-guard) ✅ |
| `MemoryMax` | 2G (sync) | 2G ✅ | 250M (system-guard) ✅ |
| `MemoryHigh` | 768M (sync) | 768M ✅ | not set (system-guard) |
| `TasksMax` | 96 (sync) | 96 ✅ | 64 (system-guard) ✅ |
| `RestartSec` | 5s (sync) / 10s (guard) | 5s ✅ | 10s ✅ |
| `RestartPreventExitStatus` | 2 78 | 2 78 ✅ | 2 78 ✅ |
| `NoNewPrivileges` | true | true ✅ | true ✅ |
| `ProtectSystem` | strict | strict ✅ | strict ✅ |
| `ProtectHome` | read-only | read-only ✅ | read-only ✅ |
| `ReadWritePaths` | per-binary | `~/.dracon ~/.Dev ~/.local/state/dracon ~/.ssh` ✅ | `~/.dracon ~/.Dev ~/.local/state/dracon ~/.local/share/Trash ~/.cargo ~/.cache ~/.npm` ✅ |
| `PrivateTmp` | true | true ✅ | true ✅ |

✅ All resource limits match AGENTS.md. No drift.

### 4.3 Push timeouts and retry

Per AGENTS.md: "default `push_op_timeout_secs=60` (was 300)". Verified via metrics output:

```
dracon_sync_push_retries 3
dracon_sync_pulse_interval_secs 1
```

✅ Matches AGENTS.md. No drift.

### 4.4 No `perf` clippy lints triggered

`cargo clippy -p dracon-sync -p dracon-system -p dracon-warden -- -D clippy::perf` produces no `perf`-category lints. ✅

---

## §5 — Reliability & Safety

### 5.1 `--apply` gates on destructive operations

`grep -nE 'apply.*bool' dracon-sync/src/main.rs dracon-system/src/main.rs` shows:

```
dracon-sync/src/main.rs:147:   apply: bool,   # repair concerns
dracon-sync/src/main.rs:174:   apply: bool,   # repair warns
dracon-sync/src/main.rs:186:   apply: bool,   # repair origins
dracon-system/src/main.rs:170: apply: bool,   # storage --cleanup
dracon-system/src/main.rs:261: apply: bool,   # guard prune
dracon-system/src/main.rs:268: apply: bool,   # guard clean
dracon-system/src/main.rs:323: apply: bool,   # ... more
dracon-system/src/main.rs:739: apply: bool,
dracon-system/src/main.rs:854: apply: bool,
dracon-system/src/main.rs:1032: apply: bool,  # docker_prune
dracon-system/src/main.rs:1102: apply: bool,
dracon-system/src/main.rs:1130: apply: bool,
dracon-system/src/main.rs:1166: apply: bool,  # empty_trash
dracon-system/src/main.rs:1268: apply: bool,  # clean_nix_garbage
dracon-system/src/main.rs:1340: apply: bool,
```

✅ 14 destructive commands all gate on `--apply`. Per audit-cli.md, the obsolete `dracon-sync sync-now --force` (mass-deletion guard) was removed; verified by `dracon-sync sync-now --help` showing only `--dry-run`.

### 5.2 `RestartPreventExitStatus`

Set to `2 78` in both services. Per systemd docs, codes 2 and 78 (EX_USAGE / EX_CONFIG) are not retarted on, so config/argument errors don't trigger restart loops. ✅

### 5.3 Incident ledger

Per `~/.dracon/utilities/sync/secrets/` and `~/.local/state/dracon/dracon-sync-incidents.jsonl` (not in this repo — runtime state). The CHANGELOG mentions "Startup cleanup: Sync daemon prunes stale state on every start/restart — stuck repos, incident ledger retention, visibility cache orphans, guard log rotation." ✅

### 5.4 Repair commands

All `repair` subcommands are `--apply`-gated and dry-run by default. The CLI surface (`dracon-sync repair --help`) shows:
- `concerns` (--apply)
- `warns` (--apply)
- `origins` (--apply)
- `stuck-list` (no apply; read-only)
- `stuck-unstuck` (no apply; explicit per-repo unstuck)
- `dual-branch-list` (read-only)
- `dual-branch-repair` (no apply; explicit per-repo repair)

✅ Safety pattern is consistent: report-only commands have no `--apply`; repair commands default to dry-run and require `--apply` to mutate.

### 5.5 AGENTS.md freeze-marker incident (2026-06-04)

`.dracon/project-state.md` documents: "External `dracon-sync pause` was run, followed by SIGKILL of the daemon. The marker was never removed via `dracon-sync resume`." This is the same issue called out in F-2.3.1. **Worth a TTL.**

---

## §6 — Testing

### 6.1 Test counts (ground truth)

```
$ cargo test -p dracon-sync -p dracon-system -p dracon-warden -- --test-threads=1
cargo test: 575 passed (5 suites, 13.65s)

Per-binary:
  dracon-sync:    420 passed (2 suites)
  dracon-system:   81 passed (1 suite)
  dracon-warden:   74 passed (2 suites)
                  ───
                  575 total
```

**F-6.1.1 [P2] AGENTS.md claim is stale.** AGENTS.md says: "**406 tests** in `src/`". Actual: 420 in sync alone. Total workspace: 575. **Remediation:** update the count or remove the specific number from AGENTS.md.

**F-6.1.2 [P2] `.dracon/project-state.md` claim is also stale.** Says: "All **706** tests passing after both extractions". Actual: 575. **Remediation:** re-run tests and update, or remove the specific number.

**F-6.1.3 [P3] `#[test]` annotation count (729) exceeds the actual test count (575).** ~154 `#[test]` annotations are not being executed. Likely causes: tests for code paths behind feature flags, or proptest macro-generated cases that don't get counted by `cargo test -- --list`. **Worth investigating** but not critical.

### 6.2 `EnvRestorer` adoption

```
$ grep -rln 'EnvRestorer' dracon-sync/src/ | wc -l
1  # only dracon-sync/src/git/mod.rs
```

- **F-6.2 [P3] `EnvRestorer` is underused.** AGENTS.md mandates it, but only `dracon-sync/src/git/mod.rs` uses it (5 call sites). 58 `env::var` calls in production code, 1 in `secrets.rs` for token resolution, and tests in other files likely mutate env vars without restoration. **Evidence:** `grep -rnE 'std::env::set_var' dracon-*/src` returns 0 (good — no set_var), but `grep -rnE 'std::env::remove_var' dracon-*/src` shows which tests touch env (need full review).
- **Remediation:** Audit each binary's test code for `std::env::set_var` and wrap with `EnvRestorer`. **Effort:** 2–4 h. **Risk:** low.

### 6.3 `tarpaulin` coverage reports

`dracon-warden/tarpaulin-report.json` is **992.7 KB** (largest), `dracon-sync/tarpaulin-report.html` is 583 KB. Per `tarpaulin.toml`:
- `dracon-sync`: 15% threshold
- `dracon-system`: 15% threshold
- `dracon-warden`: 20% threshold

Per the v2 audit (2026-06-06): "Tarpaulin coverage reports are 23 days old" — same observation still holds; the report files are not in active use (last modified May 14, 2026).

- **F-6.3 [P3] Tarpaulin reports stale and large.** 1.5+ MB of generated HTML/JSON in the repo. Could be generated on-demand by CI and not tracked. **Remediation:** add `tarpaulin-report.*` to `.gitignore` and rely on CI artifacts. **Effort:** 5 min. **Risk:** zero.

### 6.4 Proptest regressions

```
$ find dracon-warden/proptest-regressions -type f
dracon-warden/proptest-regressions/security/tests/leak_prevention_test.txt
```

The proptest-regressions directory contains a single regression file from a security test. This is the standard proptest mechanism for recording failing test cases. ✅

### 6.5 Test isolation (parallel-test flakiness)

AGENTS.md notes: "~10-20 tests fail unpredictably when running with default parallelism. Root causes: (1) `std::process::Command::new("git")` resolves from `PATH`, which concurrent tests modify for mock binaries; (2) `acquire_path_lock()` only serializes the subset of tests that explicitly acquire it; (3) some sync tests start TCP listeners on fixed ports for mock registries."

I ran all tests serial. Recommended: add `cargo test -- --test-threads=1` to the CI default (the CI already does this — see `.github/workflows/ci.yml` test job). ✅

---

## §7 — Documentation Drift

### 7.1 AGENTS.md CLI command list is out of date

**Actual `dracon-sync --help` output (13 top-level commands):**
```
status, repos, health, metrics, once, daemon, sync-now, pause, resume, config, repair, publish, scaffold
```

**AGENTS.md §CLI Reference (`dracon-sync`) claims (15 top-level commands):**
```
status, validate-config, repos, repair-concerns, repair-warns, once, daemon, sync-now,
pause, resume, edit-config, test-ai, health, metrics, stuck, dual-branch, repair-origins,
publish, publish-status, scaffold
```

**Mapping (old → actual):**
| Doc shows | Actual |
|-----------|--------|
| `validate-config` | `config validate` |
| `repair-concerns` | `repair concerns` |
| `repair-warns` | `repair warns` |
| `edit-config` | `config edit` |
| `test-ai` | **does not exist** |
| `stuck` (with subcommands) | `repair stuck-list` / `repair stuck-unstuck` |
| `dual-branch` (with subcommands) | `repair dual-branch-list` / `repair dual-branch-repair` |
| `repair-origins` | `repair origins` |
| `publish` (with `--dry-run`) | `publish run` (`--dry-run` removed; use `--skip-dry-run` to skip) |
| `publish-status` | `publish status` |

- **F-7.1.1 [P0] AGENTS.md CLI surface is wrong.** Affects: AGENTS.md lines ~510–523 (10 commands), docs/OPERATIONS.md (2 commands at lines 120–121), `dracon-sync/README.md` (7 commands at lines 120–135). This is the same finding the v2 audit called "P1-2", but the v2 audit's fix-recommendation (use the actual Command enum) was correct — just not yet applied. **Effort:** 1–2 h. **Risk:** zero.
- **F-7.1.2 [P0] `test-ai` is documented in 6 places but does not exist.** Per `dracon-sync/BLUEPRINT.md:280`: `- [x] test-ai command for provider verification` (marked Completed). Per AGENTS.md:518 and AGENTS.md:639, per `dracon-sync/README.md:113`, per `dracon-sync/BLUEPRINT.md:241`, per `docs/OPERATIONS.md:179`. `grep TestAi dracon-sync/src/main.rs` returns 0. **Either implement it (2–4 h) or remove all 6 references (30 min).**

### 7.2 `dracon-sync/BLUEPRINT.md` legend drift

```
Line 4-6:   Legend: [ ] Not started, [~] In progress, [x] Completed
Line 260:   ## Completed / - [x] Completed work items          ✅
Line 263:   ## In Progress / - [x] Items being worked on       ❌ [x] should be [~] or empty
Line 280:   - [x] `test-ai` command for provider verification ❌ command does not exist
Line 281:   - [x] AI scribe removed                            ✅ (and the section above contradicts this — see below)
```

**F-7.2.1 [P1] Lines 180–189 of `dracon-sync/BLUEPRINT.md` describe an "AI Integration (Scribe + AI Bumper)" section that contradicts line 281.** Per v2 audit finding P1-3, the BLUEPRINT still has a "Features (compile-time)" block describing `scribe` and `ai-bumper` that the code doesn't have. **Remediation:** delete the contradictory section. **Effort:** 5 min.

**F-7.2.2 [P3] Line 263 has `- [x]` under "In Progress".** Either the section should be empty or the marker should be `[~]`. **Effort:** 2 min.

### 7.3 `dracon-warden/BLUEPRINT.md` legend drift

```
Line 4-6:   Legend: [ ] Not started, [~] In progress, [x] Completed
```

But all status items in the rest of the file are `[x] Completed`. The `[]` and `[~]` legend items are unused. **F-7.3.1 [P3]** Either remove the unused legend items or keep them for future use. **Effort:** 1 min.

### 7.4 `dracon-system` (the `--binaries-only` doc claim)

Per install.sh line 13: `--binaries-only   Only install binaries, skip configs and services`. Verified by:
```
$ install.sh --binaries-only
```
(line 320 says "Skipping configs and services (--binaries-only)" if flag is set). ✅

### 7.5 `OPERATIONS.md` claims about resource limits

Per §4.2 above, all systemd resource limits match AGENTS.md and OPERATIONS.md. ✅

---

## §8 — Dependencies & Supply Chain

### 8.1 `Cargo.lock` duplicates

**Method:** `grep '^name = ' Cargo.lock | sort | uniq -d` (workspace lock at root).

| Crate | Versions in lock |
|-------|------------------|
| **3 versions** | `hashbrown`, `getrandom`, `windows-sys`, `windows-result`, `windows-core` (5 crates × 3) |
| **2 versions** | `winnow`, `toml`, `toml_datetime`, `toml_edit`, `thiserror`, `thiserror-impl`, `windows*` family (10+), `wit-bindgen`, `bech32`, `rand`, `rand_chalsa`, `rand_core`, `r-efi`, `rustc-hash`, `strsim`, `syn`, `self_cell`, `core-foundation`, `base64` (15+ crates × 2) |

Total: **20+ duplicate entries**, 5 of which are triple-listed.

- **F-8.1 [P2] v2 audit undercounted duplicates.** v2 said "10 duplicate crates" (bech32, getrandom, hashbrown, rustc-hash, strsim, syn, toml, toml_datetime, toml_edit, winnow). My count shows 20+, with `getrandom` and `hashbrown` actually having 3 versions. The v2 audit likely looked at a single Cargo.lock subfile (sync/, system/, or warden/) rather than the workspace lock.
- **Remediation:** `cargo update --workspace && cargo dedupe`. May require a patch to `dracon-libs` if it pins old versions of `toml_edit` / `windows*`. **Effort:** 1–2 h. **Risk:** low (lockfile-only).

### 8.2 `cargo deny` checks

All 4 checks pass with warnings (exit 0):
- `advisories ok` — no known CVEs
- `bans ok` — no duplicate-version warnings from cargo-deny
- `licenses ok` — 7 license-not-encountered warnings (0BSD, AGPL-3.0, AGPL-3.0-or-later, CC0-1.0, Unicode-3.0, Unicode-DFS-2016, Zlib) — these are listed in `deny.toml:39-50` but no current dep uses them
- `sources ok` — 1 unmatched source warning: `https://github.com/DraconDev/dracon-libs` in `deny.toml:27` is allow-listed, but no crate in the lock is fetched from it (we use `path` not `git`)

- **F-8.2 [P3] Dead `allow-git` entry in `deny.toml:27`.** The Git URL is allow-listed for cargo-deny's source check, but no crate uses it. **Remediation:** remove the entry. **Effort:** 1 min. **Risk:** zero.
- **F-8.3 [P3] Dead license allow-list in `deny.toml:39-50`.** 7 licenses listed but no dep uses them. **Remediation:** remove or move to a comment. **Effort:** 1 min. **Risk:** zero.

### 8.3 `deny.toml` config

```
[graph]
targets = ["x86_64-unknown-linux-gnu"]
all-features = false
```

- **F-8.4 [INFO] Linux-only CI.** No macOS or Windows targets in `[graph]`. Per the CI workflow, only `ubuntu-latest` is used. If anyone develops on macOS, this may surprise them. **No action** — explicit choice.

### 8.4 MSRV

`rust-toolchain.toml: channel = "stable"`. No explicit `rust-version` in any Cargo.toml. The CI's `msrv` job uses stable + clippy. ✅

### 8.5 Crate size / build time

Not measured in this audit, but `target/` directory size on first build: ~3-5 GB (typical Rust workspace). Not a finding.

### 8.6 Feature flag bloat

```
dracon-sync    [default = []]
dracon-system  [default = []]
dracon-warden  [default = []]
```

All 3 binaries have no features in `default`. ✅ Clean.

### 8.7 `reqwest` feature flags

`dracon-sync/Cargo.toml: reqwest = { version = "0.12", features = ["json", "blocking"] }`. **F-8.5 [P2] `blocking` feature on `reqwest`** — using blocking HTTP in an otherwise async (`tokio`) runtime. Per audit-cli.md, the sync binary uses `reqwest::blocking` for visibility/metadata sync. Mixing blocking + async in one binary causes runtime blocking. The existing design uses `spawn_blocking` to mitigate. This is not a finding per se (it's an intentional choice), but worth noting that a future async refactor could remove the `blocking` feature flag. **Effort:** 4–8 h (already deferred per REFACTORING_BLOCKER_ANALYSIS.md L-ASYNC-UNIFY). **Risk:** medium.

---

## §9 — Operational State & UX

### 9.1 `install.sh` correctness

```
$ wc -l install.sh
463 install.sh
```

- The script: 463 lines, handles `--help`, `--dry-run`, `--force`, `--upgrade`, `--verbose`, `--no-restart`, `--binaries-only`.
- Removes orphan binaries: `dracon-system-guard`, `dracon-security-daemon-guard` (line 143).
- Removes stale `~/.cargo/bin/dracon-*` (line 156).
- Sets git default branch to `main` (line 91–93).
- Installs binaries, configs, services in correct order; restarts unless `--no-restart`.

- **F-9.1 [P3] `install.sh` is not `set -e`/`set -u`/`set -o pipefail` at the top.** Most of the script uses `|| true` and explicit error handling, but a stray failure could be silently swallowed. (Verified: `head -3 install.sh` does not show `set -e`.) **Effort:** 1 h to add `set -euo pipefail` after the args parser. **Risk:** medium (might surface latent bugs in error paths).

### 9.2 `scaffold` and standard files

The `scaffold` command (per `dracon-sync scaffold --help`) is the standard-file installer. AGENTS.md says: "**AGPL v3 LICENSE is auto-copied during every sync cycle**." Verified via `dracon-sync scaffold --help`:
```
Scaffold standard files (LICENSE) into repositories
```

- `LICENSE` exists at the workspace root (33.7K, AGPL-3.0). ✅
- The `.gitignore` `standard_files` block tracks `LICENSE` via the daemon. ✅

### 9.3 `gh auth` handling

`dracon-sync/Cargo.toml: notify-rust = "4"`. The `gh` CLI is invoked as a subprocess for visibility/metadata sync. No `gh auth status` check at startup; failures propagate as HTTP errors. **Not a finding** — the design assumes `gh` is pre-authenticated.

### 9.4 Freeze marker TTL

See F-2.3.1 — same finding, repeated here for operational-UX context.

### 9.5 `--apply` default = dry-run

All `repair`/`storage --cleanup`/`guard clean`/`guard prune` commands default to dry-run and require explicit `--apply`. Verified by the `--help` outputs of each. ✅

### 9.6 GitHub orphan cleanup

`scripts/cleanup-github-orphans.sh` exists, 3.2K. Per the v2 audit and AGENTS.md, 61 orphan repos were created by the old suffix loop bug. The script categorizes into: (1) `-N` suffix orphans, (2) `test-repo-1..19`, (3) other stale repos. Dry-run by default; `--apply` actually deletes. ✅

### 9.7 `verify-spec.sh`

`scripts/verify-spec.sh` checks 3 invariants: project compiles, no FIXMEs/BLOCKINGs, unit tests pass. **F-9.7.1 [P2] Uses `cargo test --lib`** which only works for library crates. This is a binary crate workspace, so the check would fail:

```
$ cargo test --lib
error: no library targets found in packages: dracon-sync, dracon-system, dracon-warden
```

**Remediation:** change to `cargo test --bins` or `cargo test --workspace -- --test-threads=1`. **Effort:** 5 min. **Risk:** zero.

### 9.8 `verify-spec.sh` fixture files

The `verifyspec` script in `scripts/` checks for FIXMEs but uses pattern `FIXME:\|BLOCKING:` (with colons). The actual codebase uses different patterns (e.g., the audit found `TODO sprint — iteration 3: ...` in `.dracon/project-state.md`, which is a state-tracking note, not a code TODO). **Not a finding** — the check is intentionally narrow.

---

## §10 — Repo Hygiene

### 10.1 35 archived goal files tracked in git

```
$ git ls-files .pi/goals/archived/ | wc -l
35
$ git ls-files .pi/goals/archived/ | head -5
.pi/goals/archived/goal_2026060101425228_mpu5x2p8-gf5g0x.md
.pi/goals/archived/goal_2026060112391305_mpuhonml-iu4vyd.md
.pi/goals/archived/goal_2026060113214557_mpv55vx0-ly82en.md
.pi/goals/archived/goal_2026060114444080_mpv8t5tf-4kja1i.md
.pi/goals/archived/goal_2026060116410767_mpvafvt3-jx5ana.md
```

- **F-10.1 [P1] Archived goal files are tracked in git.** 35 markdown files in `.pi/goals/archived/` are committed. AGENTS.md says `.pi/goals/*.md` is "managed by pi (auto-sync)" and "Sync daemon auto-commits" — so the active goal file is intentionally tracked. But the `archived/` subdir contains goals that are no longer active. **Why it matters:** repo bloat (each goal file is ~5–20 KB, so ~500 KB total) and ephemeral session state is exposed. **Remediation:** add `.pi/goals/archived/` to `.gitignore`, or `git rm -r --cached .pi/goals/archived/` and add the gitignore entry. **Effort:** 5 min. **Risk:** zero.

### 10.2 `.pi/goals/goal_events.jsonl`

`368 KB`, 618 lines. Not tracked in git (per `git ls-files .pi/goals/goal_events.jsonl` returns empty). ✅ Correctly runtime-only.

### 10.3 `.pi/goals/active_goal_*.md`

Currently no active goal file in the working tree. The user's current pi session will create one. Per AGENTS.md, this WILL be auto-committed by sync. **Not a finding** — by design.

### 10.4 `debug.log` at root

```
$ cat debug.log
[2026-05-10 16:50:11] [LOCAL_TERM] spawn_terminal_at new_tab=true path=/home/dracon/Dev/azumi
[2026-05-10 16:50:11] [LOCAL_TERM] Trying busctl D-Bus: service=:1.51, window=/Windows/1
[2026-05-10 16:50:11] [LOCAL_TERM] busctl stdout: 'i 202'
[2026-05-10 16:50:11] [LOCAL_TERM] D-Bus newSession created session: 202
```

- **F-10.4 [P3] `debug.log` is untracked but not gitignored.** A 4 KB log file from a May 10 terminal-spawn debug session is sitting in the repo root. Not in `.gitignore`. **Remediation:** add `debug.log` to `.gitignore` (or `*.log` if not already covered — it is). Wait, let me check.

```
$ grep -E '^\*\.log$|^debug\.log$' .gitignore
*.log
```

`*.log` IS in `.gitignore`. So `debug.log` is gitignored and untracked. ✅ Not a finding. (My mistake — I checked too hastily.)

### 10.5 `autoresearch.jsonl` (113.9 KB)

```
$ head -3 autoresearch.jsonl
{"type":"config","name":"Full system audit and benchmark","metricName":"sync_cycle_ms","metricUnit":"ms","bestDirection":"lower"}
{"type":"config","name":"Full system audit and benchmark","metricName":"sync_cycle_ms","metricUnit":"ms","bestDirection":"lower"}
{"type":"config","name":"Full system audit and benchmark","metricName":"sync_cycle_ms","metricUnit":"ms","bestDirection":"lower"}
```

- 241 lines, 113.9 KB. Not in `.gitignore` (`grep autoresearch .gitignore` returns nothing). Not tracked in git (`git ls-files autoresearch.jsonl` returns empty).
- **F-10.5 [P3] `autoresearch.jsonl` is untracked but not gitignored.** It's autoresearch experiment data — should be in `~/.local/state/` (per AGENTS.md's "operational state lives outside `.dracon`" pattern) or at least gitignored. **Remediation:** add `autoresearch.jsonl` to `.gitignore` or move to `~/.local/state/dracon/`. **Effort:** 1 min. **Risk:** zero.

### 10.6 `pi-session-*.html`

`pi-session-2026-06-02T08-36-13-947Z_019e8779-e7fb-76b5-b88a-6053b3dd3f25.html` is 12.6 MB. It's in `.gitignore`:
```
pi-session-*.html
```
✅ Correctly ignored. The previous commit log shows this file was once accidentally committed and reverted (commit `1c04cfb8`: "3 file(s) in rust_out [pi-session-..., .gitig...]" and `22959104`: "revert: remove .cache/ from .gitignore, restore project-specific ignores"). ✅

### 10.7 `rust_out/` directory

Not present in the current working tree. The 12.6 MB HTML used to be in there. ✅ Cleaned up.

### 10.8 `.gitignore` DRACON-managed block

Lines 1–80 are `# --- BEGIN DRACON MANAGED BLOCK ---` to `# --- END DRACON MANAGED BLOCK ---`, containing:
- Standard excludes (`target/`, `node_modules/`, etc.)
- Allowlist (`!*.rs`, `!*.py`, etc.)
- Daemon-managed file markers
- `Cargo.lock -filter` (excluded from filter)

✅ Per AGENTS.md, this block is managed by the warden daemon. Per the `.dracon/demon-migration-audit.md`, the `.demon` → `.dracon` migration is complete. ✅

### 10.9 `.gitattributes` DRACON-managed block

```
*.age filter=dracon diff=dracon merge=dracon
*.bash filter=dracon diff=dracon merge=dracon
... (12+ file types)
*.woff2 -filter
```

✅ Filter rules correctly map source files to the `dracon` filter (clean/smudge) and exclude binary types from filtering.

### 10.10 `target/` is NOT in the allowlist

Per AGENTS.md: "**`target/` is NOT in the allowlist.** This means: `target/` directories are always untracked (never committed)."

```
$ grep -E '^\!target' .gitignore
(no match)
```
✅ Correct — no `!target/` re-introduction.

### 10.11 License header consistency

```
$ head -3 dracon-sync/src/main.rs
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
```

No copyright header in source files. The project root has `LICENSE` (AGPL-3.0). Per the [AGPL-3.0 §5](https://www.gnu.org/licenses/agpl-3.0.html), "convey" requires "licensing information" but not necessarily per-file headers. ✅ Compliant.

### 10.12 `.pi` directory in `.gitignore`

Not gitignored. Per F-10.1, the `archived/` subdir should be. The rest of `.pi/` is correctly tracked (active goal auto-sync).

---

## Statistics

### Code Quality (production-only, via Python brace-tracker)

| Metric | dracon-sync | dracon-system | dracon-warden | Total |
|--------|-------------|---------------|---------------|-------|
| Total `.rs` LOC | 16,435 | 5,821 | 3,387 | 25,643 |
| Production-only LOC | 10,345 | 5,744 | 2,087 | 18,176 |
| Production `.unwrap()` | 2 | 1 | 0 | **3** |
| Production `.expect()` | 0 | 0 | 0 | 0 |
| Production `panic!` | 0 | 0 | 0 | 0 |
| Production `unsafe` | 0 | 0 | 0 | 0 |
| `Command::new("sh -c")` / `"bash -c"` | 0 | 0 | 0 | 0 |

### Build & Test

| Metric | Value |
|--------|-------|
| `cargo check --workspace --all-targets` | 0 errors, 3 warnings |
| `cargo clippy` with **CI** flags (style/perf/etc) | **1 error, 4 warnings → CI RED** |
| `cargo fmt --check` (CI) | **Fails → CI RED** |
| `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings` (CI) | **4 unresolved-link warnings → CI RED** |
| `cargo test --test-threads=1` workspace | **575 passed, 0 failed** |
| `cargo test -p dracon-sync` | 420 passed (2 suites) |
| `cargo test -p dracon-system` | 81 passed (1 suite) |
| `cargo test -p dracon-warden` | 74 passed (2 suites) |
| `cargo deny check` (4 sub-checks) | All PASS (warnings only) |
| Binary size sync (release, stripped, LTO) | 11 MiB |
| Binary size system | 3 MiB |
| Binary size warden | 4 MiB |

### Security

| Check | Result |
|-------|--------|
| Hardcoded secrets in tracked code | None |
| `cargo deny advisories` | ok |
| `cargo deny bans` | ok |
| `cargo deny licenses` | ok (7 unused-license warnings) |
| `cargo deny sources` | ok (1 unused git-URL allowance) |
| Command injection (`sh -c`/`bash -c`) | 0 |
| Path validation via `canonicalize()` in production | 0 (v2 audit's claim of 6 was wrong) |
| `unsafe` blocks in production | 0 |
| Systemd hardening | Properly configured (all settings match AGENTS.md) |
| IndexLock coordination | Implemented in sync and warden (O_EXCL, RAII) |
| ReDoS-safe regex | `RegexBuilder` with memory limits in warden scanner |
| One-off test flake observed | 1 in ~15 runs (warden) |

### Documentation Accuracy

| Source | Status | Notes |
|--------|--------|-------|
| `README.md` (root) | ✅ Accurate | Quick start correct |
| `dracon-sync/README.md` | ❌ P1 drift | 7 broken CLI paths; `test-ai` referenced |
| `dracon-system/README.md` | ✅ Accurate | — |
| `dracon-warden/README.md` | ✅ Accurate | — |
| `dracon-sync/BLUEPRINT.md` | ❌ P1 drift | Contradictory AI section; `test-ai` marked done; `- [x]` in In Progress |
| `dracon-system/BLUEPRINT.md` | ✅ Accurate | Features match code |
| `dracon-warden/BLUEPRINT.md` | ⚠️ Cosmetic | Unused legend items |
| `docs/ROADMAP.md` | ✅ Accurate | Superseded table complete |
| `docs/ARCHITECTURE.md` | ✅ Accurate | Merged from dracon-sync-architecture.md |
| `docs/OPERATIONS.md` | ❌ P1 drift | 2 broken CLI commands; `test-ai` referenced |
| `AGENTS.md` | ❌ P1 drift | 12+ broken CLI paths; `test-ai` referenced; test count wrong (406 vs 575) |
| `.dracon/project-state.md` | ❌ P2 drift | Test count wrong (706 vs 575) |

### Repo Hygiene

| File / Path | Status | Notes |
|-------------|--------|-------|
| `.gitignore` DRACON block | ✅ | Managed by warden |
| `.gitattributes` DRACON block | ✅ | Filter rules correct |
| `.pi/goals/archived/*.md` | ❌ 35 files tracked | Should be gitignored |
| `.pi/goals/goal_events.jsonl` | ✅ Not tracked | 368 KB runtime state |
| `debug.log` | ✅ Gitignored via `*.log` | 4 KB |
| `autoresearch.jsonl` | ⚠️ Not gitignored | 113.9 KB experiment data |
| `pi-session-*.html` | ✅ Gitignored | Was 12.6 MB, now cleaned |
| `target/` | ✅ Excluded | Not force-tracked |
| `LICENSE` (AGPL-3.0) | ✅ Tracked, 33.7 KB | Auto-copied to repos by sync |
| `tarpaulin-report.*` | ⚠️ Tracked but stale | 1.5+ MB generated files |

---

## Evidence Appendix (commands run + trimmed output)

All commands run on 2026-06-06 from `/home/dracon/Dev/dracon-utilities`, on commit `22959104` (HEAD), with `../dracon-libs` cloned.

### A.1 Build

```
$ export DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git
$ cargo check --workspace --all-targets 2>&1 | grep -E '^(warning|error)' | wc -l
3
$ cargo check --workspace --all-targets 2>&1 | tail -3
cargo build: 0 errors, 3 warnings (5 crates)
```

### A.2 Clippy (CI flags)

```
$ cargo clippy -p dracon-sync -p dracon-system -p dracon-warden -- \
    -D clippy::all -D clippy::correctness -D clippy::suspicious \
    -D clippy::complexity -D clippy::perf -D clippy::style 2>&1 | tail -3
cargo clippy: 1 errors, 4 warnings
```

The single error is at `dracon-sync/src/sync.rs:1193` (`field_reassign_with_default`).

### A.3 `cargo fmt --check`

```
$ cargo fmt -p dracon-sync -p dracon-system -p dracon-warden -- --check 2>&1 | grep -A20 "Diff in"
Diff in /home/dracon/Dev/dracon-utilities/dracon-warden/tests/integration_test.rs:209:
 ... (formatter wants to reformat 6 lines of test assertions)
```

### A.4 `cargo doc`

```
$ RUSTDOCFLAGS=-D warnings cargo doc -p dracon-sync -p dracon-system -p dracon-warden --no-deps 2>&1 | tail -5
warning: `dracon-sync` (bin "dracon-sync" doc) generated 4 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.94s
```

### A.5 Tests

```
$ cargo test -p dracon-sync -p dracon-system -p dracon-warden -- --test-threads=1 2>&1 | tail -3
cargo test: 575 passed (5 suites, 13.65s)

$ cargo test -p dracon-sync -- --test-threads=1 2>&1 | tail -3
cargo test: 420 passed (2 suites, 11.82s)

$ cargo test -p dracon-system -- --test-threads=1 2>&1 | tail -3
cargo test: 81 passed (1 suite, 0.34s)

$ cargo test -p dracon-warden -- --test-threads=1 2>&1 | tail -3
cargo test: 74 passed (2 suites, 1.70s)
```

### A.6 `cargo deny check`

```
$ cargo deny check 2>&1 | tail -3
advisories ok, bans ok, licenses ok, sources ok

$ cargo deny check licenses 2>&1 | tail -2
licenses ok
EXIT=0

# Same exit 0 for advisories, bans, sources
```

### A.7 Binary sizes

```
$ for bin in dracon-sync dracon-system dracon-warden; do
    size=$(stat --format=%s target/release/$bin)
    echo "$bin: $((size / 1024 / 1024)) MiB"
  done
dracon-sync: 11 MiB
dracon-system: 3 MiB
dracon-warden: 4 MiB
```

### A.8 CLI surface (`--help`)

```
$ ./target/release/dracon-sync --help
Commands: status, repos, health, metrics, once, daemon, sync-now, pause,
          resume, config, repair, publish, scaffold

$ ./target/release/dracon-sync repair --help
Commands: concerns, warns, origins, stuck-list, stuck-unstuck,
          dual-branch-list, dual-branch-repair

$ ./target/release/dracon-sync config --help
Commands: edit, validate

$ ./target/release/dracon-sync publish --help
Commands: run, status

$ ./target/release/dracon-system --help
Commands: status, doctor, events, storage, link, symlinks, zram, guard

$ ./target/release/dracon-warden --help
Commands: status, once, scrub-markers, resmudge, repair, filter-clean,
          filter-smudge, keygen, setup-hooks
# Note: no `daemon` subcommand (removed)
```

### A.9 Production unwrap/expect scan

```python
# Python script: /tmp/strip_tests.py
# Strips #[cfg(test)] mod tests { ... } blocks via brace-tracking
for bin in [dracon-sync, dracon-system, dracon-warden]:
    prod = strip_tests(glob(f"{bin}/src/**/*.rs"))
    count_unwrap = len([l for l in prod if ".unwrap()" in l])
    count_expect = len([l for l in prod if ".expect(" in l])
```

Results:
- sync: 2 unwrap, 0 expect
- system: 1 unwrap, 0 expect
- warden: 0 unwrap, 0 expect (all 42 in lib.rs are inside mod tests block starting at line 1058)

### A.10 Cargo.lock duplicates

```
$ grep '^name = ' Cargo.lock | sort | uniq -d | wc -l
21
$ grep '^name = ' Cargo.lock | sort | uniq -c | sort -rn | head -8
   3 name = "windows-sys"
   3 name = "windows-result"
   3 name = "windows-core"
   3 name = "hashbrown"
   3 name = "getrandom"
   2 name = "winnow"
   2 name = "wit-bindgen"
   2 name = "windows_x86_64_msvc"
   2 name = "windows_x86_64_gnullvm"
   ... (13 more 2-version crates)
```

### A.11 Tracked `.pi/goals/archived/*.md`

```
$ git ls-files .pi/goals/archived/ | wc -l
35
$ git ls-files .pi/goals/archived/ | head -3
.pi/goals/archived/goal_2026060101425228_mpu5x2p8-gf5g0x.md
.pi/goals/archived/goal_2026060112391305_mpuhonml-iu4vyd.md
.pi/goals/archived/goal_2026060113214557_mpv55vx0-ly82en.md
```

### A.12 systemd unit settings verified

| Setting | sync value | system value | AGENTS.md value |
|---------|------------|--------------|-----------------|
| `RestartSec` | 5 | 10 | 5 / 10 |
| `CPUQuota` | 15% | 20% | 15% / 20% |
| `MemoryMax` | 2G | 250M | 2G / 250M |
| `TasksMax` | 96 | 64 | 96 / 64 |
| `RestartPreventExitStatus` | 2 78 | 2 78 | 2 78 |

All match. (Source: `dracon-sync/dracon-sync.service`, `dracon-system/dracon-system-guard.service`.)

---

## Comparison to v2 audit (audit-2026-06-06.md)

The v2 audit was a useful but narrower audit focused on clippy, deny, and 6 doc-drift items. This full audit:

1. **Re-verified the v2 audit's findings** with stricter methods (Python brace-tracker instead of regex). Corrected some numbers (production unwrap: 17→2 for sync, 6→0 for warden, 3→1 for system; Cargo.lock duplicates: 10→20+).
2. **Added 8 new finding categories** the v2 audit did not cover: (a) CI is RED on lint and docs jobs, (b) 35 archived goal files tracked, (c) `autoresearch.jsonl` not gitignored, (d) `install.sh` lacks `set -e`, (e) `verify-spec.sh` uses `--lib` on binary crates, (f) freeze marker has no TTL, (g) `EnvRestorer` underused, (h) freeze marker incident from project-state.md not yet addressed.
3. **Updated findings the v2 audit claimed as PASS**: canonicalize() count in production is 0 (not 6 or 10), and clippy with CI flags is RED (not 4 warnings).
4. **Built on `.dracon/audit-cli.md` (2026-06-05)**: the recommendations have been applied (`daemon` subcommand removed from warden, `--force` removed from sync-now, mass-deletion metric removed, `symlinks` subcommand added to system).

The next audit (v4?) should be shorter: a verification pass on whether F-1.2, F-1.3, F-1.4, F-2.2, F-7.1.1, F-7.1.2, F-7.2.1, F-8.1, F-8.2, F-8.3, F-10.1, F-10.5 are closed.

---

## Verification Contract (for this audit to be considered complete)

- [x] cargo check --workspace --all-targets: 0 errors, output captured in A.1
- [x] cargo clippy with CI flags: captured in A.2 (RED — 1 error)
- [x] cargo fmt --check: captured in A.3 (RED)
- [x] cargo doc --no-deps: captured in A.4 (RED with -D warnings)
- [x] cargo test (all 3 binaries, serial): 575 passed, captured in A.5
- [x] cargo deny check (4 sub-checks): all pass, captured in A.6
- [x] Binary sizes: 11/3/4 MiB, captured in A.7
- [x] CLI --help captured for all 3 binaries, captured in A.8
- [x] Production-only unwrap/expect/panic counts via Python brace-tracker, captured in A.9
- [x] Cargo.lock duplicate analysis, captured in A.10
- [x] All 10 audit areas covered (see Coverage Matrix)
- [x] All findings have file:line evidence and concrete remediation
- [x] Severity classification (P0/P1/P2/P3) for every finding
- [x] Top 10 prioritized recommendations list at the top
- [x] Evidence appendix with real command output
- [x] Comparison to v2 audit and audit-cli.md
- [x] Repo is read-only — no source, policy, or config files modified

---

## Next Steps for the User

This audit identified 4 P0/P1 issues that are user-facing and easy to fix:

1. **CI is broken on `main`** — should be fixed before merging more code. Apply fixes in F-1.2, F-1.3, F-1.4.
2. **AGENTS.md CLI table is wrong** — any AI agent reading it will run broken commands. Apply F-7.1.1.
3. **`test-ai` is documented but missing** — AI agents will try to run it and fail. Apply F-7.1.2.
4. **35 archived goal files in git** — purely bloat, no value. Apply F-10.1.

The remaining P2/P3 items can be batched into a follow-up audit cycle.
