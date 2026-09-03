# Harness-trail commit pointers — design (2026-09-03)

Status: **proposed** (no code changed). Implements the operator's direction:
use whatever in-repo papertrail exists for better commits, generally across
harnesses, with no AI in the commit path, commits shaped for AI first, and
ledgers left append-only.

## 1. What dracon-sync does today

Commit subject is a deterministic routing key for AI, built in
`dracon-sync/src/sync.rs`:

- `stage_commit_and_push` (~3301) stages, then sets `msg = blast_radius`
  (~3465) and calls `svc.commit(&msg)`. Single-line subject, first line is
  also logged via `report::log_incident` (`sync_commit`).
- `compute_blast_radius` (~2751) builds
  `{CLOSED/WIP} | N file(s) in dirs [top3] DELTA:+a/-r | METRICS`.
- `extract_task_transitions` (~2141) scans the **staged diff** (`git diff
  --cached --unified=0`) for added `+` lines matching `- [x]` / `- [~]`
  (also `* [x]`, bare `[x]`). Generic: fires on any file, any harness.
  Task text goes through `sanitize_task_name` → `compact_task_phrase`
  (3 words, cut at `:;—–`) → `truncate_task` (60 chars). Capped at
  `MAX_TASKS = 10` per category.
- `extract_goal_metadata` (~2323) is supposed to add `GOAL:complete`,
  `PAUSE:<reason>`, `TOKENS:<n>K`, `TIME:<n>m`, `EVIDENCE:<id>:<text>`,
  `SKIPPED:<id>:<reason>`. **Dead path**: it only matches staged files
  starting with `.pi/goals/` and ending `.md`, and parses an old
  JSON-header format. Current reality is elsewhere (see §2), so these
  tokens almost never fire.

## 2. Where the papertrail actually lives (observed 2026-09-03)

All examples below from `dracon-platform` (GLLA-heavy). The point is the
pattern, not the product.

- `.pi-glla/goals/<ts>-<shortid>.md` (active goal): markdown with
  `**status:**`, `## Objective` (1–2 lines), `## Verification contract`,
  `## Commits`, `## FIXes shipped` (`- [x] FIX SEVERITY: title …`),
  `## Tasks` (`- [x] …`).
- `.pi-glla/archive/<ts>-<shortid>.md` (completed goal): `# Goal` with
  `**Status**: complete`, `## Objective` (blockquote), `## Completion
  summary` (`Outcome:` / `Changed:` / `Evidence:` / `Tests:` /
  `Unresolved:` / `Next:` single lines), `## Tasks`, `## Audit history`
  (`… — approved — <model>`).
- `.pi-glla/audit-loop/findings.md` (append-only ledger, 40k+ lines):
  `- [x] FIX: SEV: <long title>` and `- [x] DECIDED: …`. **Read-only.**
  Never rewrite, reorder, or "fix the spam" — it is the ledger.
- `.pi-glla/active.jsonl` (1–5 MB) + `audits.jsonl` (3–5 MB): event log
  (`goal_created` with `objective`, `task_complete` with
  `taskId`+`evidence`, `completion_requested` with `summary`,
  `audit_result` with `verdict`+`report`, `goal_archived`) mixed with
  heavy supervisor noise (`context_*`, `session_*`, `model_*`,
  `continuation_*`). MB-scale: tail-only if ever read.
- Nested ledgers: each subproject may own its own trail, e.g.
  `web/games/wip/polis/.pi-glla/`, `web/music/.pi-glla/`,
  `web/books/.pi-glla/`. Root-only path filters miss these.
- Legacy pi paths: `.pi/goals/goal_events.jsonl` (same event shape as
  above, old location), `.pi/tasks/tasks.json` (`{nextId, tasks:
  [{id, subject, description, status}]}` with `pending`/`in_progress`).
- Other harnesses leave (almost) nothing **in-repo**: `.opencode/goals/
  state.json` observed as `{"goals": [], …}` (empty); Antigravity, Codex,
  Claude keep transcripts **outside** the repo (`~/.codex/sessions/
  -home-dracon-Dev-<slug>/…`, `~/.claude/projects/…`,
  `~/.pi/sessions/…`). Not staged → invisible to the commit path by
  construction (see §3).

## 3. Design constraints (operator-set)

1. **General, not GLLA-only.** No hard dependency on any harness. Rule:
   parse what is staged in the diff; gracefully absent → today's behavior.
2. **No AI in the commit path.** Deterministic regex/parse only, same as
   today. "Summarize" means deterministic compaction: counts, short ids,
   status tokens, truncated phrases under existing caps. No LLM, no
   network, no unbounded reads.
3. **Commits are for AI first.** The subject stays a stable machine index
   (fixed token order, fixed caps). The rich content is the committed
   trail files themselves — the daemon's commit-all policy already
   preserves them. Better commits = better **pointers** (which goal?
   which status? which severity?), not prose duplication. No commit-body
   change in this design: AI consumers read the goal files from the tree;
   humans get traceability via the short id.
4. **Ledgers stay append-only.** `findings.md` and JSONL logs are parsed
   read-only. De-spamming the *commit subject* is in scope; touching the
   ledger files is out of scope.

