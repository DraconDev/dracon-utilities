# Follow-up Tasklist (from full-audit-2026-07-03.md)

> Goal `mr5c6mz0-tbtcrj` asked: "make a tasklist of what we need to do".
> This document is the deliverable.
> Source audit: `docs/design/full-audit-2026-07-03.md`.

## What was done in the audit goal

5 of 13 audit findings addressed in this session:

| # | Action | Status | Commit |
|---|--------|--------|--------|
| 1 | Audit doc created + pushed | done | fd24652b4245 |
| 2 | P0.1 deathrun stale lock removed | done | (no commit) |
| 3 | P1.1 endless-td orphan worktree pruned | done | (no commit) |
| 4 | P1.2 darklord /tmp/baseline-check pruned | done | (no commit) |
| 5 | P2.1 /home/dracon/dracon removed from watch_roots | done | 2bc2a7f70c76 (.dracon repo) |

## Daemon state improvement

| Metric | Before audit | After audit |
|--------|--------------|-------------|
| OK repos | 16 | 18 |
| WARN repos | 10 | 8 |
| CONCERN repos | 0 | 0 |
| FAIL repos | 0 | 0 |
| Stale locks | 1 (deathrun) | 0 |
| Orphan worktrees | 2 (endless-td, darklord) | 0 |
| Watch roots | 3 (1 empty) | 2 (both canonical) |

## Follow-up tasks (proposed for future goals)

These are the items the operator chose to defer. Each is sized
small enough to be a single follow-up goal.

### Follow-up 1: cosmetic metadata-failure noise (P2.2)

**Problem:** ~14 metadata-update failures in 24h from GitLab/Codeberg.
Most are "repo not found" during auto-create flow (cosmetic — the
repo IS auto-created successfully, the metadata call just fires
once per push). Pads the journal with warnings.

**Effort:** 30 min — either suppress the warning class after
auto-create succeeds, or downgrade to info level.

**Risk:** low — just log classification.

**Suggested goal:** "Downgrade GitLab/Codeberg 'repo not found'
metadata warnings to info after auto-create succeeds."

### Follow-up 2: junk-runner screenshot sprawl (P2.5)

**Problem:** `junk-runner` has 12+ PNG screenshots in
`docs/audit-screenshots/` being committed to git. Each is 1-3MB.
Git history is ballooning.

**Effort:** 30 min — decide between: (a) `.gitignore` for
`docs/audit-screenshots/` + write screenshots to a CDN/bucket
(via `gen-*.py`), or (b) keep committing and let git LFS handle
the bloat (already considered, rejected in
`docs/design/lfs-vs-bucket-vs-grow-2026-07-03.md`).

**Risk:** low — operator decision.

**Suggested goal:** "Move junk-runner screenshots out of git to
OVH bucket (per binary-asset-strategy-2026-07-03.md)."

### Follow-up 3: web-auto nested repo cleanup (P3.2)

**Problem:** `/home/dracon/Dev/web-auto/rust-ai-web-auto/` is a
separate git repo nested inside `web-auto/`. Daemon treats them
as 2 separate watched repos. Whether this is intentional sibling
structure or accidental nesting is unclear.

**Effort:** 1 hour to investigate + decide. Options: (a) make
rust-ai-web-auto a submodule of web-auto, (b) document why it's
sibling, (c) move one out of /web-auto/.

**Risk:** medium — operator decision about project structure.

**Suggested goal:** "Decide web-auto/rust-ai-web-auto relationship
(submodule vs sibling vs sibling-out-of-parent) and execute."

### Follow-up 4: hegemon binary-asset migration (P2.4)

**Problem:** hegemon submod's local pack is 2.7GB (957 binary
files: 810 PNGs + 76 MP3s in `static/`). GitHub's 2GB pack-size
limit blocks pushes; current workaround is daemon-side
`exclude_remotes = ["github"]`. Long-term: move regenerable
content to OVH bucket.

**Effort:** days — full migration of static/ to bucket + rewrite
asset-pipeline.md. Existing design docs cover the strategy:
- `docs/design/binary-asset-strategy-2026-07-03.md`
- `docs/design/lfs-vs-bucket-vs-grow-2026-07-03.md`

**Risk:** medium — requires gen-*.py regeneration scripts +
asset-pipeline.md updates.

**Suggested goal:** "Execute hegemon static/ → OVH bucket
migration per binary-asset-strategy-2026-07-03.md."

### Follow-up 5: daemon self-healing for stale locks (P0.1 follow-up)

**Problem:** P0.1 fixed the immediate stale lock in deathrun,
but the daemon has no general self-healing for this. A future
daemon crash that leaves a lock behind will block the submod
again until manual cleanup.

**Effort:** 1-2 hours — daemon source change. Detect
"Another git process seems to be running" error; check lockfile
mtime > 30s and no git process holding it; auto-clean.

**Risk:** low — defensive feature with safe bounds
(only clean stale locks, not recent ones).

**Suggested goal:** "Add daemon self-healing for stale
.git/modules/<submod>/index.lock files (P0.1 follow-up)."

## Summary

The audit goal is materially complete. Daemon state improved from
16/10/0/0 (OK/WARN/CONCERN/FAIL) to 18/8/0/0. All actionable
P0/P1 issues resolved. Operator decisions captured for P1.3/P1.4.
Long-term P2/P3 items documented and proposed as 5 focused
follow-up goals.