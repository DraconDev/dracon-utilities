# Post-FUNDING.yml audit — final report

Date: 2026-06-11  
Goal: Add `FUNDING.yml` to the Dracon-managed default-files list, then audit the result.

## Outcome

- `FUNDING.yml` is now part of the Dracon-managed default files list, alongside `LICENSE`.
- The behavior is consistent with the existing `standard_files` flow:
  - Short-form and long-form entries parse via the same `deserialize_standard_files` path.
  - Per-repo opt-out is `skip_standard_files = ["FUNDING.yml"]` (or `[".github/FUNDING.yml"]` for the long-form target).
  - Template path resolution, AGPL LICENSE handling, and overwrite semantics are unchanged.
  - `scaffold` (and the runtime `ensure_standard_files` flow) iterate the same vector, so the new file gets scaffolded wherever the existing flow runs.
- A template was added at `~/.dracon/utilities/sync/templates/FUNDING.yml` with comments explaining GitHub's `.github/` discovery rule, the supported keys, the no-secrets contract, and the `dracon-sync` opt-out.
- The example policy (`dracon-sync/dracon-sync.example.toml`), `AGENTS.md`, and the live policy (`~/.dracon/utilities/sync/dracon-sync.toml`) all document `FUNDING.yml` as a default standard file.
- `dracon-sync scaffold` works with the new file for both single-repo and all-repos invocations.
- Every Dracon-managed repo now has a `.github/FUNDING.yml`. 19 of 20 already had it (committed manually by the operator with `github: [DraconDev]`). 1 (`dracon-ai-lib`) was missing; the standard-files flow scaffolded the empty default template into it. No existing `FUNDING.yml` content was overwritten.

## Implementation

1. **Template** at `~/.dracon/utilities/sync/templates/FUNDING.yml`.
   - Valid GitHub Sponsors YAML (validated with `python3 -c 'import yaml; ...'`).
   - All funding keys are present and empty by default (`github: []`, `custom: []`, etc.).
   - Comment block explains GitHub's `.github/` discovery rule, supported keys, and the no-secrets contract.
2. **Tests** added in `dracon-sync/src/policy.rs` and `dracon-sync/src/standard_files.rs`:
   - `test_standard_files_short_form_funding_yml` — short-form `["LICENSE", "FUNDING.yml"]` resolves correctly.
   - `test_standard_files_funding_yml_github_subdir_long_form` — long-form `target = ".github/FUNDING.yml"` works.
   - `test_funding_yml_in_dot_github_subdir` — `ensure_standard_files` creates `.github/` parent dir and copies the file.
   - `test_funding_yml_skip_standard_files_optout` — `skip_standard_files = [".github/FUNDING.yml"]` opts out cleanly.
3. **Docs** updated:
   - `dracon-sync/dracon-sync.example.toml` — Section 9 documents `FUNDING.yml`, the `.github/` subdir requirement, and the per-repo opt-out.
   - `AGENTS.md` — Section "Standard Files" explains `FUNDING.yml` placement, the no-secrets rule, and the per-repo opt-out.
   - `dracon-sync/README.md` — `scaffold` example updated.
4. **Live policy** at `~/.dracon/utilities/sync/dracon-sync.toml` — converted to all long-form so both `LICENSE` (repo root) and `FUNDING.yml` (`.github/`) can coexist (TOML allows only one array shape per `standard_files` field, so the long form is required for the subdir target).
5. **Scaffold applied** to the one missing repo:
   - `dracon-sync scaffold --repo /home/dracon/Dev/dracon-ai-lib --files '.github/FUNDING.yml'` → 1 file copied. No other repos touched.
   - Verified with `diff -q` that the installed `FUNDING.yml` is bit-identical to `templates/FUNDING.yml`.

## Final validation matrix

### Rust validation (15 repos)

Evidence: `docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/final-validation.tsv`

| Repo | fmt | test | clippy | Result |
|---|---:|---:|---:|---|
| `dracon-platform` | 0 | 0 | 0 | pass |
| `folder-auto-banner` | 0 | 0 | 0 | pass |
| `ai-auto-repo-rot-scanner-todo-agent` | 0 | 0 | 0 | pass (fmt drift fixed during this audit; verified after fix) |
| `kiki-sassy-desktop-announcer` | 0 | 0 | 0 | pass (with ALSA env) |
| `dracon-code` | 0 | 0 | 0 | pass |
| `avid` | 0 | 0 | 0 | pass |
| `ai-auto-writer` | 0 | 0 | 101 | fmt/test pass; clippy reports pre-existing `unused import: ChatRequest` and 3x `returning the result of a let binding` (unchanged from prior audit) |
| `video-factory` | 0 | 0 | 101 | fmt/test pass; clippy reports pre-existing `from_str` method name collision (unchanged from prior audit) |
| `rust-ai-web-auto` | 0 | 0 | 0 | pass |
| `dracon-utilities` | 0 | 0 | 0 | pass |
| `pully-fully-pull-based-fleet-reconciler` | 0 | 0 | 0 | pass |
| `youtube-video-uploader` | 0 | 0 | 101 | fmt/test pass; clippy reports pre-existing `PLAINTEXT_MAGIC` / `PLAINTEXT_VERSION` dead-code (unchanged from prior audit) |
| `video-uploader` | 0 | 0 | 101 | fmt/test pass; clippy reports pre-existing `if`-collapse / `Error::other` / `format!` issues (unchanged from prior audit) |
| `dracon-ai-lib` | 0 | 0 | 101 | fmt/test pass; clippy reports pre-existing `.filter_map(..)` → `.map(..)` (unchanged from prior audit) |
| `dracon-libs` | 0 | 0 | 101 | fmt/test pass; clippy reports pre-existing `policy` dead-code, derivable `Debug`, missing doc-comments (unchanged from prior audit) |

