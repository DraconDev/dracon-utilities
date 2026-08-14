# Dracon Warden

Secret, encrypt, age, git-filter — repository hardening and smudge/clean encryption for Dracon workspaces.

This repository is the **canonical standalone source** for `dracon-warden` on
GitHub, GitLab, and Codeberg. It contains the source code, embedded security
crate, `Cargo.toml`, tests, examples, and release metadata.
You can build and install this utility directly from this repo.

## Quick start (standalone build)

```bash
# Clone this repo
git clone https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter.git
cd dracon-warden-secret-encrypt-age-git-filter

# Build
cargo build --release

# Install (binary lands in target/release/)
sudo cp target/release/dracon-warden /usr/local/bin/
```

## What is in this repo

- `src/` — utility source code
- `tests/` — integration tests (if present)
- `Cargo.toml` — standalone build manifest; the security kit is under `src/security`
- `README.md` — this utility's user guide
- `BLUEPRINT.md` — design notes
- `dracon-warden.example.toml` — example config
- `No systemd service; enforced through global git hooks.` — systemd user-service unit
- `LICENSE`, `SECURITY.md`, `.gitignore`, `.github/` — repo metadata
- `docs/SOURCE_OF_TRUTH.md` — architecture + invariants

## Relationship to the monorepo

| Boundary | Decision |
|----------|----------|
| Source code | This repository's `main` branch |
| Source of truth | This standalone repository |
| Workspace integration | Included by the `dracon-utilities` meta workspace when checked out under `dracon-warden/` |
| Shared libraries | Embedded `src/security` crate plus registry dependencies |
| Operational policy | `~/.dracon/utilities/` TOML files |

## Why this name?

The descriptive name is a deliberate choice for Codeberg/Forgejo, where
descriptive repo names get upvotes and free attention because readers
immediately know what the project does. The full word list (no fillers, no
audience/UX claims) is documented in
[`docs/design/github-feature-repos.md`](https://github.com/DraconDev/dracon-utilities/blob/main/docs/design/github-feature-repos.md).

## Purpose

Encrypts secret-shaped content at rest in git while preserving normal plaintext files in the working tree. Uses age encryption and git smudge/clean filters plus a pre-commit hook for plaintext-secret prevention.

## Runtime

- Binary: `dracon-warden`
- Service: No systemd service; enforced through global git hooks.
- Example policy: `dracon-warden/dracon-warden.example.toml`
- Common commands: `dracon-warden status · dracon-warden keygen · dracon-warden setup-hooks --global · dracon-warden scrub-markers`

## Maintenance

Changes are made in this standalone repository. The `dracon-sync` daemon
watches it and pushes configured remotes; the parent meta workspace does not
mirror source files into it.

## License

AGPL-3.0-only — see [LICENSE](LICENSE).
