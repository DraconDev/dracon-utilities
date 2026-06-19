# Systemic Audit - All Hacky/Manual Solutions (2026-06-19)

## Context

Comprehensive audit of all 13 repos for hacky/manual
solutions that should be replaced with systemic ones.
This is the consolidated audit covering overrides,
.plaintext siblings, pi commits, git config, and
.gitignore bypasses.

## Summary of Findings

| Category | Count | Systemic Fix Applied | Status |
|----------|-------|---------------------|--------|
| Per-repo override files | 3 | Documented (KEEP - AGENTS.md constraint) | ✅ |
| .plaintext sibling files | 23 (was 20, +3 audit docs) | Documented (KEEP - scanner limitation) | ✅ |
| Historical pi commits | 3 (2 repos) | Documented (KEEP - daemon only checks HEAD) | ✅ |
| Repos missing local git config | 9 | Set local config in all 9 | ✅ |
| Force-added `.dracon/dracon-sync.toml` | 5 | Added `!.dracon/dracon-sync.toml` to whitelist | ✅ |
| Force-added `.dracon/project-state.md` | 11 | Added `!.dracon/project-state.md` to whitelist | ✅ |
| Force-added `.dracon/inventory_check.json` | 1 (DraconDev) | Added `!.dracon/inventory_check.json` to whitelist | ✅ |

## Details

### 1. Per-repo override files (3)

- **rust-ai-web-auto** (placeholder, no active policy)
- **dracon-ai-lib** (`owned = true` for 130 historical bad-config commits)
- **dracon-platform** (`owned = true` for 1 pi commit 620 deep)

All 3 are correctly justified per AGENTS.md constraints
(no history rewrite, no force-push to repos with > 5
commits ahead). The override file mechanism is the
correct systemic solution.

### 2. .plaintext sibling files (23)

- 19 test fixtures with intentional secret patterns
- 1 design doc with pattern descriptions
- 3 audit docs (added during this audit)

The .plaintext sibling mechanism is the correct
workaround for the warden's pre-push hook. The systemic
fix is scanner context detection (out of scope - requires
modifying `dracon-warden/src/main.rs`).

### 3. Historical pi commits (3 across 2 repos)

- **dracon-code**: 2 pi commits (7-10 deep on `audit/2026-06-18`)
  - INERT - daemon doesn't flag because HEAD is DraconDev
- **dracon-platform**: 1 pi commit (620 deep on `main`)
  - Override file handles the daemon classification

### 4. Local git config (9 repos)

Set `git config --local user.email "dracsharp@gmail.com"`
and `git config --local user.name "DraconDev"` in all 9
repos that were missing local config. This prevents
future agent sessions from committing as pi.

### 5. .gitignore whitelist additions (11 repos)

- `.dracon/dracon-sync.toml` whitelisted in 5 repos
  (ai-auto-writer, avid, dracon-ai-lib, dracon-platform,
  rust-ai-web-auto)
- `.dracon/project-state.md` whitelisted in 11 repos
  (all repos with the file tracked)
- `.dracon/inventory_check.json` whitelisted in 1 repo
  (DraconDev)

This eliminates the need for `git add -f` workarounds.
The files are now trackable natively via the whitelist
pattern in `.gitignore`.

## Systemic Solutions Applied

1. **Override files**: Kept as-is. They are the correct
   solution for historical bad-config authors that can't
   be rewritten per AGENTS.md.

2. **.plaintext siblings**: Kept as-is. They are the
   correct workaround for scanner limitations. The
   systemic fix is scanner context detection (out of
   scope).

3. **Pi commits**: Left in history. The daemon only
   checks HEAD, so historical pi commits are inert.
   The 620-deep commit on dracon-platform requires the
   override file.

4. **Local git config**: Set in all 9 repos. This is
   a systemic fix - all repos now have the correct
   local identity.

5. **.gitignore whitelist**: Added in all affected
   repos. This is a systemic fix - the files are now
   trackable without `git add -f`.

## Remaining Hacky Solutions

None identified. All hacky/manual solutions found
during the audit have been either:
- Documented as the correct systemic solution given
  the AGENTS.md constraints (overrides, .plaintext
  siblings, historical pi commits)
- Fixed systemically (local git config, .gitignore
  whitelist)

## Systemic Improvement Opportunities (Out of Scope)

1. **Scanner context detection**: The warden scanner
   could detect test/mock/documentation contexts and
   skip the scan, eliminating the need for 23 .plaintext
   sibling files. Requires modifying
   `dracon-warden/src/main.rs` (out of scope).

2. **Historical author detection**: The daemon could
   distinguish between "HEAD untrusted author" (blocks
   auto-commit) and "historical untrusted author"
   (informational only). Currently the daemon only
   checks HEAD, which is the correct behavior.

3. **.gitignore template**: The warden's .gitignore
   template could include `!.dracon/dracon-sync.toml`,
   `!.dracon/project-state.md`, and
   `!.dracon/inventory_check.json` by default, so
   per-repo additions aren't needed. Requires modifying
   `dracon-warden/src/main.rs` (out of scope).
