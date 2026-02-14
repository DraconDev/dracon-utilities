# Agentic Engineering (Dracon Style)

This doc is a north star for building "agentic engineering" tools without turning the codebase into a tangled, multi-author mess.

The point is leverage:

- A human should state intent once.
- The system should execute and verify.
- The repo should contain enough state to resume deterministically.
- There should be one obvious way to do things.

## The Core Tension

Agentic systems drift into multipolar behavior:

- multiple "sources of truth" (chat, note files, commits, logs, dashboards)
- multiple planners/actors changing the same state
- ambiguous ownership (who edits what)
- hidden state (daemon memory) that can’t be reconstructed from the repo

If you don't actively prevent it, you get a "total mess" architecture.

## Recommendation: Two-Loop Model

### Loop A: "Interactive Craft" (today)

Use when the blueprint is evolving and the human wants to steer.

- Use `dracon-ai do` for short bounded plan/execute loops.
- Keep safety gates (`--dangerous`, secret policy).
- Optimize for fast iteration and visibility.

This is the mode you’re in now: back-and-forth produces good outcomes quickly.

### Loop B: "Factory Mode" (future)

Use when the blueprint is clear enough to be treated like a test.

- A single declarative input exists.
- The executor runs headless and produces verified commits.
- Failures are crisp and localized.

Factory Mode only works if the blueprint format is strict enough.

## What "Maximum Productivity" Looks Like

Maximum productivity is not "more AI". It is:

- One intent input.
- One executable blueprint.
- One deterministic executor.
- One gatekeeper.
- One factual report stream.
- Zero hidden state.

When it works, the human experience looks like this:

1. Write intent once (minutes).
2. Blueprint is generated (minutes).
3. Executor runs headless and commits verified slices (minutes to hours).
4. If blocked, it stops with one crisp question that can be answered in one message.
5. The next run continues from the last checkpoint without re-explaining context.

The system experience looks like this:

- Every step has a verifier.
- Every verifier runs (or writes a waiver with a reason).
- Every step ends in a checkpoint (commit or explicit "no-op" checkpoint).
- The repo always contains enough state to resume.

## The Best Future Path (Pragmatic)

### 1) Treat "the plan" as a compiled artifact

Free-form `do.md` is high-bandwidth for humans but low determinism for machines.

Instead:

- Human writes intent in a loose form (`do.md` or a chat).
- A planner produces a strict blueprint (`plan/blueprint.json` or `plan/blueprint.toml`).
- The executor consumes only the blueprint (not the human notes).

This preserves creativity while giving the system something test-like.

#### Blueprint design (minimum viable)

To prevent "vibe-driven autonomy", blueprint steps must be test-like. Each step should include:

- scope: which directories/files it is allowed to touch
- intent: one-sentence goal
- actions: commands to run, edits to make (or references to scripts)
- verifiers: commands that must pass
- invariants: explicit "must not change" statements
- checkpoint: what commit should represent
- rollback: how to revert if verifier fails

### 2) One writer per file class

To avoid multipolar chaos:

- `do.md`: human-only
- `plan/blueprint.*`: planner-only
- `plan/chat.md`: executor-only (append-only)
- `plan/CONTEXT.md`: executor-only
- source code: executor + human (normal dev)

If an agent wants to "discuss", it writes into `plan/chat.md` as a question and stops.

### 3) Gatekeeper is mandatory, not optional

Wrong approaches should be treated as errors, not preferences.

A gatekeeper should enforce:

- no plaintext secrets in tracked files (`dracon-warden` already helps)
- no dangerous commands without explicit opt-in (`dracon-ai` already helps)
- no duplicate responsibilities (sync/warden/system boundaries)
- no "lockfile-only" commits (sync already guards)

Gatekeeping should happen at the boundaries:

- pre-commit / pre-push checks (optional)
- daemon loops (`dracon-sync`, `dracon-warden`) hard-fail or auto-repair
- executor step runner refuses to proceed when invariants break

### 4) Define "done" in machine-checkable terms

"Done" should not be a vibe. It should be a checklist that can be executed:

- tests pass (or explicitly waived with a reason)
- formatting/lints pass (or explicitly waived)
- repo is clean (except known ignored dirs)
- no secret markers exist in tracked plaintext
- all required files updated (docs, changelog, etc.) if the blueprint requires it

### 5) Anti-Patterns (Explicit Errors)

These are not style issues. They are productivity killers and should be treated as errors.

- Chatting instead of writing steps: untestable, unreplayable.
- Multiple ways of doing secrets/config: guarantees drift and leaks.
- Silent partial work with no checkpoint: the executor cannot resume reliably.
- Multiple sources of truth: the "real plan" is unclear (chat vs notes vs commits vs files).
- Ambiguous ownership: tools/AI edit `do.md`, humans edit `plan/` (except to delete/reset).
- Unbounded loops: "keep trying" without `max_steps`, timeouts, or a stopping condition.
- No verifiers: steps that don't run tests/lints/format (or log a waiver) are incomplete.
- Non-reproducible state: hidden daemon memory, manual steps not recorded in repo state.
- Drive-by unrelated changes: opportunistic edits not in the blueprint.
- Big-bang refactors: touching many areas without vertical slices and intermediate checkpoints.
- Error swallowing: continuing after failed commands/tests without stopping or recording why.
- Responsibility duplication: new daemons/watchers overlapping `dracon-sync`/`dracon-warden`/`dracon-system`.
- Unsafe defaults: destructive commands executed without explicit opt-in.
- Commit noise: lockfile-only commits, "WIP" commits, or commits that don't map to a blueprint step.

### 5) Prefer a single execution engine with plug-in skills

Instead of "many utilities that all do agentic things", prefer:

- a single executor engine (step runner)
- explicit capability adapters (git, files, system, search, code-semantics)
- strict IO surfaces (blueprint in, factual report out)

This keeps context small and ownership clear.

## Minimal Path To The Future (No Overbuild)

Do not jump straight to a fully autonomous daemon.

Build toward it in this order:

1. Blueprint format: strict, machine-readable, step-based.
2. Gatekeeper: enforce invariants and hard-fail on violations.
3. Executor: run one step, verify, checkpoint, repeat.
4. Reporting: append-only, factual, bounded logs.
5. Only then: automation (watchers) that trigger execution.

## What To Do Now

You already have the right primitives:

- deterministic sync + commits (`dracon-sync`)
- secret policy enforcement (`dracon-warden`)
- system checks (`dracon-system`)
- interactive command-oriented agent (`dracon-ai do`)
- minimal plan scaffolding (`dracon-code`)

Next, if you want to push toward Factory Mode without overbuilding:

1. Add a strict blueprint file format to `dracon-code` (a machine-readable queue).
2. Add a tiny executor that runs one blueprint step at a time and writes a factual report.
3. Keep `dracon-ai do` as the interactive lane; do not turn it into the daemon.
