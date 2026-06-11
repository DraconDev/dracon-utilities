# Cleanup manifest (except `.pi`)

Date: 2026-06-11  
Rule: `.pi/` local task state was not cleaned, deleted, renamed, untracked, ignored, rewritten, sanitized, or moved.

## Cleaned items

| Repo | Action | Paths / scope | Reason |
|---|---|---|---|
| `browser-extensions-shared` | cleaned tracked browser profile data | `auto-form-filler/.audit-ui/aria-check2/Default/`, `auto-form-filler/.audit-ui/aria-test2/Default/`, `auto-form-filler/.audit-ui/dd-debug/Default/`, `auto-form-filler/.audit-ui/popup-check/Default/` | Tracked browser profile/cache/history/local-storage data outside `.pi`; public-readiness blocker. |
| `browser-extensions-shared` | cleaned tracked generated coverage | `SamAI/coverage/` | Generated coverage output, not source or user-owned project asset. |
| `ai-auto-repo-rot-scanner-todo-agent` | cleaned stale local runner event file | `.ralph/audit-remediation/.ralph-runner/events.jsonl` | Stale `.ralph-runner` generated event log outside `.pi`. |
| `dracon-utilities` | cleaned stale local public key under `.demon` | `.demon/data/keys/owner_age1wz5p.pub` | Stale non-`.pi` local state; not the Warden master/team key system. |
| `one-mil-girls` | cleaned generated audit JSON | `docs/audit/2026-06-11-full-audit-v2/script-audit.json`, `docs/audit/visual-qa/convo-redesign-after/inspect/inspect.json` | Generated audit artifacts outside `.pi`; project screenshots/assets were preserved. |

## Preserved items

| Scope | Reason |
|---|---|
| All `.pi/` paths | User explicitly requested not to clean `.pi`. Before/after diffs are empty. |
| User-owned notes, screenshots, pasted-image files, project assets | Constraint requires preservation unless explicit approval is given. |
| `.env*`, secret-like files, source files with "secret" in the name | Classified as `blocked-needs-approval`; removing them could discard intentional config/examples or require rotation. |
| `.ralph/*.md` and `.ralph/*.state.json` | Classified as `blocked-needs-approval` unless clearly generated; many are local task/session notes. |
| Ambiguous docs/checklists such as `TODO.md`, `IMPLEMENTATION_TODO.md`, `docs/AUDIT-TODO.md` | Classified as `blocked-needs-approval`; may be intentional project docs. |

## Blocked by `.pi` exclusion

All `.pi/goals/active_*.md`, `.pi/goals/archived/*.md`, `.pi/goals/*.jsonl`, and other `.pi/` paths found in the candidate scan were left untouched. Evidence: `pi-proof/*.diff` files are empty (`pi_diff_count 0`).

## Blocked-needs-approval summary

Candidate scan classified these as `blocked-needs-approval`:

- `.env*` / secret-like paths and `.envrc`.
- `.ralph/*.md` / `.ralph/*.state.json` local task/session notes.
- Ambiguous TODO/checklist docs.
- Source files whose names include "secret" but are actual code/tests, not cleanup artifacts.

These were not deleted.

## Evidence

- Before inventory: `before/inventory.json`, `before/inventory.tsv`
- Per-repo before metadata: `per-repo/before.*.git.txt`
- Candidate scan: `candidates/cleanup-candidates.tsv`
- `.pi` unchanged proof: `pi-proof/*.diff` (all empty)
- After inventory and validation artifacts are written alongside this report.
