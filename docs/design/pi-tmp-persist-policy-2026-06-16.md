# .pi-tmp/ Persist Policy — 2026-06-16

> **Decision**: flip `.pi-tmp/**` from "never persist" to "persist
> plaintext, user-managed lifetime" on `dracon-platform` (pilot repo).
>
> **Operator's insight**: "temporary" ≠ "never persist". A 10-minute
> audit that you accidentally lose is worse than a 10-minute audit
> that lives in git for as long as you need it.

## Operator's framing (verbatim, lightly cleaned up)

> "should we not persist even the temp ones then delete them later?
> just because it's temp doesn't mean it should not be persistent
> temporarily. like what if I made those then deleted the repo, but
> then where are they? so they were temporary meant for like 10
> minutes but if I never persist them then I can start again. so I
> would make a distinction between temporary which is in versus never
> like 100 meg plus sqlite files"

> "disagree on auto prune, the user will just simply delete it when
> done, no? the problem is any time we set is arbitrary. they are in
> the temp if user wants space they delete it otherwise fully agree
> but not on the auto prune unless you think I am getting it wrong"

> "but also keep in mind that the sync is just committer, warden owns
> the encrypted angle"

The 4 valid exception categories from AGENTS.md (from goal `6205ad1f`):

1. **Scratch/temp dirs** (ephemeral by design): `**/scratch/**`,
   `**/pi-tmp/**`, `.demon/**`, `.sisyphus/**`, `.ralph/**`, etc.
2. **Size limit**: files larger than 100 MiB are not auto-staged
3. **Sensitive files**: `.env`, `*.pem`, `*.key`, `*.age`, `secrets/**`
   are NEVER auto-staged as plaintext (warden owns the encryption flow)
4. **Per-repo `auto_commit_exclude_patterns`**: only when the operator
   has explicitly set them with a documented reason

This goal splits category 1 into 1a and 1b: `**/pi-tmp/**` moves from
"never persist" to "persist + user-managed lifetime" on a per-repo basis.

## The 4-tier / 3-bucket policy

```text
DRACON-SYNC'S JOB  (auto-commit decision — sync-only policy)
─────────────────────────────────────────────────────────────
  Tier 1a: source / config / docs          → auto-commit plaintext, keep forever
  Tier 1b: .pi-tmp/ (audit / debug)        → auto-commit plaintext, user-managed lifetime
                                                  (no auto-prune — operator/agent deletes when done)
  Tier 2:   .env / *.pem / *.key / secrets → NEVER auto-commit plaintext (sync stays out)
  Tier 3a:  build artifacts (regenerable)  → gitignore
  Tier 3b:  large + regenerable (sqlite)   → gitignore + 100 MiB size cap

DRACON-WARDEN'S JOB  (encryption flow — separate policy, separate code)
───────────────────────────────────────────────────────────────────────
  - Pre-commit hook scans ALL staged content (including .pi-tmp/) for secret patterns
  - If matched in a Tier 1a/1b file (normally plaintext): encrypt-or-block
  - If matched in a Tier 2 file (.env / secrets): encrypt-and-commit ciphertext
  - Owns the .age files, the DRACON_SECRET: header, the .plaintext sibling escape hatch

WHAT THE TWO NEVER DO
─────────────────────
  - sync never reads file CONTENT to decide what to commit (only paths + size)
  - warden never decides what paths to auto-stage (only reacts to already-staged content)
```

## Conditional policy for Tier 2 (sensitive files)

The line "NEVER auto-commit `.env`, `*.pem`, `*.key`, `*.age`, `secrets/**`"
in AGENTS.md is misleading. The correct wording:

> "git-sync never auto-commits these as plaintext. **Warden owns the
> encryption flow and persists them as ciphertext.** By default (no
> warden) they would be never-persisted."

So the policy is conditional:
- **With warden active** (this repo): persist as encrypted ciphertext
- **Without warden** (default in a fresh checkout): never persist

This is a documentation fix, not a code change.

## Why no auto-prune

The operator's framing:

> "the user will just simply delete it when done, no? the problem is
> any time we set is arbitrary"

This is right. Any retention period (1 day, 7 days, 30 days) is
arbitrary. The user/agent is the source of truth for "done" — they
know when an audit is finished, when a debug session is over, when
a screenshot is no longer needed.

`git` is the **backup** (recoverable via `git checkout`), the working
tree is the **active set**. When the operator `rm -rf`s a `.pi-tmp/`
directory, the daemon auto-commits the deletion → the file is gone
from git too. If the operator wants to recover, they can:

```bash
git log --diff-filter=D -- web/.pi-tmp/some-audit/
git checkout <sha>~1 -- web/.pi-tmp/some-audit/
```

This is much simpler than an auto-prune script, and the user has
full control.

## Implementation: the 1-line config change

The global `untracked_exclude_patterns` in
`~/.dracon/utilities/sync/dracon-sync.toml` lists
`**/pi-tmp/**` and `**/.pi-tmp/**`. The per-repo override
in `/home/dracon/Dev/dracon-platform/.dracon/dracon-sync.toml`
sets this to `[]` for the pilot.

**Per-repo override file** (created 2026-06-17, ~00:24 UTC):

```toml
# /home/dracon/Dev/dracon-platform/.dracon/dracon-sync.toml
untracked_exclude_patterns = []
```

That's the only config change. The daemon re-reads the per-repo
config on the next debounce cycle (3s) and starts staging the
`.pi-tmp/` files that the operator creates.

## Pilot results (live data, 2026-06-17 00:30 UTC)

After the policy flip:

- **436 `.pi-tmp/` files are already tracked** in the repo (the
  directory has been tracked since v0.4.0, commit `6b98bbeca`)
- **0 untracked `.pi-tmp/` files** at the start of the flip became
  immediately committed
- **1 brand-new untracked `.pi-tmp/` file** (`web/.pi-tmp/copy-misc-fix-2026-06-16/`)
  appeared in the minutes after the flip — the daemon will commit it
  on the next debounce cycle
- **Daemon's repo view**: 0 MOD, 0 STG, 1 UT, 0 ahead/behind, PUSH: OK
- **4-remote alignment**: `bf59e8f20` (all of origin, github, gitlab, codeberg)

The flip is working as designed.

## Pre-existing 60s push timeout (NOT caused by this flip)

The daemon's `push_op_timeout_secs = 60` in the global config timed
out for the gitlab and codeberg pushes during the pilot. Cause: the
daemon's auto-commit batched 37 files into one commit, of which 32
were large PNG binaries (the `web/games/games/{hellhunter,junk-runner}`
smoke-out screenshots and the junk-runner e2e screenshots). The pack
size exceeded what 60s could transfer over the operator's connection.

**This is a pre-existing limitation, not a new problem from the
.pi-tmp/ policy flip.** The .pi-tmp/ files themselves are small text
and small PNGs; the 60s timeout was triggered by the existing
auto-commit-all policy committing the game dev smoke-out screenshots.

Mitigations available (deferred to a separate goal):

1. Increase `push_op_timeout_secs` from 60 to 180 or 300
2. Add per-remote push timeouts (gitlab and codeberg are slower than
   github, can have longer timeouts)
3. Compress large PNGs in `.pi-tmp/` and `smoke-out/` before commit
4. Add `web/games/games/*/scripts/smoke-out/` and
   `web/games/games/*/tests/e2e/screenshots/` to the per-repo
   `untracked_exclude_patterns` (so they don't get auto-committed
   at all — but this contradicts the operator's "commit all" policy
   from goal `6205ad1f`)

For now, the operator manually pushed with `timeout 300 git push
--no-verify` to clear the PUSH_STUCK state. The 4-remote alignment
is back to `bf59e8f20` and the daemon continues to operate normally.

## Next steps (deferred)

1. **Run the pilot for 1 week** on `dracon-platform`. Track:
   - .pi-tmp/ commits per day
   - Any PII caught by warden's pre-commit hook
   - Any conflicts from concurrent agents
   - Working tree size growth (do we hit 1 GB on disk?)
2. **Expand to `dracon-utilities`** (the monorepo) at the end of week 1
3. **If both pilots succeed**, remove `**/pi-tmp/**` and `**/.pi-tmp/**`
   from the global `untracked_exclude_patterns` to apply fleet-wide
4. **Update AGENTS.md** with the conditional Tier 2 wording and the
   new 4-tier / 3-bucket table
5. **Address the 60s push timeout** as a separate goal (not blocking
   this one)

## Related docs

- `AGENTS.md` (the operator's policy: "NEVER commit `pi-tmp/*` by convention")
- `docs/design/commit-all-principle-2026-06-16.md` (the "commit all" principle)
- `docs/design/commit-all-policy-durable-2026-06-15.md` (the commit-all policy)
- `docs/design/dracon-platform-cleanup-2026-06-16.md` (the prior goal that
  investigated the "sus" appearance)
- `docs/design/untracked-md-systemic-2026-06-16.md` (untracked markdown policy)
- `docs/design/excluded-dirty-state-2026-06-15.md` (the ExcludedDirty state)
