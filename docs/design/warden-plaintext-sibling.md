# Warden plaintext-sibling escape hatch

**Date:** 2026-06-06
**Status:** Approved
**Scope:** `dracon-warden` only (no changes to `dracon-sync` or `dracon-system`)

## Design (minimal)

A file is treated as **intentionally plaintext** when a sibling file with the
literal suffix `.plaintext` exists next to it in the working tree:

```
config/example.env           ← the secret
config/example.env.plaintext ← exists ⇒ skip encryption
```

That's the entire hatch. There is no central manifest, no TOML config entry, no
CLI subcommand. The user runs `touch <path>.plaintext` to opt a file in, and
`rm <path>.plaintext` to revoke.

## Enforcement surface

Every warden enforcement point that would normally encrypt a file MUST check
for the sibling first and short-circuit if it is present:

1. **Clean filter** (`smart_clean_with_path` in `dracon-security/src/modules/filter.rs`)
   — if `<path>.plaintext` exists, return the input unchanged (no encryption,
   no version header, no marker scrub). Plaintext is stored as-is in the git blob.
2. **Smudge filter** — no change; smudge only acts on already-encrypted blobs.
3. **pre-push hook** — for each regex match in the diff, check whether the
   originating file has a `.plaintext` sibling. If so, silently allow. If not,
   fail as today. Net behavior: a push with only hatched plaintext is silent;
   a push with un-hatched plaintext still fails.
4. **`scrub-markers`** — skip files that have a `.plaintext` sibling (the
   plaintext content is intentional, not a leaked marker).
5. **`resmudge`** — skip files that have a `.plaintext` sibling (no decryption
   needed because the file is not encrypted).
6. **`harden_repos`** — no change; the managed `.gitattributes` block does not
   need to know about individual hatches.
7. **`setup-hooks`** — produces the updated `pre-push` hook; existing hook
   installs are upgraded on next `setup-hooks --global` / `--local`.

## What this does NOT protect against

- The plaintext is stored verbatim in the git object database. Anyone with
  read access to the repo (or a future leak) can read it.
- The audit trail is implicit: the only record that a file was intentionally
  plaintext is the existence of `<file>.plaintext` in the working tree /
  commit history. There is no `reason` field, no expiry, no author.
- The hatch is per-file, not per-secret-value. A hatch on `config/example.env`
  applies to the whole file's content, not just one line.
- `.plaintext` siblings are themselves plaintext tracked files; they do not
  bypass the encryption filter, but they typically contain nothing of value.
- `dracon-warden repair --strict` does NOT have a hatch mode: it still
  reports plaintext committed without a `.plaintext` sibling, so
  "I forgot to add the sibling" remains visible.

## Revocation

`rm <path>.plaintext` → next `git add` triggers the clean filter → file is
encrypted on the next commit. The `.plaintext` sibling itself can be removed
from the repo with `git rm <path>.plaintext`.

## Threat model

- **In scope:** solo or small-team repos where a handful of files contain
  intentionally-public values (example keys, fixture data, benchmark
  datasets) that should not be encrypted.
- **Out of scope:** any secret that has *any* confidentiality requirement.
  The hatch is the wrong tool for those — keep them encrypted, distribute
  the decryption keys via the normal `dracon-warden keygen` / team-key flow.

## Files changed

- `dracon-warden/src/main.rs` — clean filter, pre-push hook, scrub-markers,
  resmudge
- `dracon-warden/src/tests.rs` — new tests
- `dracon-warden/BLUEPRINT.md` — section
- `README.md` — CLI table, encryption table
- `CHANGELOG.md` — Added entry
- `dracon-warden/CREDENTIALS.md` — when plaintext is appropriate
