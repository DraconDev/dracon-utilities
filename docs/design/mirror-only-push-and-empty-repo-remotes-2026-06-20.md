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

2. **New repos with no remotes are skipped or left unpushed before remotes are configured.**  
   `dracon-strategy` was created with `git init`, but because it had no commits yet, `is_repo_ready()` returned false and the daemon skipped it before `configure_all_remotes()` could add the standard mirror remotes. A later discovered repo, `DraconDev-private`, had commits but still no remotes; after remotes were added, the daemon also needed direct remote-HEAD detection because there were no remote-tracking refs yet.

## Fix

### 1. Detect mirror-only unpushed commits

For repos without upstream tracking, the daemon now checks for unpushed commits in two layers:

1. Known mirror tracking refs, when they exist:

   ```text
   refs/remotes/github/main..HEAD
   refs/remotes/gitlab/main..HEAD
   refs/remotes/codeberg/main..HEAD
   ```

2. Configured remote HEADs, when tracking refs are absent or stale:

   ```text
   git ls-remote github HEAD
   git ls-remote gitlab HEAD
   git ls-remote codeberg HEAD
   ```

The helper `count_unpushed_vs_mirrors()` returns the first non-zero tracking-ref count it finds. If that returns zero, `count_unpushed_vs_configured_remotes()` compares local `HEAD` to each configured remote's `HEAD`. Any mismatch or missing remote HEAD is treated as unpushed (`> 0`), which is enough to force dispatch to `handle_ahead_push`.

This preserves the existing `NO_UPSTREAM` reporting semantics: no tracking upstream is informational for mirror-only repos, not a concern. It only affects dispatch timing so the daemon actually reaches `handle_ahead_push`.

VS Code's Source Control UI is separate: it asks to "Publish Branch" when the current branch has no `branch.<name>.remote` / `branch.<name>.merge` upstream config. That can happen even when the daemon can push successfully to explicit mirror remotes.

### 2. Configure remotes for repos with no remotes

Before the daemon makes the `is_repo_ready()` decision, it now checks whether the repo has any remotes. If the repo has no remotes and the sync policy defines standard mirror remotes, the daemon calls `configure_all_remotes()` for that repo.

This covers both empty repos and repos that already have local commits. The daemon still does not create an artificial initial commit for an empty repo; once the user adds the first commit, normal push flow can use the configured mirrors.

The check is intentionally non-destructive:

- existing remotes are preserved;
- `ensure_remote()` updates a remote only if the URL differs;
- repos with any existing remote are not touched by this no-remote path.

### 3. Avoid auto-create spam for repos that already exist

The daemon's `auto_create` path previously attempted repository creation on every push cycle, even when the repo already existed on the remote. This caused repeated GitHub API rate-limit failures like:

```text
GraphQL: You have created too many repositories, too quickly
```

`auto_create_all_remotes()` now receives the local repo path and checks whether the remote repo already exists via `git ls-remote <remote-name> HEAD` before attempting creation. The check uses the same SSH hardening environment as daemon pushes. If the remote repo exists, the daemon records an `Ok` result for that remote and skips the create API call.

This keeps the existing `auto_create` behavior for genuinely missing repos while preventing unnecessary create attempts for the operator's existing mirror fleet.

### 4. Load Codeberg tokens from the legacy PAT directory

Codeberg auto-create needs an API token. The operator's Codeberg token historically lives in `~/.dracon/secrets/pat/codeberg.env`, while `load_secret()` only checked `~/.dracon/utilities/sync/secrets`. The daemon now falls back to the legacy PAT directory for Codeberg auto-create, without printing or exposing token values.

### 5. Configure VS Code publish upstream for mirror-only repos

After the daemon has configured remotes and the repo is ready, it now configures a publish upstream when the current branch has none:

- `origin` wins if present, for backwards compatibility with traditional repos;
- otherwise `github` is used when present, because it is the operator's primary public mirror;
- otherwise the first configured policy remote present locally is used.

The daemon writes:

```text
branch.<branch>.remote = <primary-remote>
branch.<branch>.merge = refs/heads/<branch>
```

It does not overwrite an existing upstream. After a successful push, the daemon fetches the primary remote's branch ref and points `@{u}` at it, so `git status --branch` and VS Code see a real tracking branch instead of a missing upstream.

This is only a publish-upstream hint for tools like VS Code. The daemon still pushes to every configured mirror explicitly and does not rely on `origin` or upstream tracking for mirror sync.

### 6. Show publish-upstream issues on the report table

`dracon-sync repos` now computes a `PublishState` for each repo and renders the existing `🔗 PUBLISH` column with a visible flag when there is a problem:

| State | Cell | Color | Meaning |
| --- | --- | --- | --- |
| `Missing` | `⚠️ none` | yellow | no `branch.<name>.remote` config and no `@{u}` |
| `Gone` | `⚠️ <remote/branch> (gone)` | yellow | upstream configured but `refs/remotes/<remote>/<branch>` does not exist locally |
| `Ok` | `<remote/branch>` | green | upstream configured and remote-tracking ref resolves |

The legend documents the three states so the operator can spot a problem without reading source code.

### 7. Keep daemon push hooks bypassed

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
- publish upstream configuration preserves existing upstreams and adds `github/main` for mirror-only repos;
- publish upstream refresh fetches the primary remote ref and updates `@{u}`;
- publish upstream cell rendering flags missing/gone states with `⚠️ none` / `⚠️ <remote/branch> (gone)` and color;
- `branch_upstream` returns the correct `PublishState` for missing config and gone remote-tracking refs;
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

For newly initialized or no-remote repos:

```bash
git remote -v
git config --get-regexp '^branch\.'
git status --short --branch
```

Expected result: `github`, `gitlab`, and `codeberg` remotes are present and point at the standard mirror URLs, unless the operator already configured different remotes. After the repo has commits and the daemon has configured a publish upstream, `git config` shows `branch.<branch>.remote` and `branch.<branch>.merge`; after a successful push, `git status --short --branch` should show an upstream such as `github/main` instead of VS Code's "Publish Branch" condition.

For repos that already have commits but no remotes, expected result after daemon processing is the same plus all three mirrors at `ahead=0` once auto-create/push completes.

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
- The daemon does not print secret values when loading legacy PAT files.
- The daemon does not change the mirror push path: it still pushes explicitly to all configured mirrors with `HEAD:refs/heads/<branch>`.
