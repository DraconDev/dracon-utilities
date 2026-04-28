# dracon-ai

`dracon-ai` is the **only** Dracon AI CLI. It is intentionally thin: it does **not** implement provider/model wiring itself.

Instead, it consumes the canonical AI runtime from `dracon-libs` (policy, secrets, routing, and adapters). This keeps Dracon’s “one way” rule: the utility is just a shell, the platform is the library.

## Non-Goals

- No direct provider hookup logic in this repo (no OpenRouter/OpenAI/Anthropic “native” client logic in `dracon-ai`).
- No AI dependencies in deterministic daemons (`dracon-sync`, `dracon-warden`, `dracon-system`).
- No ownership of `dracon-code` workflows (`do.md`, blueprint gates, project execution policy); those belong in the `dracon-code` project.

## Commands

### `dracon-ai` (default)

Starts `do` mode interactively.

Use this when your goal is something related to the computer (Nix changes, repo hygiene, service debugging, file operations).

By default, interactive `do` mode is opened in a **new terminal tab** when possible. Use `dracon-ai do --same-terminal` to keep it in the current terminal.

### `dracon-ai status`

Shows the resolved AI runtime view from `dracon-libs`:

- provider specs count
- active model ids
- dev model ids

### `dracon-ai do [--plan] [--dangerous] [task...]`

Computer-context assistant. It returns a small set of shell commands to run for the task.

- By default it runs the commands, captures output, and continues until done (bounded iterations).
- Plan-only: `dracon-ai do --plan ...` (or `DRACON_AI_APPLY=0`).
- Potentially dangerous commands are refused unless you pass `--dangerous` (or `DRACON_AI_DANGEROUS=1`). When refused, the command is printed so you can run it manually.

Interactive helpers:

- `/apply on|off`
- `/dangerous on|off`
- `do so` (re-run the last task)

### `dracon-ai chat [options] [prompt...]`

Sends a single prompt through the `dracon-libs` routing runtime.

- `--intent` is treated as a lane hint (mapped to `dracon-libs` routing tasks).
- If no prompt is provided, `dracon-ai chat` starts interactive mode in a **new terminal tab** when possible.
- Use `--same-terminal` to force interactive chat in the current terminal.
- Input modes:
  - `--stdin` (or `-` as prompt) reads full stdin
  - `--file <path>` reads prompt from a file
- Output modes:
  - streaming tokens to stdout by default when TTY
  - `--no-stream` collects then prints
  - `--json` prints a structured response object

Examples:

```sh
dracon-ai
dracon-ai chat Say ok only.
printf "Say ok only.\n" | dracon-ai chat --stdin
dracon-ai chat --file prompt.txt
dracon-ai chat --intent engineer "Refactor this function."
```

### `dracon-ai cmd [options] <command...>`

Runs a local command via `sh -lc`, captures bounded output, then asks the AI to analyze it.

Examples:

```sh
dracon-ai cmd "journalctl --user -u dracon-sync.service -n 200"
dracon-ai cmd --timeout-secs 20 --max-bytes 200000 "rg -n \"DRACON_SECRET\" -S ."
```

## Routing Model

`dracon-ai` does not select “provider + model” directly.

It delegates to `dracon-libs`:

- `ai-runtime-config` resolves policy + secrets into provider specs and active/dev model sets.
- `ai-routing-runtime` routes lane/task to a concrete model id.
- `ai-runtime-adapters` provides the provider implementation (currently OpenAI-compatible HTTP adapter).

`--intent` is mapped as:

- `commit`, `engineer`, `coding` -> `coding` lane
- `verify`, `fast`, `summary` -> `fast` lane
- `general` -> `general` lane
- anything else -> `custom(<intent>)`

## Policy + Secrets (Owned by dracon-libs)

`dracon-ai` follows the `dracon-libs` resolution behavior.

Canonical files (in `dracon-libs`):

- Provider secrets: `secrets/ai-provider-secrets.json`
- Routing policy: `platform/config/ai-routing-policy.json`

Secrets are referenced by env-var name (for example `ZAI_API_KEY`) via `api_key_env` entries in the routing policy.
Resolution prefers environment variables first, then the secrets file.

## “System Utilities” / Tool Execution (Implemented)

Composition is supported in three ways:

- `dracon-ai do ...` (plan+execute loop, default)
- stdin/file prompt modes (outside the REPL)
- `dracon-ai cmd ...` (one-shot capture+ask; requires `DRACON_AI_ALLOW_CMD=1`)
- REPL slash command: `/cmd <shell>`

Tool execution is intentionally bounded:

- default timeout is short
- output is truncated to a max byte budget
- captured output is injected into context as a `system` message
