# Sync Visibility Feature Plan

## Objective

Implement a `sync_visibility` feature in `dracon-sync` that automatically mirrors the publicity (public/private) status of the origin (GitHub) repository to configured mirror remotes (Codeberg, GitLab). When a GitHub repo is public, its mirrors on Codeberg and GitLab should also be public. When private, mirrors remain private.

**Expected outcome:** Zero-configuration mirror visibility alignment — when a user changes a repo from private to public on GitHub, the next `dracon-sync` cycle propagates that visibility to all configured mirrors.

---

## Implementation Plan

### Phase 1: Policy & Configuration Layer

- [ ] **Add `SyncVisibility` enum to `policy.rs`**
  - Define `enum SyncVisibility { Private, Public, Internal }` with serde support (`rename_all = "lowercase"`)
  - Default to `Private` for backward compatibility and safety-first principle
  - `Internal` variant only valid for GitHub Enterprise and self-managed GitLab

- [ ] **Add top-level `sync_visibility` to `SyncPolicy`**
  - Field: `sync_visibility: SyncVisibility` (default `Private`)
  - Controls default visibility for `auto_github_private` origin bootstrap
  - Acts as fallback for remotes that don't specify their own `sync_visibility`

- [ ] **Add per-remote `sync_visibility` to `RemoteConfig`**
  - Field: `sync_visibility: SyncVisibility` (default uses top-level `SyncPolicy` value)
  - Only relevant when `auto_create = true` (no effect on existing repos)
  - Allows fine-grained control: e.g., public GitHub, private GitLab, private Codeberg

- [ ] **Add `sync_visibility_mirrors` boolean to `SyncPolicy`**
  - Field: `sync_visibility_mirrors: bool` (default `false`)
  - Master toggle: when `false`, no visibility API calls are made (backward compatible)
  - When `true`, sync checks origin visibility and propagates to mirrors on every push

- [ ] **Update `dracon-sync.example.toml`**
  - Document `sync_visibility = "private"` at top level
  - Document `sync_visibility = "public"` per-remote override
  - Document `sync_visibility_mirrors = false` toggle
  - Add comments explaining safety-first default and opt-in nature

- [ ] **Update `validate_config` in `policy.rs`**
  - Warn if `sync_visibility = "internal"` is used with `Codeberg` auth type
  - Warn if `sync_visibility` is set on a remote with `auto_create = false` (unused setting)
  - Validate that `sync_visibility_mirrors = true` requires at least one mirror remote with `auto_create = true` or an API token configured

---

### Phase 2: API Abstraction Layer

- [ ] **Create `src/visibility.rs` module in `dracon-sync`**
  - Define `RepoVisibility` struct: `{ platform: Platform, owner: String, repo: String, is_private: bool }`
  - Define `VisibilityClient` trait with `async fn get_visibility() -> Result<bool>` and `async fn set_visibility(is_private: bool) -> Result<()>`

- [ ] **Implement `GitHubVisibilityClient`**
  - `GET /repos/{owner}/{repo}` authenticated with `GITHUB_TOKEN` (from env or secrets)
  - Extract `private` boolean from JSON response
  - Handle 404 (repo not found), 401/403 (insufficient token scope), rate limits
  - Cache result per-sync-cycle to avoid redundant API calls

- [ ] **Implement `GitLabVisibilityClient`**
  - `GET /api/v4/projects/{url_encoded_path}` authenticated with `GITLAB_TOKEN`
  - Extract `visibility` string (`"private"`, `"public"`, `"internal"`)
  - `PUT /api/v4/projects/{url_encoded_path}` with `{"visibility": "private"|"public"}` to update
  - Handle GitLab.com vs self-managed instances (use configured API endpoint)

- [ ] **Implement `CodebergVisibilityClient`**
  - `GET /api/v1/repos/{owner}/{repo}` authenticated with `CODEBERG_TOKEN`
  - Extract `private` boolean from JSON response
  - `PATCH /api/v1/repos/{owner}/{repo}` with `{"private": true|false}` to update
  - Use configured `api_endpoint` from remote config (defaults to `https://codeberg.org/api/v1`)

