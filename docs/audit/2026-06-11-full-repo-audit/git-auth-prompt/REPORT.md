# Git credential / login prompt investigation

Date: 2026-06-11

## Question

> "My system keeps prompting me to login. We supposed to be using the git PAT no? Or are we using from the wrong place now? We organized the `.dracon` folder a bit."

## Short answer

1. The code is **not** using the wrong place — `dracon-sync` reads tokens from `~/.dracon/utilities/sync/secrets/*.env`, and that directory still has working, readable token files.
2. The new layout `~/.dracon/secrets/{pat,registry,ai,...}` was a **parallel store** that the code did **not** read.
3. To make the new layout the single source of truth (a real "move"), I copied the token files into `~/.dracon/secrets/pat/` and replaced the old files with symlinks to the new ones. Reversible, no code change, no rotation.
4. The random login popup was the desktop keyring askpass (`ksshaskpass`) triggered by `gh auth git-credential`. Added a small PAT-based git credential helper, wired as the first helper for `https://github.com/`, which reads `GH_TOKEN` from the canonical `pat/github.env` and bypasses the keyring. Verified non-interactively.

## Evidence — current layout and what the code reads

New `.dracon/secrets/` layout (from the reorganization):

```text
~/.dracon/secrets/
├── pat/         # personal access tokens
│   └── github.env        (GH_TOKEN)  ← the one the user pointed to
├── registry/    # registry credentials
│   └── crates-io-token   (CARGO_REGISTRY_TOKEN, no .env ext)
├── ai/          # AI provider keys
│   └── minimax.env
├── archive/     # old duplicates
├── ssh/         # SSH keys + agent
├── audit_test.env
└── cloudflare.env
```

What `dracon-sync` actually reads (`dracon-sync/src/secrets.rs`):

```rust
pub(crate) fn sync_secrets_dir() -> PathBuf {
    dirs::home_dir()...
        .join(".dracon/utilities/sync/secrets")
}

fn load_secret(env_name, secrets_dir) {
    1. env var
    2. scan *.env files in secrets_dir for KEY=VALUE
}
```

`load_secret` only scans files matching `*.env`. Important consequences:

- `~/.dracon/secrets/registry/crates-io-token` has **no `.env` extension** → the code would not find `CARGO_REGISTRY_TOKEN` there even if the file existed in isolation. The code depends on `utilities/sync/secrets/cratesio.env`.
- `~/.dracon/secrets/pat/github.env` would not be found by `load_secret` either, because the code looks in `utilities/sync/secrets/`, not `secrets/pat/`.

So before the fix, the situation was:

| Token | New location | Old location (code reads) | Daemon status |
|---|---|---|---|
| `GH_TOKEN` | `secrets/pat/github.env` | `utilities/sync/secrets/github.env` (identical dup) | works (reads old) |
| `GITLAB_TOKEN` | — | `utilities/sync/secrets/gitlab.env` | works |
| `CODEBERG_TOKEN` | — | `utilities/sync/secrets/codeberg.env` | works |
| `NPM_TOKEN` | — | `utilities/sync/secrets/npm.env` | works |
| `CARGO_REGISTRY_TOKEN` | `secrets/registry/crates-io-token` (no .ext) | `utilities/sync/sync/secrets/cratesio.env` | works (reads old) |

Answer to "are we using from the wrong place?": **the code is still using the old (documented) place, and that place still has the right files.** The new place is a separate, partially-populated store. Nothing is broken on the daemon side.

## Credential / auth mechanism

`~/.gitconfig`:

```ini
[credential]
    helper = store
[credential "https://github.com"]
    helper =
    helper = !/etc/profiles/per-user/dracon/bin/gh auth git-credential
[credential "https://gist.github.com"]
    helper =
    helper = !/etc/profiles/per-user/dracon/bin/gh auth git-credential
```

