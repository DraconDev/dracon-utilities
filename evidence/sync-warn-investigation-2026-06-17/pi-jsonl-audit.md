# `.pi/**/*.jsonl` Exclusion Audit — 2026-06-17

## Scope
All 12 watched repositories. For each, we list every `.pi/**` directory
discovered, every `.jsonl` file inside, and whether `git check-ignore -v`
excludes it (and if so, which `.gitignore` line is responsible).

## Method
```bash
for repo in <each of 12 repos>; do
  find "$repo" -type d -name '.pi' \
    | while read d; do find "$d" -name '*.jsonl' -type f; done \
    | while read f; do
        rel="${f#$repo/}"
        ( cd "$repo" && git check-ignore -v "$rel" ) \
          || echo "  NOT-IGNORED: $rel"
      done
done
```

## Per-Repo Results

| Repo | `.pi/**/*.jsonl` count | Excluded | `.gitignore` line |
|------|------------------------|----------|-------------------|
| `ai-auto-writer` | 1 | YES | `.gitignore:15:*.jsonl` |
| `avid` | 1 | YES | `.gitignore:15:*.jsonl` |
| `browser-extensions-shared` | 7 (one per `.pi` dir: `extensions/{auto-form-filler,vidpro-extension,job-finder,ai-ats,lead-radar}/.pi/`, `docs/research/extension-research/.pi/`, repo-root `.pi/`) | YES (all 7) | `.gitignore:20:*.jsonl` |
| `dracon-ai-lib` | 1 | **NO** | (not ignored) |
| `dracon-code` | 1 | YES | `.gitignore:15:*.jsonl` |
| `DraconDev` | 1 | YES | `.gitignore:15:*.jsonl` |
| `dracon-libs` | 0 | n/a | (no `.pi/**` dir) |
| `dracon-platform` | 10 (root + `apis/`, `web/`, `web/ai-hub/`, `web/games/`, `web/games/games/{hegemon,junk-runner,hellhunter,darklord,one-mil-girls}/`) | YES (all 10) | `.gitignore:15:*.jsonl` (9), `web/games/games/one-mil-girls/.gitignore:15:*.jsonl` (1) |
| `dracon-utilities` | 1 | YES | `.gitignore:18:*.jsonl` |
| `pully-fully-pull-based-fleet-reconciler` | 1 | YES | `.gitignore:15:*.jsonl` |
| `rust-ai-web-auto` | 1 | YES | `.gitignore:15:*.jsonl` |
| `.dracon` | 0 | n/a | (no `.pi/**` dir) |

## Totals
- **25** `.pi/**/*.jsonl` files across **10** of 12 repos
- **24** excluded (all by `*.jsonl` in DRACON MANAGED BLOCK)
- **1** not excluded (`dracon-ai-lib` — its `.gitignore` does not have `*.jsonl`)
- **2** repos with no `.pi/**` directory at all (`dracon-libs`, `.dracon`)

## Where the `*.jsonl` rules live

All matching lines are inside the **`# --- BEGIN DRACON MANAGED BLOCK ---` /
`# --- END DRACON MANAGED BLOCK ---`** region of each repo's `.gitignore`.
These blocks are written and refreshed by `dracon-warden`, not by hand.

| Repo | Line |
|------|------|
| `ai-auto-writer`, `avid`, `dracon-code`, `DraconDev`, `dracon-platform`, `pully-fully-pull-based-fleet-reconciler`, `rust-ai-web-auto` | 15 |
| `dracon-utilities` | 18 |
| `browser-extensions-shared` | 20 |

(`dracon-ai-lib` is the lone exception — it has no `*.jsonl` rule at all,
so its `goal_events.jsonl` is currently not ignored. Inconsistent with the
other 10 repos.)

## What the `.jsonl` files actually contain

All 25 files are named `goal_events.jsonl` and live at
`<some>/.pi/goals/goal_events.jsonl`. They are append-only event logs
recording what happened to the goal (state transitions, edits, etc.).
The actual goal content lives in the **`.md` files** alongside them:
- `.pi/goals/active_goal_*.md` — current goal doc
- `.pi/goals/archived/goal_*.md` — finished goals

The daemon already commits the `.md` files (they are not excluded by
`*.jsonl`). The `.jsonl` files are the auxiliary event log, not the
deliverable content.

## Operator's three options

### Option A: Unignore all `.pi/**/*.jsonl` (commit them)
Add a `!`-negation line inside the DRACON MANAGED BLOCK of every affected
repo's `.gitignore`:
```gitignore
# (inside the managed block, after the *.jsonl rule)
!.pi/**/*.jsonl
```
Effect: 24 newly-tracked `.jsonl` files across 10 repos. Event logs become
part of git history.

### Option B: Keep excluding (status quo)
No change. 24 `.jsonl` files remain untracked. The `.md` goal content is
still tracked (it already is). The audit trail lives only in the local
working tree, not in git history.

### Option C: Carve out a specific subset
E.g. only unignore `.pi/goals/goal_events.jsonl` at the repo root (not the
deeply-nested ones). Or unignore only certain repos. Requires per-repo
judgment and a precise `!`-pattern that does not weaken `*.jsonl` broadly.

## Risks of any unignore
- **File size**: `goal_events.jsonl` grows monotonically (append-only
  event log). A long-running repo's file could reach tens of MB over time.
  All are well under the 100 MiB `max_stage_file_bytes` limit, so this is
  not a hard blocker, but it does mean tracked blobs grow.
- **Warden interaction**: the `!` line must be **inside** the DRACON
  MANAGED BLOCK (between the BEGIN/END markers). warden-managed blocks
  are refreshed by warden — if a future warden run re-writes the block
  without preserving the `!` line, the unignore is lost. The
  warden-managed files in this repo (`dracon-warden`) own the policy and
  the human operator who runs warden must add the `!` line in their
  warden template, or the local edit will be wiped on the next refresh.
- **Mirrors**: all 4 remotes (origin, github, gitlab, codeberg) will
  receive the new commits. No special handling needed.

## Recommendation
Option B (keep excluding) is the simplest, lowest-risk path. The `.md`
files — the actual goal content — are already tracked. The `.jsonl`
event log is auxiliary metadata that has marginal value in git history
compared to the file-size cost and the warden-managed-block maintenance
burden.

If the operator wants the event log in git for audit/rollback reasons,
**Option A** is correct, but the `!` line must be added to the warden
template (not the per-repo `.gitignore` directly), so it survives future
warden refreshes.
