# Audit Part 4 — dracon-sync (nix.rs / standard_files.rs / ownership.rs / role.rs / secrets.rs / print.rs / bump.rs / test_helpers.rs / git/ops.rs)
# Source-level audit, 2026-07-18
# For inclusion into AUDIT_FULL_2026-07-18-POSTFIX.md

## HIGH findings

### F39 — `is_trusted_origin` substring match bypassed by `https://github.com/DraconDev.evil.com/...` (HIGH, security)
**File:** `dracon-sync/src/ownership.rs:255-274`

```rust
fn is_trusted_origin(url: &str, trusted_hosts: &[String]) -> bool {
    ...
    let normalized = if let Some(idx) = url.find('@') {
        ...
        format!("{}/{}", host, path)
    } else {
        url.to_string()  // ← HTTPS URLs pass through unmodified
    };
    trusted_hosts.iter().any(|h| normalized.contains(h))  // ← substring match
}
```

- **Verified attack:** `https://github.com/DraconDev.evil.com/whatever/repo.git` contains the substring `github.com/DraconDev`. With `trusted_hosts = ["github.com/DraconDev"]`, this URL is classified as **Owned**, and the daemon auto-pushes to attacker-controlled infra that LOOKS like DraconDev. 
- **Why it matters:** This is the daemon's primary safety guard. AGENTS.md mandates "auto-commit and push" but ownership classification is the protection against pushing to malicious remotes. A bypass means the daemon may push operator's commits (and possibly secrets via config files) to attacker infra.
- **Fix:** Parse URL into host + first path segment, match both atomically. Or require trusted entries to be a precise prefix with a known separator boundary. The simplest safe check: extract `host_str()` from `url::Url::parse()` and `path_segments()[0]`, then check `(host, first_path_segment)` tuple match.

### F40 — `standard_files.rs` path traversal: `target = "/etc/passwd"` writes outside repo (HIGH, security)
**File:** `dracon-sync/src/standard_files.rs:24`

```rust
let target_path = repo.join(&cfg.target);
...
std::fs::copy(&source_path, &target_path)
```

`PathBuf::join` with absolute path REPLACES the base. So `target = "/etc/cron.daily/evil"` writes to `/etc/cron.daily/evil`. `target = "../escape.txt"` writes to parent of repo. The `validate_config` function (main.rs:1417, policy.rs:1381-1500) does NOT check `standard_files[i].target` for absolute paths or `..` segments.

- **Why it matters:** Config-driven write-anywhere under daemon's UID. Operator-owned config = low-likelihood, but the principle is "validate config" and the validator already rejects the analogous issue for `repo_name_map`.
- **Fix:** In `validate_config`, iterate `policy.standard_files` and reject any `target` whose `Path::new(...).components()` includes `Component::ParentDir` or is `Component::RootDir`.

### F41 — `git_askpass_script` writes token to `/tmp` with race window + no cleanup (HIGH, security)
**File:** `dracon-sync/src/git/ops.rs:263-285`

```rust
let tmp_path = std::env::temp_dir().join(format!("dracon-git-askpass-{}-{}.sh", ...));
let script = format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", escaped);
tokio::fs::write(&tmp_path, &script).await?;  // ← mode 0o666 (umask)
let mut perms = ...metadata()?.permissions();
perms.set_mode(0o700);
tokio::fs::set_permissions(&tmp_path, perms).await?;  // ← chmod AFTER write
Ok(tmp_path)
```

- **Two issues:**
  1. Race window between write and chmod: file is world-readable for ~1ms.
  2. No cleanup: file persists at `/tmp/dracon-git-askpass-<pid>-<nano>.sh` indefinitely. Token leaks to anyone with `/tmp` read access.
- **Fix:** Create with `OpenOptions::create_new(true).mode(0o700)` atomically. Schedule deletion on a `Drop` guard.

### F42 — `update_version_in_flake_nix` mutates wrong `version = "..."` field when comment contains `version = "..."` (HIGH, correctness)
**File:** `dracon-sync/src/nix.rs:42-86`

Line-search predicate is `line.contains("version = \"")` with no comment check. A comment line inside a `buildRustPackage` block containing `version = "1.0.0"` gets clobbered. The comment-skip check is OUTSIDE the rewrite branch.

