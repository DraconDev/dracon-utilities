# Release Target Feature: Auto-Publish to Package Registries

## Objective

Add automated package publishing to dracon-sync so that version bumps automatically trigger publishes to configured registries (crates.io, npm, PyPI). Every minor bump creates a git tag; every major bump creates a GitHub Release. Registry publishes are opt-in per-repo.

## Design Principles

- **Publish-on-version-change**: Only publish when the version actually changes (detect via pre-check against registry API)
- **Tags + Releases**: Minor → git tag `v{version}`; Major → git tag + GitHub Release
- **Per-repo opt-in**: Not every repo should auto-publish; requires explicit `auto_publish` config
- **Dry-run first**: Always run `--dry-run` before real publish, log failures to incident ledger
- **Idempotent**: Re-running on an already-published version is a no-op
- **Non-fatal**: Publish failures don't block git sync

## Configuration

### Top-level policy (`dracon-sync.toml`):

```toml
# Global toggle — must be true for any auto-publishing to occur
auto_publish = false

# Registry targets (each with a token secret)
[[publish_targets]]
name = "crates-io"
registry = "crates-io"         # crates-io | npm | pypi
token_secret = "CARGO_REGISTRIES_CRATES_IO_TOKEN"  # loaded via load_secret()
# cargo publish timeout
publish_timeout_secs = 300

[[publish_targets]]
name = "npm"
registry = "npm"
token_secret = "NPM_TOKEN"
publish_timeout_secs = 120

[[publish_targets]]
name = "pypi"
registry = "pypi"
token_secret = "TWINE_PASSWORD"
publish_timeout_secs = 120
```

### Per-repo override (`.dracon/dracon-sync.toml`):

```toml
# Which publish targets to use for this repo
auto_publish = ["crates-io"]  # or ["crates-io", "npm"], or false/[]
```

## Auth/Token Summary

| Registry | Token | Expiry | Env Var | Command |
|----------|-------|--------|---------|---------|
| **crates.io** | API token (scoped) | **Non-expiring** | `CARGO_REGISTRIES_CRATES_IO_TOKEN` | `cargo publish` |
| **npm** | Automation token | **Non-expiring** | `NPM_TOKEN` | `npm publish` |
| **PyPI** | API token (scoped) | **Non-expiring** | `TWINE_PASSWORD` | `twine upload dist/*` |

npm has granular tokens that expire in 90 days, but **automation tokens** (`npm token create --automation`) never expire and bypass 2FA — these are the right choice for dracon-sync.

## Tag + Release Logic

After a version bump to `{new_version}`:

1. **Any bump (patch/minor/major)**: Create git tag `v{new_version}`, push tag
2. **Major bump only**: Also create a GitHub Release via `gh release create v{new_version} --title "v{new_version}" --notes-from-tag`

Tags are cheap and always created. Releases are heavier and only for majors since they represent breaking changes.

## Publish Pipeline

After commit + push, if version was bumped:

1. **Pre-check**: Query registry API to see if version already exists
   - crates.io: `GET https://crates.io/api/v1/crates/{name}` → check `max_version`
   - npm: `GET https://registry.npmjs.org/{name}/{version}` → 404 = not published
   - PyPI: `GET https://pypi.org/pypi/{name}/json` → check `version` field
2. **Skip if already published** (idempotent — no wasted API calls)
3. **Dry-run**: `cargo publish --dry-run` / `npm publish --dry-run` / `twine upload --skip-existing dist/*`
4. **Real publish**: If dry-run succeeds, run actual publish command
5. **Log**: Success or failure → incident ledger

## Implementation Plan

### Phase 1: Policy + Config (~2 hours)

- [ ] Add `auto_publish: bool` (default `false`) to `SyncPolicy`
- [ ] Add `publish_targets: Vec<PublishTarget>` to `SyncPolicy`
- [ ] Define `PublishTarget` struct: `name`, `registry` (enum: CratesIo/Npm/PyPi), `token_secret`, `publish_timeout_secs`
- [ ] Add per-repo `auto_publish: Vec<String>` to `RepoPolicyOverride` (list of target names)
- [ ] Update `dracon-sync.example.toml` with publish target examples
- [ ] Update `test_sync_policy()` helper with new fields

### Phase 2: Tag + Release Module (~3 hours)