- [ ] **Add token resolution for visibility APIs**
  - Reuse existing `load_secret` infrastructure from `secrets.rs`
  - GitHub: `GITHUB_TOKEN` env var or `~/.dracon/utilities/sync/secrets/github.env`
  - GitLab: `GITLAB_TOKEN` env var or `~/.dracon/utilities/sync/secrets/gitlab.env`
  - Codeberg: `CODEBERG_TOKEN` env var or `~/.dracon/utilities/sync/secrets/codeberg.env`
  - Verify token permissions are sufficient before making API calls (graceful degradation)

- [ ] **Add rate-limit-aware retry logic**
  - Check `X-RateLimit-Remaining` / `Retry-After` headers
  - Skip visibility sync if rate-limited (log warning, don't fail the push)
  - Maximum 1 API call per remote per sync cycle (check + set combined into single conditional update)

---

### Phase 3: Integration into Push Pipeline

- [ ] **Add visibility sync step to `push_mirror_remotes` in `git.rs`**
  - After `auto_create_all_remotes` completes, call `sync_visibility_to_remotes` if `sync_visibility_mirrors = true`
  - Skip visibility sync if `auto_create` is disabled and the repo doesn't exist on the remote (no target to update)

- [ ] **Implement `sync_visibility_to_remotes` function**
  - Query GitHub origin visibility using `GitHubVisibilityClient`
  - For each mirror remote with `sync_visibility_mirrors` enabled:
    - Check current visibility using platform-specific client
    - Only call `set_visibility` if current visibility differs from origin (idempotent)
    - Log at INFO level when visibility changes, DEBUG when already matching
  - Handle errors gracefully: visibility sync failure does NOT fail the git push

- [ ] **Update `create_repo_on_github` to respect `SyncVisibility`**
  - Replace hardcoded `--private` with `--{visibility}` based on `RemoteConfig.sync_visibility` or top-level default
  - `gh repo create` supports `--private`, `--public`, `--internal`

- [ ] **Update `create_repo_on_gitlab` to respect `SyncVisibility`**
  - Replace hardcoded `--private` with `--{visibility}` flag
  - `glab repo create` supports `--private`, `--public`, `--internal`

- [ ] **Update `create_repo_on_codeberg` to respect `SyncVisibility`**
  - Replace hardcoded `"private": true` with `"private": visibility.is_private()` in JSON payload
  - Forgejo only supports `private: true/false` (no internal level)

- [ ] **Update `create_github_private_remote` in `report.rs`**
  - Accept `SyncVisibility` parameter instead of hardcoded `--private`
  - This is the origin bootstrap path; it should use the top-level `sync_visibility` default

---

### Phase 4: Testing & Verification

- [ ] **Add unit tests for `SyncVisibility` serde deserialization**
  - Test `"private"` → `Private`, `"public"` → `Public`, `"internal"` → `Internal`
  - Test unknown values default to `Private` with logged warning
  - Test default when field is absent from TOML

- [ ] **Add mock API tests for visibility clients**
  - Use `wiremock` or `tokio::net::TcpListener` to simulate GitHub/GitLab/Codeberg API responses
  - Test successful get/set visibility flows
  - Test 404, 401, 403 error handling (non-fatal)
  - Test rate-limit header handling

- [ ] **Add integration test for visibility sync in push pipeline**
  - Create temp repo with GitHub origin and mock mirror remote
  - Configure `sync_visibility_mirrors = true`
  - Verify visibility query API is called for origin
  - Verify visibility update API is called for mirror when mismatch detected
  - Verify no API call when visibility already matches (idempotency)

- [ ] **Add test for backward compatibility**
  - Config without `sync_visibility` fields → behaves exactly like before (all private)
  - Config with `sync_visibility_mirrors = false` → no API calls made

- [ ] **Add test for error resilience**
  - GitHub API returns 401 → visibility sync skipped, git push still proceeds
  - GitLab API returns 404 → visibility sync skipped for that remote only
  - Rate limit exceeded → visibility sync skipped with warning log

- [ ] **Verify existing test suite still passes**
  - `cargo test -p dracon-sync -- --test-threads=1`
  - All existing 360 tests must pass; 0 regressions

---

### Phase 5: Documentation & Release

- [ ] **Update README.md**
  - Add visibility sync to the feature list
  - Document configuration options with examples
  - Explain safety-first default (opt-in, starts private)

- [ ] **Update CHANGELOG.md**
  - Add entry for `sync_visibility` and `sync_visibility_mirrors`
  - Note backward compatibility (default `private`, feature disabled by default)

- [ ] **Add security considerations doc**
  - Explain that visibility sync requires API tokens with repo administration scope
  - Document token storage recommendations (use `~/.dracon/utilities/sync/secrets/`)
  - Warn that making a repo public is irreversible for sensitive data (history remains public)

---

## Verification Criteria

- [ ] Configuration parses correctly: `sync_visibility = "public"` in TOML resolves to `SyncVisibility::Public`
- [ ] GitHub origin visibility query returns correct `private` boolean for both public and private repos
- [ ] Mirror visibility updates only when origin and mirror visibility differ (idempotent)
- [ ] Visibility sync failure (API error, rate limit, missing token) does NOT fail the git push
- [ ] Backward compatibility: configs without the new fields behave identically to pre-feature behavior
- [ ] All 360+ existing `dracon-sync` tests pass with zero regressions
- [ ] New tests cover: serde parsing, API client mocks, pipeline integration, error resilience

---

## Potential Risks and Mitigations

1. **Accidental public exposure of private repos**
   - Mitigation: Default is `Private` for all creation. `sync_visibility_mirrors` defaults to `false`. Feature is strictly opt-in.

2. **API token scope requirements**
   - Mitigation: Document required scopes clearly. Gracefully degrade (skip sync) if token lacks admin scope.

3. **Rate limit exhaustion**
   - Mitigation: Single API call per remote per cycle. Check before update (idempotent). Respect `Retry-After` headers.

4. **GitHub → mirror visibility mapping ambiguity**
   - Mitigation: GitHub `private: false` maps to `public` on all mirrors. GitHub `private: true` maps to `private`. No "internal" propagation (Codeberg doesn't support it).

5. **Feature creep in push pipeline**
   - Mitigation: Visibility sync is a separate async step after `auto_create_all_remotes` but before `push_to_all_remotes`. It has its own error isolation (non-fatal).

---

## Alternative Approaches

1. **Webhook-driven instead of polling**
   - Instead of checking GitHub API on every sync cycle, set up a GitHub webhook that triggers when visibility changes.
   - Trade-off: More efficient (no polling), but requires public endpoint and adds infrastructure complexity.
   - Recommendation: Polling is simpler and fits the existing sync daemon model.

2. **CLI-driven instead of API-driven**
   - Use `gh repo view --json isPrivate` and `gh repo edit --visibility` instead of REST API calls.
   - Trade-off: Simpler auth (reuses `gh` CLI auth), but adds subprocess overhead and requires `gh`/`glab` installation.
   - Recommendation: Use REST APIs for GitHub/GitLab to avoid CLI dependency; use Forgejo API for Codeberg since no CLI exists.

3. **Global toggle only (no per-remote override)**
   - Only have `sync_visibility_mirrors` at the top level; all mirrors get the same visibility as origin.
   - Trade-off: Simpler config, but prevents mixed visibility (e.g., public GitHub, private GitLab).
   - Recommendation: Keep per-remote override for flexibility; most users will use the default.

4. **Lazy visibility sync (only on creation)**
   - Only set visibility when `auto_create` creates a new repo; never update existing repos.
   - Trade-off: Safer (no changes to existing repos), but doesn't handle visibility changes over time.
   - Recommendation: Full sync model — visibility changes on GitHub should propagate to mirrors.
