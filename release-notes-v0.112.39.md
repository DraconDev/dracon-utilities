# Release Notes — v0.112.39 (2026-07-23) — deathrun size fix + frame-dump prevention

**Headline**: deathrun's 2 GiB problem is fixed (orphan cutover,
github pushes resumed), the audit-screenshot-bloat class is prevented
fleet-wide, and a BROKEN_HISTORY detector now guards against damaged
gitdirs. **Also an important diagnosis correction** (see below).
**825 daemon tests**, clippy + deny clean.

---

## ⚠️ Diagnosis correction (read this first)

The initial investigation concluded deathrun had "**2092 missing
objects — broken history on both sides (local + gitlab)**" and that
the daemon's auto-repair largeblob rewrites had broken its history.
**That was a false alarm from a probe bug on my part.**

`git rev-list --objects HEAD` appends **paths** to blob/tree lines as
`<sha> <path>`. Piping that raw into `git cat-file --batch-check`
makes cat-file mis-parse every `<sha> <path>` line and print
`<sha> <path> missing`. My first probe counted those mis-parses as
missing objects: 20679 (local) and 2092 (gitlab) "missing" — all
false. The corrected probe (strips paths first, `awk '{print $1}'`)
shows **0 missing objects everywhere** (deathrun current, gitlab
clone, github clone, whole fleet).

**The auto-repair largeblob rewrites did NOT break history.**
deathrun was **fat, not broken**. The orphan cutover was still the
right fix for the real problem (size) and is a valid outcome, but it
was motivated partly by this false premise. The shipped probe has the
path-strip fix and returns 0 false positives across the fleet.

## The real problem (size — confirmed, and fixed)

deathrun's pushable branch was **2.85 GiB**, past github's 2 GiB pack
limit, so the daemon skipped github for days (gitlab/codeberg kept
working). The bloat, measured with correctly-typed blob lines (valid):

- `docs/audit-browser-v3/` — **1542.8 MB / 4311 files** (browser
  audit screenshots; the same ~1.7 MB screenshot committed over and
  over)
- `docs/audit-howto-visual/` — 629.6 MB / 1444 files
- `docs/audit-buttons-v1/` — 194.3 MB / 553 files
- `.pi/chrome-screenshots/` — 585 MB / 2016 files (regeneratable)
- Everything else (more audit dirs, sprite sheets, audio, src) —
  ~0.4 GB, mostly legit

Working tree was only 140 MB — it was all *history*.

## The fix: orphan cutover

1. Trimmed `.pi/chrome-screenshots` (149 MB) from the tree +
   gitignored it. Tracked content: **268 MB**.
2. Orphan root commit `a77d795b rebirth: clean root (orphan cutover)`
   (2388 files, 261.2 MB); main moved to it.
3. **gitlab**: unprotect main → force-push → re-protect (main is
   protected there). **github**: `--force-with-lease=main` (the lease
   ref is the REMOTE branch name, not the tracking name — that
   detail cost one retry).
4. **github accepted the push (261 MB ≪ 2 GiB)** — pushes resumed
   after days of the guard skipping it.
5. Nothing lost: `backup/pre-deathrun-rewrite-*` +
   `backup/pre-sync-largeblob-fix-*` hold the pre-rewrite history;
   github's complete old history (`29e8ab38`) is preserved until
   github's gc.
6. Verified: daemon built 3 commits on the orphan root, all 3 forges
   + local at `036dedd8`, parent gitlink converged, deathrun
   `🟢 synced · healthy`, CONCERN 0.

## Prevention (so the class never recurs)

1. **BROKEN_HISTORY detection** (`report.rs`):
   `probe_missing_objects` (with the path-strip fix) +
   `BROKEN_HISTORY:N` state flag → CONCERN with hint "history damaged
   (N objects missing) — fresh clones fail; needs clone-from-forge
   or orphan cutover". Cached 24h alongside the size probe
   (`CachedRepoSize.missing_objects`, serde-defaulted for old cache
   files). A genuinely damaged gitdir will be caught; the probe no
   longer cries wolf.
2. **Frame-dump prevention** (`dracon-warden.toml`):
   `hygiene_patterns` now ignores `**/.pi/chrome-screenshots/` and
   `**/audit-*/screenshots/` fleet-wide — regeneratable audit frame
   dumps are warden-`.gitignore`d while the audit `.md` REPORTS still
   go up (they're the deliverable). Applied to deathrun's managed
   `.gitignore` block via `dracon-warden once`. Same anti-rebloat
   class as hegemon's `**/.state-recon/**` (goal f228b540).
   AGENTS.md's commit-all policy documents the exception.
3. **Auto-repair pre-flight** (`rewrite_ahead_paths`): refuses to
   rewrite a damaged gitdir (missing objects) with an alert instead
   of writing broken history. Cheap insurance for a real class (even
   though the motivating incident turned out to be a probe artifact).

## Test discipline

- `cargo test --workspace --locked` ✅ **825 daemon**, warden 83,
  security ~111, system 86 — 0 failed
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean
- Corrected probe returns **0 missing objects on every fleet repo**
  (no false positives)

Design doc: `docs/design/audit-screenshot-bloat-deathrun-2026-07-23.md`