`dracon-utilities` workspace: `cargo test --workspace -- --test-threads=1` → **701 passed, 9 ignored (22 suites)**.

The 6 clippy 101 entries are pre-existing in their repos and unchanged by the `FUNDING.yml` change. They are not blockers for the goal; they are blockers for a stricter clippy policy and should be tracked separately.

### Non-Rust validation

Evidence: `docs/audit/2026-06-11-full-repo-audit/post-funding/non-rust/non-rust.tsv`

| Repo | Validation |
|---|---|
| `one-mil-girls` | `bun test` and `bun run check` passed |
| `Junk-Runner-bevy/web` | `bun run build` and `bun run check` passed |
| `browser-extensions-shared` | No root package scripts; hygiene blocker (tracked secrets / browser profile data) remains; not in scope for this goal |
| `DraconDev` | No documented local build/test command; docs/profile triage remains; not in scope for this goal |

### Dependency / license

Evidence: `docs/audit/2026-06-11-full-repo-audit/post-funding/deps/deps.tsv`

| Repo | `deny.toml` | `cargo deny check` | `cargo tree -d` |
|---|---|---:|---:|
| `dracon-utilities` | yes | 0 | 0 |
| `dracon-libs` | yes | 0 | 0 |
| `dracon-code` | yes | 0 | 0 |
| `youtube-video-uploader` | yes | 0 | 0 |
| `video-uploader` | yes | 0 | 0 |
| `dracon-platform` | no | n/a | 0 |

### Spec / config

- `scripts/verify-spec.sh` → PASS (3/3 invariants).
- `dracon-sync config validate` → `✅ Policy is valid`.

## Hygiene / public-readiness

Evidence: `docs/audit/2026-06-11-full-repo-audit/post-funding/hygiene.tsv`

| Repo | has `.github/FUNDING.yml` |
|---|---|
| All 20 Dracon-managed repos | yes |

The new template:

- Is public, version-controlled, and contains no secrets (no API keys, tokens, passwords).
- Has the no-secrets contract explicitly stated in the file's comments and in `AGENTS.md`.
- The Warden key-management layer treats `FUNDING.yml` as plain text (it is in the ignore list of any secret-handling filter, since it has no `DRACON_SECRET` markers).

The pre-existing public-readiness blockers from the prior audit remain documented:

- `browser-extensions-shared` — not public-ready (tracked secrets, browser profile data, no root package scripts). User previously objected to publishing this repo.
- `dracon-ai-lib` — local validation passes; push remains blocked (now AHEAD:15 after the FUNDING.yml commit).
- `DraconDev` — needs explicit docs/public triage.
- `one-mil-girls`, `Junk-Runner-bevy` — build/test pass; tracked `.pi/goals` and audit screenshots are present and must not be deleted without approval.

## Remaining blockers (unchanged from prior audit)

1. **`browser-extensions-shared`** — tracked secrets, browser profile data, `.ralph` state, screenshots. Not public-ready without explicit cleanup/rewrite/rotation approval.
2. **`dracon-ai-lib`** — AHEAD:15; push blocked. Needs explicit remote/recreate/rewrite decision.
3. **`ai-auto-writer`, `video-factory`, `youtube-video-uploader`, `video-uploader`, `dracon-ai-lib`, `dracon-libs`** — pre-existing clippy warnings (unchanged by this change). Not blockers for the FUNDING.yml goal; tracked separately.
4. **Tracked local state / notes / screenshots** in several repos — not deleted, rewritten, or untracked without approval.

## Evidence inventory

- Fresh sync inventory: `docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json` / `inventory.tsv`
- Per-repo git metadata: `docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/*.git.txt`
- Rust validation matrix + per-repo logs: `docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/`
- Non-Rust validation matrix: `docs/audit/2026-06-11-full-repo-audit/post-funding/non-rust/non-rust.tsv`
- Dependency / license checks: `docs/audit/2026-06-11-full-repo-audit/post-funding/deps/deps.tsv`
- Hygiene summary: `docs/audit/2026-06-11-full-repo-audit/post-funding/hygiene.tsv`
- Sync policy: `~/.dracon/utilities/sync/dracon-sync.toml`
- Sync template: `~/.dracon/utilities/sync/templates/FUNDING.yml`
- Example policy: `dracon-sync/dracon-sync.example.toml` (Section 9)
- AGENTS.md: "Standard Files" section
- Test code: `dracon-sync/src/policy.rs` and `dracon-sync/src/standard_files.rs`