## 4. Proposed change (no new config)

No new `SyncPolicy` field (avoids the `RepoPolicyOverride` tripwire in
`policy.rs`). Extend existing parsers within existing caps:

**4a. Retarget goal-file detection (fixes the dead path).**
Match staged paths against all of (root and nested):
- `**/.pi-glla/goals/*.md` and `**/.pi-glla/archive/*.md` (new format)
- `.pi/goals/**/*.md` including `archived/` (legacy back-compat)
Keep the current `.pi/goals/*.md` prefix as one case, not the only case.

**4b. Parse the new markdown format deterministically.**
For the first matched goal file (bounded read, e.g. first 8 KiB):
- short id from filename: `<ts>-<shortid>.md` → `GOAL:<shortid>`.
- `**Status**: complete|paused|active` (also `**status:**` variant) →
  `GOAL:complete` / `GOAL:paused` (same token names as today).
- `## Objective`: first non-empty line, run through the existing
  `compact_task_phrase`/`truncate_task` pipeline → `OBJ:<text>`.
- archive only, `## Completion summary` `Outcome:` line, same pipeline →
  `OUTCOME:<text>`.
- archive only, `## Audit history` line containing `approved`/`rejected` →
  `AUDIT:approved` / `AUDIT:rejected`.
All tokens join the existing `metrics` vec in a fixed order:
`GOAL:<id> GOAL:<status> AUDIT:<verdict> OBJ:<…> OUTCOME:<…>`, then the
existing `EVIDENCE:`/`SKIPPED:`/`TOKENS:`/`TIME:` tokens (kept for the
legacy JSON format; new format has no per-task evidence fields, so they
simply stay absent).

**4c. De-spam checkbox subjects without AI (parser fix, not ledger fix).**
In `extract_task_transitions`, before the generic 3-word compaction,
match `FIX[: ]<SEV>` and `DECIDED` case-insensitively:
`- [x] FIX: HIGH: …` / `- [x] FIX MED: …` / `- [x] DECIDED: …`.
Aggregate per commit: `FIX:H<n>/M<n>/L<n>` (only nonzero severities,
order H/M/L) and `DECIDED:<n>`. Fall back to current `CLOSED:` names for
non-matching lines. This turns `CLOSED: FIX, FIX, FIX, … +10more` into
`CLOSED: FIX:H2/M3` — same determinism, same caps, machine-countable.
The ledger file itself is untouched.

**4d. pi tasks pointer (small, bounded).**
If the staged diff includes `**/.pi/tasks/tasks.json`, parse it
(best-effort `serde_json`, bounded: first 64 KiB) and take
`in_progress` task `subject`s through the existing compaction pipeline,
capped by the same `MAX_TASKS`, emitted as today's `WIP:` prefix. Absent
or unparseable → skip silently.

**4e. Explicitly not read in the commit path.**
`active.jsonl` / `audits.jsonl` / `goal_events.jsonl` tails,
`~/.codex/sessions/`, `~/.claude/projects/`, `~/.pi/sessions/`,
`.opencode` state. Rationale: MB-scale I/O per commit with cooldown
complexity (JSONL), or outside the watch root (external sessions —
ownership/privacy boundary the daemon must not cross for a commit
message). If a future need arises, it belongs behind an explicit opt-in
knob with caching, not in the default path. Not proposed here.

## 5. Example (illustrative, caps applied)

Before:
`CLOSED: FIX, FIX, FIX, FIX, FIX, FIX, FIX, FIX, FIX, FIX +10more |
4 file(s) in .pi-glla,web […] DELTA:+40/-6`

After (same commit):
`CLOSED: FIX:H1/M3/L2 DECIDED:1 | 4 file(s) in .pi-glla,web […]
DELTA:+40/-6 | GOAL:177yuu GOAL:complete AUDIT:approved OBJ:books audit
pass`

## 6. Tests (per AGENTS.md test discipline)

- New-format goal md: status variants, objective first-line extraction,
  archive outcome + audit verdict, short-id from nested path
  (`web/games/wip/polis/.pi-glla/archive/<ts>-<id>.md`).
- Legacy format unchanged: old `.pi/goals/*.md` JSON header still parses
  (back-compat regression test).
- Severity aggregation: mixed `FIX: HIGH/MED/LOW`, `FIX MED` (no colon),
  `DECIDED`, non-matching lines fall back to `CLOSED:` names; caps hold
  (`MAX_TASKS`, 60 chars, top-3 files).
- tasks.json: `in_progress` subjects → `WIP:`, malformed JSON → skip,
  oversized file → bounded read.
- Gates: `cargo test --workspace --locked`, `cargo build --release
  --locked`, `cargo deny check`, `cargo clippy --workspace --locked --
  -D warnings` from the monorepo root.

## 7. Rollout / risk

- Read-only parsing of staged diff + bounded reads of staged goal files
  only. No staging behavior change, no push behavior change, no new
  config, no migration.
- Worst case (unparseable trail): tokens absent, subject identical to
  today. No failure mode blocks a commit.
- Follow-up (not this change): if the pointer tokens prove useful,
  consider a `commit_body_trail` opt-in later. Deliberately deferred —
  AI-first consumers already have the files in-tree.
