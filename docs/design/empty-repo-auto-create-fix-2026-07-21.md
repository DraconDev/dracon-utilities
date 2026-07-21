# Design: Empty-repo bootstrap + never-pushed ahead detection (2026-07-21)

> Versions: v0.112.29 (auto-create on discovery + gitlab URL fix) and
> v0.112.30 (root-commit bootstrap + ahead/push detection + codeberg
> exclusion). Written after live verification against the `convos`
> repo, the first repo created under the v0.112.28 codeberg-quota
> posture.

## The operator-visible bug

The operator ran `git init convos`, dropped 4 files in, and watched
the daemon do **nothing** for 12 hours: `❌ CONCERN · 4 ut · no
commits yet — make first commit to enable push` (v0.112.29 improved
the hint from the misleading "push: fail · set upstream"). The
commit-all principle says the daemon should have committed those
files itself.

## Root causes (four distinct bugs)

### Bug 1: daemon loop bailed before the existing bootstrap

`daemon.rs`'s per-repo loop did:

```rust
if !is_repo_ready(&repo) { continue; }   // unborn HEAD → false
```

`sync.rs::sync_repo_with_ahead_since` already contained an empty-repo
bootstrap (`git add -A` + `git commit -m "initial"`), but the daemon
loop never dispatched `sync_repo` for empty repos, so the bootstrap
was unreachable from the main flow. Additionally, that bootstrap
violated policy: no `max_stage_file_bytes`, no exclude patterns, no
ownership gate, and the commit result was discarded (it printed
"created initial commit" even when the commit failed).

**Fix (v0.112.30)**: the daemon loop now calls the new
`sync::bootstrap_empty_repo_commit` directly at the `is_repo_ready`
site, gated on `git::is_stable_empty_repo` (see below). The sync.rs
bootstrap block now calls the same helper and falls through so the
CLI (`sync-now`) path commits **and pushes** in one invocation.

### Bug 2: mid-clone vs operator-init discrimination

`is_repo_ready` returns false for BOTH a mid-clone repo (MUST NOT
touch — the daemon would `git add` a half-checked-out working tree)
and a stable `git init` repo (safe to commit). The new
`git::is_stable_empty_repo` distinguishes them:

1. `.git` is a real directory (worktree pointers skipped).
2. `.git/HEAD` starts with `ref: refs/` (the `git init` state).
3. No `*.lock` files directly in `.git/` (`index.lock` = checkout in
   progress, `HEAD.lock`, `packed-refs.lock`, `shallow.lock`,
   `FETCH_HEAD.lock`).
4. No `objects/pack/tmp_pack_*` (in-flight clone/fetch download).

The residual window (fetch done, branch ref not yet written) is
closed by git itself: clone writes `refs/heads/<branch>` atomically
with the other refs BEFORE checkout, so `git rev-parse HEAD` already
succeeds there and the `index.lock` check covers checkout.

### Bug 3: "never pushed" repos looked fully synced

After the bootstrap created the root commit, the daemon loop ran
`configure_publish_upstream_if_missing` → wrote `branch.main.remote`
+ `branch.main.merge` → `has_tracking_upstream` = true. But
`refs/remotes/origin/main` did not exist (never pushed), so
libgit2's ahead/behind computed 0, and:

- daemon loop: `has_local_or_pending_work = dirty || ahead>0 ||
  behind>0 || !has_origin || !has_upstream` → **false** → repo
  skipped forever, report would show a false "synced".
- `handle_ahead_push`: `should_push = ahead>0 || !has_upstream` →
  **false** → even if dispatched, no push.

**Fix (v0.112.30)**: new `git::upstream_tracking_ref_missing`
(upstream configured but `refs/remotes/<remote>/<branch>` absent).
The daemon-loop ahead override now also runs when the upstream is
configured-but-missing, with `git::count_all_head_commits`
(`git rev-list --count HEAD`) as the final fallback: when no
remote-tracking ref exists anywhere, every commit is definitionally
unpushed. `handle_ahead_push` treats a missing tracking ref as
push-needed.

Semantics note: this also covers "remote branch deleted" (merged PR
with branch deletion) — the daemon's mirror design treats a missing
remote branch as drift and re-pushes, consistent with the existing
`!has_upstream` path.

### Bug 4: codeberg guaranteed-failure push spam (v0.112.28 follow-up)

convos is the first repo created after v0.112.28 flipped codeberg to
`auto_create = false`. The daemon still configured the codeberg
remote (`configure_standard_remotes_if_missing`) and pushed to it on
every cycle, failing with `Forgejo: Push to create is not enabled`
— guaranteed-failure spam, and the `codeberg_public_only` gate
didn't help because the operator had flipped convos public.

