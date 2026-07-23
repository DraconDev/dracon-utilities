# Release Notes — v0.112.28 (2026-07-20)

**Headline**: Operator can now flip repo visibility (`make-public` / `make-private`),
new repos skip codeberg by default to protect the 85 GiB grace quota, and
the `pi-goal-loop-audit` "unowned" warning is fixed by whitelisting the
operator's GitHub-noreply identities.

---

## What's new

### 1. `dracon-sync make-public <repo>` and `make-private <repo>`

New CLI subcommands that flip repo visibility across **github + gitlab**.
Skips codeberg by default to protect the 85 GiB grace quota; pass
`--include-codeberg` to flip it too.

```bash
dracon-sync make-public dracon-sync                # github + gitlab → public
dracon-sync make-public dracon-sync --include-codeberg   # all 3 → public
dracon-sync make-private pi-goal-loop-audit        # back to private
```

What it does:

1. Reads the repo's origin URL (`git config --get remote.origin.url`)
2. Calls the GitHub REST API (`gh api -X PATCH /repos/{owner}/{repo} -f private=…`)
3. Calls the GitLab REST API (`PUT /projects/{owner}%2F{repo}` with `visibility=…`)
4. Optionally calls Codeberg REST API (`PATCH /repos/{owner}/{repo}` with `private=…`)
5. Updates the local visibility cache on success so `repos` reflects the
   new state immediately

#### Latent bug fixed: GitHub auto-create was hardcoded `--private`

While implementing `make-public`, audit found that
`multi_remote.rs:create_repo_on_github` hardcoded `--private` regardless of
the `private` parameter in the signature. This meant the daemon could
never auto-create a public repo. Fixed: now passes `--public` when
`private=false`. Test added: `test_create_repo_on_github_public_flag_when_private_false`.

### 2. New repos skip codeberg by default (quota protection)

`[[remotes]]` for codeberg in the global `dracon-sync.toml` now has
`auto_create = false` (was `true`). New repos will auto-create on
github + gitlab only. To opt a specific repo back IN to codeberg
auto-create, set in the repo's `.dracon/dracon-sync.toml`:

```toml
auto_create_on_codeberg = true
```

This protects the 85 GiB grace quota (currently at 85 GiB used). See
[`docs/design/codeberg-quota-leak-fix-2026-07-13.md`](docs/design/codeberg-quota-leak-fix-2026-07-13.md)
for the full context.

### 3. GitHub noreply identities whitelisted

The `trusted_emails` list in `~/.dracon/utilities/sync/dracon-sync.toml`
now includes `dracon@users.noreply.github.com` and
`DraconDev@users.noreply.github.com` — the GitHub web-UI default identity
(`username@users.noreply.github.com`). Without this, commits authored via
the GitHub web editor or as PR merge commits tripped `untrusted_author`
and the repo showed `🚫 unowned` (e.g. `pi-goal-loop-audit`).

The noreply identities are NOT publicly forgeable — only the GitHub
account holder can produce commits with them. The ownership-substring
bypass (audit F39) is unchanged: this only affects the author-email
whitelist, not the ownership tuple.

---

## Files changed

- `dracon-sync/Cargo.toml` — bumped to `0.112.28`
- `dracon-sync/src/policy.rs` — added `RepoPolicyOverride.auto_create_on_codeberg: Option<bool>`
- `dracon-sync/src/git/multi_remote.rs`:
  - `auto_create_all_remotes` now takes `codeberg_override: Option<bool>`
  - `create_repo_on_github` honors `private` parameter (was hardcoded `--private`)
  - `push_mirror_remotes` threads the override through
- `dracon-sync/src/visibility.rs`:
  - Added `set_github_visibility(owner, repo, private)` using `gh api -X PATCH`
  - Added `flip_repo_visibility(...)` wrapper that calls all three per-remote setters
- `dracon-sync/src/git/mod.rs`:
  - Updated 5 `auto_create_all_remotes` test call sites
  - Updated 3 `create_repo_on_github` test call sites
  - Added `test_create_repo_on_github_public_flag_when_private_false`
  - Added `test_auto_create_all_remotes_codeberg_override_opt_in`
- `dracon-sync/src/report.rs` — 3 `push_mirror_remotes` call sites now pass `repo_override.auto_create_on_codeberg`
- `dracon-sync/src/sync.rs` — 1 `push_mirror_remotes` call site now passes the override
- `dracon-sync/src/main.rs`:
  - New `Command::MakePublic` and `Command::MakePrivate` variants
  - New `handle_visibility_flip` helper (used by both variants)
- `~/.dracon/utilities/sync/dracon-sync.toml`:
  - Added noreply emails to `trusted_emails`
  - Changed codeberg `auto_create = true` → `false` with documentation

## Test discipline

- `cargo test --workspace --locked` ✅ **755 daemon + others, 0 failed**
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean
- Daemon auto-deploy ✅ healthy