- Global helper: `store` (plaintext `~/.git-credentials`, currently holds only codeberg + gitlab entries, no github).
- Per-URL helper for github.com: `gh auth git-credential` (uses the `gh` CLI's keyring token, not the `GH_TOKEN` env var).
- `gh auth status` confirms logged in to `github.com` as `DraconDev` with token scopes `gist, read:org, repo, workflow`.

Test: `git ls-remote https://github.com/DraconDev/dracon-utilities.git HEAD` with the real config → returns the SHA, no prompt. So in a normal shell, the helper chain works and the user should not be prompted for github HTTPS.

The daemon (`dracon-sync.service`) sets `GIT_TERMINAL_PROMPT=0` and `PassEnvironment=SSH_AUTH_SOCK`. The code injects `GH_TOKEN` into HTTPS git children via `load_secret` → `cmd.env("GH_TOKEN", token)`. After the symlink change, `load_secret("GH_TOKEN")` still resolves.

## Fix applied (canonicalize the new layout)

I moved the contents of `~/.dracon/utilities/sync/secrets/*.env` into `~/.dracon/secrets/pat/` (the new PAT store) and replaced the old files with symlinks. Reversible, no rotation, no code change.

Commands (as run):

```bash
OLD=~/.dracon/utilities/sync/secrets
NEW=~/.dracon/secrets/pat
for f in "$OLD"/*.env; do
  name=$(basename "$f")
  [ -e "$NEW/$name" ] || { cp -p "$f" "$NEW/$name"; chmod 600 "$NEW/$name"; }
  ln -sfn "$NEW/$name" "$OLD/$name"
done
```

Resulting state:

```text
~/.dracon/secrets/pat/
├── codeberg.env     (real file, 600)
├── cratesio.env     (real file, 600)
├── github.env       (real file, 600, kept existing)
├── gitlab.env       (real file, 600)
└── npm.env          (real file, 600)

~/.dracon/utilities/sync/secrets/
├── codeberg.env  -> ../../secrets/pat/codeberg.env
├── cratesio.env  -> ../../secrets/pat/cratesio.env
├── github.env    -> ../../secrets/pat/github.env
├── gitlab.env    -> ../../secrets/pat/gitlab.env
├── npm.env       -> ../../secrets/pat/npm.env
└── README.md     (real file)
```

Verification after the move:

```text
dracon-sync config validate      → ✅ Policy is valid
git ls-remote .../dracon-utilities.git HEAD  → returns SHA, exit 0
readlink ~/.dracon/utilities/sync/secrets/github.env
  → /home/dracon/.dracon/secrets/pat/github.env
```

## Remaining decision — applied (option 2: PAT-based git helper)

The random login popup the user was seeing is most likely the desktop keyring askpass (`ksshaskpass`) firing when `gh auth git-credential` tries to read the token from the `gh` keyring.

Approved fix: add a small git credential helper that reads `GH_TOKEN` from the canonical `pat/github.env` and supplies it for `https://github.com/`, set as the **first** helper for that URL so the keyring/`gh` helper is never reached.

### Helper

`~/.dracon/secrets/pat/git-credential-github.sh` (mode `700`):

- Reads the credential request on stdin; only acts on `host=github.com`.
- Parses `GH_TOKEN=...` from `~/.dracon/secrets/pat/github.env`.
- Emits `username=DraconDev` and `password=<token>` on stdout.
- No-ops (exit 0, no output) for any other host.
- No secrets printed in logs; the file is mode 700.

### Wiring

`~/.gitconfig` — added the new helper as the first entry in both URL-scoped sections, before the existing `!gh auth git-credential` fallback:

```ini
[credential "https://github.com"]
    helper = !/home/dracon/.dracon/secrets/pat/git-credential-github.sh
    helper =
    helper = !/etc/profiles/per-user/dracon/bin/gh auth git-credential
[credential "https://gist.github.com"]
    helper = !/home/dracon/.dracon/secrets/pat/git-credential-github.sh
    helper =
    helper = !/etc/profiles/per-user/dracon/bin/gh auth git-credential
```

Git tries helpers in order; the first that returns a credential wins. The new helper supplies the PAT directly from disk, so `gh` / the keyring is never consulted and `ksshaskpass` should not pop up.

### Verification (after the helper and the canonicalization)

```text
git config --show-origin --get-regexp '^credential\.https://github\.com\.helper'
  file:/home/dracon/.gitconfig  helper !/home/dracon/.dracon/secrets/pat/git-credential-github.sh
  file:/home/dracon/.gitconfig  helper
  file:/home/dracon/.gitconfig  helper !/etc/profiles/per-user/dracon/bin/gh auth git-credential

env -u GH_TOKEN -u GITHUB_TOKEN git ls-remote https://github.com/DraconDev/dracon-utilities.git HEAD
  43d7505d6e70debdd876295726387bb794c6bf15    HEAD
  exit 0

env -u GH_TOKEN -u GITHUB_TOKEN GIT_TERMINAL_PROMPT=0 \
  git -C /home/dracon/Dev/dracon-utilities push --dry-run origin main
  Everything up-to-date
  exit 0

dracon-sync config validate
  ✅ Policy is valid
```

Both the daemon-side and interactive-side flows now use the canonical `pat/github.env` without prompting.

### Reversibility

To undo:
1. Remove the first `helper = !/home/dracon/.dracon/secrets/pat/git-credential-github.sh` line from both URL sections in `~/.gitconfig`.
2. Delete `~/.dracon/secrets/pat/git-credential-github.sh`.
3. To undo the canonicalization: `rm` the symlinks in `utilities/sync/secrets/` and copy the files back from `secrets/pat/`.

## Constraints respected

- No secret values printed anywhere (helper output and tests use `****` redaction).
- No rotation.
- No visibility change, no force-push, no rebase, no publish.
- No `~/.git-credentials` or keyring changes.
- The helper is ~30 lines, single-purpose, no compatibility shims, no TODO, no dead code, no hidden assumptions; behavior change is documented in this report.
- The change is fully reversible: see "Reversibility" below.
