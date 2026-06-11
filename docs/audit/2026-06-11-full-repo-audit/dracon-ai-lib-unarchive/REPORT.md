# `dracon-ai-lib` unarchive and push recovery

Date: 2026-06-11

## Outcome

`DraconDev/dracon-ai-lib` has been unarchived and `dracon-ai-lib` push health has been restored.

Current state:

```text
repo: /home/dracon/Dev/dracon-ai-lib
branch: main
upstream: origin/main
modified: 0
staged: 0
untracked: 0
ahead: 0
behind: 0
push: OK
status: healthy
```

## Archive rationale assessment

Prior archive evidence showed the repo was intentionally archived on 2026-06-08 as part of a redirect to `ai-api-sdk`:

- `gh repo view` before: `isArchived=true`, `archivedAt=2026-06-08T20:06:42Z`
- Archive commit: `4fc7206 archive: mark lib as archived, redirect to ai-api-sdk`
- Follow-up commit: `14397af archive: fix remaining dracon-ai-sdk references to ai-api-sdk`
- `CONSUMERS.md` and `CHANGELOG.md` warned: archived/frozen at `v0.2.0`

However, no hard blocker was found that requires the repo to remain archived:

- The library still has a distinct direct-BYOK Rust client role.
- The workspace is healthy locally.
- The user explicitly approved unarchiving if there was no good reason to stay archived.
- The archived notices were stale relative to the active direct-BYOK library contract.

Decision: unarchive and keep `dracon-ai-lib` active for direct BYOK Rust consumers, while preserving the guidance that `ai-api-sdk` is the right path for shared gateway/multi-consumer deployments.

## External GitHub state change

Command attempted:

```sh
gh api -X PATCH repos/DraconDev/dracon-ai-lib \
  -f archived=false \
  -f description='Standalone Rust workspace for an importable BYOK AI client library.'
```

Before:

```json
{"archived":true,"default_branch":"main","full_name":"DraconDev/dracon-ai-lib","visibility":"private"}
```

After:

```json
{"archived":false,"default_branch":"main","description":"Standalone Rust workspace for an importable BYOK AI client library.","full_name":"DraconDev/dracon-ai-lib","visibility":"private"}
```

`gh repo view` after:

```json
{
  "archivedAt": null,
  "createdAt": "2026-05-31T20:31:51Z",
  "defaultBranchRef": {"name": "main"},
  "description": "Standalone Rust workspace for an importable BYOK AI client library.",
  "isArchived": false,
  "updatedAt": "2026-06-11T13:11:47Z",
  "url": "https://github.com/DraconDev/dracon-ai-lib",
  "visibility": "PRIVATE"
}
```

## Docs updated

Removed stale archived/frozen notices from:

- `README.md`
- `CONSUMERS.md`
- `CHANGELOG.md`

Preserved the substantive guidance:

- `dracon-ai-lib` is active for direct BYOK Rust consumers.
- `ai-api-sdk` remains the recommended path for shared gateway, multi-consumer, quota, BYOK-upload, or cross-language deployments.

Verification:

```text
grep -RIn "archived|ARCHIVED|frozen at v0.2.0|repo is archived" README.md docs crates CONSUMERS.md CHANGELOG.md
docs/archive/legacy-key-management-design.md:3:**Status:** historical draft, archived 2026-06-10. Do not use as guidance for
```

The remaining match is a historical draft under `docs/archive/`, not a stale repo-level archived notice.

## Sync and push

Sync committed the docs update:

```text
📝 committed 3 file(s) in /home/dracon/Dev/dracon-ai-lib
```

Push:

```text
git push origin main
Everything up-to-date
```

Post-push dry-run:

```text
git push --dry-run origin main
Everything up-to-date
```

## Validation

After unarchive/docs update:

```text
cargo fmt --all --check
fmt_exit=0

cargo test --manifest-path dracon-ai-lib/Cargo.toml -- --test-threads=1
parsed validation tests: passed=181 failed=0 ignored=0

cargo clippy --manifest-path dracon-ai-lib/Cargo.toml --workspace -- -D warnings
Finished `dev` profile [unoptimized + debuginfo]
```

## Final inventory evidence

`dracon-sync repos --json --full-path` row:

```text
repo	branch	modified	staged	untracked	ahead	behind	state_flags	push_status	hint
/home/dracon/Dev/dracon-ai-lib	main	0	0	0	0	0	OK	OK	healthy
```

Final Git evidence:

```text
* main 32ccd9f [origin/main] 3 file(s) [README.md, CHANGELOG.md, CONSUMERS.md] DELTA:+3/-4
rev-list --count main ^origin/main = 0
rev-list --count origin/main ^main = 0
```

## Remaining blockers

None for the requested unarchive/push-health goal.

The repo remains private, because the task did not request visibility change. No secrets were exposed, rotated, rewritten, or pushed. No `.pi/` paths were changed.

## Evidence inventory

All evidence is stored under:

`/home/dracon/Dev/dracon-utilities/docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/`

Key files:

- `inventory-before.tsv`
- `inventory-after.tsv`
- `rationale-evidence.txt`
- `current-state.txt`
- `unarchive-api-attempt.log`
- `validation-after-unarchive.log`
- `sync-now.log`
- `git-push.log`
- `post-verification.txt`
