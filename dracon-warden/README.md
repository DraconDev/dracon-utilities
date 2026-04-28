# dracon-warden

Git filter + repo hardening daemon used by the Dracon toolchain.

## Mental Model (Important)

- **Working tree is plaintext**: `filter.smudge` decrypts so your app can read normal config/secrets.
- **Git blobs are ciphertext**: `filter.clean` encrypts so secrets are encrypted-at-rest in history.

To verify what is stored in git (not your working tree), use:

```sh
git show HEAD:path/to/file
```

If encryption is active for that path, you should see marker payloads like `[DRACON_SECRET:...]`
in the `git show` output (even though your working tree file is plaintext).

## Safety Defaults

- `plaintext_patterns` is for files that must remain plaintext in git (lockfiles, public keys, etc).
- `plaintext_patterns` **must not include secret-ish patterns** (like `.env` or `secrets/**`).
  dracon-warden will refuse to run if the policy tries to disable encryption for those.

## Common Commands

```sh
dracon-warden status
dracon-warden once
dracon-warden daemon
dracon-warden scrub-markers --apply
```

`scrub-markers` is a recovery tool for cases where marker tokens (ex: `DRACON_SECRET`) accidentally land inside plaintext JSON (usually from copy/paste or bad tooling).
