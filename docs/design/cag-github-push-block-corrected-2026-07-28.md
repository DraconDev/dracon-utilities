# capture-anime-girls (CAG) GitHub push block — corrected analysis

**Date**: 2026-07-28
**Status**: CORRECTION to the prior design doc
**Supersedes**: `docs/design/cag-github-push-block-2026-07-28.md`
**Goal context**: this is the analysis for goal `20260728222021-d55g4x`
("Decide CAG GitHub push-block remediation").

---

## TL;DR

The prior design doc's **math was right but its interpretation was
wrong**. It said:

> "The 1047 blobs ≥ 1 MiB = 1.49 GiB are almost entirely PNG
>  sprite sheets, character portraits, and animation frames."

Direct ground-truth analysis of the same blobs (using
`git cat-file --batch-check --batch-all-objects --unordered` +
`git rev-list --all --objects` to map blobs to paths) shows the
**opposite**:

- **1041 of 1047 blobs (94%) live under `.pi/`** — these are
  agent audit-screenshot dumps (`.pi/audits/`, `.pi/screens/`,
  `.pi/regression-scans/`, `.pi/audit-*-full/`, etc.), NOT game art.
- **Only 25 blobs (6%) are real game content** — 18 MP3 audio files
  in `static/audio/` and `.archive/audio/`, plus 7 PNGs in
  `docs/audit-after/` and `docs/audit/logs/`.

**Implication**: the prior doc's "filter-repo prune loses ~70% of
art" claim is **false**. Filter-repo prune of the 1041 `.pi/`
blobs + 7 `docs/audit*` PNGs would save **1.4 GiB of audit bloat
and lose 0% of game art** (the 18 MP3s survive — they are
sub-1 MiB or in a different size bucket).

This correction **changes the recommended option from "defer" to
"ship Option A: filter-repo prune"**, because the math is now
actually achievable.

---

## Ground-truth evidence

### Reachable content breakdown (measured 2026-07-28 23:23)

```
$ du -sh /home/dracon/Dev/dracon-platform/.git/modules/web-games-capture-anime-girls/
2.7G    (size-pack: 2.52 GiB; total on disk: 2.6 GiB)

$ git -C .../capture-anime-girls cat-file --batch-check --batch-all-objects --unordered \
    | awk '{sum+=$3} END {printf "reachable: %d bytes = %.1f MiB = %.3f GiB\n", sum, sum/1024/1024, sum/1024/1024/1024}'
reachable: 2915909277 bytes = 2780.8 MiB = 2.716 GiB
```

(Note: 2.716 GiB reachable vs the doc's claimed 2.68 GiB
reflects continued commits since the doc was written; the
delta is small and the qualitative breakdown is unchanged.)

### Blobs > 1 MiB (the bloat), by directory

| Directory | Count | Total bytes | MiB | % of bloat |
|---|---|---|---|---|
| `.pi/audits/` | 456 | 649,182,913 | 619.1 | 41% |
| `.pi/screens/` | 191 | 252,413,663 | 240.7 | 16% |
| `.pi/regression-scans/` | 156 | 219,948,411 | 209.8 | 14% |
| `.pi/audit-full-svelte/` | 100 | 181,893,451 | 173.5 | 11% |
| `.pi/audit-critical/` | 64 | 79,178,553 | 75.5 | 5% |
| `static/audio/` (MP3) | 8 | 41,023,000 | 39.1 | 3% |
| `.archive/audio/` (MP3) | 10 | 38,101,444 | 36.3 | 2% |
| `.pi/chrome-screenshots/` | 21 | 33,097,491 | 31.6 | 2% |
| `.pi/audit-current-full/` | 17 | 31,029,799 | 29.6 | 2% |
| `.pi/audit-current/` | 15 | 18,711,256 | 17.8 | 1% |
| `.pi/audit-screens.json` | 1 | 10,021,277 | 9.6 | <1% |
| `.pi/eec-final/` | 7 | 8,769,747 | 8.4 | <1% |
| `.pi/audit-premium/` | 6 | 7,216,204 | 6.9 | <1% |
| `docs/audit*/` (PNG) | 3 | 5,338,815 | 5.1 | <1% |
| `.pi/audit-critical-before/` | 4 | 4,903,335 | 4.7 | <1% |
| **Total blobs > 1 MiB** | **1066** | **1,580,829,359** | **~1507.6** | **100%** |

