# dracon-warden v0.113.1 — full-audit remediation batch 2 (hook layer + smudge)

Released 2026-07-26. Remediation batch 2 of
[`AUDIT_FULL_2026-07-26.md`](./AUDIT_FULL_2026-07-26.md): 3 HIGH
(WARDEN-H1, H2, H3) + 1 MEDIUM (WARDEN-M2). Each was reviewed
independently against the source before acceptance; two of the four
were repaired beyond the original patches (see "repaired during
review" below). All behavioral verification ran against real scratch
repos with the real hooks as shell subprocesses (not mocks).

## Fixed

### WARDEN-H1 — production filter-smudge still corrupted whole-file-encrypted binary secrets

The 2026-07-21 H9 regression was only fixed in helpers the binary
never calls. `DraconWarden::smudge` / `Warden::smudge` went straight
to `String::from_utf8_lossy` — every invalid-UTF-8 byte of a
decrypted binary secret became U+FFFD → corrupted worktree → next
clean re-encrypted the corruption.

Both entry points now delegate to a shared `smudge_with_security`
that tries `decrypt_whole_file_tag` FIRST and returns raw bytes.
The new byte-identical round-trip test goes through the production
entry-point path (the old test exercised the helper directly and
passed while production stayed broken).

### WARDEN-H2 — global pre-commit hook hard-blocked commits in every non-hardened repo

The operator's `core.hooksPath = ~/.config/git/hooks` shadowed all
repo-local hooks fleet-wide, and the global `pre-commit` exited 1
unless `.gitattributes` contained `filter=dracon` — so every
third-party clone, every scratch repo, every non-wardened repo on
the machine refused to accept commits.

The hook now:

- **Chains to an existing repo-local `pre-commit`** (anti-recursion
  via the warden header marker in the seeded snippet).
- **No-ops unless the repo is warden-managed** — detected by
  repo-LOCAL `filter.dracon.clean` config (the global
  `~/.gitconfig` had the key, defeating the check), `filter=dracon`
  in `.gitattributes`, or a `.dracon/` dir.

Managed-drift (some markers present, `.gitattributes` missing) still
blocks.

**Repaired during review**: the original patch's "managed-marker"
check used `git config --get filter.dracon.clean`, which reads
global config — but the operator's global `~/.gitconfig` already had
the key, so the check was dead. Now uses `git config --local --get`.

### WARDEN-H3 — pre-rebase `head -100` checked the NEWEST 100 commits

`git rev-list` is newest-first, so the cap dropped the OLDEST
commits — precisely those most likely already published. Replaced
with a boundary-commit check (remote containment is
ancestor-closed: if the oldest commit of the range is on no
remote, no newer one can be). One `git branch -r --contains`
instead of up to 100 subprocesses.

Same edit fixes WARDEN-M17: the range tip is now
`${2:-HEAD}` (the two-argument form `git rebase <upstream> <branch>`
previously computed an empty `$1..HEAD` range and passed while
published `$2` commits were rewritten).

### WARDEN-M2 — pre-push secret scan missed single-quoted secrets

`\x27` is not a hex escape in GNU grep ERE (the class became
`["x27]`, matching literal x/2/7 instead of a single quote).
Replaced with the shell `'\''` idiom; verified against GNU grep
3.12: a single-quoted `password =` or `api_key =` assignment now matches; values
containing x/2/7 do not false-positive. E2E: a push adding an
`api_key = '<redacted>'` assignment with a live-looking value
(e.g. `sk-live-123`) is refused.

## Tests

- 92 → 93 unit tests in `dracon-warden` (all green; integration
  suite unchanged at 10/10).
- Behavioral verification: 8 scratch-repo scenarios (incl. the M17
  two-arg form, the H2 un-managed-repo passthrough, the H2
  managed-drift block).

## Upgrade notes

- Re-run `dracon-warden setup-hooks` after upgrade — the
  `pre-commit` hook content changed (chains to repo-local + uses
  `--local` config probe).
- The pre-rebase hook is unchanged in shape but its commit-scan
  logic is now boundary-based. No operator action needed beyond
  the hook re-install.

## Audit cross-references

- `AUDIT_FULL_2026-07-26.md` — §"WARDEN-H1 / H2 / H3 / M2"
- `docs/design/incident-amend-race-and-trust-2026-07-25.md` —
  §"F0.1 follow-up" (the hook this batch repaired)