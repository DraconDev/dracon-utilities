# Hegemon state investigation — 2026-07-05

User question: "we are still excluding github but lets just investigate the
project we might need to clean up some older files or look in what is
the hangup."

## TL;DR — is the github exclusion the main hangup?

**No.** The github exclusion is a downstream symptom, not the root cause.

The actual hangup is upstream: **the asset regeneration pipeline keeps
adding blobs to the repo faster than any cleanup strategy can remove
them**. Pack grew from 1.98 GB → 2.23 GB in ~5 hours today. Working
tree shows evidence of multi-rounds regeneration (terrain-painted-v15
has versions v8, v9, v9b-v9k, .backup-r3, .backup-r7, .backup-r8,
.backup-r9, .backup-r9b-9k, .backup-r9k — **20+ variants of the same
asset**).

The github exclusion is mechanically required and will stay required as
long as the gen pipeline writes to `static/` and `git add`s the output.

**Top 3 prioritized next steps** (full list in §7):

1. **Fix the v0.95 cut leftover in engine code** — 195 svelte-check
   errors in 4 files (`init.ts`, `saveStore.ts`, `ai.ts`,
   `game.svelte.ts`). The cut deleted `data/buildings.ts` and the
   `/towns/[race]/` route but didn't update the engine to drop `RaceId`.
   ~2-4 hours to clean up.

2. **Clean 196 MB of pre-v1.0 .backup-* dirs** in static/assets/. These
   are local-disk leftovers from prior regen runs and are NOT in git.
   Won't shrink the github pack, but it's 196 MB of working tree
   cruft. ~30 min to `rm -rf`.

3. **Re-enable github** by migrating static/assets/ → OVH bucket.
   This is the durable fix per `binary-asset-strategy-2026-07-03.md`
   and `lfs-vs-bucket-vs-grow-2026-07-03.md`. Blockers: OVH
   credentials not loaded; `web/lifecycle.json` missing; 49 of 75
   gen-*.py scripts unported. Estimate: medium (3-5 sessions).

## 1. Is the github exclusion still mechanically required?

Yes. Re-measured today (2026-07-05 11:50):

- **Tracked blob total**: 2.23 GB (2,389,986,891 bytes / 3,643 blobs)
- **Largest pack file**: `pack-52524f2a6cad49f76d9150e16393dc426581ef80.pack`
  = 2,445,154,379 bytes = **2.28 GiB**
- **Total of all packs**: 4.0 GB (loose objects + 4 pack files)
- **github's hard limit**: 2.00 GiB per pack (verified at
  https://docs.github.com/en/get-started/working-with-large-files/conditions-for-large-files)

Pack grew from 1.98 GB → 2.23 GB today (+250 MB in ~5 hours). The
growth direction is unambiguous: **pack is growing, not shrinking**.

**Verdict**: The github exclusion `exclude_remotes = ["github"]` in
`/home/dracon/Dev/dracon-platform/web/games/wip/hegemon/.dracon/dracon-sync.toml`
is correct and remains required. The override is also defensive — there
is no `github` remote configured locally (`.gitmodules` declares one
but the worktree has no `remote.github.url`).

But: **the exclusion is downstream of the asset pipeline sprawl**. As
long as `gen-*.py` scripts write PNGs to `static/` and they get
committed, pack size grows.

## 2. Test suite health

Ran `bun test src/lib/game/smoke.test.ts`:

```
1383 tests  1130 pass  33 skip  103 todo  117 fail  16 errors
Pass rate: 81.7%
Time: 4.30s
Exit: 1
```

### 2.1 Failure categories

**104 fail — asset-existence** (8.5% of all failures): tests assert
PNG/JPG files exist at specific paths, but the files don't exist
because the regen script moved them. Examples:

- `static/assets/creatures-painted-v6-padded/` has 56 files (was)
- `static/assets/terrain-painted-v15/dirt.png` exists (was)

**8 fail — /new-game page** (v0.95 universal-cut UI regression): tests
expect "Race (8 cards) + Map Size sections" but the new /new-game
template only has Mode + Map Size + AI bonus selectors. The Race cards
were supposed to be removed; tests weren't updated.

**5 fail — miscellaneous**: visual-verification screenshot tests (8-12
files), v0.56 floor invariants (mtime check), v0.62 quality audit
(overlap with code-referenced assets).

