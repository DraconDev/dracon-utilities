# dracon-code

`dracon-code` is a small repo-scaffolding + context persistence utility.

It exists to support a "one way" coding workflow:

- Human writes intent in `do.md` (single file).
- AI/tools write state into `plan/` (log, context, decisions).
- Git is the versioned record of the work.

This tool does not own sync/warden/system roles. It just writes/reads files inside the repo.

## Commands

### `dracon-code init`

Creates (if missing):

- `do.md` (human-owned intent)
- `plan/README.md`
- `plan/roadmap.md`
- `plan/chat.md`
- `plan/CONTEXT.md`
- `plan/DECISIONS.md`

It will not overwrite existing files unless `--force` is passed.

### `dracon-code append`

Appends a message to `plan/chat.md`.

- `--stdin` reads the message from stdin.
- otherwise positional text is used.

### `dracon-code snapshot`

Writes a quick repo snapshot (cwd, git status) into `plan/CONTEXT.md`.
