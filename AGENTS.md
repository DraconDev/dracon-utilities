# Dracon Utilities

CLI binaries for dracon system services. These install to `~/.local/bin/` and run as systemd user services.

## Architecture

```
dracon-utilities/           <- CLI binaries (this repo)
├── dracon-sync/            -> ~/.local/bin/dracon-sync
├── dracon-system/          -> ~/.local/bin/dracon-system
└── dracon-warden/          -> ~/.local/bin/dracon-warden

dracon-libs/                <- Shared libraries (REQUIRED for building)
├── services/ai/            <- AI adapters, router, lanes
└── tools/sync/dracon-git/  <- git operations library
```

**Key point:** `dracon-utilities` contains the CLI wrappers. `dracon-libs` contains shared library code. Only the CLI binaries get installed.

## Prerequisites

**Required sibling directory:** `dracon-libs` must be checked out as a sibling to `dracon-utilities`:

```
~/Dev/
├── dracon-utilities/    <- this repo
└── dracon-libs/         <- required for building
    ├── services/ai/
    └── tools/sync/dracon-git/
```

Clone if needed:
```bash
git clone https://github.com/your-org/dracon-libs.git ../dracon-libs
```

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

### dracon-system Protected Paths

`dracon-system` protects critical system directories from accidental deletion. The following are always protected (exact match):

`/`, `/home`, `/etc`, `/usr`, `/var`, `/boot`, `/nix`, `/run`, `/sys`, `/dev`, `/proc`

You can add custom protected paths in `dracon-system.toml`:

```toml
[guard]
# Additional directories to protect from cleanup operations (storage --cleanup, empty_trash, etc.)
# Use absolute paths. Paths are canonicalized before comparison.
# protected_paths = ["/mnt/data", "/opt/important"]
```

Safety: every `remove_dir_all` call site in `dracon-system` checks the path against both system and user-protected paths before executing. The `--apply` flag is required for destructive operations.

## CLI Reference

All binaries support `-V, --version` and `-v, --verbose` (repeatable up to 2x for `-vv`).

### dracon-sync

```
dracon-sync [OPTIONS] <COMMAND>
Commands:
  status           Show resolved policy path and sync scope
  repos            One-off report across discovered repositories
  repair-concerns  Repair concern repos (dry-run by default; use --apply to execute)
  repair-warns     Repair warn repos (dirty-only triage; dry-run by default)
  once             Run one sync pass
  daemon           Run continuous sync loop [--interval-secs override]
  sync-now         Sync a specific repository now
  edit-config      Open sync policy in the system editor
  test-ai          Test AI providers connectivity
  stuck            Manage repos permanently stuck on push
  dual-branch      Manage repos with both main and master branches
```

**Nested subcommands:**
- `dracon-sync stuck list` — list stuck repos
- `dracon-sync stuck unstuck <repo>` — unstuck a specific repo
- `dracon-sync dual-branch list` — list repos with dual main/master
- `dracon-sync dual-branch repair <repo>` — consolidate to master

**Global flags:** `-v` / `-vv` increase verbosity; `-V` prints version.

### dracon-system

```
dracon-system [OPTIONS] <COMMAND>
Commands:
  status   Show core path and service status
  doctor   Run deterministic diagnostics
  storage  Analyze storage hotspots [--cleanup] [--apply]
  link     Manage symlink ownership (status | doctor | apply)
  guard    Guard runtime (once | daemon | prune | clean)
  events   Show recent events [-t N] [-s source] [-s severity]
  zram     Zram stats [--status] [--gen-config]
```

### dracon-warden

```
dracon-warden [OPTIONS] <COMMAND>
Commands:
  daemon         Run forever with filesystem event debounce
  once           Run one hardening pass [repo]
  status         Show resolved policy path and watch roots
  filter-clean   Git filter clean (stdin -> stdout)
  filter-smudge  Git filter smudge (stdin -> stdout)
  scrub-markers   Scan DRACON_SECRET markers [--apply] [repo]
  resmudge       Fix ciphertext stuck in working tree [--apply] [repo]
  repair         System-wide repair pass [--dry-run] [--strict] [repo]
  keygen         Generate new age keypair
```

## AI Configuration

dracon-sync uses AI for commit messages (scribe) and version bumping. Configure providers in `~/.dracon/ai.toml`:

```toml
[[providers]]
name = "mistral"
env = "MISTRAL_API_KEY"
endpoint = "https://codestral.mistral.ai/v1"
model = "codestral-latest"

[[providers]]
name = "nvidia"
env = "NVIDIA_API_KEY"
endpoint = "https://integrate.api.nvidia.com/v1"
model = "stepfun-ai/step-3.5-flash"

[[providers]]
name = "openrouter"
env = "OPENROUTER_API_KEY"
endpoint = "https://openrouter.ai/api/v1"
model = "nvidia/nemotron-3-super-120b-a12b:free"

[[providers]]
name = "openrouter"
env = "OPENROUTER_API_KEY"
endpoint = "https://openrouter.ai/api/v1"
model = "openrouter/free"
```

### API Keys

Store keys in `~/.dracon/ai/secrets/*.env`:
- `mistral.env` → `MISTRAL_API_KEY=...`
- `nvidia.env` → `NVIDIA_API_KEY=...`
- `openrouter.env` → `OPENROUTER_API_KEY=...`

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
