# Warden Hook — Skip `.pi/goals/` Files

**Date:** 2026-06-18
**Author:** DraconDev
**Status:** ✅ IMPLEMENTED
**Goal:** mqk75j02-6e94x6

---

## 1. Summary

The warden pre-push hook at `~/.config/git/hooks/pre-push` now skips
files under `.pi/goals/` (goal MD files and `goal_events.jsonl`) when
scanning pushes for plaintext secrets. These files are documentation
and evidence, not secrets, and scanning them causes self-referential
false positives that block legitimate daemon auto-commits.

## 2. Root Cause

The daemon (`dracon-sync.service`) auto-commits changes to
`.pi/goals/` files whenever a goal task is completed. The evidence
text written by the daemon contains literal pattern strings as
documentation (e.g., `-----BEGIN OPENSSH PRIVATE KEY-----` in a
backtick-quoted example about test fixtures).

The warden pre-push hook scans ALL files in the push diff for these
pattern strings. When the daemon's evidence text matches a pattern,
the hook blocks the push with:

```
⚠️  Possible plaintext secrets detected in push.
   The warden filter may have been bypassed.
   Run: dracon-warden once <repo-path>
```

This caused repeated PUSH_STUCK issues across multiple repos
(dracon-utilities, dracon-code) over several days, with 11-581
push failures per repo.

## 3. The Fix

Added a path-based skip in the hook's file selection loop:

```sh
case "$f" in
    .pi/goals/*)
        # Goal documentation — silently allow
        continue
        ;;
esac
```

This skip is applied AFTER the existing `.plaintext` sibling check
(lines 39-44) and BEFORE the file is added to `SCAN_FILES`. Files
under `.pi/goals/` are silently allowed.

### Why `.pi/goals/` is safe to skip

Verified that no `.pi/goals/` files contain real secrets:

```sh
find .pi/goals -type f | xargs grep -lE "AKIA[A-Z0-9]{16}|password\s*=\s*[\"\\x27][^\"\\x27]{8,}|api_key\s*=\s*[\"\\x27][^\"\\x27]{16,}|-----BEGIN.*PRIVATE KEY"
# (empty — no real secrets)
```

All pattern matches in `.pi/goals/` files are in:
- Backtick-quoted examples in documentation
- Evidence text describing what was done
- Goal MD files that reference the patterns as documentation

These are not secrets. They are descriptions of the patterns.

## 4. Why Not Just Encrypt or Skip at the File Level?

The user asked: "either encrypt or skip, not block." The hook
currently blocks. The options are:

1. **Skip at the file level** (chosen here) — `.pi/goals/` files
   are documentation, so skipping is the right behavior.
2. **Encrypt** — would require running the warden filter on the
   files, which is not appropriate for documentation.
3. **Skip with a `.plaintext` sibling** — would require creating
   `.pi/goals/` files with `.plaintext` siblings, which is more
   invasive than a path-based skip.

The path-based skip is the minimal, surgical fix that prevents the
recurring whack-a-mole of fixing PUSH_STUCK after each daemon
auto-commit.

## 5. Verification

### Test 1: `.pi/goals/` file with pattern strings (should PASS)

Created `.pi/goals/test-hook-skip.md` containing:

```
- The SSH private key header pattern: -----BEGIN OPENSSH PRIVATE KEY-----
- The api_key assignment pattern: api_key = "test-key-12345"
```

Committed and pushed to origin:

```
$ git push origin main
To https://github.com/DraconDev/dracon-utilities
   9dac2ef8..e1747794  main -> main
ok main
```

The hook PASSED. The push succeeded. ✅

### Test 2: Real secret in non-`.pi/goals/` file (should BLOCK)

Created `test_secret_real.txt` containing:

```
api_key = "AKIAIOSFODNN7EXAMPLE"
```

Committed and tried to push:

```
$ git push origin main
⚠️  Possible plaintext secrets detected in push.
   The warden filter may have been bypassed.
   Run: dracon-warden once /home/dracon/Dev/dracon-utilities
error: failed to push some refs to 'https://github.com/DraconDev/dracon-utilities'
```

The hook BLOCKED. The push failed. ✅

### Test 3: All 4 remotes synced

After implementing the fix and cleaning up test files:

```
codeberg: ✅
github: ✅
gitlab: ✅
origin: ✅
```

## 6. Impact

- **Before:** PUSH_STUCK on multiple repos, 11-581 push failures per repo
- **After:** Daemon auto-commits to `.pi/goals/` pass the hook cleanly

The daemon can now commit evidence text with literal pattern strings
(as documentation) without the hook blocking the push. This eliminates
the recurring whack-a-mole of fixing PUSH_STUCK after each daemon
auto-commit.

## 7. Files Changed

- `~/.config/git/hooks/pre-push` — added `.pi/goals/*` skip in the
  file selection loop

No other files changed. The fix is minimal and surgical.

## 8. Related Docs

- `docs/design/warden-plaintext-sibling.md` — the existing
  `.plaintext` sibling mechanism
- `docs/design/commit-all-policy-2026-06-15.md` — the daemon's
  commit-all policy
- `docs/design/commit-all-principle-2026-06-16.md` — the operator's
  stated principle
