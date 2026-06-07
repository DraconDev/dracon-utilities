# Dracon Warden Improvement Blueprint

## Status Legend
- [ ] Not started
- [~] In progress
- [x] Completed

---

## CRITICAL Security Vulnerability (Fixed)

### 1. Encryption failure falls back to plaintext
- **Location:** `dracon-security/src/filter.rs:112-117, 180-185, 264-268`
- **Problem:** When encryption fails, secrets were passed through to git UNENCRYPTED. This is a critical security vulnerability.
- **Impact:** Secrets could be committed in plaintext if encryption has any issue
- **Fix:** 
  - Changed `clean_env()` and `clean_env_all()` to return error instead of plaintext fallback
  - Added `scan_and_replace_fallible()` method to `SecretScanner` 
  - Updated `clean()` to use fallible replacement that errors on encryption failure
- **Priority:** Critical
- **Status:** [x]

---

## Code Quality Fixes

### 2. Dead code - unused function
- **Location:** `main.rs:386-389`
- **Problem:** `should_passthrough_filter_path()` always returns `false`, parameter unused
- **Fix:** Marked with `#[allow(dead_code)]` - may be implemented in future
- **Priority:** Low
- **Status:** [x]

### 3. Redundant `let _ =` before `?`
- **Location:** `main.rs:820`
- **Problem:** `let _ = resmudge_repos(...)?` - confusing syntax, `?` already handles the result
- **Fix:** Removed redundant `let _ =`
- **Priority:** Low
- **Status:** [x]

---

## What Warden Does

dracon-warden is a Git filter + repository hardening daemon:

1. **Git Filter Operations**: Implements `clean` (encrypt on commit) and `smudge` (decrypt on checkout) using age encryption
   - Working tree remains plaintext (developers see normal config)
   - Git blobs contain ciphertext with `[DRACON_SECRET:...]` markers

2. **Repository Hardening**: Manages `.gitignore` and `.gitattributes` to enforce encryption policies

3. **Secret Scanning**: Detects AWS keys, OpenAI keys, GitHub tokens, Stripe keys via regex

4. **Recovery Tools**: Commands for scrubbing leaked markers and re-decrypting stuck files

5. **Daemon Mode**: Watches filesystem with debouncing for auto-hardening

6. **Plaintext-sibling escape hatch** (opt-in): A file with a `<path>.plaintext`
   sibling is intentionally plaintext. The clean filter returns the file
   unchanged, the pre-push hook skips it, and `scrub-markers` / `resmudge`
   leave it alone. See `docs/design/warden-plaintext-sibling.md` for the
   full design, threat model, and revocation story.

---

## Key Files

| File | Purpose |
|------|---------|
| `dracon-warden/src/main.rs` | CLI entry point, daemon, hardening logic |
| `dracon-security/src/filter.rs` | clean/smudge filter implementations |
| `dracon-security/src/crypto.rs` | age encryption/decryption |
| `dracon-security/src/scanner.rs` | Secret pattern detection |
| `dracon-security/src/identity.rs` | KeyRing management |

---

## Remaining (Low Priority)

### 4. Missing context on file write
- **Location:** `main.rs:1041`
- **Problem:** `fs::write()` lacked `.with_context()` unlike other file operations
- **Fix:** Added `.with_context(|| format!("failed writing {}", path.display()))`
- **Priority:** Low
- **Status:** [x]

### 5. Inconsistent error handling
- **Location:** `main.rs:393, 403`
- **Problem:** `.unwrap_or_default()` used for reading files - acceptable for "create if missing" pattern
- **Priority:** Low
- **Status:** [x] (documented as intentional - these are "create if missing" reads)

### 6. Silent git command failure
- **Location:** `main.rs:979-980`
- **Problem:** Git command failures silently skip
- **Fix:** Added warning message on failure
- **Priority:** Low
- **Status:** [x]