- **Fix:** Add `!trimmed.starts_with('#')` to the rewrite predicate, OR switch to rnix-parser for structural parsing.

### F43 — `extract_version_from_cargo` doesn't handle trailing `;` (HIGH, correctness)
**File:** `dracon-sync/src/bump.rs:1-22`

A line like `version = "1.0.0";` (legal TOML with semicolon) parses to value=`"1.0.0";` — the trailing `;` prevents `strip_suffix('"')` from matching, returning `None`. Silent failure on valid TOML.

- **Fix:** Use `toml::Value::parse(content)` and read `package["version"]` directly. Or regex with optional `;`.

### F44 — `Ownership::classify` step 3: OR-of-untrusted instead of AND-of-trusted (HIGH, classification logic)
**File:** `dracon-sync/src/ownership.rs:236-249`

The intent is "if HEAD author has BOTH untrusted email AND untrusted name, flag unowned". The code reads:
```rust
if !head_email_trusted && !head_name_trusted { ... unowned ... }
```
which IS OR-of-untrusted. So a repo with HEAD author `DraconDev <untrusted@evil.com>` (bad email, legit name) → `!email_trusted && !name_trusted` is FALSE (name IS trusted) → skip step 3 → continues to step 5 → **Owned**. The safety guard is bypassed by ONE trusted value.

- **Fix:** Tighten: only flag unowned if NEITHER email NOR name is trusted. Same as current — but ALSO log a warning when one is untrusted. Or invert: require BOTH trusted to skip the flag.

### F45 — `test_helpers.rs:65, 92` `mem::forget(tmp)` permanently strands TempDirs (HIGH, test infra)
**File:** `dracon-sync/src/test_helpers.rs:65, 92`

`std::mem::forget(tmp)` is a permanent leak. The test temp dir is on disk forever (until OS cleanup). For a long-running test runner this fills disk over hours.

- **Fix:** Move temp dir lifetime management to a per-test `Drop` with a global registry, or document that `create_test_repo` should only be called from `#[ctor]` setup with explicit end-of-suite cleanup.

### F46 — `EnvRestorer::Drop` is racy with `set_var` during unwinding (HIGH, UB)
**File:** `dracon-sync/src/test_helpers.rs:187-194`

`Drop` runs during unwinding. `std::env::set_var` is racy with other threads reading env vars. `set_var` was marked `unsafe` in recent Rust nightly (RFC 2990) precisely for this reason.

- **Fix:** Defer env var restoration to a non-Drop mechanism — explicit cleanup at end of test function.

## MEDIUM findings

### F47 — `kill_process_group` uses shell-format negative PID + hard-coded 200ms SIGTERM-to-SIGKILL (MEDIUM)
**File:** `dracon-sync/src/git/ops.rs:14-27`

`format!("-{pid}")` for `pid = u32::MAX` produces `"-4294967295"`; on Linux process groups modulo `pid_max`. Hard-coded 200ms between SIGTERM and SIGKILL may not give pack-objects enough time to clean up.

- **Fix:** Use `libc::killpg(pid, SIGTERM)` directly. Increase gap to 2-3 seconds.

### F48 — `is_git_push_progress_line` substring heuristics extend timeout on error messages (MEDIUM)
**File:** `dracon-sync/src/git/ops.rs:42-52`

Substring `delta` matches error messages like `error: cannot merge without a merge base (use --allow-unrelated-histories for a delta-branch merge strategy)`. Substring `bytes` matches `0 bytes allocated` from `GIT_TRACE=1`. Loose substring heuristics extend deadline on adversarial input → effective no-timeout.

- **Fix:** Use regex like `^\s*(?:[Cc]ounting|[Ww]riting|[Cc]ompressing|[Rr]eceiving|[Rr]esolving deltas).*\d+%`.

### F49 — `run_child_inner` polls every 250ms instead of using `tokio::select!` (MEDIUM, resource)
**File:** `dracon-sync/src/git/ops.rs:108-150`

Polling loop is legacy ergonomics. tokio has `child.wait()` as a future we could `select` against.

### F50 — `stderr_task` silently drops lines on pipe error (MEDIUM)
**File:** `dracon-sync/src/git/ops.rs:98-117`

`while let Ok(Some(line))` exits silently on `Err`. Partial stderr + timeout hides the real cause.

