# Warden Pub-Key Tracking

This document explains how `dracon-utilities` tracks the operator's
warden (age) public keys so they can be recovered from git history if
the local `.dracon/data/keys/` directory is ever lost.

## Why pub keys (and not private keys)

Age encryption uses an asymmetric keypair:

- **Private key** (`*.age`, `id_age`, `*.key`): kept secret. NEVER
  tracked. NEVER pushed to any remote.
- **Public key** (`*.pub`): shared with collaborators so they can
  encrypt data the operator can decrypt. Safe to publish.

The warden's encryption is set up so the operator can always
re-decrypt their own data on a fresh machine by:

1. Cloning `dracon-utilities`
2. Reading `.dracon/data/keys/*.pub` from the working tree
3. Either: re-deriving the private key from a backup, or asking the
   operator to add the private key to a fresh
   `~/.dracon/data/keys/owner_nixos.age`

Because pub keys are public information, tracking them in git is a
**safe and best-practice** backup mechanism.

## What is tracked

All operator-owned `*.pub` files under `~/.dracon/data/keys/` are
force-tracked via this `.gitignore` allowlist (managed by
`dracon-warden`):

```gitignore
# Block everything in .dracon/data/keys/ first
.dracon/data/keys/
.dracon/data/keys/*
# Allow public keys
!.dracon/data/keys/*.pub
```

The broader `!*.age` and `!*.toml` allowlists earlier in `.gitignore`
do NOT apply here because the explicit blocklist on lines 110-111
takes precedence (more specific path matches win). Verified via
`git check-ignore -v`.

The currently tracked set (as of June 2026) is:

| File | Purpose | Tracked? |
|------|---------|----------|
| `owner_nixos.pub` | Operator's age identity on nixos | ✓ |
| `owner_age15xjl.pub` | Older operator identity (rotated) | ✓ |
| `owner_age1f7y5.pub` | Older operator identity (rotated) | ✓ |
| `master.pub` | Warden's master pub key | ✓ |
| `micro2_git_key.pub` | Micro2 host git operations | ✓ |
| `micro2_libs_key.pub` | Micro2 host libs operations | ✓ |
| `manifest.toml` | Key manifest (NOT a key — config) | ✗ |
| `*.age` / `id_age` / `*.key` | Private keys (BLOCKED) | ✗ |

The `manifest.toml` is intentionally NOT tracked: it contains local
config (last rotation date, machine name) and is small enough to
recreate from the operator's memory.

## What is NOT tracked

Private keys and identity files:

- `*.age` (age private keys) — blocklisted by `.gitignore`
- `id_age` — blocklisted (default age identity name)
- `*.key` (machine-specific SSH/age keys in
  `~/.dracon/data/keys/`) — blocklisted

`git check-ignore -v ~/.dracon/data/keys/machine_micro2.age` confirms
the blocklist is active.

## Adding a new pub key

When the operator generates a new age keypair (e.g. for a new
machine), the workflow is:

```bash
# Generate the new keypair on the new machine
dracon-warden keygen  # writes to ~/.dracon/data/keys/owner_<host>.pub and owner_<host>.age

# On the canonical dracon-utilities working tree, copy the pub key
cp ~/.dracon/data/keys/owner_<host>.pub /home/dracon/Dev/dracon-utilities/.dracon/data/keys/

# Stage, commit, push (the daemon auto-commits in normal operation)
cd /home/dracon/Dev/dracon-utilities
git add .dracon/data/keys/owner_<host>.pub
git commit -m "track new operator pub key for <host>"
git push origin main
```

The daemon's `auto_commit = true` will pick up the staged file on the
next sync cycle (typically within `inactivity_push_delay_secs`).

## If the allowlist is too narrow

The current allowlist `!.dracon/data/keys/*.pub` matches every file
ending in `.pub`. If the operator wants a stricter allowlist (e.g.
only `owner_*.pub` and `master.pub`), edit the `.gitignore` block
managed by `dracon-warden`:

```gitignore
!.dracon/data/keys/owner_*.pub
!.dracon/data/keys/master.pub
```

The warden will preserve this allowlist through harden passes
(because it's in the managed block). To re-trigger the allowlist
generation, run `dracon-warden once` and review the diff.

## Recovery procedure

If `~/.dracon/data/keys/` is lost on a fresh machine:

1. Clone `dracon-utilities` from any public remote (origin, gitlab,
   codeberg). The 6 tracked `*.pub` files are all in the working
   tree at `.dracon/data/keys/`.
2. Either:
   - Restore the private key from a separate backup
     (recommended: encrypted USB, password manager, paper backup)
   - Re-encrypt all data with a new keypair; the old keypair is
     lost and any age-encrypted secrets become unrecoverable.
3. The operator can verify the recovery by running
   `dracon-warden status` and confirming the new machine's pub key
   matches one of the tracked `*.pub` files.

## Why tracking is `auto` and `opt-out`

The `standard_files` system in `dracon-sync` automatically restores
tracked pub keys on every new repository scaffold (per the
`standard_files_auto` policy). This means:

- A fresh clone of any watched repo already has the operator's pub
  keys in `.dracon/data/keys/` if they were originally tracked in
  `dracon-utilities`.
- The `scaffold` subcommand on a new repo will not auto-add pub keys
  (pub keys are not in the standard-files set), but the operator can
  copy them in manually from `dracon-utilities` if needed.
