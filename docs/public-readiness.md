# Public Readiness Assessment

Generated: 2026-06-10

## Verdict

`dracon-utilities` is **not safe to publish as-is**. It is a plausible public candidate after a dedicated public-release cleanup branch, but the current tree and reachable history still contain local agent/task state, audit artifacts, operational logs, and secret-shaped fixture strings that should not be exposed without review.

Explicit public-release steps: [docs/public-release-plan.md](docs/public-release-plan.md).

## Current-tree evidence

Evidence collected in:

- `../.local/state/dracon/dracon-utilities-public-readiness-evidence/git-metadata.txt`
- `../.local/state/dracon/dracon-utilities-public-readiness-evidence/working-risk-paths.tsv`
- `../.local/state/dracon/dracon-utilities-public-readiness-evidence/history-risk-paths.tsv`
- `../.local/state/dracon/dracon-utilities-public-readiness-evidence/secret-shaped-content.tsv`
- `../.local/state/dracon/dracon-utilities-public-readiness-evidence/env-classification.tsv`

Summary:

| Area | Evidence | Public-readiness impact |
|---|---:|---|
| Git state | `git status --porcelain=v2 --untracked-files=all` is clean on `main` | Good |
| Remotes | GitHub, GitLab, Codeberg, and origin configured | Good for mirrors; do not change visibility during audit |
| Tracked `.env*` | 0 files | Good |
| Working-tree high-risk paths | 63 paths | Blocker until removed/rewritten or explicitly approved |
| Reachable-history high-risk paths | 131 paths | Blocker for public release unless history is rewritten and verified |
| Secret-shaped content matches | 25 redacted matches, all in README/tests/scanner fixtures | Needs public-scan review or fixture hardening |
| Warden/key state | `dracon-warden status` resolves policy and pubkey source | Good for local encryption behavior |

The working-tree high-risk paths are dominated by local state:

- `.pi/goals/...`
- `.ralph/...`
- `.sisyphus/...`
- `.demon/data/keys/owner_age1wz5p.pub`
- `debug.log`
- `autoresearch.jsonl`
- audit notes under `.dracon/`, `docs/audit/`, and root audit files

These are not secrets by themselves, but they expose internal workflow, operational metrics, local paths, and agent task history. They should not be published without explicit approval and cleanup.

## History evidence

Reachable history contains the same local-state families plus older audit/task artifacts. Because public publishing exposes history by default, this repo is **not public-ready until a public-release branch removes or rewrites those paths and verifies the rewritten history**.

Do not rewrite history without backing up and rotating any real secrets that may be found.

## Secret-shaped content

The scan found secret-shaped strings in test/fixture/documentation contexts, including example GitHub/GitLab/Slack/AWS/age strings. No raw secret values are included in this report. Public scanners may still flag these unless they are clearly marked as fixtures or removed.

## CI and validation evidence

The CI workflow is build/test/lint oriented and does not publish artifacts by default. It uses GitHub Actions secrets only for CLA handling and does not run crates.io/GitHub Release publishing from `ci.yml`.

Validation evidence is stored under:

`../.local/state/dracon/dracon-utilities-public-readiness-evidence/validation/`

Passing commands:

- `cargo fmt -p dracon-sync -p dracon-system -p dracon-warden -- --check`
- `cargo clippy -p dracon-sync -p dracon-system -p dracon-warden --all-targets --no-deps`
- `cargo build -p dracon-sync -p dracon-system -p dracon-warden`
- `cargo test --workspace -- --test-threads=1`
- `cargo tree -d`
- `cargo deny check`
- `./scripts/verify-spec.sh`
- `cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1`

Notes:

- `cargo fmt --workspace --check` is not a valid `cargo fmt` invocation on this toolchain.
- `cargo fmt --all --check` is not a suitable repo-equivalent here because the sibling `dracon-libs` checkout currently has a missing module path that rustfmt tries to resolve; CI uses the package-specific command above.

## Warden evidence

Evidence collected in:

`../.local/state/dracon/dracon-utilities-public-readiness-evidence/warden/warden-evidence.log`

Relevant results:

- `dracon-warden status` resolves the policy and pubkey source successfully.
- `key-inventory.sh` reports the expected mesh and standalone key layout.
- `verify-master-recipient.sh` passed and found no private master key in history.
- `master-key-recovery-preflight.sh` refuses generation/import without `DRACON_ALLOW_MASTER_REGEN=1`.
- With `DRACON_ALLOW_MASTER_REGEN=1`, the preflight reports unrelated missing off-box files in `dracon-platform`; this does not change the current `dracon-utilities` readiness verdict.

## Required public-release cleanup

Before making this repo public:

1. Create a public-release branch.
2. Remove or rewrite local state and audit artifacts from current tree and reachable history:
   - `.pi/`
   - `.ralph/`
   - `.sisyphus/`
   - `.demon/`
   - `debug.log`
   - `autoresearch.jsonl`
   - internal audit/task notes
3. Review secret-shaped fixture strings and either remove them or make them unmistakably synthetic.
4. Add a public `SECURITY.md` or replace internal-only guidance with public-safe documentation.
5. Review `AGENTS.md`; it is useful internally but should not be published unchanged unless you are comfortable exposing agent workflow details.
6. Re-run the scans in this report and the validation commands above.
7. Only then change visibility or push public mirrors.

## Recommendation

Do **not** publish `dracon-utilities` as-is. It is a good candidate for a future public release, but only after a deliberate public-release branch removes internal state/history and passes fresh scans.
