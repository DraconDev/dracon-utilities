# Release Notes — v0.112.10 (2026-06-17)

> **Headline**: the daemon now commits EVERYTHING untracked by default.
> The `untracked_exclude_patterns` list is empty. Short-lived files
> (`.pi-tmp/`, `scratch/`, `tmp/`, `.demon/`, `.sisyphus/`, `.ralph/`)
> are valid git content — the user/agent manually deletes them when
> they're done.

## What changed

### Policy: global `untracked_exclude_patterns = []`

The operator's framing (lightly cleaned up):

> "the most sensible thing is that we have a global rule, and unless
> it's something that would be very wrong to put on the repo we put
> it there. i think all untracked excludes arguably are wrong. just
> because they are short lived files doesn't mean we shouldn't put
> them there."

The previous list of 11 patterns conflated "short-lived" with "very
wrong to commit". They are not the same thing. The new policy:

```text
DEFAULT BEHAVIOR:
  The daemon commits ALL untracked files by default.

THE ONLY THINGS IT REFUSES TO COMMIT:
  1. Files > 100 MiB (max_stage_file_bytes = 104857600)
  2. Things git already ignores (.gitignore rules)
  3. Per-repo opt-outs (only when a specific repo sets
     untracked_exclude_patterns in its .dracon/dracon-sync.toml)

NOT REFUSED (despite being short-lived):
  .pi-tmp/    scratch/    tmp/
  .demon/    .sisyphus/    .ralph/
```

### Why this matters

- **Disaster recovery**: a 10-minute audit that you accidentally
  delete from the working tree is still in git history.
- **Cross-machine sync**: the 4-remote push now carries ALL your
  work, not just the "important" files.
- **No more "UT pile-up"**: the daemon's `untracked` count drops
  to 0 for repos with no `.gitignore` or >100 MiB files. The
  "untracked-only" state in the daemon's `repos` table disappears.
- **User-controlled cleanup**: when the user is done with a
  `.pi-tmp/` directory, they `rm -rf` it. The daemon commits the
  deletion. If they want it back, it's in git.

### What is NOT changed

- `auto_commit = true` (was already on)
- `max_stage_file_bytes = 104857600` (100 MiB cap unchanged)
- `.gitignore` rules (unchanged — build artifacts still ignored)
- Warden's pre-commit hook (still scans all staged content for
  secret patterns; encrypts-or-blocks)
- Per-repo `.dracon/dracon-sync.toml` override mechanism (now
  reserved for repos that need to opt back INTO excluding
  something)

## Files changed

- `~/.dracon/utilities/sync/dracon-sync.toml`: `untracked_exclude_patterns`
  changed from 11 patterns to `[]` (with extensive comment explaining why)
- `AGENTS.md`: commit policy section rewritten with the new framing
- `CHANGELOG.md`: this entry
- `docs/design/pi-tmp-persist-policy-2026-06-16.md`: rewritten to
  reflect the global change (was per-repo pilot, now fleet-wide)

## Verification

- All 12 daemon-watched repos are ✅ OK + 🟢 synced + healthy
- 4-remote alignment verified for `.dracon`, `dracon-utilities`,
  `dracon-platform`
- `cargo build --release --locked` succeeds (5 pre-existing warnings,
  no new ones)
- `cargo test --workspace --locked` expected: 856 passed, 0 failed,
  9 ignored (no regression)

## Sub-crate versions

- `dracon-sync`: 0.1.10 → 0.1.11
- `dracon-system`: 0.2.5 → 0.2.6
- `dracon-warden`: 0.3.5 → 0.3.6

All 3 sub-crates will be re-published to crates.io as v0.1.11, v0.2.6,
v0.3.6 immediately after this release. (No code change — same source,
new version metadata.)

## Follow-up

The 60s `push_op_timeout_secs` is a pre-existing limitation surfaced
during this work (game-dev smoke-out PNG commits can take >60s to push
to gitlab/codeberg). Deferred to a separate goal.
