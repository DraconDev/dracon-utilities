# Audit Part 2 — dracon-sync (main.rs / policy.rs / exclude.rs / visibility.rs / release.rs)
# Source-level audit, ~8,800 LOC, 2026-07-18
# For inclusion into AUDIT_FULL_2026-07-18-POSTFIX.md

## HIGH findings

### F20 — `GIT_COMMAND_LOCK` is broken: guard dropped immediately (HIGH, real)
**File:** `dracon-sync/src/policy.rs:9-69`, `policy.rs:346-352`, `git/mod.rs:8-14`

```rust
pub(crate) static GIT_COMMAND_LOCK: Mutex<()> = Mutex::new(());

pub(crate) struct GitCommand {
    inner: StdCommand,
}

impl GitCommand {
    pub(crate) fn new() -> Self {
        let _command_guard = GIT_COMMAND_LOCK.lock().expect("git command lock poisoned");
        Self {
            inner: StdCommand::new(git_binary()),
        }
    }
}
```

- **What:** `let _command_guard = ...lock()...` binds the guard; the next line returns `Self { ... }` and the function exits. `_command_guard` is dropped. Subsequent `.args(...)`, `.status()`, `.output()` via `Deref<Target=StdCommand>` happen with NO lock held. The comment claims "continuing would risk overlapping git operations" but the guard doesn't survive long enough to prevent that.
- **Why it matters:** **498 call sites** of `git_cmd()`/`tokio_git_cmd()` (counted via `grep -rn "git_cmd()\|tokio_git_cmd()"`). Every concurrent thread that calls these runs git in parallel. The lock provides ZERO serialization despite being a `pub(crate) static` named like a real lock. Operator's mental model is wrong — they assume git commands are serialized.
- **Real-world impact:** Two concurrent pushes to the same bare remote could interleave and produce pack-corruption or non-fast-forward errors the daemon then blames on the remote. Index-lock races (F3) are even worse when concurrent git commands fight over the same lock file.
- **Fix:** Either (a) rename `_command_guard` → `guard` (drop the leading underscore; the compiler will hold it) AND redesign the wrapper so the guard lives as long as the inner command, OR (b) eliminate the wrapper entirely and document that concurrent git commands are fine (modern git handles concurrent commands on separate working trees; the daemon's single-repo-per-task model already serializes per-repo operations).
- **Verification:** Spawn N threads calling `git_cmd().args(["status"]).output()` concurrently with a `sleep` injected; assert they DO overlap (proving the lock is broken — that's the bug).

### F21 — Same broken pattern in `release.rs:790-792` test code (HIGH, defense-in-depth)
**File:** `dracon-sync/src/release.rs:790-792`, `git/misc.rs:79`

```rust
let _lock = crate::git::acquire_path_lock();
} // immediately dropped
```

`acquire_path_lock` (in `git/misc.rs:79`) is otherwise a real parking_lot lock with `try_lock` loop — but the test releases it before doing anything that needs serialization. The test PATH-mutation passes only because the mock gh is unique to this test, not because of the lock.

- **Fix:** Scope the lock to cover the actual call: `let _lock = ...; let result = create_github_release(...).await; drop(_lock);`

## MEDIUM findings

### F22 — Seven SyncPolicy fields `#[allow(dead_code)]` "intentional future-policy config; not yet wired into runtime" (MEDIUM)
**File:** `dracon-sync/src/policy.rs:483-613, 830, 835`

Fields with `#[allow(dead_code)]`: `push_debounce_secs` (489), `sem_max_concurrent_sync` (525), `settling_max_delay_secs` (597), `dirty_max_age_action` (604), `min_commit_interval_secs` (611), `untracked_warn_threshold` (497), `settling_max_delay_secs` (830, per-repo override), `dirty_max_age_action` (835, per-repo override). AUDIT-3-UTILITIES-2026-07-10.md CONCERN #6 is referenced.

- **What:** Operator config that loads but is silently ignored. A user setting `min_commit_interval_secs = 60` to slow down thrashing would see no effect.
- **Fix:** Either wire them or emit a `warn!`-level log on parse (not just `#[allow(dead_code)]`). The design is sound; wiring is preferred.

### F23 — Path traversal in `standard_files[].target` and `.source` not validated (MEDIUM)
**File:** `dracon-sync/src/main.rs:1417`, `dracon-sync/src/policy.rs:1381-1500` (`validate_config`)

`validate_config` checks `repo_name_map` keys for `/` and `\` (line 1412) but does NOT validate `standard_files[i].target` or `standard_files[i].source` for absolute paths (`/etc/passwd`), `..` segments, or null bytes.

- **What:** A policy file with `[[standard_files]] target = "../../etc/passwd"` and `overwrite = true` will overwrite files outside the repo root when `dracon-sync scaffold` runs. Operator-owned policy file = low-likelihood, but the principle is "validate config" and the validator already rejects the analogous issue for `repo_name_map`.
- **Fix:** In `validate_config`, iterate `policy.standard_files` and reject any `target` or `source` whose `Path::new(...).components()` includes `Component::ParentDir` or is `Component::RootDir`.

### F24 — `get_github_visibility` returns `true` (private) on ANY error → 24h false-negative cache (MEDIUM)
**File:** `dracon-sync/src/visibility.rs:222-243`

`get_github_visibility` returns `true` (private) on ANY error: gh not installed, no auth, network failure, rate limit, malformed JSON. The 24h cache then writes `visibility=private`. A transient outage (gh 1h down, CI service interruption, temporary 401) bakes a "private" stamp for 24h that may be wrong.

- **What:** False negatives on visibility → 24h of (a) incorrect `(private)` in `dracon-sync repos` column, (b) codeberg pushes skipped for repos that are actually public.
- **Fix:** Distinguish "gh missing/unauthenticated" (cache 24h) from "network/transient" (cache 5 min) from "definitively private per GitHub API" (cache 24h). Return `Result<bool>` instead of `bool`.

### F25 — `SyncPolicy::load` order of operations: 0-silently-replaced-before-warning (MEDIUM)
**File:** `dracon-sync/src/policy.rs:1184-1240`

Order: `stage_op_timeout_secs == 0` first replaces 0 with the default 60, THEN `stage_op_timeout_secs < 10` runs the warning. Net effect: setting `stage_op_timeout_secs = 0` in TOML silently becomes 60, **with no warning**. Same for `push_retries == 0` → silently becomes 3 (no warning).

- **Why it matters:** Silent default-replacement vs noisy validation is inconsistent. `validate_config` adds an ERROR for `push_retries = 0` at line 1518, but `SyncPolicy::load` is called from many places without going through `validate_config`.
- **Fix:** Either reorder `load()` so 0 → warning + default, or remove the redundant default-replacement since `serde(default=...)` already supplies 60.

### F26 — `extract_repo_name` SSH fallback returns full URL on parse failure (MEDIUM)
**File:** `dracon-sync/src/release.rs:163-198`

When `url.starts_with("git@")` but the URL has no `:` (malformed), the code falls back to `url.clone()` — the FULL URL is then passed to `gh release create --repo <full-url>` which fails.

- **Fix:** Return `Result<String>` instead of fallback-on-failure.

### F27 — `RemoteConfig::resolve_account` SSH branch uses `url.rfind(':')` — wrong for `https://user:pass@host/...` (MEDIUM)
**File:** `dracon-sync/src/policy.rs:152-164`

For `git@gitlab.com:DraconDev/<repo>.git` this correctly finds the colon. BUT for `https://user:pass@host/...` (legal but rare), the colon after `user` would be picked first by `rfind(':')`, returning garbage. Also `auto_create_account` defaults to `""` (empty string), and `resolve_account` returns empty when extraction fails.

- **Fix:** In `resolve_account` when neither explicit nor extraction succeeds, return `Err(anyhow!(...))` and surface as a config warning.

## LOW findings

### F28 — `is_excluded_dir_name` silent no-match on mid-pattern glob (LOW)
**File:** `dracon-sync/src/exclude.rs:951-970`

Pattern `.tmp-` requires the prefix to start with `.`. A pattern like `tmp-` (without leading dot) falls through to `ends_with('*')` glob check but is not actually a glob — silently returns false. Setting `tmp-*` as a prefix pattern does nothing because the matcher doesn't recognize `*` mid-pattern.

- **Fix:** Emit `warn!()` when `pattern.contains('*') && !pattern.ends_with('*')`.

### F29 — `parse_visibility_cache` two-line format not future-extensible (LOW)
**File:** `dracon-sync/src/visibility.rs:128-149`

Two-line format `visibility=<state>\n<ts>`. A future daemon version that adds a third line would be silently dropped. Use serde on a documented schema.

## INFO / non-issues (verified clean)

- **Token-in-curl-stdin:** `visibility.rs:255-285, 360-385, 540-560` passes tokens (GitLab PRIVATE-TOKEN, Codeberg Authorization) to `curl` via stdin (`-H @-` + `stdin.write_all(...)`), NOT in argv. This is correct security practice — tokens don't appear in `/proc/<pid>/cmdline`. ✅
- **Zero `TODO`/`FIXME`/`XXX`/`HACK` comments** across all 5 files.
- **Zero `panic!()`** in library code.
- **Zero `unsafe` blocks**.
- **Zero `unwrap()`/`expect()`** in production (non-test) code paths across all 5 files. All 70+ occurrences are inside `#[cfg(test)]` blocks. Exemplary.
- **Operator-policy defaults correct:** `max_stage_file_bytes = 100 * 1024 * 1024` (100 MiB), `push_op_timeout_secs = 300` (5 min), `default_untracked_exclude_patterns` matches the design doc (scratch dirs + 9 codeberg-leak-fix patterns). `test_example_toml_matches_policy_defaults` regression test prevents drift. ✅
- **Broken-pipe panic hook** at `main.rs:325-333` is fragile if Rust's panic formatting changes but acceptable as a UX nicety. ✅
- **`exclude.rs` pattern matching logic, dir-name exclusion, gitlink handling** all correct. Tests at lines 486-715 verify the 6/9 submodule gitlink propagation regression fix. ✅
- **`release.rs` publish pipeline** is end-to-end gated correctly: auto_tag → auto_release → auto_publish → nix_auto_update. Tag rollback on push failure (line 92) is a thoughtful touch. ✅

## Summary

| Severity | Count |
|---|---|
| **HIGH** | 2 (F20 lock broken, F21 same pattern in tests) |
| **MEDIUM** | 6 (F22 dead-code fields, F23 path traversal, F24 visibility false-negative cache, F25 zero-silent-replace, F26 SSH parse fallback, F27 rfind(':') bug) |
| **LOW** | 2 (F28 mid-pattern glob silent, F29 cache format not extensible) |
| **INFO** | clean (5 items) |

The headline finding is **F20** — `GIT_COMMAND_LOCK` does not actually serialize git commands despite being a `pub(crate) static Mutex`. 498 call sites are affected. The fix requires a structural change to how `GitCommand` carries its lock guard, not a one-line patch.
