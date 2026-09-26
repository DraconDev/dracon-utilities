# Parent history bloat: corrected targeting (2026-09-26)

Measurement (live `rev-list --objects --all` + `cat-file`, raw bytes):

- Parent total: **24.2 GB**, of which `web/` is 23.8 GB.
- `web/music/libs`: **15.9 GB** — 2,961 JSON blobs (cookbook/catalog
  churn: ~38 MB live worktree rewritten hundreds of times by loops,
  every version committed). History-only bloat; worktree is 38 MB.
- `web/music/build-dev`: **2.3 GB** — build output committed 5+
  weeks ago, now untracked/ignored. Pure history residue.
- `web/books/static`: **3.6 GB** — actual media; product call needed.
- `web/books/build`, `web/books/build-dev` (7.8 GB worktree):
  ignored, 0 tracked files — disk waste, not a git problem.

## What this overturns

The convergence proposal targets "web/books media lanes" (~4 GB)
for quarantine — that misses the actual bloat. Two-thirds of the
parent problem is loop-churned JSON in `web/music/libs`, plus
committed build output in `web/music/build-dev`.

## Recommendation

- `web/music/libs` + `web/music/build-dev`: excision-shaped, not
  quarantine-shaped — but ONLY via the sanctioned-slimming
  procedure, and only after the operator confirms old catalog
  versions are worthless (live 38 MB re-commits fresh post-cutover;
  build-dev has nothing to recommit). Parent rewrite stays a
  bigger blast radius than CAG (266k objects, parent of 10
  submodules) — sequence it after CAG proves the procedure.
- `web/books/static`: needs the product-vs-regenerable call before
  any decision (excise / overflow-migrate / permanently except).
- Until then: documented parent-only exception stands (history
  accepted as-is, convergence verified on all other lanes).
  Per the no-cutout rule, the daemon keeps committing the parent
  and every lane regardless.
