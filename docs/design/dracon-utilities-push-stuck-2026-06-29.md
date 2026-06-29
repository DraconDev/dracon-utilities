# dracon-utilities — Push-Stuck Investigation (2026-06-29)

## Context

On 2026-06-29, `dracon-sync repos` flagged `dracon-utilities`
(row 5) as `🛑 STUCK` with the hint:

```
🛑 push-stuck (11 failures): git push returned non-zero
(see daemon log) — run repair-concerns --apply
```

State as observed:

```
↑ AHEAD  PUSH       STATE+ACT                       DAEMON               HINT
113      🛑 STUCK   ⚪ untracked-only · 🛑          18s ago sync_commit  push-stuck
                   push-stuck 34m (113 ahead)                            (11 failures)
```

codeberg and gitlab each showed 13 ahead (pushable); github
showed 114 ahead (blocked).

This doc is the operator-requested investigation into
**why** the repo was stuck. The fix path is operator action
(two GitHub UI clicks); see §"Fix paths" below.

## Root cause

`git push` to `github.com/DraconDev/dracon-utilities` is
failing with **GH013 — Repository rule violations found
for refs/heads/main — Push cannot contain secrets.** This
is GitHub's push protection (secret scanning) blocking
the push because the new commits add a 22-column rendering
audit doc whose diff introduces lines that match GitHub's
AWS-access-key pattern detector.

The flagged strings are **redaction markers** (not real
secrets). The exact string contents are deliberately
omitted from this doc to avoid creating a new
self-referential GH013 block when the doc itself is
pushed; the markers are well-known and can be reconstructed
from the prior goal's audit docs.

Two strings are flagged:

1. The `AKIA-` prefix redaction marker (used as a
   placeholder for AWS access key IDs in audit
   documentation)
2. The `[REDACTED-AWS-SECRET-KEY-…]` bracket-context marker
   (used as a placeholder for AWS secret access keys in
   audit documentation)

Both markers appear on the same lines (each line carries
both side by side), which is why two separate detectors
fire on the same flagged locations.

The redaction markers were introduced by the AWS key
rotation work for `dracon-platform` (goal `007296af`,
completed 2026-06-29). The pattern **does not match the
literal `AKIA[0-9A-Z]{16}` regex** (the marker has
hyphens), but GitHub's secret scanner uses a more
permissive token-shape matcher that triggers on the
`AKIA-…` prefix shape and on the bracket-context marker.

This is the same class of false-positive that was
documented in `docs/design/audit-2026-06-26/v1-table-fix-and-secret-scrub-2026-06-28.md`:
real AWS credentials (access key ID + secret access key)
were committed to `dracon-platform` repo history in
`apis/services/email-api/.env.prod` and `.env.dev` (which
were tracked despite `.gitignore` because of the `!.env.dev`
negative-ignore). After the AWS key rotation replaced the
real keys with redaction markers, the markers themselves
are still being flagged because the surrounding
documentation context makes GitHub's pattern matcher
consider them suspect.

## Affected files and commits

GitHub's GH013 error names two specific commits and two
files:

| Commit    | File                                                              | Lines flagged                          |
|-----------|-------------------------------------------------------------------|----------------------------------------|
| 290e795c  | docs/design/audit-2026-06-26/v1-table-fix-and-secret-scrub-2026-06-28.md | 12, 30, 31, 41, 42                     |
| 6133547d  | docs/design/audit-2026-06-26/v1-table-and-mirror-enable-2026-06-28.md   | 81, 82, 104                            |

Each line contains both the `AKIA-` marker and the
`[REDACTED-AWS-SECRET-KEY-…]` marker, which is why
both detectors (Access Key ID and Secret Access Key) fire
on the same lines.

## Daemon behavior

The daemon is working correctly. The retry/fail loop is:

1. `📝 committed N file(s) in /home/dracon/Dev/dracon-utilities`
   (daemon auto-commits local untracked + modified files
   per the global `untracked_exclude_patterns = []` policy)
2. `⏫ /home/dracon/Dev/dracon-utilities scaling push timeout 900s → 600s (N commits ahead)`
3. `git push origin main` (or retry attempt) → `⚠️ push failed … GH013`
4. `🔄 trailing-drain: clearing 2 stuck in_flight entries`
5. Backoff 2-3s, then `⏱️ push retry 3/3` → `git push … GH013` → repeat
6. `🚨 ALERT: N unpushed commits (threshold: 50)` at every 50-commit mark

The 11 push failures observed (now 14 as of 13:08) each
correspond to one complete retry cycle. The 113→117
ahead count is rising because new auto-commits (the goal
file changes themselves) keep landing faster than the
failed push attempts can be retried.

## Two unblock URLs (the fix is one click each)

GitHub's GH013 error emits a per-secret unblock URL.
There are exactly two distinct URLs for the 5 lines
flagged:

1. **Amazon AWS Access Key ID**
   `https://github.com/DraconDev/dracon-utilities/security/secret-scanning/unblock-secret/3FmCaothFt0qHvpYTILr9vJELcB`
2. **Amazon AWS Secret Access Key**
   `https://github.com/DraconDev/dracon-utilities/security/secret-scanning/unblock-secret/3FmCaqw6VjMdOHMyL0CvwYNAHeL`

