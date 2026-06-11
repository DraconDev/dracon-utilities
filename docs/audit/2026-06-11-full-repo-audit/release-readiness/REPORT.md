# Release Readiness Assessment

Date: 2026-06-11  
Verdict: **Not release-ready / blocked for publication.**

## Short answer

The codebase is technically healthier than it was earlier today, but it is **not safe to publish or run an external release yet**.

Main blockers:

1. **Public-readiness blocker for `dracon-utilities`**
   - `docs/public-readiness.md` and `docs/public-release-plan.md` say `dracon-utilities` is **not safe to publish as-is**.
   - Current/reachable history contains local agent/task state, audit artifacts, operational logs, and secret-shaped fixture strings that need explicit cleanup/approval before public exposure.

2. **Mirror/release blocker for `one-mil-girls`**
   - GitHub push is OK, but GitLab mirror `main` is protected with push access set to `No one`.
   - Evidence: `one-mil-girls-gitlab-protected-branch.json` and `one-mil-girls-gitlab-push.log`.

3. **Current inventory still has WARN/user-change rows**
   - No unexplained `CONCERN`/`STUCK_PUSH` remains after fetching `dracon-platform`.
   - Remaining rows are `DIRTY`/`WARN` with `push_status=OK`, caused by preserved user changes and branch state.
   - `Junk-Runner-bevy` is currently on `tauri2` with local changes; pushing `HEAD` to `origin main` is rejected because remote `main` has work not integrated locally.

## Fresh sync inventory

Command:

```bash
DRACON_SYNC_GIT_BIN=${DRACON_SYNC_GIT_BIN:-/run/current-system/sw/bin/git} \
  dracon-sync repos --json --full-path
```

Evidence:

- `inventory-current.json`
- `inventory-current.tsv`

Latest non-OK rows observed:

```text
repo                                  modified staged untracked ahead push_status state_flags hint
dracon-platform                       2        0      0         0     OK          DIRTY     run repair-warns --apply
browser-extensions-shared             1        0      4         0     OK          DIRTY     run repair-warns --apply
Junk-Runner-bevy                      4        0      0         0     OK          DIRTY     run repair-warns --apply
dracon-utilities                      1        0      0         0     OK          DIRTY     run repair-warns --apply
pully-fully-pull-based-fleet-reconciler 2      0      1         0     OK          DIRTY     run repair-warns --apply
dracon-code                           2        0      0         0     OK          DIRTY     run repair-warns --apply
```

Interpretation: these are release-readiness warnings, not hidden sync failures. They need review/preservation decisions before release.

## Public-readiness status

Existing durable evidence:

- `/home/dracon/Dev/dracon-utilities/docs/public-readiness.md`
- `/home/dracon/Dev/dracon-utilities/docs/public-release-plan.md`
- `/home/dracon/Dev/dracon-utilities/docs/audit/2026-06-11-full-repo-audit/final/REPORT.md`
- `/home/dracon/Dev/dracon-utilities/docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md`

Summary from `docs/public-readiness.md`:

- `dracon-utilities` is **not safe to publish as-is**.
- Working-tree high-risk paths and reachable-history high-risk paths were previously documented.
- Public release requires an explicit public-release branch, cleanup/rewrite approval, public-safe docs, secret-shaped fixture review, and fresh scans.

This is the primary release blocker.

## Technical validation

### `dracon-utilities`

Fresh checks passed:

```text
cargo fmt --all --check                         pass
cargo clippy --workspace -- -D warnings         pass
cargo deny check                                advisories ok, bans ok, licenses ok, sources ok
scripts/verify-spec.sh                          PASS
dracon-sync config validate                     Policy is valid
dracon-sync scaffold --dry-run                  No standard files to scaffold
cargo test --workspace -- --test-threads=1      705 passed, 9 ignored
```

### `rust-ai-web-auto`

Fresh checks passed:

```text
cargo fmt --all --check                         pass
cargo clippy --workspace -- -D warnings         pass
cargo test --workspace -- --test-threads=1      145 passed, 9 ignored
```

### `dracon-platform`

Fresh checks passed:

```text
cargo fmt --all --check                         pass
cargo clippy --workspace -- -D warnings         pass; cargo wrapper reported 0 errors, 2 warnings
cargo test --workspace -- --test-threads=1      268 passed, 6 ignored
scripts/check-env-encryption.sh                 All 13 tracked .env* file(s) are encrypted
```

## Git/push evidence

### `dracon-platform`

Initial inventory briefly showed `AHEAD:1,STUCK_PUSH`, but direct evidence showed no local/remote divergence:

```text
git rev-list --count main ^origin/main = 0
git rev-list --count origin/main ^main   = 0
git push --dry-run origin main           = Everything up-to-date
```

After `git fetch origin main`, the stale ahead count cleared. Current WARN is preserved user changes.

### `Junk-Runner-bevy`

Current branch is `tauri2`, not `main`:

```text
git branch --verbose --verbose
* tauri2
  bevy-legacy
  main
  temp_fix
```

Pushing current `HEAD` to `origin main` is rejected:

```text
! [rejected] main -> main (fetch first)
error: failed to push some refs to 'https://github.com/DraconDev/Junk-Runner-bevy.git'
hint: Updates were rejected because the remote contains work that you do not have locally.
```

This is a release-branch decision blocker: do not force-push or rebase without explicit approval.

### `one-mil-girls`

GitHub push is OK, but GitLab mirror is blocked by protected branch policy:

```text
remote: GitLab: You are not allowed to push code to protected branches on this project.
! [remote rejected] main -> main (pre-receive hook declined)
```

GitLab API confirms `main` push access is `No one`.

Evidence:

- `one-mil-girls-gitlab-protected-branch.json`
- `one-mil-girls-gitlab-push.log`

Required decision: unprotect/adjust GitLab `main` push access, push to an unprotected mirror branch, or remove the GitLab mirror remote for this repo.

## Release decision

### Safe to publish now?

**No.**

### Safe to run internal release tooling?

**No, unless explicitly limited to internal/private mirrors and excluding blocked mirrors.**

The release pipeline should not be allowed to publish or change visibility until:

1. Public-readiness cleanup/approval is complete for `dracon-utilities`.
2. GitLab protected-branch blocker for `one-mil-girls` is resolved or the GitLab mirror is disabled for release.
3. Current WARN/user-change rows are reviewed and either accepted, committed intentionally, or left out of the release scope.
4. Release branch/tag policy is explicit.

## Required next decisions

1. **Public release scope**
   - Is the intended release only internal/private installers, or public GitHub/Crates/NPM visibility?
   - If public: execute the public-release plan on an approved branch.

2. **GitLab mirror policy**
   - For `one-mil-girls`, decide whether to unprotect/adjust GitLab `main`, use an unprotected branch, or remove the GitLab mirror from release scope.

3. **Branch ownership**
   - For `Junk-Runner-bevy`, decide whether `tauri2` is release content or whether release should target `main`.
   - Do not force-push or rebase without approval.

4. **WARN/user-change review**
   - Review current `DIRTY` rows before release. These are preserved user changes and should not be silently included or discarded.

## Conclusion

The project is **not release-ready yet**. The technical validation picture is mostly good for the affected Rust repos, and the sync notification/concern work is fixed/surfaced. The release blockers are governance/public-readiness and mirror-policy decisions, not unresolved compiler/test failures.
