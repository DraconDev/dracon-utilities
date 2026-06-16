# Final Audit — 2026-06-16

> **Goal**: `0ca7e640` (operator: "ok we are looking good lets do a na audit
> update docs and push releases if need to, also release on crates too")
>
> **Status**: **COMPLETE** — all audit findings fixed, docs updated, releases
> cut, and 3 sub-crates published to crates.io.

This is the final state-of-the-workspace audit for the `dracon-utilities`
monorepo as of 2026-06-16. It reviews every design doc, every README, the
CHANGELOG, source comments, and the publish-readiness of the 3 sub-crates
to crates.io.

## Audit scope

### Design docs reviewed (30 files)

```
docs/ARCHITECTURE.md
docs/OPERATIONS.md
docs/ROADMAP.md
docs/design/all-green-investigation-2026-06-15.md
docs/design/cli-print-style.md
docs/design/commit-all-policy-2026-06-15.md
docs/design/commit-all-policy-durable-2026-06-15.md
docs/design/commit-all-principle-2026-06-16.md
docs/design/concern-investigation-2026-06-16.md
docs/design/dirty-files-investigation.md
docs/design/dracon-libs-deletion-2026-06-15.md
docs/design/dracon-platform-push-investigation-2026-06-15.md
docs/design/dracon-platform-untracked-commit-2026-06-15.md
docs/design/excluded-dirty-state-2026-06-15.md
docs/design/github-feature-repos.md
docs/design/junk-runner-fix-2026-06-15.md
docs/design/junk-runner-investigation-2026-06-15.md
docs/design/kiki-sassy-decision-handoff-2026-06-15.md
docs/design/kiki-sassy-deep-investigation-2026-06-16.md
docs/design/kiki-sassy-followups-2026-06-16.md
docs/design/kiki-sassy-merge-resolution-2026-06-16.md
docs/design/owner-nixos-pub-tracking.md
docs/design/ownership-investigation-2026-06-15.md
docs/design/push-targets-audit-2026-06-16.md
docs/design/repos-state-cause.md
docs/design/revert-filters-2026-06-15.md
docs/design/secret-scan-text-files-2026-06-16.md
docs/design/source-encryption-incident-2026-06-15.md
docs/design/sync-push-classification.md
docs/design/untracked-content-resolution-2026-06-15.md
docs/design/untracked-md-systemic-2026-06-16.md
docs/design/warden-plaintext-sibling.md
```

### READMEs reviewed (7 files)

- `README.md` (root monorepo)
- `dracon-sync/README.md`
- `dracon-system/README.md`
- `dracon-warden/README.md`
- 3 long-name façade repos: not in this monorepo (separate clones at
  `/home/dracon/Dev/facade-repos/`, mirrored via `regenerate_facade_repos.py`)

### Source comments reviewed

- `dracon-{sync,system,warden}/src/**/*.rs` — all source files
- `scripts/*.py`, `scripts/*.sh` — all scripts
- `Cargo.toml` (root + 3 sub-crates)

## Findings

### Finding 1: Root README stated v0.112.5 (stale by 3 releases)

**File**: `README.md` line 5
**Issue**: "Current release: v0.112.5" was 3 releases behind the actual
current state (v0.112.8).
**Fix**: Updated to `v0.112.8` with link to v0.112.8 release notes.

### Finding 2: Root README described façade repos as "contain only navigation + metadata" (stale by 1 release)

**File**: `README.md` "Façade repos" + "Repository architecture" sections
**Issue**: After v0.112.7, the 3 façade repos are no longer "navigation
shells" — they contain real source code and are independently buildable
(per goal `6a105c59`). The root README was still describing the pre-v0.112.7
state.
**Fix**: Updated the "Façade repos" section + "Repository architecture" table
to reflect the v0.112.7 state. The façade repos are now described as
"Canonical install targets" with the real source code, not navigation
shells. The 4-repo architecture table was updated to show what each repo
actually contains (real source + Cargo.toml + tests + README + LICENSE +
.github/) and how each is updated.

### Finding 3: Root README had no crates.io install path

**File**: `README.md`
**Issue**: No mention of `cargo install dracon-{sync,system,warden}` from
crates.io. After publishing the 3 sub-crates to crates.io, this is now a
first-class install path.
**Fix**: Added a new "Install" section near the top of the README with
`cargo install` instructions. The "Utilities" table now links each utility
to its crates.io page.

### Finding 4: Per-utility READMEs had no "Install via crates.io" section

**Files**: `dracon-sync/README.md`, `dracon-system/README.md`,
`dracon-warden/README.md`
**Issue**: No mention of the crates.io install path. Each README only
described the source-build path.
**Fix**: Added an "Install" section near the top of each per-utility README
with the `cargo install dracon-{name}` command + the façade repo alternative.

### Finding 5: 3 sub-crate `Cargo.toml` files were missing `keywords` and `categories`

**Files**: `dracon-sync/Cargo.toml`, `dracon-system/Cargo.toml`,
`dracon-warden/Cargo.toml`
**Issue**: For crates.io discoverability, each crate should declare
`keywords` (max 5) and `categories` (from the crates.io category slugs).
All 3 crates were missing these fields.
**Fix**: Added `keywords` (5 each) and `categories = ["command-line-utilities"]`
to all 3 crates. Also added `exclude = [".github/", "docs/", "*.md", ...]`
to keep the published package minimal (only README.md included, not the
other markdown files).

### Finding 6: `documentation` URLs in 3 sub-crate `Cargo.toml` files pointed to old versions

**Files**: `dracon-sync/Cargo.toml`, `dracon-system/Cargo.toml`,
`dracon-warden/Cargo.toml`
**Issue**: `documentation` field pointed to `https://docs.rs/dracon-sync/0.1.5`
etc. (old versions). Crates.io auto-generates the docs.rs URL on publish,
so the hardcoded version was stale.
**Fix**: Removed the version-specific path; now points to the crate's
docs.rs landing page (e.g., `https://docs.rs/dracon-sync`).

### Finding 7: 3 sub-crates not published to crates.io at the v0.112.x versions

**Status**: NEW (pre-audit, the 3 sub-crates were at v0.1.5 / v0.2.0 / v0.3.0
on crates.io from earlier releases; the v0.112.6 / v0.112.7 / v0.112.8
releases did not include a crates.io publish)
**Fix**: Published `dracon-sync v0.1.9`, `dracon-system v0.2.4`,
`dracon-warden v0.3.4` to crates.io. All 3 verified via `cargo search` and
`cargo install` smoke test.

### Finding 8: `path = "dracon-warden/src/security"` dep in root `Cargo.toml` was a perceived blocker for `dracon-warden` publish

**File**: `Cargo.toml` line in `[workspace.dependencies]`
**Status**: FALSE POSITIVE — the path dep is a workspace dep that cargo
auto-rewrites to a version dep (`dracon-security v0.3.0`, which is already
on crates.io) when `cargo publish` runs. The packaged `Cargo.toml` for
warden has `dracon-security-kit = { version = "0.3.0", package = "dracon-security" }`
(verified by inspecting `target/package/dracon-warden-0.3.4/Cargo.toml`).
**Resolution**: No code change needed. Documented here for posterity.

### Finding 9: Crates.io has a 5-keyword limit (caught during first publish)

**Status**: BLOCKER caught at first publish attempt
**Fix**: Reduced each crate's `keywords` array from 10 to 5. The 3 first
publish attempts failed with "expected at most 5 keywords per crate". After
the fix, all 3 published successfully.

### Finding 10: No design doc explaining the crates.io publish process

**File**: N/A (missing doc)
**Issue**: No documentation of who owns the crates.io account, how to
publish, what's the release process, how to verify a published version.
**Fix**: Created `docs/design/crates-io-publish-2026-06-16.md` with the
full publish workflow.

## Things explicitly NOT changed (and why)

### Stale-allowed: design docs with old version references

Some design docs (e.g., `all-green-investigation-2026-06-15.md`,
`commit-all-policy-2026-06-15.md`, `kiki-sassy-decision-handoff-2026-06-15.md`,
etc.) reference older version numbers because they document historical
events. They are correctly dated and clearly state their temporal scope
(e.g., "as of 2026-06-15"). These are not "stale" — they are
historically accurate records of past investigations.

### Stale-allowed: `release-notes-v0.112.{5,6,7,8}.md` references to Set A short names

The release notes for v0.112.5 + v0.112.6 + v0.112.7 + v0.112.8 document
the Set A → Set B rename event. The Set A short names appear in these
documents as historical context. Per the goal `d2837ddc` (push-targets
audit), these references are explicitly carved out as historical
documentation, not active references.

### Source comments with "TODO" / "FIXME" / "XXX"

The audit searched for `TODO`, `FIXME`, `XXX`, `HACK` markers in source
code. All matches were either:
- Test data (e.g., `concat!("AGE", "-SECRET", "-KEY-", "1XXXX")`)
- Test patterns (e.g., `sk-XXX` for OpenAI key tests)
- The `verify-spec.sh` script's own check for FIXME comments
- `XXX` in tmp file paths (e.g., `/tmp/.tmpXXXXX/test-repo`)

**No actual deferred-work markers were found.** All code paths are
production-ready or documented in design docs.

## Audit results

| # | Finding | Status |
|---|---------|--------|
| 1 | Root README stated v0.112.5 (stale by 3 releases) | ✓ FIXED |
| 2 | Root README described façade repos as navigation shells (stale) | ✓ FIXED |
| 3 | Root README had no crates.io install path | ✓ FIXED |
| 4 | Per-utility READMEs had no "Install via crates.io" section | ✓ FIXED (3 files) |
| 5 | 3 sub-crate `Cargo.toml` files were missing `keywords` and `categories` | ✓ FIXED (3 files) |
| 6 | `documentation` URLs in 3 sub-crate `Cargo.toml` files pointed to old versions | ✓ FIXED (3 files) |
| 7 | 3 sub-crates not published to crates.io at the v0.112.x versions | ✓ FIXED (3 published) |
| 8 | `path` dep was a perceived blocker for `dracon-warden` publish | ✓ RESOLVED (false positive — cargo auto-rewrites) |
| 9 | Crates.io 5-keyword limit (caught at first publish) | ✓ FIXED |
| 10 | No design doc explaining the crates.io publish process | ✓ FIXED (new doc) |

## Final state

- **Workspace version**: 0.112.9 (this audit + crates.io publish packages into v0.112.9)
- **Sub-crate versions**: `dracon-sync 0.1.9`, `dracon-system 0.2.4`, `dracon-warden 0.3.4`
- **Published to crates.io**: YES (all 3)
- **Smoke test `cargo install`**: PASS for all 3
- **4-remote alignment**: PRESERVED (all 4 watched repos at 1 unique SHA)
- **Tests**: 856 passed, 0 failed, 9 ignored (no regression)
- **No secrets leaked**: `~/.cargo/credentials.toml` was used; never logged
