# Historical Pi Commit Audit (2026-06-19)

## Context

Three commits across 2 repos are authored by `pi` (a
transient agent identity). This audit evaluates each:
(1) the commit content, (2) depth from HEAD, (3) whether
rewriting is feasible without violating AGENTS.md,
(4) the decision (keep with override or rewrite).

## Commit 1: dracon-code `c3159191d`

- **SHA**: `c3159191d544f543c055b7812fdc14bb6330501e`
- **Author**: `pi-audit <pi@dracon.local>`
- **Date**: 2026-06-18 22:10:21 +0100
- **Message**: `audit(audit/2026-06-18): rename 'Dependency Analysis' to 'Dependencies' to match contract`
- **Branch**: `audit/2026-06-18` (not main)
- **Depth from HEAD**: 7 commits
- **HEAD author**: DraconDev (`b6d4ec525`)
- **Daemon classification**: `✓ owned (trusted_email)` — no override needed

**Systemic fix possible?** **YES** (technically) but
**NOT RECOMMENDED**. The commit is 7 deep, so a
force-push would only rewrite 7 commits. However:
1. AGENTS.md says "NEVER force-push to repos with > 5
   commits ahead" — 7 is over the threshold
2. The daemon correctly classifies the repo as
   `✓ owned (trusted_email)` based on the HEAD author
3. The branch `audit/2026-06-18` is a feature branch,
   not main — rewriting it is lower risk but still
   not recommended

**Decision**: **KEEP**. The daemon doesn't flag the
repo because the HEAD is DraconDev. The historical
pi commits are inert. No override file is needed.

## Commit 2: dracon-code `da74bfd20`

- **SHA**: `da74bfd2053d5be4e6b8a1237fc7e4d977e21ae8`
- **Author**: `pi-audit <pi@dracon.local>`
- **Date**: 2026-06-18 22:09:39 +0100
- **Message**: `audit(audit/2026-06-18): full health audit, 0 auto-fixes, 3 deferred items`
- **Branch**: `audit/2026-06-18` (not main)
- **Depth from HEAD**: 10 commits
- **HEAD author**: DraconDev (`b6d4ec525`)
- **Daemon classification**: `✓ owned (trusted_email)` — no override needed

**Systemic fix possible?** **YES** (technically) but
**NOT RECOMMENDED**. Same reasoning as Commit 1:
10 commits is over the 5-commit threshold, and the
daemon doesn't flag the repo.

**Decision**: **KEEP**. Historical artifact, not a
problem.

## Commit 3: dracon-platform `311f1889f`

- **SHA**: `311f1889fc12c16d11d701f30d27f77dc9f53094`
- **Author**: `pi <pi@dracon.uk>`
- **Date**: 2026-06-19 08:27:58 +0100
- **Message**: `docs(goals): add layout-width recommendation research doc`
- **Branch**: `main`
- **Depth from HEAD**: 620 commits
- **HEAD author**: DraconDev (after the 4-commit force-rewrite on 2026-06-19)
- **Daemon classification**: `✓ owned (override)` — override file at `dracon-platform/.dracon/dracon-sync.toml`

**Systemic fix possible?** **NO** without violating
AGENTS.md. Rewriting 620 commits of history would be a
massive force-push that violates:
1. "NEVER rewrite history"
2. "NEVER force-push to repos with > 5 commits ahead"

The commit is a documentation-only change (a `.md`
file in `.pi/goals/`), so the security risk is minimal.
The override file handles the daemon classification
correctly.

**Decision**: **KEEP with override**. The override
file is the correct systemic solution given the
AGENTS.md constraint. The commit is on all 4 remotes
and will not be rewritten.

## Summary

| Commit | Repo | Depth | Daemon flags? | Decision |
|--------|------|-------|---------------|----------|
| `c3159191d` | dracon-code | 7 | NO (HEAD is DraconDev) | KEEP (inert) |
| `da74bfd20` | dracon-code | 10 | NO (HEAD is DraconDev) | KEEP (inert) |
| `311f1889f` | dracon-platform | 620 | YES (but override handles) | KEEP + override |

**Key insight**: The daemon's ownership detection only
checks the HEAD author, not historical commits. So
historical pi commits don't trigger the warning UNLESS
the HEAD is also pi-authored. For dracon-code, the HEAD
is DraconDev, so the 2 pi commits in history are inert.
For dracon-platform, the 4 HEAD pi commits were
rewritten, but the 620-deep commit would require a
massive force-push to fix, so the override file is the
correct solution.

## Systemic improvement opportunity

The daemon could be improved to:
1. Only flag `untrusted_author` if the HEAD is by an
   untrusted author (current behavior — already correct)
2. Ignore historical untrusted authors if the HEAD is
   trusted (current behavior — already correct)
3. Provide a "historical untrusted author" warning
   separately from the "HEAD untrusted author" warning

The current behavior is already correct — the daemon
only checks the HEAD. The 3 historical pi commits are
inert and don't affect the daemon's classification.
