# Release Notes — v0.112.32 (2026-07-21) — dracon-warden

**Headline**: The audit warden batch (`AUDIT_FULL_2026-07-21.md`) —
2 HIGH + 4 MEDIUM fixes. Headline architectural change: **dracon-warden
now builds the LOCAL `src/security` crate source via a path
dependency** (previously it built the published crates.io
`dracon-security v0.3.0`, so fixes to the local source never reached
the binary). The security crate is now a full workspace member, so
`cargo test --workspace --locked` runs its tests too.

---

## Fixes (audit finding IDs in parentheses)

### 1. `harden_repo` no longer wipes operator .gitignore/.gitattributes content (H8/F4.1)

`harden_repo` passed `build_gitignore_block_with_existing(...)` (which
returns ONLY the managed block) straight to `apply_overwrite_file`,
wiping ALL operator content outside the delimited block on every
harden pass — verified in this repo's own history (commit `3a67685f`
deleted the operator's 8-line nested-repo section). The surgical
`replace_managed_block` is promoted from `#[cfg(test)]` to production
and used for BOTH files: replace only the delimited block, preserve
everything outside it, append if absent. Atomic temp+rename write
kept. **Live-verified**: `dracon-warden once
/home/dracon/Dev/dracon-utilities` preserved the operator's "NESTED
STANDALONE REPOS" section; the only diff was inside the managed block.
Regression test: operator content before AND after the block survives
two harden passes, exactly one block remains.

### 2. Whole-file-encrypted BINARY secrets round-trip as bytes (H9/F4.2)

`smart_smudge` converted decrypted plaintext via
`String::from_utf8_lossy`, replacing every invalid UTF-8 sequence with
U+FFFD — silently corrupting whole-file-encrypted binary secrets (DER
`.key`, SQLite under `secrets/**`, `.kdbx`) on checkout, after which
the corrupted file was re-encrypted into history and the original
bytes were lost. New `decrypt_whole_file_tag` helper: when the ENTIRE
content is one secret tag (`[MARKER:<b64>]`, the format
`smart_clean_with_path` uses for binary files in sensitive locations),
decrypt to RAW BYTES. Wired into `seal_smudge` (git smudge filter) and
`decrypt_file` (recursive disk decryption). Inline tags in textual
content still use `smart_smudge` (test pins both paths). Round-trip
test: random non-UTF-8 bytes → clean → smudge → **byte-identical**.

**NOTE**: this fix required the path-dependency change above — the bug
was verified present in the published v0.3.0 (`from_utf8_lossy` at
filter.rs:246 of the registry copy), and the local-source fix alone
would never have reached the warden binary.

### 3. `allow_v1_fallback` gate wired (M29/F4.3)

FDRACONWARDEN-001's remediation kept a runtime escape hatch for legacy
V1 (AES-CFB) ciphertexts and documented "set `allow_v1_fallback =
true` in the policy, decrypt once to re-encrypt under V2" — but the
field didn't exist on `WardenPolicy` (serde silently ignored the TOML
key) and `set_allow_v1_fallback` had zero callers. New
`allow_v1_fallback: bool` (default false) on `WardenPolicy`, wired to
the process-global gate in `WardenPolicy::load` (the single chokepoint
every migration-relevant command goes through). Test: flag on → gate
on; flag absent → gate off.

### 4. `setup-hooks --local` works (M30/F4.4)

Ran `git config local core.hooksPath <dir>` (missing `--`) — git
rejects with "key does not contain a section: local", so the command
ALWAYS failed after the hook files were already written (partial
application). Same bug class as the dracon-sync test-config incident
the same week, but in production code. Behavioral test: after
`setup-hooks --local`, `git config --local --get core.hooksPath`
succeeds and the pre-push hook file exists.

### 5. Filter-clean fails closed for oversized/refused inputs (M31/F4.5)

The >10 MiB guard and the FDRACONWARDEN-002 path guards wrote the
input back to stdout and exited 0 in BOTH directions — in the clean
direction that commits the file UNENCRYPTED (a 15 MiB `.env` lands in
history in plaintext, silently). All three guards now fail closed
(non-zero exit, git aborts the add) for clean via a shared
`filter_clean_refusal_reason` predicate; smudge passthrough stays
correct (keeps ciphertext as-is).

### 6. Pre-push hook handles filenames with spaces (M32/F4.6)

`for f in $(git diff --name-only ...)` word-split on whitespace — a
file named `prod secrets.env` split into `prod` + `secrets.env`,
neither fragment was scanned, and a plaintext secret in a
space-containing filename pushed clean. The hook now iterates
`git diff --name-only -z` (via `tr '\0' '\n'` + `IFS= read -r`) and
passes accepted files to the scan via `xargs -0` (no word-splitting,
no glob expansion). NOTE: `--pathspec-from-file` was tried first but
`git diff` does NOT support it (usage error, exit 129 — verified
against git 2.51.2). Behavioral test: `prod secrets.env` containing an
`AKIA...` line exits the hook 1.

---

## Architectural change: security crate is now built from source

- `dracon-warden/Cargo.toml`: `dracon-security-kit = { package =
  "dracon-security", version = "0.3.0", path = "src/security" }`
  (was registry-only). Works for both the monorepo workspace build and
  the standalone dracon-warden repo (`src/security` is tracked inside
  it).
- Root `Cargo.toml`: `dracon-warden/src/security` added to workspace
  `members` — `cargo test --workspace --locked` now runs the security
  crate's ~109 tests (lib + integration + proptest).
- Also fixed a pre-existing clippy lint in the security source
  (needless borrow at filter.rs:103) exposed by membership.

## Test discipline

- `cargo test --workspace --locked` ✅ all green (dracon-sync 797,
  warden 81 + security crate ~109, dracon-system 86)
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean
- Live: `dracon-warden once` on the meta-repo preserved operator
  .gitignore content (H8)
