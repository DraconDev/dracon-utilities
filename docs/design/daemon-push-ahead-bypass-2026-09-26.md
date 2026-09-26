# Daemon push-ahead bypass when classification starves (design, 2026-09-26)

## Problem (observed live 2026-09-25/26)

`dracon-strategy` sat ↑1 (later ↑2) unpushed for 6+ hours with ZERO
push attempts. Cause is phase ordering, not push failure: per-repo
triage runs classify → commit → push, and classification
(filter-aware `git diff HEAD`, hardcoded 30s timeout in
`git/diff.rs::git_diff_head_files`) timed out every cycle on a
static 53 MB file (128s needed under load ~100; streak 54+). The
repo was skipped as undispatched before the push phase was ever
reached. The "Stuck Ahead (Unpushed)" alert (`daemon.rs`, sustained
`ahead_since`) paged correctly but misdiagnosed ("push may be
failing") and took no action.

Generalized: ANY repo whose worktree classification cannot pass
can never push already-committed work, no matter how pushable it
is. Push needs only ref comparison + pack — never a worktree diff.

## Proposal: push-ahead bypass

When a repo is both (a) ahead of its publish ref for longer than
the existing Stuck Ahead threshold sustained window, AND (b) on a
classification-failure streak ≥ N (suggest N=3 consecutive), the
daemon dispatches a **push-only job** that skips worktree
classification/commit entirely and runs the normal multi-remote
push flow for the ahead commits.

Safety properties (unchanged from normal push):
- Fast-forward only: the existing `force_push_when_behind = false`
  default still governs; a behind repo never takes the bypass.
- Same remote set, same per-remote timeouts, same warden
  pre-push hooks (a secrets/BAD_AUTHORS refusal still blocks).
- The dirty worktree is untouched: its files stay for later
  normal cycles (and keep their own classification streak).
- Bypass attempts are journal-logged distinctly
  (e.g. `push-ahead-bypass`) so they are distinguishable from
  triage pushes in forensics.

## Alternatives considered

- **Raise the 30s diff timeout**: does not fix the ordering —
  a slow-enough file still gates already-committed pushes, just
  at a higher load. Orthogonal; may still be worth a tunable.
- **Operator hand-push**: exactly what the bypass automates
  (verified live via dry-run: fast-forward, seconds). Manual
  forever is not a fleet answer.

## Implementation notes

- Follow the codebase's pure-helper convention: a
  `push_ahead_bypass_due(ahead_since, streak, now) -> bool`
  decision helper with unit tests (cf. `hold_warn_due`,
  `status_inspection_wedged` in `daemon.rs`).
- If the streak threshold becomes a policy field, it needs the
  per-repo story per AGENTS.md test discipline (field +
  `Option<>` override + merge + tripwire entry) — same as
  `auto_bump_versions`.
- Fix the Stuck Ahead alert text while here: distinguish
  "push attempted and failing" (push errors in journal) from
  "push never attempted" (classification/starvation hold active).
