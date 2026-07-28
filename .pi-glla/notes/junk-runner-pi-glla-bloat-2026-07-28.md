# Junk-runner `.pi-glla/active.jsonl` bloat — 2026-07-28

## Summary

During the wrap-up of the auto-mirror retry-softening goal
(`20260727233951-rrwbjk`) on 2026-07-28, fleet verification
discovered a NEW ❌ CONCERN: **junk-runner's pushable branch
is 3.77 GiB** (over GitHub's 2 GiB pack limit), with daemon
log entry:

```
🚫 skipping github push for /home/dracon/Dev/dracon-platform/web/games/wip/junk-runner:
   pushable branch is 3.77 GiB (exceeds github's 2 GiB pack limit).
   Needs history rewrite / OVH migration; will resume once shrunk below 2 GiB.
```

This is a different cause from the existing `capture-anime-girls`
(CAG) CONCERN (art-PNG bloat, 1.49 GiB of PNGs > 1 MiB per
`docs/design/cag-github-push-block-2026-07-28.md`).

## Root cause: session-scratch bloat, not art bloat

Verified reachable-content breakdown of junk-runner:

| Object | Size | Count | Total |
|---|---|---|---|
| `.pi-glla/active.jsonl` | 15.0 MiB | 412 | 6.18 GiB |
| `src/assets/audio/music/cockpit/cockpit_theme_3.mp3` | 17.1 MiB | 1 | 17.1 MiB |
| `src/assets/audio/music/planet/planet_theme_2.mp3` | 16.5 MiB | 1 | 16.5 MiB |
| Other reachable | — | — | rest |

**412 historical copies of `.pi-glla/active.jsonl` in junk-runner's
git history.** Each daemon commit-all cycle appends 13+ lines to
the JSONL and creates a new 15 MB blob. Over many hours of active
glla-loop work, the JSONL grows monotonically (session-scratch
data accumulates).

**Why this contradicts the existing policy**: AGENTS.md says
session-scratch dirs like `**/pi-tmp/**`, `**/scratch/**`,
`.demon/**`, `.sisyphus/**`, `.ralph/**` are valid git content —
"the user/agent can `rm` them from the working tree when they're
done, and the daemon will commit the deletion." But
`.pi-glla/active.jsonl` is **never deleted by the orchestrator** —
it's the persistent append-only state of the glla loop. So the
"short-lived file" policy assumption is violated for this file
specifically.

## Fix plan

### Per-repo: stop the bleeding

The daemon's per-repo config supports
`auto_commit_exclude_patterns` (per AGENTS.md "Per-repo overrides"
section). Adding the pattern to junk-runner stops the daemon from
auto-committing the file on every cycle:

```toml
# /home/dracon/Dev/dracon-platform/web/games/wip/junk-runner/.dracon/dracon-sync.toml
owned = true
auto_commit_exclude_patterns = [".pi-glla/active.jsonl"]
```

(CHANGED semantics 2026-07-22 v0.112.34: `auto_commit_exclude_patterns`
means "don't auto-commit these files". The daemon UNSTAGES them after
each commit so its own `git add -A` doesn't sweep them into the next
manual commit, but **preserves worktree content**. So the operator's
edits to excluded files stay on disk, visible in `git status` as
modified-unstaged.)

### Affected repos

| Repo | `.pi-glla/active.jsonl` in history | Add exclusion? |
|---|---|---|
| `junk-runner` | 412 copies (6.18 GiB redundant) | **YES — bloat source** |
| `dracon-platform` | 1 copy (transient) | YES — preventive |
| `pi-goal-loop-audit` | 0 copies (separate gitdir) | NO — not affected |
| `dracon-utilities` | 0 copies (meta-repo) | NO — meta-repo is not auto-committed |

The exact list of repos to update will be confirmed by querying
each watched repo with `git rev-list --all --objects | grep -c pi-glla/active.jsonl`.
Any repo with > 0 copies needs the exclusion.

### History-rewrite (deferred)

Removing the existing 6.18 GiB of `.pi-glla/active.jsonl` blobs
from junk-runner's history requires a `filter-repo --invert-paths`
rewrite (or equivalent) + force-push. This is the same class of
operation that AGENTS.md "History-rewrite ENFORCEMENT stack" warns
against — but the warden's `pre-push` hook can be bypassed with
`DRACON_ALLOW_REWRITE=1` for deliberate operator rewrites.

**DEFERRED until the bleed-stop (per-repo exclusion) is in place
and proven stable for at least one daemon cycle.** A history rewrite
while the file is still being committed would be wasted.

## Verification

After the per-repo exclusion is added and committed, the daemon's
next cycle should:

1. NOT auto-commit `.pi-glla/active.jsonl` (verify with
   `journalctl --user -u dracon-sync.service --since "5m ago"`
   showing no `📝 committed ... pi-glla/active.jsonl` lines).
2. Show the file as modified-unstaged in
   `git status` (per the v0.112.34 semantic).
3. `dracon-sync repos` should show junk-runner STILL ❌ CONCERN
   (the history still has 6.18 GiB of bloat), but the HINT
   should change from "ongoing bloat" framing to "awaiting
   history rewrite" framing.

History-rewrite completion criterion (later, deferred):

- `git cat-file --batch-check --batch-all-objects --unordered | grep pi-glla/active.jsonl | wc -l` returns 0
- `dracon-sync repos` shows junk-runner at `✅ CLEAN` or back to `🔄 ACTIVE` (not ❌ CONCERN)
- `git count-objects -vH` shows size-pack < 2 GiB

## Cross-references

- `docs/design/cag-github-push-block-2026-07-28.md` — the original
  GitHub push block, different cause (art PNGs)
- `AGENTS.md` "Per-repo overrides" — the
  `auto_commit_exclude_patterns` knob
- `AGENTS.md` "Excluded-path semantics (CHANGED 2026-07-22,
  v0.112.34)" — what exclusion does and does not do
- `AGENTS.md` "History-rewrite ENFORCEMENT stack" — what
  `filter-repo --invert-paths` requires (operator override)