**16 errors**:
- 14 are JS runtime errors (likely cascading from missing assets
  imported by components)
- 2 are `FileNotFoundError` on `mines-v2/wood-mill-sprite.png`

**103 todo — quarantined historical tests** (`docs/TEST-QUARANTINE.md`
explains): doc-missing historical version assertions, intentionally
quarantined 2026-06-28 (gap-closure T1.3). Not blocking; expected.

### 2.2 Top 3 high-priority failures

1. **`/new-game` SSR renders all 8 race names** — the v0.95 cut removed
   the Race section but the page still has the data binding (inert).
   When SSR runs, the rendering assertion fails. Root cause: v0.95 cut
   didn't fully remove the Race UI.

2. **`static/assets/creatures-painted-v6-padded/` has 56 files** — was
   true at one point; current state has 0 files. The gen-*.py script
   moved on to v8 without deleting the old path. Tests still point at
   v6-padded. Root cause: tests and assets out of sync.

3. **`mines-v2/wood-mill-sprite.png` FileNotFoundError** — the path
   was deleted from `mines-v2/` but tests/imports still reference it.
   Root cause: rename was committed but references weren't updated.

## 3. Source-code drift (v0.95 universal cut completion)

**Working tree**: clean (0 untracked, 0 modified; the dirty files I
saw initially were the active goal in `.pi/goals/`, not real drift).

**svelte-check**: **195 errors and 23 warnings in 21 files**.

### 3.1 Files with errors (concentrated in 4)

| File | Errors | What's wrong |
|------|-------:|--------------|
| `src/lib/game/state/game.svelte.ts` | 8 | `RaceId` references in MapTile, ownerHeroId, resourceEvent |
| `src/lib/game/systems/ai.ts` | 6 | RaceId still passed to AI town generation |
| `src/lib/state/saveStore.ts` | 1 | `SaveSlot` type has optional `race?` field that should be removed |
| `src/lib/game/init.ts` | 1 | RaceId still imported |
| `src/lib/game/smoke.test.ts` | 4 | Test fixtures reference per-race types |

Sample errors:
- `Error: Type 'undefined' is not assignable to type 'RaceId'.`
- `Error: Type '"universal"' is not assignable to type 'RaceId | undefined'.`
- `Error: Property 'ownerHeroId' does not exist on type 'MapTile'.`
- `Error: Property 'resourceEvent' does not exist on type 'MapTile'.`

### 3.2 v0.95 cut state

The cut was **partially complete**:

**Done**:
- `/new-game` UI: Mode + Map Size + AI bonus sections, no Race cards
- `/race-select`: thin redirect file (1107 bytes), will be deleted "in
  a later stage"
- `/towns/[race]/`: DELETED
- `data/buildings.ts`: DELETED (per-race building trees gone)
- `BUILDINGS_POOL = TOWN_BUILDINGS` alias in `races.ts` (universal
  pool)

**Not done**:
- `RaceId` still referenced in engine code (game.svelte.ts, ai.ts,
  init.ts)
- `SaveSlot.race?` optional field still in saveStore.ts
- 946 "race" mentions in src/ total (many are comments/data, but ~20
  are active code paths)
- /how-to-play still references /race-select
- v0.95 universal-cut tests for `startNewGame` (without playerHeroDefId)
  are passing because the function is now correctly written, but the
  old tests for the Race UI section still expect it

### 3.3 Why this matters

The cut succeeded at the **UI surface** (no race picker visible) but
not at the **engine layer**. Type-checking fails. If the operator
re-enables strict CI svelte-check, the build will fail.

The latent type errors don't block dev because `bun:test` doesn't
enforce types, and SvelteKit's dev server does its own type-erased
transformation. But this is technical debt.

## 4. OVH bucket migration readiness

The infrastructure is **built but not started for hegemon**.

### 4.1 What's built

- `web/scripts/ovh-bucket-ops.mjs` (27 KB, 2026-06-29) — full lifecycle
  CLI: list/replace/delete/gc/snapshot/policy
- `web/games/libs/platform/ovh-bucket.ts` — runtime loader, thin
  re-export of `@dracon/ovh-bucket` package
- `web/docs/OVH-BUCKET-OPS.md` — comprehensive operational reference
- `web/scripts/check-bucketing-compliance.mjs` — compliance gate
- `web/lifecycle.json` schema defined in OVH-BUCKET-OPS.md §1

