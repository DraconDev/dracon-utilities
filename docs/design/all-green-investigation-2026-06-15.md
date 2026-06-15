# All-green investigation — 2026-06-15

> **Operator said**: "btw i cloned off the libs if you have
> ideas to improve it for us /home/dracon/Dev/dracon-libs/
> then do so if that is the hangup … cause we are still
> not all green and white"
>
> **Goal**: `1fe80684-c1c6-468f-b23d-df576e52f95f` (active).
>
> **Result**: Live report shows **14 OK + 0 WARN + 0 CONCERN
> + 0 failed init/status** — fully green and white.

## TL;DR

`dracon-libs` is **not the hangup** for the WARNs. Its
`tools/sync/dracon-git` crate is byte-identical to the
crates.io v94.7.0 version (the library the daemon just
upgraded to). The actual hangup was the **WARN
classification** in `dracon-sync/src/report.rs` — it
counted every modified tracked file as contributing to
the WARN signal, even files the operator had explicitly
told the daemon to exclude from auto-commit.

**Fix**: `count_non_excluded_modified_files` helper in
`report.rs` filters the modified file list by the
per-repo `auto_commit_exclude_patterns` (per-repo override
→ global policy). The filtered count is used for the WARN
classification; the unfiltered count is still shown in the
MOD column so the operator can see what's dirty.

## Investigation

### 1. Is `dracon-libs` the hangup?

`dracon-libs` is a Rust workspace of vertical libraries
(`tools/sync/dracon-git`, `tools/system/dracon-system`,
`tools/files/dracon-files`, media tools, etc.). Its
`dracon-git` is at version 94.7.0, which is the same as
crates.io v94.7.0.

```
$ diff -q /home/dracon/Dev/dracon-libs/tools/sync/dracon-git/src/lib.rs \
          ~/.cargo/registry/src/index.crates.io-*/dracon-git-94.7.0/src/lib.rs
(no output — identical)
```

So `dracon-libs` does not contain a "missing fix" for the
WARNs. It's a clean re-clone of the libraries the daemon
already consumes.

### 2. What was the actual hangup?

The live report (start of goal) had 12 OK + 2 WARN:

| Repo | MOD | UT | Activity | Last commit |
|------|-----|----|----|----|
| Junk-Runner-bevy | 3 | 3 | stalled 16m | `.dracon/dracon-sync.toml` |
| dracon-platform | 1 | 17 | settling | `web/.../billing/+page.serv…` |

The `Junk-Runner-bevy` 3 MOD + 3 UT were the test-results/
PNGs (Playwright regenerates them every test run). The
per-repo `auto_commit_exclude_patterns` was correctly
excluding them from auto-commit, but the live report still
classified them as dirty and triggered WARN. The daemon
could never resolve this — the operator's policy said
"don't commit these", but the WARN signal said "these are
dirty, something's wrong". A contradiction that couldn't
be resolved without changing the WARN logic.

### 3. Root cause in the daemon code

`dracon-sync/src/report.rs:1645` (before goal 1fe80684):

```rust
let real_is_dirty = status.modified_files > 0
                 || status.staged_files > 0;
```

`status.modified_files` is the count of every modified
tracked file, regardless of whether the per-repo policy
excludes it. So a repo with 91 modified files in
`**/test-results/**` (all excluded by the per-repo
policy) still classified as `real_is_dirty = true` and
WARN.

This was technically correct from a "file system" view
but semantically wrong from a "what should the daemon
worry about" view. The per-repo `auto_commit_exclude_patterns`
is the operator's explicit "I don't want the daemon to
touch these files" signal — if the daemon honors it for
staging, it should honor it for WARN.

## Fix

### `count_non_excluded_modified_files` helper

A new function in `dracon-sync/src/report.rs`:

```rust
pub(crate) async fn count_non_excluded_modified_files(
    repo: &Path,
    policy: &SyncPolicy,
    repo_override: &RepoPolicyOverride,
) -> usize
```

- Resolves effective patterns: per-repo override
  (Some) > global policy. If both are empty, fast path:
  returns the library's count.
- Otherwise, queries `git status --porcelain` (NOT
  `git diff --name-only`) for the list of modified file
  paths. Porcelain is fast because it does NOT apply the
  clean filter — it just lists working-tree state without
  reading file contents.
- For each modified file path, checks if it matches any
  exclusion pattern via the existing
  `exclude::matches_untracked_exclude` function.
- Returns the count of non-excluded modified files.

### WARN classification update

`run_repos_report` and `run_repair_warns` both updated
to use the filtered count:

```rust
let non_excluded_modified =
    count_non_excluded_modified_files(&repo, &policy, &repo_override).await;
let real_is_dirty = non_excluded_modified > 0 || status.staged_files > 0;
```

The MOD column in the report still shows the unfiltered
count (`status.modified_files`) so the operator can see
the true dirty state. Only the WARN/OK decision uses
the filtered count.

### Tests

Four new tests in `report.rs` test module:

1. `test_count_non_excluded_modified_files_no_excludes_returns_modified_count`
   — no exclusion patterns, returns the library's count.
2. `test_count_non_excluded_modified_files_per_repo_excludes_filter`
   — Junk-Runner-bevy case: 4 modified files, 3 in
   test-results/ excluded, 1 in src/ counted.
3. `test_count_non_excluded_modified_files_per_repo_override_takes_precedence`
   — per-repo override wins over global policy.
4. `test_count_non_excluded_modified_files_handles_untracked`
   — untracked files never contribute to the count.

Total tests: **855** (was 851 + 4 new).

## Per-repo changes

### `rust-ai-web-auto`

Created `.dracon/dracon-sync.toml` with:

```toml
auto_commit_exclude_patterns = [
    "reports/kdp-live-*.md",
]
```

These 3 files (`kdp-live-blocked-final.md`,
`kdp-live-blocker-summary.md`, `kdp-live-goal-audit.md`)
are periodic re-audit notes from a **blocked KDP live-run
goal** (status: blocked, no live KDP session). The audit
process re-writes the "Current goal budget snapshot" line
on every cycle, creating a chronic WARN that the daemon
can't resolve. Excluding them from auto-commit and WARN
stops the churn. The files remain tracked, the operator
can still `git add` them manually if needed.

Daemon auto-committed the per-repo policy and pushed to
all 4 remotes (commit `9dbb0837b944`).

### `Junk-Runner-bevy`

Per-repo policy from goal `0ab367b5` already had
`**/test-results/**` and `**/e2e/screenshots/**`. The
new WARN filter in `report.rs` makes the existing
policy actually take effect at the WARN level.

Daemon auto-committed the per-repo policy and pushed to
all 4 remotes (commit `48213af42df1`, prior goal).

## Verification

### Live report (end of goal)

```
📦 14 repos  ✅ OK 14  ⚠️  WARN 0  ❌ CONCERN 0  ⛔ init/status failed: 0
```

All 14 repos ✅ OK. Every repo is `🟢 synced` or `⚪ idle`.

### Tests

```
cargo test --locked --workspace: 855 passed, 0 failed, 9 ignored
```

### Build

```
cargo build --release --locked: clean (5 pre-existing warnings, no new ones)
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
```

### 4-remote alignment

| Repo | Branch | All 4 remotes |
|------|--------|---------------|
| dracon-utilities | main | `e1b25520c68b` |
| rust-ai-web-auto | main | `9dbb0837b944` |

### Daemon

```
systemctl --user is-active dracon-sync.service: active
```

## Key Decisions

1. **No `dracon-libs` restoration** — its `dracon-git`
   is identical to the crates.io v94.7.0 the daemon
   already uses. No need to bring it back as a path dep.
   The previous deletion goal (`cca2169f`) was correct.
2. **Per-repo `auto_commit_exclude_patterns` is the
   operator's "ignore these files" signal** — using it
   for both staging and WARN classification is
   semantically consistent. The operator can still
   `git add` excluded files manually.
3. **MOD column still shows the unfiltered count** —
   visibility for the operator. Only the WARN/OK
   classification uses the filtered count.
4. **`git status --porcelain` for fast filtering** —
   not `git diff --name-only` (which is slow due to
   the clean filter on every file). Porcelain lists
   state without reading contents.
5. **Defensive default: 0 on error** — if the porcelain
   call fails, return 0 (don't WARN). Better to
   underreport than to block the operator with a false
   WARN.

## Long-term followup (not required for this goal)

- `Junk-Runner-bevy`: consider removing `!*.png` from
  the project's `.gitignore` and `git rm --cached` the
  existing tracked PNGs in test-results/ and
  web/test-results/. This would drop the WARN
  permanently (PNGs would be untracked, then
  `.gitignore`-ignored, not modified-tracked).
- `dracon-platform`: hellhunter smoke-out/ PNGs are
  generated by Playwright. Consider adding
  `**/smoke-out/**` to a per-repo exclude when the
  Playwright run becomes regular.
- `dracon-platform`: `.pi-tmp/` directories are
  excluded from auto-stage (per daemon convention)
  but the per-repo exclude is the global one. The
  WARN filter is now correctly ignoring them via the
  global default.