- [ ] Create `dracon-sync/src/release.rs` module
- [ ] `create_version_tag(repo, version)` — creates `v{version}` tag and pushes it
- [ ] `create_github_release(repo, version)` — uses `gh release create` for major bumps
- [ ] Add tag creation to `sync.rs` after version bump (in the `version_bumped = true` path)
- [ ] Only create release for major bumps (requires manual major, so this is always user-initiated)
- [ ] Tests: mock `git tag` + `git push origin v{ver}`, mock `gh release create`

### Phase 3: Registry Publish Module (~4 hours)

- [ ] Extend `release.rs` with publish functions
- [ ] `check_registry_version(target, name, version)` — HTTP GET to registry API
- [ ] `publish_to_crates_io(repo, token, timeout)` — `cargo publish` with env var
- [ ] `publish_to_npm(repo, token, timeout)` — `npm publish` with env var
- [ ] `publish_to_pypi(repo, token, timeout)` — `twine upload` with env var
- [ ] `dry_run_publish(target, repo, token)` — runs with `--dry-run` flag first
- [ ] All publish functions use `load_secret()` for token injection
- [ ] All publish functions log to incident ledger on failure
- [ ] Tests: mock `cargo publish --dry-run`, mock registry API responses, test idempotency

### Phase 4: Pipeline Integration (~2 hours)

- [ ] In `sync.rs`, after successful push where `version_bumped = true`:
  1. Call `create_version_tag(repo, new_version)`
  2. If major bump, call `create_github_release(repo, new_version)`
  3. If `auto_publish` enabled, check each `publish_target`:
     - Pre-check registry API (skip if version exists)
     - Dry-run publish
     - Real publish if dry-run passes
- [ ] Make publish failures non-fatal (log warning, continue)
- [ ] Add `publish_timeout_secs` to `run_child()` calls
- [ ] Tests: integration test with mock publish targets

### Phase 5: CLI Command (~1 hour)

- [ ] Add `dracon-sync publish [repo]` subcommand for manual publish
- [ ] Add `dracon-sync publish --dry-run [repo]` for validation
- [ ] Add `dracon-sync publish-status [repo]` to check published versions across registries

### Phase 6: Validation + Docs (~1 hour)

- [ ] Update `validate_config` to validate publish targets
- [ ] Update README/AGENTS.md with publish feature docs
- [ ] Full test suite run

## Verification Criteria

- [ ] `auto_publish = false` → no publish attempts, no API calls
- [ ] Version bump → git tag created and pushed
- [ ] Major version bump → git tag + GitHub Release
- [ ] Already-published version → skipped (idempotent)
- [ ] `cargo publish --dry-run` failure → no real publish, incident logged
- [ ] `cargo publish` success → logged, no error
- [ ] Publish failure → git sync continues, incident logged
- [ ] Missing token → publish skipped for that target, warning logged
- [ ] Per-repo override correctly gates which targets apply

## Potential Risks and Mitigations

1. **Accidental publish of broken package**
   Mitigation: Dry-run before every real publish; pre-check registry version; opt-in per-repo

2. **crates.io name squatting**
   Mitigation: First publish is always manual (user creates the crate); auto-publish only updates existing crates

3. **npm token expiry (granular tokens)**
   Mitigation: Require automation tokens (non-expiring); document this in config comments; warn on 401 responses

4. **PyPI requires pre-built dist/ directory**
   Mitigation: Only attempt `twine upload` if `dist/` exists; skip otherwise with warning

5. **Tag collision (tag already exists on remote)**
   Mitigation: Check `git tag -l v{version}` before creating; skip if exists

6. **Publish race condition (two sync cycles publishing simultaneously)**
   Mitigation: Registry APIs reject duplicate versions; idempotent by design

## Alternative Approaches

1. **Publish only on explicit command (no auto)**: Remove `auto_publish`, require `dracon-sync publish` every time. Simpler but defeats the "invisible infrastructure" philosophy.

2. **Publish on tag push only**: Instead of detecting version bumps in `sync_repo`, use a git hook or GitHub Action that publishes when a `v*` tag is pushed. This is the standard industry pattern but requires CI infrastructure.

3. **Cargo credential provider**: Use `cargo:token-from-stdout` credential provider that invokes a dracon-sync helper binary. More complex but tighter security (token never in env). Overkill for single-user setup.
