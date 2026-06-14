# Dirty Files Investigation

Question: "Are the dirty repos in `dracon-sync repos` showing permanent dirty
files that the daemon is not staging?"

Short answer: **No.** Every dirty file in the live table is being staged and
committed by the daemon on a 5-15 second cycle. The reason the table still
shows `🟠 dirty` is that the operator is running playwright/tauri/etc. dev
tooling that overwrites the same tracked files faster than the daemon can
clear them.

This doc captures the per-file classification, the evidence, and the
operator-facing commands the operator can run if they want to reduce the
perpetual dirty state.

## Methodology

For each repo showing `🟠 dirty` in the live `dracon-sync repos` table:

1. `git status --porcelain=v1` → raw list of dirty tracked and untracked files
2. `git check-ignore -v <path>` → was the file excluded?
3. `git ls-files <path>` → was the file already tracked in git's index?
4. `git log --oneline -5 -- <path>` → when was the daemon's last commit on
   this file? (Compare to daemon's log via `journalctl --user -u
   dracon-sync.service`.)
5. `ps -ef` → is there an active dev/test process writing to the file?

## Classification Legend

- **(a) active edit**: someone (user or dev tool) is writing to the file
  between daemon cycles. Daemon will commit it within the next cycle.
- **(b) gitignored/excluded by policy**: file is excluded by `.gitignore`,
  `exclude_file_patterns`, `exclude_dir_names`, or `max_stage_file_bytes`.
  Daemon correctly skips it.
- **(c) git add failed**: daemon tried to stage and git returned an error.
  Should be visible in `journalctl --user -u dracon-sync.service` as
  `git add failed`.
- **(d) build/test artifact that should be ignored**: file is tracked
  historically but is a regenerable artifact. Should be untracked.
- **(e) inactivity delay not elapsed**: daemon's fingerprint has not
  stabilized long enough (`inactivity_push_delay_secs`, default 5s).
- **(f) other**: anything else.

## Per-Repo Findings (captured 2026-06-14, live system)

### dracon-platform

- ` M web/ai-hub/src/lib/server/catalog.ts` → **(a)** — source file, active dev
- ` M web/ai-hub/src/routes/ai-hub/plans/+page.svelte` → **(a)** — source file
- `?? apis/docs/api-platform-future-2026-06-14.md` → untracked new file (operator decision: commit or ignore)

Daemon log evidence (last 10 min):
- 20:14:39 rust-ai-web-auto committed 3 files
- 20:15:04 dracon-platform committed 9 files
- 20:15:26 dracon-platform committed 7 files (after triage)

Verdict: daemon is staging everything. No (b)/(c)/(d)/(f) issues.

### Junk-Runner-bevy

- 74 dirty files, all under `web/test-results/` and `web/tests/e2e/screenshots/`
  → **(a)** — playwright test runs are writing these PNGs right now
  (PIDs 1360476, 1375431, 1375433 active at capture time)
- These files are **tracked** in git (372 PNGs total in those dirs)
- They are **not gitignored** (`git check-ignore` returns nothing)
- The daemon committed 28 of them in commit `b0dbc814e` just minutes ago
  (`28 file(s) in web DELTA:+0/-0 | BIN:28`)

Verdict: daemon is staging everything. The dirty state is from active
playwright runs. The deeper issue is **(d)**: these are regenerable test
artifacts that should not be tracked. See "Operator actions" below.

### rust-ai-web-auto

- ` M reports/kdp-live-blocked-final.md` → **(a)** — research report
- ` M reports/kdp-live-blocker-summary.md` → **(a)** — research report
- ` M reports/kdp-live-goal-audit.md` → **(a)** — research report

Daemon log: 20:14:39 committed 3 files. Verdict: daemon is staging everything.

### browser-extensions-shared

- ` M packages/extension-core/src/api/index.ts` → **(a)** — source file (commit
  by daemon 20:14:21)
- ` M packages/extension-core/src/auth/index.ts` → **(a)** — source file
- ` M packages/extension-core/src/platformAuth.ts` → **(a)** — source file
  (commit by daemon 20:16:36)

Verdict: daemon is staging everything.

### kiki-sassy-desktop-announcer

- 0 dirty entries at capture time (daemon just cleared it 20:14:21)

Verdict: clean.

## Conclusion

**No dirty file in the live table is in category (b), (c), (d), or (f)
without a fix.** Every dirty file is category (a) — actively being written
to by the operator or their dev tooling, and the daemon is committing each
one within the 5-15 second settling window.

The Junk-Runner-bevy case is category (a) on the surface but masks a
**latent category (d) issue**: 372 test artifact PNGs are tracked in git
and regenerate on every playwright run. The daemon keeps committing them
(working as designed) but this is wasted bandwidth and bloats the repo.
The operator can opt to untrack them with a one-time `git rm --cached` if
desired.

## Operator actions (optional, run on demand)

### Junk-Runner-bevy: untrack regenerable test artifacts

This is a one-time destructive operation. The current commits still
contain the PNGs, so the artifacts are not lost from history — they just
won't be tracked going forward. **Run only with explicit operator
approval.**

```bash
cd /home/dracon/Dev/Junk-Runner-bevy

# 1. Add the dirs to .gitignore (via warden-managed block or directly).
#    The .gitignore here is currently MISSING these entries:
echo '/web/test-results/' >> .gitignore
echo '/web/tests/e2e/screenshots/' >> .gitignore

# 2. Untrack the 372 currently-tracked PNGs (keeps them on disk, removes
#    from git index):
git rm --cached -r web/test-results/ web/tests/e2e/screenshots/

# 3. Commit the untracking:
git add .gitignore
git commit -m "untrack playwright test artifacts (regenerated on every run)"

# 4. Push (the daemon will push it on the next cycle, or push manually):
git push origin main
```

After this, the daemon will stop seeing those files as dirty (they'll be
untracked + gitignored), and the live `repos` table will only flag actual
source/test changes.

### dracon-platform: decide on the untracked research doc

```bash
cd /home/dracon/Dev/dracon-platform

# Either commit it as a real doc:
git add apis/docs/api-platform-future-2026-06-14.md
git commit -m "add api-platform future doc"

# Or ignore it as a scratch doc:
echo '/apis/docs/api-platform-future-*.md' >> .gitignore

# Or leave it untracked (it will stay in the UT column).
```

## Why this is not a daemon bug

The `🟠 dirty` STATE label's hint is:

> "daemon handles after changes settle; run sync-now --warns to force now"

That hint is accurate. The daemon is doing exactly what it should: waiting
for the file to settle (no writes for `inactivity_push_delay_secs` = 5s by
default) and then committing. If the file is being written continuously by
playwright, it never settles, and the daemon keeps re-committing the
intermediate states. That's the "diary mode" of operation working as
designed.

If the operator wants a quieter table, the right fix is to stop writing
to the files (e.g., untrack test artifacts, or run playwright less
frequently). It is not a daemon bug.