### 4.2 What's missing for hegemon

- `web/lifecycle.json` itself is **missing** (compliance check fails:
  "LIFECYCLE POLICY MISSING")
- No `web/games/.env.ovh` file present
- No OVH secrets in `/home/dracon/.dracon/secrets/`
- No `GAMES_OVH_*` env vars loaded
- No bucket publish hook from hegemon's gen-*.py scripts (49 of them
  that target static/)
- No hegemon lifecycle entry: `hegemon: { "keep_last_n_versions": 1 }`

### 4.3 Work estimate

**Small** (1-2 sessions):
- Write `web/lifecycle.json` with hegemon entry
- Create `.env.ovh` with operator-supplied credentials
- Add a publish wrapper script

**Medium** (3-5 sessions):
- Update ~10-15 high-traffic gen-*.py scripts to publish to bucket
  instead of writing to `static/`
- Add the runtime loader ref to `src/lib/audio/`, `src/lib/components/`
- Verify hero portraits and music route through bucket
- Set `static/assets/` to gitignored (or at least the gen-*.py outputs)

**Large** (1+ week):
- Migrate ALL 49 gen-*.py scripts
- Replace every `static/assets/` reference with bucket URL
- Update svelte-check + smoke tests to point at bucket URLs

### 4.4 Blockers

1. **OVH credentials not loaded** — operator needs to provide
   `OVH_ENDPOINT` + `OVH_APPLICATION_KEY` + `OVH_APPLICATION_SECRET` +
   `OVH_CONSUMER_KEY` + `OVH_BUCKET` (or `GAMES_OVH_*` equivalents)
2. **lifecycle.json missing** — needs to be created with hegemon entry
3. **49 gen-*.py scripts unported** — they currently write to local
   `static/`, not bucket
4. **No automation for Layer-2 publish** — the design doc mentions
   "Layer 2 publish" but no implementation exists yet

## 5. Asset pipeline sprawl

- **Gen scripts**: 75 unique gen-*.py files
- **Total scripts**: 129 in scripts/
- **Static dir**: 824 MB on disk (grew from 684 MB yesterday)
- **Files**: 2,281 (2091 png, 76 mp3, 55 json, 51 jpg, 8 svg)
- **Backup dirs**: 42 .backup-* dirs in static/assets/ (NOT in git)
- **Backup dir disk usage**: **196 MB**

### 5.1 Top 5 backup dirs by size

| Backup dir | Size |
|------------|-----:|
| terrain-painted-v15.backup-r9k | 88 MB |
| creatures-painted-v3.backup-creatures | 75 MB |
| towns-3x3-mmx-v2.backup-r4 | 10 MB |
| towns-3x3-mmx-v2.backup-r5 | 5 MB |
| mines-v3.backup-r3 | 4.1 MB |

### 5.2 Per-asset-version sprawl

terrain-painted-v15 has these tracked versions in git:
v8, v9, v9b-v9k (10 variants), .backup-r3, .backup-r7, .backup-r8,
.backup-r9, .backup-r9b-9k, .backup-r9k (~20 variants of the same
asset).

### 5.3 Where the pack size comes from

```
Tracked blob total: 2.23 GB across 3,643 files
  - static/assets/: 824 MB local / ~1.99 GB tracked (regenerable)
  - static/assets/music/: 171 MB (mp3 files, committed)
  - static/assets/.backup-*: 196 MB local-only (NOT in git)
  - .pi/investigation/: ~50 MB (audit screenshots, in git)
```

### 5.4 What's regenerable

