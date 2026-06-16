# Crates.io Publish Process — 2026-06-16

> **Goal**: `0ca7e640` (operator: "release on crates too")
>
> **Status**: **PUBLISHED** — `dracon-sync v0.1.9`, `dracon-system v0.2.4`,
> `dracon-warden v0.3.4` are live on crates.io.

This design doc captures the crates.io publish workflow, who owns the
account, what's the release process, how to verify a published version, and
the lessons learned from the first publish (2026-06-16).

## crates.io account

- **Owner**: `DraconDev` (operator's account)
- **Token storage**: `~/.cargo/credentials.toml` (mode 0600)
- **Token env var (alternative)**: `$CARGO_REGISTRY_TOKEN` (only for CI;
  local use prefers the file-based credential)
- **Token scope**: full publish + yank

The token is **NEVER** committed, logged, or extracted to a file in any
repo. The `~/.cargo/credentials.toml` is in the operator's home directory
and is not part of the monorepo's `.gitignore` (it's outside the monorepo).

## Published crates (as of 2026-06-16)

| Crate | Version | crates.io page |
|-------|---------|----------------|
| `dracon-sync` | 0.1.9 | https://crates.io/crates/dracon-sync |
| `dracon-system` | 0.2.4 | https://crates.io/crates/dracon-system |
| `dracon-warden` | 0.3.4 | https://crates.io/crates/dracon-warden |

The pre-existing crates (`dracon-sync 0.1.5`, `dracon-system 0.2.0`,
`dracon-warden 0.3.0`) were published in earlier releases (May 2026). The
2026-06-16 publish caught up to the v0.112.9 workspace version.

## Release process (operator runbook)

The crates.io publish is a **separate step** from the GitHub release. It
is done after the GitHub release is cut, with the version bumps already in
place.

### Step 1: Verify the version bumps are in place

```bash
cd /home/dracon/Dev/dracon-utilities
grep '^version' Cargo.toml                      # root: 0.112.9
grep '^version' dracon-sync/Cargo.toml         # 0.1.10 (or next)
grep '^version' dracon-system/Cargo.toml       # 0.2.5 (or next)
grep '^version' dracon-warden/Cargo.toml       # 0.3.5 (or next)
```

### Step 2: Verify each crate's `Cargo.toml` has the required metadata