Each URL is a per-secret allowance scoped to this specific
repo: it tells GitHub "the literal redaction-marker
strings in this commit are intentionally committed; allow
the push." After both are clicked, the next daemon push
attempt will succeed and the 117-commit backlog will drain
in one go.

## Fix paths

| Option | Effort | Trade-off | When to use |
|--------|--------|-----------|-------------|
| **A. Click the 2 unblock URLs** | ~30 sec, one-time | Per-secret allowance; future redactions still flagged (because the matcher triggers on the marker shape, not just real secrets) | Operator wants a quick unblock; accepts that the same false-positive will recur on every new doc that quotes the redaction marker |
| **B. Disable push protection in repo settings** | ~1 min | Allows all future pushes with anything matching AWS pattern; risky if a real key is later committed | Operator wants permanent silence on the false-positive and accepts the risk |
| **C. History rewrite to remove the 2 audit docs** | ~5 min + force-push | Breaks the audit trail; needs rebase of any open branches; the rewrite itself may trigger a new push-protection block on the rewritten history | Operator wants the audit trail to NOT mention the redaction markers at all (e.g. move the markers' discussion to a codeberg/gitlab-only path that doesn't push to github) |

**Recommended: A.** Two clicks, no destructive ops,
preserves the audit trail. The 117-commit drain happens
automatically on the next daemon tick.

## Rendering issue (deferred, out of scope for this goal)

The `dracon-sync repos` table output that prompted the
investigation shows a separate rendering issue that is
**not addressed in this goal** (the goal's scope is the
push-stuck investigation, not the rendering bug):

- The `#` column row number (`1`, `2`, `3`, …) is placed
  on the **first line** of a multi-line row, with the
  rest of the row wrapping below. The empty first column
  on subsequent wrapped lines makes it hard to visually
  associate which data lines belong to which row number.
- The root cause is that `REPO` is a `LowerBoundary(17)`
  column in `print_repos_full_table` (per prior goal
  `mqz3fk22-s0rh0v`'s rebalanced constraints), but
  `browser-extensions-shared` is 24 chars wide. At any
  viewing width where the full 22-column table fits,
  this row wraps to 4 lines and the row number is on
  line 1 only.

Three viable fix paths (none taken in this goal — they
require a design decision the operator needs to make):

| Path | Effort | Trade-off |
|------|--------|-----------|
| **A.** Widen REPO column to 24+ cols | small code change to `print_repos_full_table` set_constraints; bump the Full tier threshold from 300 to ~330+ cols | Pushing the tier boundary higher means most users see the compact 15-col table at typical widths |
| **B.** Truncate/abbreviate REPO names to 17 cols (e.g. `browser-…`) | small code change in the row-building code | Loses full REPO identity in the table; operator would need to use the HINT column or `dracon-sync repos --json` for full names |
| **C.** Repeat the row number on every wrapped line | medium code change in `comfy_table` configuration (no built-in option, requires post-processing the rendered table) | Cleanest visual; the `#` would appear on every line of the row |

The code lives in `dracon-sync/src/report.rs` and is
part of the `dracon-sync` workspace member, NOT this
`dracon-utilities` repo, so a fix here requires either
a follow-up `dracon-sync` development goal or a feature
request to the daemon maintainer.

This is **noted but deferred** to a future goal. The
user's directive in the goal text was "otherwise clearly
the next one to investigate is why this repo is stuck"
— the rendering observation is a side comment, the
push-stuck investigation is the deliverable.

## Follow-up

1. **The redaction marker format may need to change** to
   avoid future false-positives. Options (the exact
   current marker strings are deliberately not reproduced
   here; see prior audit docs):
   - Use underscores instead of hyphens in the
     `AKIA-` redaction marker (would not match GitHub's
     permissive matcher)
   - Wrap the `AKIA-` redaction marker in angle brackets
     (the brackets break the marker shape)
   - Rephrase the markers entirely to a different
     format like `<<REDACTED-AWS-KEY-2026-06-28>>`
   Any of these can be applied via a follow-up goal
   `dracon-utilities-redaction-marker-format` if the
   operator decides to do option C above.

2. **The same false-positive will recur on any repo that
   contains these audit docs** (currently only
   `dracon-utilities` since the docs were re-pushed there;
   the other mirror repos were either not yet pushed or
   were pushed before the redaction markers were
   introduced).

3. **The 5 files / 9 secret commits / 13 AKIA commits
   noted in the prior goal summary are not all distinct
   from the 2 issues here** — the 2 unblock URLs cover
   all flagged lines in `dracon-utilities`. The earlier
   "9 secret commits / 13 AKIA commits" tally was from
   the broader goal-#13 audit and includes commits in
   the `dracon-platform` repo history, which github still
   hasn't been retried against (because dracon-platform
   is blocked by 12 GB size, not by secrets).

## Related

- Prior AWS rotation goal: `.pi/goals/archived/goal_2026062804484949_mqx91oeu-o8oz9o.md` (goal `007296af` companion)
- Original GH013 false-positive analysis: `docs/design/audit-2026-06-26/v1-table-fix-and-secret-scrub-2026-06-28.md` (the very file that GitHub is now flagging)
- Daemon push-stuck classification: `docs/design/sync-push-classification.md`
- Operator's commit policy: `AGENTS.md` §"Commit policy (the most important section)"