**Fix (v0.112.30)**: `codeberg_push_excluded` — skip codeberg at
configure-time AND push-time when (a) its effective auto_create is
off (global false + no per-repo `auto_create_on_codeberg` opt-in)
AND (b) the repo has no `refs/remotes/codeberg/main` tracking ref
(never pushed there — local check, no network). Pre-v0.112.28 repos
all have the tracking ref, so they keep pushing. Because the
filtered list drives `configure_all_remotes` +
`remove_stale_remotes`, the dead codeberg remote is also removed
from `.git/config` on the first push under this rule (verified live
on convos).

**Latent bug fixed in passing**: both the new exclusion and the
v0.112.28 `auto_create_all_remotes` codeberg-override arm matched on
the RAW `auth_type` field. The operator's config sets no `auth_type`,
so the raw field is the `GitHub` default and the Codeberg arm never
fired — the per-repo `auto_create_on_codeberg` opt-in was silently
ignored. Both sites now use `effective_auth_type()` (push_url
auto-detect). Regression test:
`test_codeberg_excluded_via_push_url_autodetect`.

## The bootstrap itself

`sync::bootstrap_empty_repo_commit(repo, policy, excluded_dir_names,
dry_run)`:

1. Gate on `policy.auto_commit`.
2. Ownership gate identical to the daemon loop's (empty repos classify
   on `user.email` + origin URL; no HEAD author exists yet).
3. Enumerate untracked files via `untracked_entries`
   (`git ls-files --others --exclude-standard -z`) — respects
   `.gitignore` including the warden-managed secrets block.
4. Apply the same per-entry policy filters as the normal pipeline:
   `auto_stage_untracked`, `matches_untracked_exclude`,
   `should_stage_entry` (excluded dirs, file patterns,
   `max_stage_file_bytes`, per-repo `auto_commit_exclude_patterns`).
5. `git add -A -- <explicit paths>` via `stage_existing_files`
   (never bare `git add .`).
6. `git commit --no-verify -m "auto: initial commit (N files)"`.

Returns `Ok(false)` when nothing policy-compliant exists to stage
(empty working tree, all files excluded/oversized) — the repo stays
CONCERN with the accurate "no commits yet" hint until the operator
adds committable content. Bootstrap failures (e.g. no `user.email`
anywhere) cool down for 300s instead of retrying every 1s cycle.

## v0.112.29 regression fixed in v0.112.30

The v0.112.29 create-only auto-create ran on every 1s cycle for
every not-ready repo; each attempt issues `git ls-remote` (SSH
round-trip) per configured remote → 2 SSH connections/sec per empty
repo forever. v0.112.30 throttles it to one attempt per 300s per
repo (`auto_create_cooldowns`); the first attempt stays immediate.

## Live verification (convos)

1. `🌱 /home/dracon/Dev/convos created root commit (4 files, empty
   repo bootstrap)` at 13:47:25.
2. Push initially rejected non-fast-forward: the operator had
   manually created the github repo with `--add-readme` during
   v0.112.29 debugging → unrelated histories. Resolved by
   `git merge origin/main --allow-unrelated-histories` (kept the
   README). Note: the daemon's OWN auto-create creates EMPTY repos
   (no README), so this divergence only happens when the operator
   manually creates the forge repo with content — an operator-action
   case, intentionally NOT auto-resolved.
3. `🔧 configured publish upstream for main on origin`, push to
   github + gitlab succeeded (`263a30b` on both).
4. After the codeberg-exclusion deploy: next push removed the
   codeberg remote (`remove_stale_remotes`), no more
   `Forgejo: Push to create` failures.
5. `repos -s`: `🔄 ACTIVE · 🟢 synced 1m · healthy`.

## Tests added (25 total across v0.112.29/30)

- `is_stable_empty_repo`: fresh init / index.lock / tmp_pack /
  HEAD.lock / detached HEAD (5)
- `upstream_tracking_ref_missing`: no config / config-without-ref /
  ref-exists (3)
- `count_all_head_commits` (1)
- `bootstrap_empty_repo_commit`: creates root commit / respects
  .gitignore / skips oversized / all-oversized → false /
  auto_commit=false / unowned skipped / nothing to stage (7)
- `sync_repo` empty-repo end-to-end (1)
- `codeberg_push_excluded`: off+no-ref / tracking-ref / opt-in /
  global-on / no-codeberg-remote / push_url-autodetect (6)
- `has_codeberg_tracking_ref`: absent / present (2)

Daemon total: 783 tests.
