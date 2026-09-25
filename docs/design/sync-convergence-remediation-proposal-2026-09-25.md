# Sync-convergence remediation proposal (2026-09-25)

PROPOSAL ONLY — nothing here has been executed. The daemon stays frozen
(`dracon-sync.freeze` held, PID 3669702) until the operator approves one
option. Sealed evidence: `audit/sync-convergence-2026-09-25/` (`evidence.json`
hash verified OK; result `pending`).

## Why convergence is impossible today

| # | Lane | Blocker | Measured |
|---|---|---|---|
| 1 | dracon-platform (parent) | reachable history 28.5 GB vs 2 GiB fail-closed ceiling | `internal-high-water-blockers.md`; live HEAD `86654b25` descends from snapshot, so strictly larger now |
| 2 | capture-anime-girls | same ceiling, 3.5 GB | live HEAD `bd26d9cd`, forward-grown |
| 3 | deathrun | same ceiling, 2.37 GB | live HEAD `f51b651a`, forward-grown |
| 4 | junk-runner | sealed snapshot object `6748649…` purged; ancestry unprovable; 28 unrelated-history conflicts + 612 guard violations | `cat-file` fatal live |
| 5 | freeport | post-snapshot `reset: moving to HEAD` in reflog (2026-09-25 14:58:42 +0100) | reflog entry persists live |

Bloat profile (read-only `verify-pack`/`rev-list`, 2026-09-25): parent is
diffuse product media (7.3k PNG, 2.2k WebP, 1.5k JPG; top blobs ~5 MB);
CAG is committed `pi-session-*.html` exports (~9.6 MB) + `static/audio/*.mp3`;
deathrun is `audit-evidence/*.log` (top 6.6 MB).

## Option 1 — History slimming (filter-repo + force-push)

Viable exactly for CAG and deathrun (clean excisable classes). NOT
recommended for the parent (diffuse product media; excision = deleting
shipped assets from history).

Per repo (CAG, deathrun), in order:

```bash
cd <repo>
git bundle create ~/dracon/backups/<repo>-pre-slim-20260925.bundle --all
git filter-repo --invert-paths --path 'pi-session-*.html' --path static/audio \
  --path audit-evidence --force   # paths per repo; see below
# CAG: --path 'pi-session-*.html' --path static/audio
# deathrun: --path audit-evidence
DRACON_ALLOW_REWRITE=1 git push --force-with-lease=<old>:<new> <remote> main
```

Preconditions (operator-owned, irreversible): gitlab branch unprotection for
the two lanes during the push window; `DRACON_ALLOW_REWRITE=1` escape past
warden hooks; all three remotes (github/gitlab/codeberg) force-updated to the
same new tip; every other clone re-cloned. Then re-run the bucket-guard
projection to confirm < 2 GiB before re-approval.

Risk: any missed clone reintroduces old objects on next push; reflog-based
evidence for the OLD snapshot stays invalid (must re-snapshot anyway).

## Option 2 — Raise the ceiling (policy change)

Edit `web/config/bucket-strategy.json` governing GitHub ceiling above the
measured maxima (≥ 30 GB for the parent). No rewrite, no re-clone.

Why weakest: the 2 GiB ceiling mirrors real provider pack limits (github 2 GiB
pack cap already bit deathrun once, 2026-07-23). Raising the number does not
raise github's limit — pushes would fail remotely instead of locally, turning
a clean local block into partial multi-remote divergence. Recommend rejecting
unless paired with provider-side exceptions that do not exist on our tiers.

## Option 3 — Quarantine (shrink convergence scope)

Keep guards and history; remove the three oversized lanes from the
convergence set: add per-repo `owned`/scope exclusions, archive current
remotes as-is, converge everything else. Parent gitlinks for CAG/deathrun pin
to last-good SHAs.

Cheapest and fully forward-only, but concedes the original goal's lane set —
the three lanes stay permanently diverged/divergent. Best combined with
Option 1 for CAG/deathrun and quarantine for the parent's media problem.

## Junk-Runner and Freeport (all options)

Both need a NEW immutable snapshot; the old one is unusable (purged object,
post-snapshot reset). After the operator picks 1/2/3:

- junk-runner: resolve the unrelated-history divergence with a normal
  forward merge (`--allow-unrelated-histories`, 28 conflicts resolved by
  hand), push fast-forward, then snapshot.
- freeport: no repair possible or needed — the reset is a historical fact.
  The new snapshot draws the line AFTER it; the resumed contract must state
  pre-snapshot reflog entries are out of scope.

## Recommended hybrid (needs approval)

1. Option 1 for CAG + deathrun (clean classes, bounded blast radius).
2. Option 3 for the parent (quarantine `web/books` media lanes or accept a
   documented parent-only exception; do NOT rewrite 26 GB of product media).
3. Forward-merge junk-runner; accept freeport's pre-snapshot reflog.
4. Take a NEW quiescent snapshot, regenerate evidence, re-run the full
   verification contract, then `dracon-sync resume`.

## Approval gate

Reply with the option number (or a variant). On approval I will execute only
the approved scope, starting with bundle backups, and stop at the first
deviation for re-approval. No `resume`, `repair --apply`, push, or rewrite
runs before that.
