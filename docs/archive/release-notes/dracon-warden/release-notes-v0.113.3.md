# dracon-warden v0.113.3 (2026-08-09)

Git filter encryption and repository hardening for secrets at rest.

## What's Changed

- Bump version to 0.113.3
- (See CHANGELOG.md for the full list of changes in this release)

## Install

```bash
cargo install dracon-warden --version 0.113.3
```

## Usage as a git filter (smudge/clean)

```bash
# In each repo you want to encrypt:
dracon-warden init
git config filter.dracon-warden.clean \"dracon-warden clean %f\"
git config filter.dracon-warden.smudge \"dracon-warden smudge %f\"
```

**Full Changelog**: https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter/compare/0.113.2...v0.113.3
