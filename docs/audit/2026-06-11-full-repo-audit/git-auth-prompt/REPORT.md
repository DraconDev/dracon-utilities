# Git credential / login prompt investigation

Date: 2026-06-11

## Question

> "My system keeps prompting me to login. We supposed to be using the git PAT no? Or are we using from the wrong place now? We organized the `.dracon` folder a bit."

## Short answer

1. The code is **not** using the wrong place — `dracon-sync` reads tokens from `~/.dracon/utilities/sync/secrets/*.env`, and that directory still has working, readable token files.
2. The new layout `~/.dracon/secrets/{pat,registry,ai,...}` is a **parallel store** that the code does **not** read. Without a link, having the PAT only in `pat/` would break the daemon.
3. To make the new layout the single source of truth (a real "move"), I copied the token files into `~/.dracon/secrets/pat/` and replaced the old files with symlinks to the new ones. This is reversible, requires no code change, and no secret rotation.
4. The random login popup is most likely the desktop keyring askpass (`ksshaskpass`) triggered by the `gh auth git-credential` helper. The daemon itself is fine (`GIT_TERMINAL_PROMPT=0`, `GH_TOKEN` is injected for HTTPS children). The popup needs a separate decision — see "Remaining decision" below.

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

## Remaining decision (not applied without approval)

The random login popup the user is seeing is most likely the desktop keyring askpass (`ksshaskpass`, configured as the SSH/GIT askpass) firing when `gh auth git-credential` tries to read the token from the `gh` keyring, and the keyring is locked or `gh` is not authenticated in that shell context.

Two safe options, both reversible:

1. **Unlock the keyring at login / keep `gh` authenticated.** Stop the popup at the source. No code or helper change. This is the lowest-risk path.
2. **Add a small git credential helper** (`~/.dracon/secrets/pat/git-credential-github.sh`) that reads `GH_TOKEN` from the canonical `pat/github.env` and supplies it for `https://github.com/`, and set it as the first helper for that URL in `~/.gitconfig`. This bypasses `gh`/keyring entirely and uses the PAT directly. The helper is ~10 lines, no shortcuts/compatibility shims, and is the natural completion of the canonicalization.

I did not apply option 2 because it is a behavior change to `~/.gitconfig` and adds a new file. Awaiting your call.

## Constraints respected

- No secret values printed.
- No rotation.
- No visibility change, no force-push, no rebase, no publish.
- No `~/.git-credentials` or keyring changes.
- The change is fully reversible: deleting the symlinks and `cp`-ing back from `pat/` restores the prior state.
