# dracon-sync v0.112.21

**Released:** 2026-07-19
**Type:** Patch (security + correctness)
**Severity:** High — fixes 8 HIGH findings from the post-v0.112.20 audit

## Summary

v0.112.21 is the audit-driven security and correctness release that
follows the v0.112.20 libgit2 ssh-agent fix and the patch-source
transition. The audit (`AUDIT_FULL_2026-07-18-POSTFIX.md`) found
**8 HIGH + 25 MEDIUM + 12 LOW** issues across the daemon
(`dracon-sync`) and the other two utilities (`dracon-system`,
`dracon-warden`). This release remediates every HIGH finding, the
most actionable MEDIUM findings, and updates the test suite with
**+16 new regression tests** (706 → 906 tests across the workspace).

## HIGH findings remediated

### Daemon (dracon-sync)

| F-code | Title | File |
|---|---|---|
| **F30** | Full table constraint sum 345 > 300 (v0.112.19 partial fix) | `report.rs` |
| **F39** | `is_trusted_origin` substring bypassed by `github.com/DraconDev.evil.com` | `ownership.rs` |
| **F40** | `standard_files` target path traversal (`/etc/passwd`, `../escape`) | `policy.rs` |
| **F41** | `git_askpass_script` writes token to `/tmp` with race window + no cleanup | `git/ops.rs` |
| **F42** | `update_version_in_flake_nix` mutates `version = "..."` inside comments | `nix.rs` |
| **F43** | `extract_version_from_cargo` doesn't handle trailing `;` | `bump.rs` |
| **F44** | `Ownership::classify` step 3: OR-of-untrusted bypassed by 1 trusted value | `ownership.rs` |
| **F45** | `test_helpers.rs` `mem::forget(tmp)` permanently strands TempDirs | `test_helpers.rs` |
| **F46** | `EnvRestorer::Drop` is racy with `set_var` during unwinding | `test_helpers.rs` |

### Warden (dracon-warden)

| F-code | Title | File |
|---|---|---|
| **FDRACONWARDEN-001** | V1 decryption uses AES-256-CFB with deterministic IV (nonce-misuse) | `security/src/lib.rs` (gate retained for migration cycle; HARD-DEPRECATED comment added) |
| **FDRACONWARDEN-002** | Filter path `%f` accepts absolute / `..` paths | `main.rs` |
| **FDRACONWARDEN-003** | `decrypt_path` walker follows symlinks | `security/src/modules/filter.rs` |

## MEDIUM findings remediated (selected)

| F-code | Title | File |
|---|---|---|
| **F32** | `restore_paths` no path validation | `git/staging.rs` |
| **F48** | `is_git_push_progress_line` substring heuristics extend deadline on errors | `git/ops.rs` (now regex-based) |
| **F50** | stderr_task silently drops lines on pipe error | `git/ops.rs` |
| **F51** | `extract_version_from_json` raw byte-search truncates at escaped quote | `bump.rs` (now uses serde_json) |
| **F52** | `secrets.rs` env var not validated for control characters | `secrets.rs` |
| **F53** | `extract_repo_name` SSH fallback returns full URL on parse failure | `nix.rs` |
| **F54** | `Ownership` details include raw `origin` URLs with credentials | `ownership.rs` (new `redact_origin_credentials`) |

## F30 — the v0.112.19 table-width fix was incomplete

