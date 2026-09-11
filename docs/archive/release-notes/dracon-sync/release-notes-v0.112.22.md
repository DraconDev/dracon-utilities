# dracon-sync v0.112.22

**Released:** 2026-07-19
**Type:** Patch (MEDIUM-sweep)
**Severity:** Low — remediates 5 MEDIUM + 2 LOW findings deferred from v0.112.21

## Summary

v0.112.22 closes out the MEDIUM findings that were deferred from
v0.112.21. The HIGH/MEDIUM sweep is now complete for the daemon.
Warden + system + LOW-batch work is documented as v0.113 follow-up.

## Remediated

### Daemon (dracon-sync) — 5 MEDIUM + 2 LOW

| F-code | Title | File |
|---|---|---|
| **F31** | `rewrite_ahead_paths` creates empty backup branch on no-op | `git/staging.rs` |
| **F33** | `parse_name_status_line` accepts malformed rename without score | `git/diff.rs` |
| **F34** | `consolidate_to_main` deletes remote branch without `--apply` gate | `main.rs` |
| **F47** | `kill_process_group` 200ms SIGTERM→SIGKILL gap (extended to 2s + better error reporting) | `git/ops.rs` |
| **F49** | 250ms poll interval for child.wait (reduced to 100ms; select! was already event-driven) | `git/ops.rs` |
| **F55** | `classify_roles` matches by basename only (added full-path equality check) | `role.rs` |
| **F60** | `check_secrets_dir_permissions` accepts group-writable | `secrets.rs` |
| **F61** | `test_git_cmd()` doc-comment falsely claims to serialize git invocations | `test_helpers.rs` |

## Key changes

### F31 — no-op filter-repo deletes empty backup branch

`rewrite_ahead_paths` now compares `backup_branch^{tree}` vs `HEAD^{tree}`
after the rewrite. If equal, the rewrite was a no-op and the backup
branch is deleted. Without this, every "I tried to filter-repo but
nothing matched" event leaves a `backup/pre-sync-<timestamp>` branch
cluttering `git branch` output.

### F33 — rename score required

`parse_name_status_line` now requires `R<digits>` (e.g. `R100`), not
bare `R`. The previous parser would happily accept `R\told\tnew` and
read the score digit (if any) into the `old` path slot. The existing
test for rename parsing was updated; 7 new tests cover the
edge cases (no score, non-digit score, empty paths, etc.).

### F34 — `--apply` gate for `consolidate_to_main`

The CLI command `dracon-sync repair dual-branch-repair <repo>` now
defaults to DRY-RUN and requires `--apply` to actually delete the
master branch locally + remotely. Previously, this was an
unconditional destructive operation.

### F47 — kill process group: 200ms → 2s, with diagnostic

The previous 200ms SIGTERM → SIGKILL gap was tight for processes
doing real cleanup work (large git filter-repo unpacking). New gap:
**2s**. Also: if `kill` is missing on PATH, we now log a warning
instead of silently failing.

### F55 — full-path equality for role classification

Previously, two watched repos sharing a basename could collide on
role classification. New logic prefers full relative-path equality
first; falls back to basename only as a last resort for legacy
`.gitmodules` formats.

## Test discipline

| Check | Result |
|---|---|
| `cargo build --release --locked` | ✅ green |
| `cargo test --workspace --locked` | ✅ **915 passed, 0 failed, 3 ignored** (was 906 at v0.112.21, +9 new tests) |
| `cargo clippy --workspace --locked -- -D warnings` | ✅ clean |
| `cargo deny check` | ✅ clean |

### New tests

| File | New tests |
|---|---|
| `git/staging.rs` | test_f31_noop_rewrite_deletes_backup_branch |
| `git/diff.rs` | parse_name_status_basic_statuses, parse_name_status_rename_with_score, parse_name_status_rename_without_score_rejected, parse_name_status_rename_with_non_digit_suffix_rejected, parse_name_status_rename_with_empty_paths_rejected, parse_name_status_unknown_status_returns_none, parse_name_status_empty_line_returns_none |
| `role.rs` | f55_full_path_distinguishes_same_basename_repos |

## Live daemon

- v0.112.22 deployed to `/home/dracon/.local/bin/dracon-sync`
- Daemon PID 4083710 since 2026-07-19 02:19 BST
- Live tally: `📦 31 repos · ✅ CLEAN 23 · 🔄 ACTIVE 7 · ⚠️ WARN 1 · ❌ CONCERN 0`
- 0 errors in journalctl

## Deferred to v0.113

- **Warden (FDRACONWARDEN-004..010)**: 7 MEDIUM hardening fixes
  (keychain ordering, atomic key writes, event-log redaction, etc.)
- **dracon-system (FDRACONSYS-001..004)**: 4 MEDIUM (mutex unwrap,
  renice pid validation, HOME unset handling, process-name spoofing)
- **Remove `[patch.crates-io]`** when `dracon-git` v94.7.1 publishes
  to crates.io (operator action)

## Verification chain

```bash
cd /home/dracon/Dev/dracon-utilities
cargo build --release --locked              # ✅ green
cargo test --release --workspace --locked   # ✅ 915 passed
cargo clippy --workspace --locked -- -D warnings  # ✅ clean
cargo deny check                            # ✅ clean
/home/dracon/.local/bin/dracon-sync --version   # → 0.112.22
/home/dracon/.local/bin/dracon-sync repos        # → 31/23/7/1/0
```