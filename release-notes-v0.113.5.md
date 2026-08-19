# dracon-warden v0.113.5 (2026-08-19)

**Status:** local release candidate prepared from the locked standalone
checkout. Registry publication, tag creation, and forge release remain
explicit operator-approved steps.

## What's changed

- **Narrow machine-local hygiene defaults:** omitted `hygiene_patterns` now
  covers Pi harness/runtime state (`**/.pi*`), Chromium trace output
  (`**/chrometrace.log`), and regeneratable frontend caches
  (`**/.svelte-kit/`, `**/.vite/`, `**/.turbo/`, and `**/.cache/`).
- **Explicit overrides remain compatible:** `hygiene_patterns = []` still
  disables the default list for an operator who deliberately wants that
  behavior.
- **No broad log default:** Warden does not ship a blanket `*.log` rule.
  Existing repository-local rules outside Warden's managed block remain
  untouched.
- **Managed ignore propagation:** hardening preserves operator content while
  replacing only the Warden-managed block, so the machine-local baseline can
  be applied consistently across nested repositories.
- **Previously accumulated security and hook hardening:** this release also
  carries the unreleased recipient authorization, whole-file binary handling,
  hook chaining, tag-push scan, V1 fail-closed, merge-driver, scanner, and
  race-safety fixes documented in `CHANGELOG.md`.

## Verification

The candidate is built with the committed lockfile:

```bash
cargo fmt -- --check
cargo test --workspace --locked -- --test-threads=1
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo build --release --locked
scripts/verify-install.sh "$HOME/.local/bin/dracon-warden"
```

For a source checkout, install the artifact atomically after the build:

```bash
install -d "$HOME/.local/bin"
tmp="$(mktemp "$HOME/.local/bin/.dracon-warden.XXXXXX")"
install -m 0755 target/release/dracon-warden "$tmp"
mv -f -- "$tmp" "$HOME/.local/bin/dracon-warden"
```

## Current release boundary

This file records the source and local-build candidate. Do not use
`cargo install dracon-warden --version 0.113.5`, create `v0.113.5`, or publish
to a registry until the operator explicitly authorizes those external steps.
