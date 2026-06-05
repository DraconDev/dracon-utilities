# Dracon Warden — AI Agent Guidelines

## What This Project Does

`dracon-warden` is a Git filter + repo hardening tool. It encrypts secrets at rest in git while keeping plaintext in your working tree.

- **Working tree** = plaintext (your app reads normal `.env` files)
- **Git blobs** = ciphertext (stored as `[DRACON_SECRET:base64_age_ciphertext]`)
- Verify with: `git show HEAD:path/to/file`

## Critical: Do NOT Delete or Modify These

### Master Key Directory
- **`~/.dracon/`** — Contains ALL keys: `identity.age`, `master.age`, `keys/*.age`, `teams/*.key`, `registries.age`, machine keys (`machine_*.age`), and owner keys (`owner.age`). Deleting this makes **all encrypted secrets permanently unrecoverable**.

### Encrypted Content in Git
- Files like `.env` in git repos show as `[DRACON_SECRET:base64_age_ciphertext]` — this is **correct and intentional**. Do NOT:
  - Run "scrub", "clean", "decrypt", or "repair" on these markers
  - Recommend removing or un-encrypting these files
  - Suggest adding `.env` to `.gitignore`

### Prohibited Actions
- Delete, modify, or "repair" anything in `~/.dracon/`
- Run `dracon-warden repair`, `dracon-warden scrub-markers --apply`, or similar recovery commands that would alter encrypted content
- Overwrite, move, or rename key files
- Modify `.env` files or suggest making them plaintext in git
- Add secret files to `.gitignore`

## Architecture for Reference

```
~/.dracon/identity.age          — Master x25519 private key
~/.dracon/master.age           — Sovereign master key  
~/.dracon/keys/*.age           — Additional identities
~/.dracon/data/keys/machine_*.age — Machine-level secret keys
~/.dracon/data/keys/owner_*.pub  — Owner key for repo authorization
```

Binary: `/home/dracon/.local/bin/dracon-warden`
Commands: `status`, `once`, `scrub-markers`, `resmudge`, `repair`, `keygen`, `setup-hooks`