Required fields for crates.io:
- `name`
- `version`
- `description` (1-2 sentences, will be the search snippet)
- `license` (SPDX identifier, e.g., `AGPL-3.0-only`)
- `repository` (URL)
- `readme` (path to README.md, relative to Cargo.toml)
- `keywords` (max 5, comma-separated)
- `categories` (from https://crates.io/category_slugs)

Optional but recommended:
- `homepage`
- `documentation` (crates.io auto-generates the docs.rs URL, so this is
  just a hint; safest to point to the crate's docs.rs landing page without
  a version, e.g., `https://docs.rs/dracon-sync`)

### Step 3: Verify the crate builds standalone

```bash
cd /home/dracon/Dev/dracon-utilities/<crate>
cargo build --release --locked
```

The standalone build must succeed without the monorepo workspace. Note
that `<crate>` is a workspace member, so `cargo build` from inside the
subdir will still work; what we need to verify is that the published
artifact (built from the packaged Cargo.toml) compiles cleanly.

### Step 4: Run `cargo publish --dry-run --allow-dirty`

```bash
cd /home/dracon/Dev/dracon-utilities/<crate>
cargo publish --dry-run --allow-dirty
```

This packages the crate, runs all tests, and uploads the package metadata
to crates.io (without actually publishing). The `--allow-dirty` is needed
because the working tree has uncommitted local changes from the
`scripts/regenerate_facade_repos.py` mirror.

Look for these in the output:
- `Packaging <crate> v<X.Y.Z>` — confirms the version
- `Verifying <crate> v<X.Y.Z>` — runs the test suite
- `Finished <profile> target(s)` — compilation succeeded
- `Uploading <crate> v<X.Y.Z>` — the upload (skipped in dry-run)
- `warning: aborting upload due to dry run` — expected

### Step 5: Publish for real

```bash
cd /home/dracon/Dev/dracon-utilities/<crate>
cargo publish --allow-dirty
```

cargo reads the token from `~/.cargo/credentials.toml` automatically.
The `--allow-dirty` is needed for the same reason as the dry-run.

Look for:
- `Uploading <crate> v<X.Y.Z>`
- `Published <crate> v<X.Y.Z> at registry 'crates-io'`

### Step 6: Verify with `cargo search`

```bash
cargo search dracon-sync    # Expected: shows v0.1.9
cargo search dracon-system  # Expected: shows v0.2.4
cargo search dracon-warden  # Expected: shows v0.3.4
```

### Step 7: Verify with `cargo install` smoke test

```bash
mkdir -p /tmp/crates-io-smoke
cd /tmp/crates-io-smoke
cargo install dracon-sync --root /tmp/crates-io-smoke/install
/tmp/crates-io-smoke/install/bin/dracon-sync --version
# Expected: prints "dracon-sync <version>"
```

Repeat for `dracon-system` and `dracon-warden`.

### Step 8: Verify the docs.rs page is generated

crates.io auto-triggers a docs.rs build on publish. After a few minutes:

- https://docs.rs/dracon-sync/<version>
- https://docs.rs/dracon-system/<version>
- https://docs.rs/dracon-warden/<version>

Each should show the rendered README + API docs.

## Order of publishes (when publishing 3 crates)

The order matters for the dependency graph:

1. **`dracon-sync`** (no internal deps, only external `dracon-git`) — publish first
2. **`dracon-system`** (no internal deps, only external `dracon-system-lib`) — publish second
3. **`dracon-warden`** (depends on `dracon-security v0.3.0` from crates.io) — publish last

The `dracon-warden` crate has a `path = "dracon-warden/src/security"` dep
in the root `Cargo.toml`, but cargo auto-rewrites this to
`version = "0.3.0"` when packaging for publish (because the path dep
also has `version = "0.3.0"` specified). The packaged
`target/package/dracon-warden-<version>/Cargo.toml` shows the rewritten
version dep.

## Lessons learned (from 2026-06-16 first publish)

### 1. Crates.io has a 5-keyword limit

The first publish attempt failed with:
> `error: failed to publish ... expected at most 5 keywords per crate`

**Fix**: Reduced each crate's `keywords` array from 10 to 5. The 5 most
relevant keywords per crate are:

- `dracon-sync`: `git`, `sync`, `daemon`, `auto-commit`, `multi-remote`
- `dracon-system`: `disk`, `process`, `service`, `diagnostics`, `doctor`
- `dracon-warden`: `git`, `secret`, `encrypt`, `age`, `filter`

### 2. Categories must be valid slugs

The first attempt used categories like `development-tools::build-utils`
(which is valid) and `cryptography` (which is valid). But for simplicity
and discoverability, all 3 crates use `["command-line-utilities"]` which
is the most relevant single category.

The full list of valid slugs is at https://crates.io/category_slugs.

### 3. `documentation` field is overridden by docs.rs

The `documentation` field in `Cargo.toml` is a hint to crates.io. The
actual docs.rs URL is auto-generated from the crate name + version. The
operator-set `documentation` field is shown on the crates.io page but
docs.rs uses its own URL. So it's safest to point `documentation` to the
crate's docs.rs landing page (without a version), e.g.,
`https://docs.rs/dracon-sync` — the visitor will be redirected to the
latest version.

### 4. `--allow-dirty` is needed

The monorepo's working tree has uncommitted local changes from the
`regenerate_facade_repos.py` mirror (which writes to the façade repos at
`/home/dracon/Dev/facade-repos/`, not to the monorepo itself, so the
monorepo is usually clean — but the operator's local dev environment may
have other uncommitted changes from the daemon's auto-commits).

`cargo publish` fails by default on a dirty working tree. `--allow-dirty`
opts in to publish despite the dirty tree. This is safe because the
publish packages the crate based on `Cargo.toml` + the source files, not
on uncommitted changes (unless those changes affect the published files).

### 5. The path dep is auto-rewritten

The `path = "dracon-warden/src/security"` dep in the root `Cargo.toml` is
**NOT** a blocker for `cargo publish`. Cargo detects the path dep + the
specified version, and when packaging, rewrites the path dep to a version
dep. The published `dracon-warden` crate depends on
`dracon-security v0.3.0` from crates.io, not on a local path.

## Future maintenance

### Yanking a broken version

If a published version is broken (security issue, regression, etc.), the
operator can yank it:

```bash
cargo yank --version <X.Y.Z> <crate>
```

Yanking prevents new `cargo install`s but does not break existing
installs. After yanking, publish a new patch version with the fix.

### Updating the published metadata

To change the description, keywords, categories, or other metadata,
publish a new patch version. crates.io does not allow editing metadata of
an already-published version (only yanking).

### Deprecating a crate

To deprecate a crate, publish a new version with `rust-version` and
`--message` documenting the deprecation, then yank all old versions.
crates.io will show a deprecation banner.

## Related docs

- `docs/design/github-feature-repos.md` — the 4-repo architecture
- `docs/design/push-targets-audit-2026-06-16.md` — the push targets audit
- `docs/design/final-audit-2026-06-16.md` — the final audit (this doc is referenced from there)
- `release-notes-v0.112.9.md` — release notes for the v0.112.9 release
  (which packages the crates.io publish)