### F51 — `extract_version_from_json` raw byte-search fragile to escaped quotes (MEDIUM)
**File:** `dracon-sync/src/bump.rs:24-44`

A JSON value like `{"version": "1.0.0\"hotfix"}` — first `find('"')` matches the `\"` quote marker. Returns `1.0.0\` and drops the rest.

- **Fix:** Use `serde_json::Value::parse(content).ok()?.get(key)?.as_str()?`.

### F52 — `secrets.rs` env var not validated for control characters (MEDIUM)
**File:** `dracon-sync/src/secrets.rs:23-28`

A `GH_TOKEN=$'foo\nbar\nevil'` env var would be used as-is. Git credential protocols use `\n` as a request terminator — an injection here could smuggle commands to the credential helper.

- **Fix:** After `std::env::var`, scan for `val.chars().any(|c| c.is_control())` and refuse with a warning.

### F53 — `extract_repo_name` SSH fallback returns full URL on parse failure (MEDIUM)
**File:** `dracon-sync/src/nix.rs:283-293`

Same pattern as F26 from part 2. URL forms with `ssh://git@host:port/path` get the port colon picked first.

### F54 — `Ownership` details include raw `user.email` and `origin` URLs (MEDIUM, PII/log leakage)
**File:** `dracon-sync/src/ownership.rs:175, 218-220, 240-244`

`detail: format!("origin = {}", url)` includes the URL verbatim. If URL contains `https://user:password@host/...`, password leaks into JSON report.

- **Fix:** Strip `user:password@` from URLs before formatting.

### F55 — `classify_roles` matches by basename, not path (MEDIUM)
**File:** `dracon-sync/src/role.rs:97-115`

Two watched repos with the same basename (e.g. `released/junk-runner` and `wip/junk-runner`) would BOTH be classified as Submod-of the first parent whose `.gitmodules` mentions a submod named `junk-runner`.

- **Fix:** Use **relative-path equality**: `my_path` relative to `other_path` compared to `entry.path`.

### F56 — `print.rs:should_color` env var mutation without EnvRestorer (MEDIUM, test infra)
**File:** `dracon-sync/src/print.rs:74-83`

Manual `NO_COLOR` save/set/restore pattern without `EnvRestorer`. If test panics between set and restore, env var leaks.

## LOW findings

### F57 — `has_flake_nix` follows symlinks silently (LOW)
**File:** `dracon-sync/src/nix.rs:14-16`

`Path::is_file()` follows symlinks. Attacker-planted `flake.nix` symlink to `/etc/passwd` would be read into a PR body.

### F58 — `restore_paths` (staging.rs) — see F32 (LOW, defense-in-depth)

### F59 — `git_askpass_script` token escaping incomplete for shell metacharacters (LOW)
**File:** `dracon-sync/src/git/ops.rs:270`

`token.replace('\'', "'\"'\"'")` handles single quotes but tokens with `\` followed by `'` break POSIX shell quoting.

### F60 — `secrets.rs` parent dir perms not checked (LOW)
**File:** `dracon-sync/src/secrets.rs:155-164`

Only checks the .env file mode, not parent directory. A 0600 file in a 0777 directory is still effectively world-writable.

### F61 — `test_git_cmd()` is just `crate::git::git_cmd()` — no actual logic (LOW, dead helper)
**File:** `dracon-sync/src/test_helpers.rs:36-39`

The helper is just a wrapper. Doc-comment promises "serializes git invocations" but the implementation does NOT.

## INFO / non-issues

- `bump.rs` has no actual version-bump logic — only extractors. Bumping lives elsewhere (release.rs).
- `format_bytes` in `print.rs` works correctly within the 2^53 byte range.

## Summary

| Severity | Count |
|---|---|
| **HIGH** | 8 (F39 trusted-origin bypass, F40 standard_files path traversal, F41 askpass token leak, F42 nix comment clobber, F43 TOML semicolon, F44 classify logic, F45 mem::forget leak, F46 EnvRestorer UB) |
| **MEDIUM** | 10 |
| **LOW** | 5 |
| **INFO** | clean |

**Top priority fixes:**
1. **F39** (ownership substring bypass) — the primary safety guard, exploitable today.
2. **F41** (askpass token leak) — credential leak via `/tmp` file.
3. **F40** (standard_files path traversal) — write-anywhere under daemon's UID.
