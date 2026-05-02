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

## Design Philosophy: Sync is Invisible Infrastructure

dracon-sync is designed to be **invisible infrastructure** for an AI coder. The AI works on one repo at a time, makes changes, and sync handles the rest — the AI never needs to think about commits, pushes, or cross-repo coordination.

**The AI workflow:**
1. User says "work on dracon-utilities"
2. AI reads `dracon-utilities/.dracon/project-state.md`
3. AI makes changes
4. Sync daemon auto-commits and pushes
5. Done

**What sync provides:**
- Auto-commit on every change (AI doesn't need to think about git)
- project-state.md as the AI's working memory (context survives sessions)
- Incident ledger for debugging (AI can read what went wrong)
- Freezing for pause (AI can pause sync during delicate operations)

**What sync doesn't need:**
- Global workspace state (AI works on one repo at a time)
- Session logging (AI doesn't "resume" — each session is fresh)
- Interactive features (AI runs non-interactively)

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

## Operational State

Operational state (mutable files written at runtime) lives **outside the `.dracon` git tree** to prevent self-referential churn:

```
~/.local/state/dracon/
├── dracon-sync-incidents.jsonl   # Append-only incident ledger (2MB, 10k+ lines)
└── dracon-sync-stuck-push-repos.json  # Stuck push tracking
```

The incident ledger is appended every sync cycle. Keeping it at `~/.local/state/dracon/` instead of inside `.dracon` prevents the sync daemon from auto-committing its own operational data.

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

### dracon-sync Repo Discovery

Repo discovery searches up to **4 levels deep** from each watch root. Dot-prefixed directories (e.g. `.config/`, `.dracon/`) are descended into if they contain a `.git` directory — only skipped after the `.git` check fails. The hardcoded exclusions are `objects` and whatever is in `exclude_dir_names` from policy.

### dracon-sync Push Behavior

Push operations use `push_with_retries` with SSH hardening (`ConnectTimeout`, `ConnectionAttempts`) and automatic HTTPS fallback on persistent timeout. The `push_retries` policy setting is respected. All transient network failures should now trigger retries rather than failing immediately.

### dracon-sync Automatic Remote Creation

When `auto_github_private = true` in `dracon-sync.toml`, any repo in a watched root without an origin remote will automatically get:

1. A private GitHub repo created via `gh repo create --private`
2. SSH remote added: `git@github.com:<account>/<repo>.git`
3. Initial commit pushed: `git push -u origin HEAD`

Requirements: `gh` CLI installed and authenticated (`gh auth status`).

```toml
[sync]
auto_github_private = true
auto_github_private_account = "YourOrgOrUsername"
```

**⚠️ CRITICAL: NEVER create suffixed repos (repo-1, repo-2, repo-N).**
If the GitHub repo already exists, reuse it. A previous suffix loop in `create_github_private_remote` created 15+ orphan repos (`dracon-demons-1` through `-9`). This happens when `gh repo create` fails with "Name already exists" and the code appends `-1`, `-2` instead of just reusing the existing repo. This pattern is explicitly banned in all repo creation functions.

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

dracon-sync uses AI for commit messages (scribe) and version bumping. Configure providers in `~/.dracon/utilities/sync/ai.toml`:

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

Store keys in `~/.dracon/utilities/sync/ai/secrets/*.env`:
- `mistral.env` → `MISTRAL_API_KEY=...`
- `nvidia.env` → `NVIDIA_API_KEY=...`
- `openrouter.env` → `OPENROUTER_API_KEY=...`

### Test AI Providers

```bash
dracon-sync test-ai
```

## The Scribe: AI Working Memory

The scribe is **you** (AI). The daemon maintains `.dracon/project-state.md` in each repo. This file is the **primary interface** between sync and the AI coder.

### Why Frequent Commits?

Sync commits every change because:
- The AI reads git history to understand past work
- Every commit is a checkpoint the AI can recover to
- More commits = better context for the AI's "what was I doing?"
- Commits are cheap; context is valuable

### Format

```markdown
# Project State

## Current Focus
{one line: what you're working on right now}

## Context
{why: what problem are you solving? what prompted this change?}

## Completed
- [x] {what you finished, with context}

## In Progress
- [x] {what you're actively working on}

## Blockers
- {what's stopping progress: missing info, user decision needed, dependency}

## Next Steps
1. {immediate next action}
2. {what comes after}
```

### Rules

- **Current Focus** must be one line — it becomes the commit body
- **Context** helps the AI recover understanding after time away
- **Blockers** tell the AI what it can't proceed on
- **Next Steps** give the AI a clear path forward
- Be specific: "Fix TOCTOU race in warden keygen" not "fix bugs"
- Don't document mechanical changes — only semantic state
- If the file doesn't exist, create it when you have something meaningful to say

### Example

```markdown
# Project State

## Current Focus
Refactor incident ledger to XDG state directory

## Context
The 2MB incident ledger was inside the .dracon git repo, causing
self-referential churn. Every sync cycle added to the ledger,
which dirtied the repo, which triggered another commit. Moved
the ledger to ~/.local/state/dracon/ to break the cycle.

## Completed
- [x] Moved incident_ledger_path() from ~/.dracon/ to ~/.local/state/dracon/
- [x] Moved stuck_repos_path() to XDG-compliant location
- [x] 248 tests passing after path changes

## Blockers
- None

## Next Steps
1. Monitor for 24h to confirm no self-referential churn
2. Continue reviewing dracon-system for any orphaned state files
```
