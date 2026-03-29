# Dracon Utilities

CLI binaries for dracon system services. These install to `~/.local/bin/` and run as systemd user services.

## Architecture

```
dracon-utilities/           <- CLI binaries (this repo)
├── dracon-sync/            -> ~/.local/bin/dracon-sync
├── dracon-system/          -> ~/.local/bin/dracon-system
└── dracon-warden/          -> ~/.local/bin/dracon-warden

dracon-libs/tools/          <- Shared libraries (not installed)
├── sync/dracon-git/        <- git operations library
├── system/dracon-system/   <- system diagnostics library
└── config/dracon-config/   <- config parsing library
```

**Key point:** `dracon-utilities` contains the CLI wrappers. `dracon-libs` contains shared library code. Only the CLI binaries get installed.

## Installation

All binaries install to `~/.local/bin/`:

```bash
./install.sh
```

## Services

Services are in `~/.config/systemd/user/`:

| Service | Binary | Purpose |
|---------|--------|---------|
| dracon-sync.service | dracon-sync daemon | Git sync automation |
| dracon-system-guard.service | dracon-system guard daemon | Disk/process protection |
| dracon-warden.service | dracon-warden daemon | Security hardening |

```bash
# Restart after install (install.sh does this automatically)
systemctl --user restart dracon-sync.service
systemctl --user restart dracon-system-guard.service
systemctl --user restart dracon-warden.service
```

## Policy Files

| Utility | Policy Path |
|---------|-------------|
| dracon-sync | ~/.dracon/utilities/sync/dracon-sync.toml |
| dracon-system | ~/.dracon/utilities/system/dracon-system.toml |
| dracon-warden | ~/.dracon/utilities/warden/dracon-warden.toml |

## AI Configuration

dracon-sync uses AI for commit messages (scribe) and version bumping. Configure providers in `~/.dracon/ai.toml`:

```toml
[[providers]]
name = "openrouter"
env = "OPENROUTER_API_KEY"
endpoint = "https://openrouter.ai/api/v1"
model = "openrouter/free"

[[providers]]
name = "gemma"
env = "GOOGLE_API_KEY"
endpoint = "https://generativelanguage.googleapis.com/v1beta"
model = "gemma-3-27b-it"
adapter = "gemini"
auth_header = "x-goog-api-key"
auth_prefix = ""

[[providers]]
name = "nvidia"
env = "NVIDIA_API_KEY"
endpoint = "https://integrate.api.nvidia.com/v1"
model = "nvidia/nemotron-3-nano-30b-a3b"
```

### API Keys

Store keys in `~/.dracon/ai/secrets/*.env`:
- `openrouter.env` → `OPENROUTER_API_KEY=...`
- `gemini.env` → `GOOGLE_API_KEY=...`
- `nvidia.env` → `NVIDIA_API_KEY=...`

### Test AI Providers

```bash
dracon-sync test-ai
```

## The Scribe

The scribe is **you** (AI). The daemon maintains `.dracon/project-state.md` in each repo. AI generates commit messages that include the "Current Focus" line.

### Format

```markdown
# Project State

## Current Focus
{one line: what you're working on right now}

## Completed
- [x] {what you finished, with context}

## In Progress
- [x] {what you're actively working on}

## Open Issues
- {blockers, decisions needed, things to investigate}
```

### Rules

- **Current Focus** must be one line — it becomes the commit body
- Be specific: "Fix TOCTOU race in warden keygen" not "fix bugs"
- Include context: "Binary files bypass encryption — needs user decision on approach"
- Don't document mechanical changes — only semantic state
- If the file doesn't exist, create it when you have something meaningful to say