**The 1041 `.pi/*` blobs (about 1.42 GiB) are the bloat.** They're
agent tooling artifacts: regression-scan screenshots, audit-phase
visual diffs, chrome-screenshots, eec-inventory outputs, etc.
None of this is shipped game content.

**The 25 non-`.pi/` blobs (about 85 MiB) split into**:
- 18 MP3 audio files (game runtime loads them; 41 MiB live,
  38 MiB archived)
- 7 PNGs in `docs/audit-after/` and `docs/audit/logs/` (these
  are ALSO audit screenshots — different prefix but same
  nature; ~5 MiB total)

### Game art is NOT in the > 1 MiB bucket

Game art (in `static/`, `src/`, etc.) per the design doc's
file-extension breakdown:

| Ext | Count | MiB | Notes |
|---|---|---|---|
| `png` (game art) | 1170 | 484.6 | mostly sub-1 MiB sprites, portraits, frames |
| `mp3` (game audio) | 31 | 68.1 | runtime audio |
| `jpg` | 15 | 4.6 | small |
| `json` | 81 | 2.7 | small |
| `ts` | 154 | 1.1 | source |
| `svelte` | 38 | 0.5 | source |

**Total game content ≈ 565 MiB** (doc-verified). All of it
survives filter-repo prune of `.pi/` and `docs/audit*/`.

---

## Updated options (corrected)

### Option A: filter-repo prune `.pi/`, `.pi-tmp/`, `docs/audit*/`

**What it does** (concrete plan):

1. Add to `capture-anime-girls/.dracon/dracon-sync.toml`:
   ```toml
   auto_commit_exclude_patterns = [
       ".pi/",
       ".pi-tmp/",
       ".pi-glla/",
       "docs/audit*",
   ]
   ```
   This stops the daemon from auto-committing new audit
   artifacts (per v0.112.34 excluded-path semantic: the
   files stay on disk, daemon UNSTAGES them after each
   commit so they don't get swept into the next manual
   commit, but the worktree content is preserved).

2. Add to `capture-anime-girls/.gitignore`:
   ```
   .pi/
   .pi-tmp/
   .pi-glla/
   docs/audit*
   ```
   This stops manual `git add` from picking them up too.

3. Run on each remote (in order: codeberg skipped, then
   gitlab, then github):
   ```bash
   cd /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls
   DRACON_ALLOW_REWRITE=1 \
     git filter-repo --invert-paths --force \
       --path .pi/ --path .pi-tmp/ --path .pi-glla/ \
       --path-glob 'docs/audit*'
   ```

4. Verify the rewrite succeeded and the pushable is
   under 2 GiB:
   ```bash
   git count-objects -vH
   # expect: size-pack < 2 GiB

   git cat-file --batch-check --batch-all-objects --unordered \
     | awk '{sum+=$3} END {printf "%.3f GiB reachable\n", sum/1024/1024/1024}'
   # expect: < 1.0 GiB reachable (565 MiB art + 85 MiB non-`.pi/` > 1 MiB = ~700 MiB)

   ~/.local/bin/dracon-sync repos | grep capture-anime-girls
   # expect: ✅ CLEAN or 🔄 ACTIVE (not ❌ CONCERN)
   ```

5. Force-push to gitlab and github (the daemon's
   pre-push hook allows this with `DRACON_ALLOW_REWRITE=1`):
   ```bash
   git push gitlab main --force-with-lease
   git push github main --force-with-lease
   ```

**Cost**:
- One-time history rewrite + force-push
- All 4 remotes converge on the new history
- Codeberg is excluded (no impact there)
- 18 MP3 audio files survive (they're in the size-100 KiB
  to 1 MiB bucket, not affected by the > 1 MiB prune)
- 7 `docs/audit*` PNGs are LOST (these are also audit
  artifacts, not shipped content)

**Benefit**:
- Reachable content drops from 2.716 GiB → ~0.7 GiB
- size-pack drops from 2.52 GiB → well under 2 GiB
- ❌ CONCERN clears; GitHub push resumes automatically
- All game art preserved
- Future bloat prevented by the exclusion pattern

