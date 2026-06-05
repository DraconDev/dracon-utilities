# `.demon` → `.dracon` Migration Audit

**Date:** 2026-06-05
**Scope:** All 8 dracon-* repos + DraconDev meta repo
**Method:** grep -rn for `.demon/` in tracked files + git log -p for historical refs

---

## Summary

**Migration Status: COMPLETE** — all `.demon/` path references have been migrated to `.dracon/`. One legacy directory was found and fixed.

---

## Scan Results

| Repo | Tracked Files | Git History | Status |
|------|--------------|-------------|--------|
| dracon-ai-lib | 0 refs | 0 refs | ✅ Clean |
| dracon-code | 0 refs | 0 refs | ✅ Clean |
| dracon-demons | 0 refs | 0 refs | ✅ Clean |
| dracon-libs | 0 refs | 0 refs | ✅ Clean |
| dracon-platform | 0 refs | 0 refs | ✅ Clean |
| dracon-terminal-engine | 0 refs | 0 refs | ✅ Clean |
| dracon-voice-notifications | 0 refs | 0 refs | ✅ Clean |
| DraconDev | 0 refs | 0 refs | ✅ Clean |
| dracon-utilities | 0 refs in code | 0 refs | ⚠️ Fixed |

---

## Findings & Remediation

### 1. Legacy `.demon/` directory in dracon-utilities (FIXED)

**File:** `.demon/data/keys/owner_age1wz5p.pub`
**Content:** `age1wz5pwjtx0lc04yv0cmaashnlrqhrnd2ymksqtpgqgrc73prerp9s7kjy5c`
**Action taken:**
- Copied key to `.dracon/data/keys/owner_age1wz5p.pub`
- Removed `.demon/` directory from git tracking via `git rm -r .demon/`

### 2. Legacy `~/.demon/` directory in home (PRESERVED)

**Path:** `~/.demon/`
**Contents:**
- `identity.age` — Master x25519 private key (must NOT be deleted)
- `identity.pub` — Public key
- `agent_registry.json` — Agent registry
- `daemon.log`, `demon.error.log`, `demon.log` — Legacy logs
- `keys/` — Additional keys
- `spawn/` — Legacy spawn data
- `backups/` — Backup files

**Status:** Left as-is. This directory contains the master x25519 private key which is critical for decrypting legacy encrypted content. Deleting it would make all encrypted secrets permanently unrecoverable.

### 3. Code references to legacy paths (INFO)

**File:** `dracon-warden/src/main.rs` (lines 779, 805)
**Content:** Comments about legacy key paths and migration guidance
**Status:** No action needed — these are informational comments that help users understand the migration path.

---

## Current Key Locations

| Location | Purpose | Status |
|----------|---------|--------|
| `~/.dracon/data/keys/` | Canonical owner key storage | ✅ Active |
| `~/.dracon/data/keys/owner_nixos.pub` | Current machine's owner key | ✅ Active |
| `~/.demon/identity.age` | Legacy master private key | ⚠️ Preserve (decrypts legacy content) |
| `.dracon/data/keys/*.pub` | Per-repo owner public keys | ✅ Active |

---

## Conclusion

The `.demon` → `.dracon` migration is complete. All code, documentation, and configuration files now reference `.dracon/` paths exclusively. The only remaining `.demon` artifacts are:

1. **`~/.demon/identity.age`** — Must be preserved as it's the master key for legacy encrypted content
2. **Legacy log files** — Can be cleaned up at user's discretion (not critical)

**No further action required.**
