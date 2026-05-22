# proc-macro-error Monitoring Investigation

Date: 2026-05-22

## Current Status

**`proc-macro-error v1.0.4` is NOT yanked.** Verified via crates.io API on 2026-05-22.

## Dependency Chain

```
dracon-warden → dracon-security
                     └── age 0.10.1
                           └── i18n-embed-fl 0.7.0 (proc-macro)
                                 └── proc-macro-error 1.0.4
```

## Risk Assessment

- **Current risk: LOW** — crate is not yanked, builds work fine
- **Future risk: MODERATE** — crate is unmaintained (last publish 2022, GitLab repo by CreepySkeleton). If it gets yanked or removed from the index, `cargo build` for dracon-warden will break
- **Note**: i18n-embed-fl 0.9.x has already migrated away from proc-macro-error to proc-macro-error2 (a maintained fork), and i18n-embed-fl 0.10.0 uses yet another approach

## Upgrade Path

The cleanest fix is upgrading `age` from `0.10` → `0.11`, which drops the `i18n-embed-fl` dep entirely:

| Crate | Current | Target |
|-------|---------|--------|
| `age` | 0.10.1 | 0.11.3 |
| `secrecy` | 0.8.0 | 0.10.3 (required by age 0.11) |

### Required API Changes (age 0.10 → 0.11)

**1. `Encryptor::with_recipients` now takes an iterator:**
```rust
// OLD (age 0.10):
age::Encryptor::with_recipients(recipients)  // Vec<Box<dyn age::Recipient + Send>>

// NEW (age 0.11):
age::Encryptor::with_recipients(recipients.iter().map(|r| r.as_ref()))?  // impl Iterator<Item = &dyn Recipient>
```

**2. `Decryptor` changed from enum to struct:**
```rust
// OLD (age 0.10):
let decryptor = age::Decryptor::new(cursor)?;
match decryptor {
    age::Decryptor::Recipients(d) => d.decrypt(iter::once(identity as &dyn age::Identity)),
    age::Decryptor::Passphrase(_) => { /* handle passphrase case */ }
}

// NEW (age 0.11):
let decryptor = age::Decryptor::new(cursor)?;
if decryptor.is_scrypt() {
    // passphrase-encrypted
} else {
    decryptor.decrypt(iter::once(identity as &dyn age::Identity))?
}
```

**3. `secrecy` version bump: `0.8` → `0.10`:**
- `ExposeSecret` trait API is compatible
- Need to bump in both `dracon-warden/Cargo.toml` and `dracon-warden/src/security/Cargo.toml`
- 5 call sites use `.expose_secret()` — all should work with secrecy 0.10

### Affected Files
- `dracon-warden/Cargo.toml` — `age = "0.10"` → `"0.11"`, `secrecy = "0.8"` → `"0.10"`
- `dracon-warden/src/security/Cargo.toml` — same changes
- `dracon-warden/src/security/src/lib.rs` — ~26 compile errors after upgrade:
  - 5x secrecy `ExposeSecret` trait mismatch (fix: bump secrecy)
  - 8x `Decryptor::Recipients` / `Decryptor::Passphrase` enum variants removed
  - 8x `Encryptor::with_recipients` needs iterator argument
  - 5x other Decryptor pattern matches

## Recommendation

**No action needed now.** Since `proc-macro-error` is not yanked, builds work fine. Monitor the situation. If it gets yanked, the upgrade path to `age 0.11` is understood and documented above.