~47 gen-*.py scripts regenerate static/assets/* output from
`scripts/style-pipeline/<thing>.txt` prompts. The PNG output is
deterministic given the prompt and model — but the prompts themselves
have drifted over time (r3 → r9k means 10 prompt revisions).

## 6. Agent / goals backlog

- `goal_events.jsonl` size: 388 KB
- 4 .pi/goals/ entries, 2 active
- 1h activity: **3943 commits** (hegemon is being actively regenerated)
- 6h activity: 0 (the daemon commits are showing only 1h because the
  activity counter is per-hour, not rolling)
- 24h activity: 0 (same)

The 1h: 3943 number is the running daemon commit rate on hegemon. This
explains the pack growth.

## 7. Prioritized next-steps list

### Immediate (1 session, < 4 hours)

1. **Clean 196 MB of pre-v1.0 .backup-* dirs** (operator can run:
   `cd static/assets && ls -d *.backup* | xargs rm -rf`).
   *Impact: local-disk only, doesn't affect github pack.*

2. **Fix v0.95 cut leftover in engine code** — 195 svelte-check
   errors. Need to:
   - Remove `RaceId` references in `init.ts`, `ai.ts`,
     `game.svelte.ts`
   - Remove `race?` field from `SaveSlot` type
   - Update `MapTile` interface to drop `ownerHeroId` /
     `resourceEvent` fields
   *Impact: unblocks strict CI; no runtime change.*

### Short-term (next session or two, 1-2 days)

3. **Update or delete failing smoke tests** — 117 fails + 16 errors
   need attention:
   - 8 /new-game tests need rewriting for the universal UI
   - 104 asset-existence tests need either: (a) deleted if assets
     truly are gone, or (b) updated to point at current asset paths
   - 2 FileNotFoundError tests need path updates

4. **Add svelte-check to the test pipeline** — currently `bun test`
   is the only check. Add `npx svelte-check` as a hard gate so the
   195 errors don't accumulate silently.

### Medium-term (next week, 3-5 days)

5. **Re-enable github by migrating static/assets/ → OVH bucket.**
   Steps:
   - Operator: provide OVH credentials (`GAMES_OVH_*` env vars)
   - Write `web/lifecycle.json` with hegemon entry
   - Update top 10-15 gen-*.py scripts to publish to bucket
   - Add runtime loader integration in hegemon components
   - Move static/assets/ output to bucket; set gitignore
   - Verify pack size drops below 2 GiB
   - Remove `exclude_remotes = ["github"]` from hegemon's daemon
     config

### Long-term (1-2 weeks)

6. **Migrate ALL 49 gen-*.py scripts** to bucket
7. **Audit and prune historic tracked assets** (terrain-painted-v8
   through v9k, .backup-r3 through .backup-r10k, etc.). Could be
   done via orphan-branch rewrite (existing
   `auto_repair_concerns` path) or `git filter-repo` cleanup, but
   the operator has ruled this out per
   `binary-asset-strategy-2026-07-03.md`.
8. **Document the asset-pipeline migration in a single design doc**
   so future agents don't rediscover the same findings.

## 8. Findings not directly related to github

- **svelte-check is broken** with 195 errors. The dev workflow works
  around this; CI does not.
- **/new-game SSR test fails** — the v0.95 cut removed the visible
  Race UI but left the data binding inert. SSR still renders the
  data and fails the test.
- **mines-v2/wood-mill-sprite.png FileNotFoundError** — a renamed
  asset whose references weren't updated.
- **103 todo tests are quarantined** (intentional, per
  `docs/TEST-QUARANTINE.md`). Not a problem.
- **Working tree is clean** (the .pi/goals/* files are goal state,
  not real drift).

## 9. Direct answer to the user's question

> "is the github exclusion the main hangup?"

**No, the github exclusion is downstream.** The main hangup is the
asset regeneration pipeline writing to `static/` (which then gets
committed), causing unbounded pack growth. The exclusion is correct
given the current architecture, but it's not the architectural
problem — the architectural problem is that there's no bucket-publish
hook, so every gen-*.py run adds ~50-250 MB to the pack.

The most leveraged next step is **fix the v0.95 cut leftover in the
engine** (195 svelte-check errors → 0 in a few hours). The most
durable next step is **wire the OVH bucket + port the gen scripts**.

## 10. Files

This doc: `docs/design/hegemon-state-investigation-2026-07-05.md`
(composed 2026-07-05, committed and pushed separately).

Related docs in `docs/design/`:
- `full-audit-2026-07-05.md` — 26-repo push health audit
- `binary-asset-strategy-2026-07-03.md` — why bucket is the right
  answer for binary assets
- `lfs-vs-bucket-vs-grow-2026-07-03.md` — comparison of LFS vs bucket
- `nested-on-main-architecture-2026-07-02.md` — the standalone removal
  migration

Related docs in `docs/design/` (parent repo):
- `/home/dracon/Dev/dracon-platform/web/docs/OVH-BUCKET-OPS.md` —
  full lifecycle CLI reference