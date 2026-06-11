# Public release branch cleanup

Branch: `public-release-utilities-2026-06-11`

## What changed on this branch

This branch removes tracked local/internal material that should not be part of a public release candidate:

- `.pi/`
- `.ralph/`
- `.sisyphus/`
- `.demon/`
- `.dracon/`
- `debug.log`
- `autoresearch.jsonl`
- `AUDIT.md`
- `docs/audit/2026-06-11-full-repo-audit/`
- `docs/public-readiness.md`
- `docs/public-release-plan.md`
- `docs/public-release-branch/`

It also adds `PUBLIC_RELEASE_NOTES.md` as the public-facing release note for the three utilities.

## Validation

The branch was validated with:

```bash
cargo fmt --check
cargo test --workspace -- --test-threads=1
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check
cargo build --release -p dracon-sync -p dracon-system -p dracon-warden
./scripts/verify-spec.sh
dracon-sync config validate
dracon-sync scaffold --dry-run
./install.sh --dry-run
```

All checks passed. `cargo test` and `./scripts/verify-spec.sh` were run with the local sync freeze marker temporarily moved aside and the sync daemon stopped, then both were restored afterward.

## Push status

`git push --dry-run origin HEAD` reported the branch as new and did not push anything.

No tag, release, publish, visibility change, or actual push was performed.
