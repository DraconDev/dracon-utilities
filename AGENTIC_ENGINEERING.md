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

## The Best Future Path (Pragmatic)

### 1) Treat "the plan" as a compiled artifact

Free-form `do.md` is high-bandwidth for humans but low determinism for machines.

Instead:

- Human writes intent in a loose form (`do.md` or a chat).
- A planner produces a strict blueprint (`plan/blueprint.json` or `plan/blueprint.toml`).
- The executor consumes only the blueprint (not the human notes).

This preserves creativity while giving the system something test-like.

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

### 5) Prefer a single execution engine with plug-in skills

Instead of "many utilities that all do agentic things", prefer:

- a single executor engine (step runner)
- explicit capability adapters (git, files, system, search, code-semantics)
- strict IO surfaces (blueprint in, factual report out)

This keeps context small and ownership clear.

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