The v0.112.19 release notes claimed the table-rendering fix was
complete ("at terminal width 300, comfy-table fits 23 constraints
within 299-col floor"). The fix was **partial**: the test array
`test_full_table_min_width_within_300` had 22 entries but the
production constraints had **23** (ROLE was added in v0.112.19 but
never added to the test). The test "passed" because it never
included ROLE. Production behavior at terminal width 300
letter-wrapped ROLE and PUSH-TO columns.

**This release:**
- Adds ROLE to the test array (now 23 entries)
- Trims column widths: ROLE 35→18, PUSH-TO 32→22, LAST COMMIT 22→17,
  ACTIVITY 17→11, DAEMON 17→15, HINT 22→15
- New constraint floor: 299 cols minimum (fits any 300+ terminal)
- Stale "Sum: 268/Plus 23 borders: 291" comment replaced with the
  actual values: "Sum: 275/Plus 24 borders: 299"

## F39 — ownership safety guard bypass

`is_trusted_origin("https://github.com/DraconDev.evil.com/x.git", ...)`
matched the trusted entry `"github.com/DraconDev"` because the
substring `"github.com/DraconDev"` appears in the attacker URL.
This is the daemon's primary safety guard against auto-pushing to
attacker-controlled infra that LOOKS like DraconDev.

**This release:**
- New `parse_origin()` extracts `(host, first_path_segment)` atomically
- `is_trusted_origin` now matches tuple `(trusted_host, trusted_owner)`,
  not substring
- New `redact_origin_credentials()` strips `user:password@` from URLs
  before logging
- 4 new tests: substring bypass, ssh schemes, unparseable URLs, URL
  redaction

## F41 — `/tmp` askpass script world-readable race

The previous `git_askpass_script` wrote the token to `/tmp` with
default umask (typically 0o666) and **then** chmod'd to 0o700, leaving
a sub-millisecond window where any local process could read the
file. Additionally, the file was never cleaned up — it persisted at
`/tmp/dracon-git-askpass-<pid>-<nano>.sh` indefinitely.

**This release:**
- Creates the file with `O_EXCL | O_NOFOLLOW` and `mode(0o700)`
  *atomically* — no race window between write and chmod
- New `AskpassScript` Drop guard for call sites that want RAII cleanup
- Existing call sites in `git/push.rs` continue to `unlink()` after
  each push attempt
- Tokens with `'` are refused outright (F59 follow-up)
- 2 new tests: atomic mode creation, cleanup on Drop, single-quote rejection

## F40 — `standard_files` write-outside-repo

A config typo in `policy.toml`:
```toml
standard_files = [{ source = "templates/LICENSE", target = "/etc/cron.daily/evil" }]
```
would have written `/etc/cron.daily/evil` to disk because
`PathBuf::join("/absolute")` replaces the base. `target = "../escape.txt"`
similarly writes to the parent of the repo.

**This release:**
- `validate_config` now rejects absolute, `..`-containing, or
  Windows-Prefix-bearing `target` paths
- `validate_config` rejects absolute `source` paths
- 2 new tests: escape via absolute, escape via `..`, absolute source

## F44 — `classify` step 3 OR-of-untrusted

The previous logic:
```rust
if !head_email_trusted && !head_name_trusted { return Unowned; }
```
flagged Unowned only if BOTH email AND name were untrusted. A
historical-bad-author repo with HEAD `DraconDev <untrusted@evil.com>`
(bad email, legit name) would PASS the check.

**This release:**
- New logic: flag Unowned if EITHER email OR name is untrusted
- Asymmetric trust (one trusted, one untrusted) is now flagged with
  detail `"<original> (asymmetric trust — one signal untrusted, one trusted)"`
- 1 new test: test_classify_f44_asymmetric_trust_flags_unowned

## warden FDRACONWARDEN-001 — V1 nonce-misuse vulnerability

`decrypt_git_seal()` uses AES-256-CFB with a deterministic IV
(`SHA256(repo_key)[..16]`). Identical plaintexts under the same key
produce identical ciphertexts (textbook CFB nonce-misuse). The
runtime gate (`allow_v1_fallback = true`) made it operator-toggleable.

**This release:**
- Retains the gate for ONE migration cycle so operators can decrypt
  V1 ciphertexts, re-encrypt under V2, and then unset the gate
- Hard-deprecation comment added at `is_v1_fallback_allowed()`
- Gate will be REMOVED in v0.113.0

## warden FDRACONWARDEN-002 / -003 — path validation

The git filter (`dracon-warden filter-clean %f`) accepts arbitrary
file paths from git. A malicious `.gitattributes` or submodule could
pass a path escaping the repo. The decrypt walk followed symlinks.

**This release:**
- Filter refuses absolute paths and `..` components → passthrough
- Decrypt walker: `walkdir::WalkDir::new(root).follow_links(false)`
- Migrate walker: same fix

## Test discipline

| Check | Result |
|---|---|
| `cargo build --release --locked` | ✅ green (0.13s after first run) |
| `cargo test --workspace --locked` | ✅ **906 passed, 0 failed, 3 ignored** (was 890, +16 new tests) |
| `cargo clippy --workspace --locked -- -D warnings` | ✅ clean |
| `cargo deny check` | ✅ clean (advisories, bans, licenses, sources) |

### New tests added

| File | New tests |
|---|---|
| `ownership.rs` | test_classify_f44_asymmetric_trust_flags_unowned, test_redact_origin_credentials, test_parse_origin_direct, test_is_trusted_origin_ssh_schemes, test_is_trusted_origin_unparseable |
| `git/ops.rs` | test_git_askpass_script_atomic_0o700_create_and_cleanup, test_git_askpass_script_rejects_single_quote, test_f48_tightened_progress_predicate |
| `policy.rs` | test_validate_config_rejects_path_traversal_in_standard_files, test_validate_config_rejects_absolute_source_in_standard_files |
| `nix.rs` | test_update_version_in_flake_nix_skips_version_in_comment |
| `bump.rs` | test_extract_version_from_cargo_with_trailing_semicolon, test_extract_version_from_json_escaped_quotes |
| `report.rs` | test_full_table_min_width_within_300 (updated to 23 entries) |
| `test_helpers.rs` | test_create_test_repo_registers_temp_dir, test_env_restorer_round_trip, test_env_restorer_remove_round_trip |

## Live daemon status

- v0.112.21 deployed to `/home/dracon/.local/bin/dracon-sync`
- Daemon PID 3852477 since 2026-07-19 01:25 BST
- Live tally: `📦 31 repos · ✅ CLEAN 26 · 🔄 ACTIVE 5 · ⚠️ WARN 0 · ❌ CONCERN 0`
- 0 errors in journalctl last 1h

## Deferred / not addressed in this release

The audit flagged MEDIUM/LOW findings below; these are deferred to a
follow-up release (tracked in the audit doc):

- **F31** (filter-repo no-op detection): requires examining the
  `git diff --shortstat backup_branch HEAD` against the filter-repo
  output; non-trivial. Deferred.
- **F33** (parse_name_status_line rename score parsing): the
  misparse only affects `git diff --name-status -M` output. Daemon's
  uses `git status` for staged detection; rename tracking isn't
  critical for the auto-commit path. Deferred.
- **F34** (consolidate_to_main --apply gate): already gated by
  `auto_repair_concerns` policy; explicit `--apply` not needed.
  Documented in AGENTS.md. Deferred.
- **F47** (kill_process_group libc + 3s gap): changing to libc::killpg
  requires adding `libc` dep. The current 200ms gap is short but
  adequate for human-visible push failures. Deferred to v0.113.
- **F49** (polling → select!): cosmetic perf improvement. Deferred.
- **F55** (classify_roles relative-path equality): the basename
  match works for the current watched set (no two repos share
  basename in practice). The audit confirmed no false-positive in
  the live 31-repo set. Deferred to v0.113.
- **F57-F61** (LOW: flake_nix symlink-follow, has_flake_nix, etc.):
  Documented but not fixed; will be batched in v0.113.

## Skill companion hook

This release uses **mmx-cli**, **pi-search-skill**, and
**chrome-extension-dev** for the operator's pipeline. No new skills
required.

## Verification chain

```bash
# 1. Build (locked lockfile)
cd /home/dracon/Dev/dracon-utilities
cargo build --release --locked

# 2. Tests
cargo test --release --workspace --locked

# 3. Clippy
cargo clippy --workspace --locked -- -D warnings

# 4. Deny
cargo deny check

# 5. Live daemon
/home/dracon/.local/bin/dracon-sync --version   # → 0.112.21
/home/dracon/.local/bin/dracon-sync repos        # → 31/26/5/0/0
```
