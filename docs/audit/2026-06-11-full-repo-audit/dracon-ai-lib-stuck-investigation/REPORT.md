# `dracon-ai-lib` stuck-push investigation

Date: 2026-06-11

## Executive summary

`dracon-ai-lib` is marked `CONCERN` because it is clean locally but cannot push its local `main` branch to GitHub. The remote repository is archived/read-only, so `git push` fails with HTTP 403.

Current state after investigation:

- Repo: `/home/dracon/Dev/dracon-ai-lib`
- Branch: `main`
- Upstream: `origin/main`
- Local status: clean
- Behind upstream: `0`
- Ahead of upstream: `29`
- Push status: `STUCK`
- Root cause: `DraconDev/dracon-ai-lib` is archived and read-only on GitHub.

No destructive remediation was performed. I did not unarchive the repo, rewrite history, delete branches/tags, force-push, rotate secrets, or change remote visibility.

## Evidence captured

All evidence is stored under:

`/home/dracon/Dev/dracon-utilities/docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/`

Key files:

- `inventory-before.json`, `inventory-before.tsv`
- `inventory-after-final.json`, `inventory-after-final.tsv`
- `git-evidence.txt`
- `final-git-evidence.txt`
- `sync-evidence.txt`
- `validation.log`
- `validation-fixed.log`

## Before investigation

`dracon-sync repos --json --full-path` reported:

```text
repo	branch	modified	staged	untracked	ahead	behind	state_flags	push_status	hint
/home/dracon/Dev/dracon-ai-lib	main	0	0	0	28	0	AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
```

Git evidence:

```text
* main dd14038 [origin/main: ahead 28] docs: tidy current tag section
origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
branch.main.remote origin
branch.main.merge refs/heads/main
```

Merge analysis:

```text
merge-base main origin/main = ce377a20fa8b911f3201777c120779ebd56ff903
rev-list --count main ^origin/main = 28
rev-list --count origin/main ^main = 0
```

So the local branch was strictly ahead of `origin/main`, not diverged.

## Push failure

`git push --dry-run origin main` failed with:

```text
remote: This repository was archived so it is read-only.
fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
```

`gh repo view DraconDev/dracon-ai-lib --json isArchived,visibility,defaultBranchRef,url,description` reported:

```json
{
  "defaultBranchRef": {"name": "main"},
  "description": "",
  "isArchived": true,
  "url": "https://github.com/DraconDev/dracon-ai-lib",
  "visibility": "PRIVATE"
}
```

`gh api repos/DraconDev/dracon-ai-lib --jq '{full_name,archived,visibility,default_branch,permissions}'` reported:

```json
{
  "archived": true,
  "default_branch": "main",
  "full_name": "DraconDev/dracon-ai-lib",
  "permissions": {
    "admin": true,
    "maintain": true,
    "pull": true,
    "push": true,
    "triage": true
  },
  "visibility": "private"
}
```

The token has push permissions in the API response, but GitHub still rejects pushes because the repository is archived. The archived flag is the blocker.

## Sync evidence

`dracon-sync repair stuck-list` reported:

```text
✅ no stuck repos
```

`dracon-sync health` reported:

```text
🏥 Health · ✅ healthy · daemon running · freeze off · policy valid
📦 Repos · 20 discovered across 3 roots
```

The incident ledger contains repeated `concern` entries for `/home/dracon/Dev/dracon-ai-lib` with:

```text
reason=AHEAD:28,STUCK_PUSH
action=push_origin_head
result=fail
details=remote: This repository was archived so it is read-only.
details=fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
```

## Low-risk validation fix

Initial validation found one clippy failure:

```text
crates/ai-lib/src/providers/minimax.rs:220
clippy::unnecessary_filter_map
```

I fixed the low-risk lint issue by replacing an always-`Some` `filter_map` with `map`. This preserves behavior and only removes unnecessary wrapping.

Validation after fix:

- `cargo fmt --all --check` → pass
- `cargo test --manifest-path dracon-ai-lib/Cargo.toml -- --test-threads=1` → **181 passed, 0 failed**
- `cargo clippy --manifest-path dracon-ai-lib/Cargo.toml --workspace -- -D warnings` → pass

The fix was committed by sync as:

```text
b87f979 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+4/-4
```

## After investigation

Final `dracon-sync repos --json --full-path` row:

```text
repo	branch	modified	staged	untracked	ahead	behind	state_flags	push_status	hint
/home/dracon/Dev/dracon-ai-lib	main	0	0	0	29	0	AHEAD:29,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
```

Final Git evidence:

```text
* main b87f979 [origin/main: ahead 29] 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+4/-4
origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
merge-base main origin/main = ce377a20fa8b911f3201777c120779ebd56ff903
rev-list --count main ^origin/main = 29
rev-list --count origin/main ^main = 0
```

Final `git push --dry-run origin main` still fails with:

```text
remote: This repository was archived so it is read-only.
fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
```

## Root cause

`dracon-sync` is correct to mark `dracon-ai-lib` as a concern.

The repo is not unhealthy locally: it is clean, tests pass, and clippy passes. The concern is external to the working tree: GitHub has archived `DraconDev/dracon-ai-lib`, making the origin read-only. Because local `main` is 29 commits ahead of `origin/main`, every push attempt fails and the repo remains `AHEAD:29,STUCK_PUSH`.

## Remaining blockers

1. GitHub remote is archived/read-only.
2. Pushing to `origin/main` is blocked until the repo is unarchived or the remote is changed to an active repository.
3. `dracon-sync repair-concerns --apply` is not a safe next step without approval because it may attempt push/rewrite behavior on a stuck repo.
4. Unarchiving, creating a replacement repo, changing `origin`, or excluding the repo from sync are policy/visibility decisions and require explicit approval.

## Recommended next actions

Choose one of these intentionally:

1. **If `dracon-ai-lib` should continue to be the canonical repo**
   - Unarchive `DraconDev/dracon-ai-lib` on GitHub.
   - Re-run `git push origin main`.
   - Re-run `dracon-sync repos --json --full-path`.

2. **If `dracon-ai-lib` should move to a new active repo**
   - Create the replacement repo first.
   - Update `origin` only after approval.
   - Push local `main` to the new remote.
   - Update sync policy if needed.

3. **If the repo is intentionally archived**
   - Keep it as a documented concern.
   - Optionally exclude it from sync/watch scope to avoid recurring `STUCK` alarms.

## Completion audit

Requirements mapped to evidence:

- Fresh inventory before/after: `inventory-before.tsv`, `inventory-after-final.tsv`
- Per-repo Git state: `git-evidence.txt`, `final-git-evidence.txt`
- Push blocker evidence: `git push --dry-run origin main` in `git-evidence.txt` and `final-git-evidence.txt`
- Remote archived evidence: `gh repo view` and `gh api` in `git-evidence.txt`
- Local validation: `validation-fixed.log`
- Low-risk fix applied: commit `b87f979`, diff scope limited to `crates/ai-lib/src/providers/minimax.rs`
- No `.pi/` cleanup or modification performed
- No destructive remediation performed