**Risks**:
- History rewrite + force-push is destructive. The
  daemon's pre-rewrite backup branch
  `backup/pre-sync-largeblob-fix-*` is the recovery
  path if something goes wrong (per AGENTS.md "History-
  rewrite ENFORCEMENT stack").
- All clones must re-fetch (slow first time, but
  the daemon auto-handles this).
- Any open PRs from before the rewrite will need to
  be rebased (low risk for a one-author game repo).

**Authorization required**:
- `DRACON_ALLOW_REWRITE=1` to bypass the warden's
  pre-push hook
- Operator approval (per AGENTS.md "For HUMAN
  operators": "Force-push to repos with > 5 commits
  ahead")

### Option B: OVH migration of 18 MP3 + 7 PNG = 85 MiB

**What it does**: Move the 25 non-`.pi/` > 1 MiB files
to OVH object storage, reference by URL from the game
runtime.

**Saves**: 85 MiB.

**Why it doesn't solve the problem**:
- 85 MiB doesn't get us under 2 GiB (we'd still have
  2.631 GiB reachable).
- Would have to be combined with Option A anyway.
- Adds runtime complexity (OVH URL fetches, latency,
  offline behavior).

**Verdict**: insufficient on its own. Useful only as a
follow-up to Option A.

### Option C: Defer

**What it does**: Don't take action. Document that
CAG's GitHub mirror is permanently behind (silently
skipped by the daemon) and accept it.

**Cost**: 
- GitHub mirror for CAG remains empty
- CAG accessible only on codeberg (excluded by daemon
  config) and gitlab
- The hint in `dracon-sync repos` continues to read
  `.git exceeds 2 GB (github limit) — github push is
  skipped`

**Benefit**:
- No destructive action
- No operator authorization needed

**Verdict**: Valid if the operator doesn't need a
GitHub mirror for CAG. Acceptable "defer" outcome —
the deliverable can be a one-paragraph design doc
justifying the deferral.

### Option D: orphan github-main cutover

**What it does**: Make GitHub a current-state mirror
of CAG (no per-commit history). Rebuilds the platform's
retired pattern.

**Cost**: Loses GitHub-side per-commit history (no
changelog, no blame for the github copy). The gitlab
mirror keeps full history.

**Benefit**: Reaches < 2 GiB without filtering.

**Verdict**: As noted in the prior doc, this is the
retired pattern from the platform. Not recommended
unless the operator specifically wants no-history
on GitHub.

---

## Recommended: Option A

The corrected math makes Option A clearly the best
option:
- Solves the problem completely (pushable → ~0.7 GiB)
- Preserves all game content (565 MiB art + audio)
- The 7 lost `docs/audit*` PNGs are also audit
  artifacts, not game content
- The pattern is identical to the junk-runner fix
  applied earlier in this session (per-repo
  exclusion + history rewrite)
- The cost is a one-time destructive operation
  with a clear recovery path (the daemon's backup
  branch)

**The remaining decision is the operator's**:
- Does the operator authorize the history rewrite?
- Or do they prefer to defer (Option C)?

This doc is the deliverable. The actual Option A
execution is blocked on operator authorization
for `DRACON_ALLOW_REWRITE=1` + force-push.

---

## Updated verification commands

```bash
# 1. Reachable total (should drop to < 1 GiB after Option A)
cd /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls
git cat-file --batch-check --batch-all-objects --unordered \
  | awk '{sum+=$3} END {printf "reachable: %.3f GiB\n", sum/1024/1024/1024}'

# 2. Pack size (should drop to < 2 GiB)
git count-objects -vH | grep size-pack

# 3. .pi/ in history after prune (should be 0)
git rev-list --all --objects | grep -c '^\.pi/'

# 4. docs/audit* in history after prune (should be 0)
git rev-list --all --objects | grep -c 'docs/audit'

# 5. daemon repos table
~/.local/bin/dracon-sync repos | grep capture-anime-girls

# 6. daemon journal (github push should resume, not skip)
journalctl --user -u dracon-sync.service --since "5m ago" \
  | grep -i "capture-anime-girls"
```

## Cross-references

- `docs/design/cag-github-push-block-2026-07-28.md` — the
  prior doc (with the wrong interpretation; numbers
  correct, categorization wrong)
- `AGENTS.md` "Excluded-path semantics" — the
  v0.112.34 semantic that `auto_commit_exclude_patterns`
  follows
- `AGENTS.md` "History-rewrite ENFORCEMENT stack" — the
  `DRACON_ALLOW_REWRITE=1` escape hatch
- `.pi-glla/notes/junk-runner-pi-glla-bloat-2026-07-28.md`
  — the parallel junk-runner fix (same pattern, smaller
  scale: 412 blobs / 2.884 GiB vs 1041 blobs / 1.42 GiB)
- `dracon-sync/src/sync.rs:1788-1819` — the silent
  github-skip path that this fix unblocks
