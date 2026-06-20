# Mirror-only push detection and empty-repo remote setup

Date: 2026-06-20  
Related goals: mirror-only push reliability; `.dracon` unpushed-branch concern; `dracon-strategy` empty-repo remote setup.

## Problem

The daemon's normal ahead/behind accounting comes from `GitService::get_status()`, which is backed by `git status --porcelain --branch`. That output only includes `ahead N` / `behind N` when the current branch has an upstream tracking branch configured.

Most operator repos now use explicit SSH mirror remotes:

```toml
[remote "github"]
  url = "git@github.com:DraconDev/{repo}.git"

[remote "gitlab"]
  url = "git@gitlab.com:dracondev/{repo}.git"

[remote "codeberg"]
  url = "git@codeberg.org:dracondev/{repo}.git"
```

They intentionally do **not** configure `branch.main.remote` / `branch.main.merge`. The daemon pushes with explicit refspecs (`HEAD:refs/heads/main`), so upstream tracking is not required for push.

This created two reliability gaps:

1. **Mirror-only repos can have unpushed commits but report `ahead = 0`.**  
   `.dracon` is the key example. Without an upstream tracking branch, `git status` may report no ahead count even though `HEAD` is ahead of one or more mirror tracking refs.

2. **Newly initialized repos with no commits are skipped before remotes are configured.**  
   `dracon-strategy` was created with `git init`, but because it had no commits yet, `is_repo_ready()` returned false and the daemon skipped it before `configure_all_remotes()` could add the standard mirror remotes.

## Fix

### 1. Detect mirror-only unpushed commits

For repos without upstream tracking, the daemon now checks the known mirror tracking refs directly:

```text
refs/remotes/github/main..HEAD
refs/remotes/gitlab/main..HEAD
refs/remotes/codeberg/main..HEAD
```

The helper `count_unpushed_vs_mirrors()` returns the first non-zero count it finds. In the daemon pulse loop, when `!has_upstream && status.ahead == 0`, the daemon overrides `status.ahead` with that value.

This preserves the existing `NO_UPSTREAM` reporting semantics: no tracking upstream is informational for mirror-only repos, not a concern. It only affects dispatch timing so the daemon actually reaches `handle_ahead_push`.

### 2. Configure remotes for empty repos

Before the daemon skips an unready repo, it now checks whether the repo has any remotes. If the repo has no remotes and the sync policy defines standard mirror remotes, the daemon calls `configure_all_remotes()` for that repo.

This gives an empty repo its standard `github` / `gitlab` / `codeberg` remotes before the first commit is made. The daemon still does not create an artificial initial commit; once the user adds the first commit, normal push flow can use the configured mirrors.

The check is intentionally non-destructive:

- existing remotes are preserved;
- `ensure_remote()` updates a remote only if the URL differs;
- repos with any existing remote are not touched by this empty-repo path.

### 3. Avoid auto-create spam for repos that already exist

The daemon's `auto_create` path previously attempted repository creation on every push cycle, even when the repo already existed on the remote. This caused repeated GitHub API rate-limit failures like:

```text
GraphQL: You have created too many repositories, too quickly
```

`auto_create_all_remotes()` now receives the local repo path and checks whether the remote repo already exists via `git ls-remote <url> HEAD` before attempting creation. If the remote repo exists, the daemon records an `Ok` result for that remote and skips the create API call.

This keeps the existing `auto_create` behavior for genuinely missing repos while preventing unnecessary create attempts for the operator's existing mirror fleet.

### 4. Keep daemon push hooks bypassed

The daemon intentionally runs its own security checks before auto-commit/auto-push:

- warden pre-commit hook runs on daemon commits;
- warden hardening pass runs separately;
- daemon push commands use `--no-verify` so the interactive pre-push hook does not block automated mirror sync on false positives.

This matters because the warden pre-push hook scans the pushed diff for plaintext secret patterns. A unit test fixture containing the standard AWS example key (`AKIAIOSFODNN7EXAMPLE`) previously caused `dracon-code` to fail all mirror pushes. The daemon should not be blocked by its own defensive hook when its separate security checks have already run.

## Verification

### Unit tests

`cargo test -p dracon-sync --locked` must pass.

New coverage includes:

- `remote_repo_exists()` success/failure behavior via a fake git command;
- existing configure/push tests remain intact.

### Build and policy checks

```bash
cargo build --release --locked
cargo deny check
```

Both must pass. Warnings that pre-exist this change are acceptable; new errors are not.

### Live checks

For every watched repo with commits:

```bash
for r in github gitlab codeberg; do
  git rev-list --count refs/remotes/$r/main..HEAD
done
```

Expected result: `0` ahead for all three mirrors.

For newly initialized repos with no commits:

```bash
git remote -v
```

Expected result: `github`, `gitlab`, and `codeberg` remotes are present and point at the standard mirror URLs, unless the operator already configured different remotes.

For daemon processing evidence:

```bash
journalctl --user -u dracon-sync.service --since "10 min ago" --no-pager
```

Expected evidence: logs show the daemon processed the affected repos, configured remotes for empty repos, and pushed mirror-only repos when unpushed commits were detected.

## Non-goals

- The daemon does not create an artificial initial commit for empty repos.
- The daemon does not overwrite operator-configured remotes.
- The daemon does not treat missing upstream tracking as a concern for mirror-only repos.
- The daemon does not disable warden security scanning; it only bypasses the interactive pre-push hook for daemon-managed pushes.
