# Post-gate push stall: systematic design (2026-11-04)

> **Status**: hook fix + spool reclaim SHIPPED 2026-11-04.
> Daemon tranche/pacing knobs SPECIFIED here, not yet implemented
> (needs full-workspace gate run per AGENTS.md test discipline;
> sibling crate is mid-rewrite).

## The stall, restated correctly

The live incident (browser-extensions-shared, 2026-11-04) was not "five
separate defects" and not the earlier lockfile-only blocker. The push
pipeline on that repo combined four independent mechanisms, each of which
seemed sufficient to block alone:

| Stage | Mechanism | Wall clock |
|---|---|---|
| `git push` | pre-push hook: 48-workspace 4-gate on offer tip | ~80 min |
| hook lease | preparation lock wait budget = 300 s, holds reach 887 s | exit 75 |
| hook cache | checkpoint key includes the revision SHA | 0% reuse on tip move |
| daemon | strict-series pre-push serialised head-of-line | queue behind 1 gate |

Any one of the middle two guarantees the fourth. The hook change
(`scripts/git-hooks/run-push-gates.mjs`) below removes the SHA from the
checkpoint key; the spool script below reclaims 5.5 GB of epoch leftovers
that the daemon's unqualified lossless path had preserved forever.

## What shipped (this change)

### 1. Tip-delta cache (hook, browser-extensions-shared)

`workspaceContentKey()` replaces `{...retryIdentity, sha, ...}` as the
checkpoint identity. The key is the workspace's content: blob OIDs of its
own tracked files + its transitive local-dependency workspaces + the
lockfile + the top-level manifest.

- Tip moves, leaf bytes unchanged → green receipt reuses (tip-delta).
- Any touched file → new key → full gates, as before.
- Lease-hold expiry and the 24 h receipt freshness bound are untouched.
- Dependencies are in the closure, so a library change invalidates its
  consumers (the old `isDependency` always-rerun rule still applies on top).

Tests: `tip advance with an unrelated change reuses the untouched leaf
receipt` (tip moves, untouched leaf reuses); `changing a dependency
workspace invalidates its consumer receipt (closure)`. All 17 non-toolchain
tests pass (the 6 toolchain-pinning tests are pre-existing slow: they pin
`bun@1.3.14`, the host has `1.3.3`, so each re-downloads the toolchain).

### 2. Verified spool reclaim (daemon-nested repo script)

`dracon-sync/scripts/spool-reclaim-verified.py` — the operator-maintenance
bundle+delete the user approved, executable only under
`dracon-sync maintenance --`. It deletes a spool only after proving every
object in its retained packs is already in the main object store
(`git cat-file --batch-all-objects` vs `git verify-pack -v`); missing
objects first get their pack bundled + manifest to the backup dir.
`preserve-rejected` / `original-index` markers are never touched.

Run 2026-11-04 on browser-extensions-shared (2,816 spools found):
2,815 deleted (5,467,798,095 bytes), 0 bundles needed (all 2815 verified
fully duplicated), 3 skipped (2 index-only live-cohort, 1 retained-pack
spool under an active reclaim). Backup dir created but unused:
`~/dracon/backups/spool-bundle-browser-extensions-shared-2026-11-04/`.

## Specified, not yet implemented

### 3. Push tranches (daemon, `publication.rs` / policy)

Strict-series runs the whole `remote..local` range through the hook once.
With a deep backlog the affected set is "everything touched in N commits",
re-run per retry. Tranche the publication: publish the first
`push_tranche_commits` commits as an intermediate tip (fast-forward,
hook revalidates just those), then the next tranche, etc. Each tranche is
a normal fast-forward push, so the hook cache (once content-addressed)
makes later tranches cheap.

- New policy knob `push_tranche_commits: Option<u64>` (default `None` =
  no truncation; global-only first, per-repo override per the AGENTS.md
  tripwire rules).
- `publication.rs::sync_repo_publication_at`: cap the offered tip to
  `remote + N` when ahead > N and no failure/outstanding blocker.
- Tripwire: `test_repo_override_field_coverage_tripwire` + consumption
  test + end-to-end (a 3xN backlog publishes in N tranches, converges to
  the same final tip).

### 4. Producer pacing (loop/agent policy)

Daemon-side pacing cannot stop a loop agent from landing 160 commits/hour,
but the queue math can be made boring: with (1), each push only pays for
bytes that changed since the last green receipt, so producer rate stops
being the binding constraint. Loop AGENTS.md files in daemon-watched
repos should still say: land small, green units; do not stack > 1 h of
unpushed work without a clean pull. The 4-gate is the backstop, not the
throttle.

## Invariants preserved

- Failed/partial gate sets never seal a receipt (unchanged).
- Receipt freshness (24 h) unchanged; content key does not extend trust.
- Lease budget (900 s) unchanged by this design; the hook keeps failing
  closed at exit 75 with a detectable expiry message.
- No receipt reuse across runners/toolchains (runner identity still in key).
- Spool deletion never touches `preserve-rejected` / `original-index`.
