# Release Notes — v0.112.29 (2026-07-21)

**Headline**: Fixes three issues found during the `convos` empty-repo
investigation:

1. **Empty local repos are now auto-created on github + gitlab** before
   the daemon checks `is_repo_ready`. Previously, a fresh `git init`
   repo with no commits would silently skip auto-create forever, leaving
   the operator staring at "❌ CONCERN · run repair-concerns --apply
   (set upstream)" until they made their first commit — at which point
   the daemon would finally try to push and fail because the
   corresponding forge-side repo didn't exist.

2. **Empty repos show an accurate status** (`EMPTY`, hint "no commits
   yet — make first commit to enable push") instead of the misleading
   "push: fail" label. No push was attempted, so "FAIL" was a false
   positive that pushed the operator toward `repair-concerns --apply`,
   which would fail with "src refspec HEAD does not match any" on an
   empty repo.

3. **`make-public`/`make-private` for GitLab is fixed.** The
   `GITLAB_API_PROJECTS` template constant had two `{}` placeholders,
   but `str::replace("{}", &encoded)` replaces ALL occurrences — both
   `{}` slots got `DraconDev%2Fconvos` substituted in, producing
   `projects/DraconDev%2Fconvos%2FDraconDev%2Fconvos`. GitLab returned
   404 for every visibility flip with "repo not found" even though the
   repo existed and the token was valid. Pre-existing bug since the
   visibility code was added; masked because the existing
   `sync_mirror_visibility` flow calls `set_gitlab_visibility` only on
   repos whose visibility actually needs changing (rare), and the
   `make-public` CLI was new in v0.112.28.

---

## What's new

### 1. Empty-repo auto-create on discovery

In `daemon.rs`, the per-repo sync loop now calls
`push_mirror_remotes_create_only` (a new `multi_remote.rs` helper that
runs `auto_create_all_remotes` without the push step) BEFORE the
`is_repo_ready` check. This ensures:

- Brand-new `git init` repos get their github + gitlab counterparts
  auto-created immediately when the daemon discovers them.
- Per-remote `auto_create` is honored (codeberg skipped by default
  per v0.112.28's quota posture; per-repo opt-in via
  `auto_create_on_codeberg = true`).
- Idempotent: `remote_repo_exists` (via `git ls-remote`) is checked
  before any `gh repo create` / `glab repo create`, so already-existing
  repos are skipped without making the API call.

The empty-repo path is **inert** once the operator makes their first
commit — the regular `is_repo_ready → push_mirror_remotes` flow takes
over.

### 2. Empty-repo status hint

Added `EMPTY_REPO` flag in `repo_state_flags_with_push_failure` (fired
when `status.last_commit_hash.is_none()`). The hint for this flag is:

> no commits yet — make first commit to enable push

And `push_status` derivation now produces `EMPTY` (not `FAIL`) for
empty repos — no push was attempted, so `FAIL` was a false positive.

### 3. `set_gitlab_visibility` URL fix

`const GITLAB_API_PROJECTS` changed from
`https://gitlab.com/api/v4/projects/{}%2F{}` (TWO placeholders, buggy)
to `https://gitlab.com/api/v4/projects/{}` (ONE placeholder, correct).
The `encoded = "{owner}%2F{repo}"` substitution now lands in the single
slot, producing the correct URL.

Same bug shape was present in `set_codeberg_visibility`'s template
`codeberg.org/api/v1/repos/{}/{}` — but that template uses
`format!("{}/{}", owner, repo)` (NO `%2F` in encoded) and TWO
placeholders, so the duplication is harmless (`DraconDev/convos` is
just substituted twice). GitLab was unique because the URL path
REQUIRES `%2F` between owner/repo, which the encoded string carried.

### 4. Latent bug fixed: `create_repo_on_github` no longer hardcodes `--private`

(This is the bug from v0.112.28; already released but worth noting.)

`multi_remote.rs:create_repo_on_github(account, repo_name, private)`
now honors the `private` parameter. Previously `--private` was
hardcoded, making public auto-create impossible.

---

## Files changed

- `dracon-sync/Cargo.toml` — bumped to `0.112.29`
- `dracon-sync/src/daemon.rs` — added pre-`is_repo_ready` auto-create
  call (with extensive comment explaining the empty-repo case)
- `dracon-sync/src/git/multi_remote.rs` — added
  `push_mirror_remotes_create_only` helper
- `dracon-sync/src/report.rs`:
  - Added `EMPTY_REPO` flag in `repo_state_flags_with_push_failure`
  - Updated `repo_hint` to return the empty-repo hint BEFORE the
    `NO_UPSTREAM` branch
  - Updated push_status derivation to return `EMPTY` for empty repos
  - Updated `make_status` test helper to default
    `last_commit_hash = Some("deadbeef")` (existing tests scenario is
    "repo with commits")
  - Updated `test_repo_is_warn_untracked_only_is_not_warn` to set a
    commit hash (not an empty-repo scenario)
  - Added `test_repo_state_flags_empty_repo` and
    `test_empty_repo_push_status_is_empty_not_fail`
- `dracon-sync/src/visibility.rs`:
  - Fixed `GITLAB_API_PROJECTS` template (one `{}` placeholder)
  - Added `test_gitlab_api_url_construction` regression test

## Test discipline

- `cargo test --workspace --locked` ✅ **758 daemon + others, 0 failed**
  (+3 new: 2 empty-repo tests, 1 gitlab URL construction test)
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean

## Live verification

- `convos` (the operator's fresh `git init` repo) was created on
  github + gitlab automatically by the daemon's new code path
- `dracon-sync make-public convos` → both github AND gitlab flipped
  to public (gitlab URL bug fixed)
- `repos -s` for convos: `❌ CONCERN · 4 ut · no commits yet — make
  first commit to enable push` (accurate hint, no misleading "push:
  fail")