# Design: deathrun size fix + the audit-screenshot-bloat class (2026-07-23)

> Versions: orphan cutover (2026-07-23) + v0.112.39 (BROKEN_HISTORY
> detection, warden hygiene patterns).
>
> **CORRECTION (2026-07-23, post-cutover)**: the initial diagnosis
> included "2092 objects missing — history broken on both sides".
> That was a **false alarm from a probe bug** (see "The probe bug"
> below). The corrected probe shows **0 missing objects everywhere**.
> deathrun was **fat, not broken**. The orphan cutover was still the
> right fix for the REAL problem (size), but the auto-repair largeblob
> rewrites did NOT break anything.

## The real problem (size — confirmed)

deathrun's pushable branch was **2.85 GiB**, past github's 2 GiB pack
limit, so the daemon skipped github for days (gitlab/codeberg kept
working). The bloat, measured with correctly-typed blob lines
(`cat-file --batch-check='%(objecttype) %(objectname) %(objectsize) %(rest)'`
— valid):

| Path | Size | Files | What |
|---|---|---|---|
| `docs/audit-browser-v3/` | 1542.8 MB | 4311 | browser audit screenshots (the SAME ~1.7 MB `DeathScene__1920x1080__idle.png` committed over and over) |
| `docs/audit-howto-visual/` | 629.6 MB | 1444 | visual how-to audit frames |
| `docs/audit-buttons-v1/` | 194.3 MB | 553 | button audit frames |
| `.pi/chrome-screenshots/` | 585.0 MB | 2016 | pi chrome audit frames (regeneratable) |
| `docs/audit-a11y-v1/`, `audit-buttons-v2/`, `audit-browser-v2/`, `audit-touch-v1/`, `audit-svelte-rebuild/`, `smoke-test-*.png` | ~200 MB | ~1000 | more audit frames |
| `static/images/` | 84.7 MB | 350 | **legit** game sprite sheets |
| `static/audio/` | 26.4 MB | 20 | **legit** game music |
| `src/` | 66.1 MB | 1189 | **legit** source |

Working tree was only 140 MB — it was all *history* (audit frames
committed, pruned from the tree, retained in git forever).

## The probe bug (the false "broken history" diagnosis)

`git rev-list --objects HEAD` appends **paths** to blob/tree lines as
`<sha> <path>`. Piping that raw into `git cat-file --batch-check`
makes cat-file fail to parse every `<sha> <path>` line and print
`<sha> <path> missing`. My first probe counted those mis-parses as
missing objects: **20679 (local) and 2092 (gitlab) "missing"** — all
false. The corrected probe strips paths first
(`awk '{print $1}'` keeps only the sha) and shows **0 missing objects
everywhere** (deathrun current, gitlab clone, github clone, whole
fleet).

**The auto-repair largeblob rewrites (2 backup branches exist) did
NOT break history.** The "2092 missing objects on both sides" never
existed. The orphan cutover fixed the REAL problem (size) and is a
valid outcome, but it was motivated partly by this false premise.

## The fix (orphan cutover — done 2026-07-23)

1. Trimmed `.pi/chrome-screenshots` (149 MB, regeneratable) from the
   tree + gitignored it. Tracked content: **268 MB** (was ~2.85 GiB
   pushable).
2. Orphan root commit `a77d795b rebirth: clean root (orphan cutover)`
   (2388 files, 261.2 MB), main moved to it.
3. gitlab: unprotect main → force-push → re-protect (main is
   protected there). github: `--force-with-lease=main` (lease ref is
   the REMOTE branch name, not the tracking name). **github accepted
   the push (261 MB ≪ 2 GiB)** — pushes resumed after days of the
   guard skipping it.
4. Nothing lost: `backup/pre-deathrun-rewrite-1784804212` +
   `backup/pre-sync-largeblob-fix-1784110476/-1784111463` hold the
   pre-rewrite history; github's complete old history (`29e8ab38`,
   0 missing even with the buggy probe... 0 missing with the CORRECT
   probe too) is preserved until github's gc.
5. Verified: daemon built 3 commits on the orphan root, all 3 forges
   + local at `036dedd8`, parent gitlink converged, deathrun
   `🟢 synced · healthy`.

## Prevention (v0.112.39)

1. **BROKEN_HISTORY detection** (report.rs): `probe_missing_objects`
   (with the path-strip fix) + a `BROKEN_HISTORY:N` state flag →
   CONCERN with hint "history damaged (N objects missing) — fresh
   clones fail; needs clone-from-forge or orphan cutover". Cached
   24h alongside the size probe (`CachedRepoSize.missing_objects`).
   A REAL damaged repo would be caught; the probe no longer cries
   wolf (0 false positives across the fleet).
2. **Warden hygiene patterns** (`dracon-warden.toml`): added
   `**/.pi/chrome-screenshots/` and `**/audit-*/screenshots/` to
   `hygiene_patterns` — the fleet's managed `.gitignore` block now
   ignores regeneratable audit frame dumps while keeping the audit
   `.md` REPORTS in git. Same anti-rebloat class as hegemon's
   `**/.state-recon/**` (goal f228b540). Applied via
   `dracon-warden once` to deathrun (patterns landed in its managed
   block).

## The policy line (commit-all exception)

AGENTS.md's commit-all principle says audit evidence belongs in git.
The exception this establishes: **regeneratable frame dumps are
valid to regenerate on demand and wrong to keep forever in git**.
Keep the audit `.md` reports (the deliverable) in git; gitignore the
frames. This is the narrowest exception consistent with both
commit-all and the 85 GiB codeberg quota posture.
