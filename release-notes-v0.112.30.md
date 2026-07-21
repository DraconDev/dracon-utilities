# Release Notes — v0.112.30 (2026-07-21)

**Headline**: The daemon now bootstraps brand-new `git init` repos
end-to-end: it creates the root commit from the operator's untracked
files (policy-aware), detects the never-pushed state correctly, pushes
to github + gitlab, and no longer spams guaranteed-to-fail codeberg
pushes for repos created under the v0.112.28 codeberg-quota posture.

Driven by the `convos` investigation: the operator ran `git init`,
dropped 4 files in, and the daemon did nothing for 12 hours
(`❌ CONCERN · no commits yet`). Four distinct bugs were found and
fixed; see `docs/design/empty-repo-auto-create-fix-2026-07-21.md`.

---

## What's new

### 1. Empty-repo root-commit bootstrap

The daemon loop previously bailed on `!is_repo_ready` BEFORE
dispatching `sync_repo`, making the existing (policy-violating,
unreachable) bootstrap in `sync.rs` dead code for the daemon flow.

New `sync::bootstrap_empty_repo_commit`:

- Gated on `git::is_stable_empty_repo` — distinguishes operator-init
  (safe to commit) from mid-clone (MUST NOT touch) via lock-file and
  `tmp_pack_*` checks.
- Ownership gate identical to the daemon loop's.
- Full staging policy: `git ls-files --others --exclude-standard`
  (respects `.gitignore` / warden secrets block), `auto_stage_untracked`,
  `untracked_exclude_patterns`, `auto_commit_exclude_patterns`,
  `max_stage_file_bytes`, explicit-path `git add -A -- <paths>`.
- Commit message: `auto: initial commit (N files)`.
- Failures cool down 300s (no 1s-cycle log spam); `auto_commit =
  false` and nothing-to-stage leave the repo alone (accurate CONCERN
  hint remains).

The `sync.rs` bootstrap block now uses the same helper and falls
through, so `dracon-sync sync-now <repo>` commits AND pushes in one
invocation.

### 2. "Never pushed" no longer looks synced

After the root commit, `configure_publish_upstream_if_missing` writes
the branch config, which made `has_upstream = true` while
`refs/remotes/origin/main` didn't exist → libgit2 ahead = 0 →
`has_local_or_pending_work = false` → repo skipped forever, and
`handle_ahead_push`'s `should_push` was false too. A freshly-bootstrapped
repo would have sat at a false "synced" state with its root commit
never pushed.

- New `git::upstream_tracking_ref_missing` (upstream configured,
  remote-tracking ref absent).
- Daemon-loop ahead override now also runs for that case, with
  `git::count_all_head_commits` as the final fallback (no tracking
  ref anywhere ⇒ every commit is unpushed).
- `handle_ahead_push` treats a missing tracking ref as push-needed.

This also covers the "remote branch deleted" drift case, consistent
with the daemon's mirror design.

### 3. Codeberg skipped for new repos (v0.112.28 follow-up)

New repos under the quota posture got a codeberg remote configured and
every push failed with `Forgejo: Push to create is not enabled` —
guaranteed-failure spam every sync cycle.

New `codeberg_push_excluded` skips codeberg at configure-time and
push-time when effective auto_create is off (global false + no
per-repo opt-in) AND the repo has no codeberg tracking ref (local
check, no network). Pre-v0.112.28 repos (all have the tracking ref)
keep pushing. The dead remote is removed from `.git/config` on the
first push via `remove_stale_remotes` (verified live on convos).

**Latent v0.112.28 bug fixed**: the codeberg arms in both the new
exclusion and `auto_create_all_remotes` matched the RAW `auth_type`
field; the operator's config leaves it unset (defaults to `GitHub`),
so the arms never fired and the per-repo `auto_create_on_codeberg`
opt-in was silently ignored. Both now use `effective_auth_type()`
(push_url auto-detect). Regression test included.

### 4. v0.112.29 auto-create spam throttled

The v0.112.29 create-only auto-create ran every 1s cycle per
not-ready repo (2 SSH `ls-remote`/sec per empty repo, forever).
Now throttled to one attempt per 300s per repo; first attempt stays
immediate.

---

## Files changed

- `dracon-sync/Cargo.toml` — 0.112.30
- `dracon-sync/src/git/status.rs` — `is_stable_empty_repo`,
  `upstream_tracking_ref_missing`, `count_all_head_commits` (+9 tests)
- `dracon-sync/src/sync.rs` — `bootstrap_empty_repo_commit`, sync.rs
  bootstrap rewrite, `handle_ahead_push` missing-ref fix (+8 tests)
- `dracon-sync/src/git/multi_remote.rs` — `codeberg_push_excluded`,
  `has_codeberg_tracking_ref`, `push_mirror_remotes` exclusion,
  `auto_create_all_remotes` effective_auth_type fix (+8 tests)
- `dracon-sync/src/daemon.rs` — bootstrap call at `is_repo_ready`
  site, ahead-override extension, `auto_create_cooldowns` +
  `empty_bootstrap_cooldowns`, configure-time codeberg skip
- `docs/design/empty-repo-auto-create-fix-2026-07-21.md` — full
  root-cause analysis

## Test discipline

- `cargo test --workspace --locked` ✅ **783 daemon** (+25 over
  v0.112.29), 0 failed
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean

## Live verification (convos)

- `🌱 convos created root commit (4 files, empty repo bootstrap)`
- Push to github + gitlab succeeded (`263a30b`, then `15ce735`)
- Unrelated-histories divergence (operator's manual
  `gh repo create --add-readme`) resolved by manual
  `git merge --allow-unrelated-histories` — daemon-created repos are
  EMPTY so this only happens after manual forge-side creation; not
  auto-resolved by design
- codeberg remote auto-removed on first post-fix push; no more
  `Forgejo: Push to create` failures
- `repos -s`: `🔄 ACTIVE · 🟢 synced 1m · healthy` (was
  `❌ CONCERN · no commits yet`)
