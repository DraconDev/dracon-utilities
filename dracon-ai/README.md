# dracon-ai

`dracon-ai` is the **only** Dracon AI CLI. It is intentionally thin: it does **not** implement provider/model wiring itself.

Instead, it consumes the canonical AI runtime from `dracon-libs` (policy, secrets, routing, and adapters). This keeps Dracon’s “one way” rule: the utility is just a shell, the platform is the library.

## Non-Goals

- No direct provider hookup logic in this repo (no OpenRouter/OpenAI/Anthropic “native” client logic in `dracon-ai`).
- No AI dependencies in deterministic daemons (`dracon-sync`, `dracon-warden`, `dracon-system`).

## Commands

### `dracon-ai status`

Shows the resolved AI runtime view from `dracon-libs`:

- provider specs count
- active model ids
- dev model ids

### `dracon-ai chat [--intent <intent>] <prompt|->`

Sends a single prompt through the `dracon-libs` routing runtime.

- `--intent` is treated as a lane hint (mapped to `dracon-libs` routing tasks).
- If the prompt argument is `-`, `dracon-ai` reads the prompt from stdin (so it composes with system utilities).

Examples:

```sh
dracon-ai chat "Say 'ok' only."
printf "Say 'ok' only.\n" | dracon-ai chat -
dracon-ai chat --intent engineer "Refactor this function."
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

## “System Utilities” / Tool Execution (Planned)

Right now, composition is done via stdin piping (use `-`).

Future direction (not implemented yet):

- An explicit tool registry (`--allow-exec`) with strict schemas and tight output bounds.
- Deterministic transcripts for any tool calls.
- Sandbox-by-default behavior so the AI CLI cannot “just run anything”.

