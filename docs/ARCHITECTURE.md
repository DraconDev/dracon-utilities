# Architecture

dracon-utilities provides three CLI binaries for local system automation. They install to `~/.local/bin/` and run as systemd user services (except dracon-warden, which uses git hooks).

## Service Overview

```
dracon-utilities/
├── dracon-sync/      →  ~/.local/bin/dracon-sync      (systemd: dracon-sync.service)
├── dracon-system/    →  ~/.local/bin/dracon-system     (systemd: dracon-system-guard.service)
└── dracon-warden/    →  ~/.local/bin/dracon-warden     (git hooks, no systemd service)
```

### dracon-sync — Invisible Git Sync

An auto-commit, multi-mirror daemon that watches repos, commits every change with deterministic facts-based messages, and pushes to GitHub, GitLab, and Codeberg simultaneously.

**Core loop:** watch → detect change → wait for stability → commit → push to origin + mirrors.

**Key design decisions:**
- Deterministic commit messages (no AI) — extractable facts from diffs for `git log --grep=` queries
- Fingerprint-based scheduling — only syncs after the repo state stabilizes for N seconds
- Filter-only cooldown — detects clean/smudge loops and backs off
- Push timeout of 60s per remote — prevents one hung push from blocking the daemon
- `IndexLock` coordination — prevents working-tree writes during git checkout

### dracon-system — Disk & Process Guard

Proactive disk space monitoring, automatic cleanup, and process management.

**Core loop:** check disk → clean stale targets → monitor processes → renice hogs.

**Key design decisions:**
- Graduated renice (never kills) — higher CPU → higher nice value
- Build-aware cleanup — protects target/ dirs with active cargo/rustc processes
- Proactive cleanup at 50% — prevents disk pressure from building up
- Inode monitoring — catches the "many small files" failure mode

### dracon-warden — Repo Encryption & Hardening

Git filter + repo hardening. Encrypts secrets at rest with age encryption while keeping plaintext in the working tree.

**Core mechanism:** `filter.clean` encrypts on staging, `filter.smudge` decrypts on checkout. Git hooks enforce that the filter is configured before allowing commits or pushes.

**Key design decisions:**
- `IndexLock` coordination — prevents race between warden writes and git checkout
- Age x25519 keys — one keypair per machine, pubkeys published per-repo
- DRACON_SECRET markers — encrypted payloads are tagged, not raw ciphertext
- Defense-in-depth — pre-push hook scans for plaintext secrets as a second layer

## dracon-sync: AI-to-AI Commit Protocol

Commit messages are deterministic facts extracted from diffs, not AI-generated prose. This makes the git log a queryable database for downstream AI agents.

### The Protocol

**The Worker (AI agent)** edits files and yields control. It never runs `git commit`.

**The Committer (dracon-sync)** wakes up and:
1. Parses the diff — extracts file counts, line deltas, task state transitions
2. Generates a routing key — `CLOSED: task name | N file(s) in DIRS DELTA:+A/-B`
3. Commits and pushes

### Commit Format

```
[INTENT] | N file(s) in DIRS [files] DELTA:+A/-B [METRICS]
```

Examples:
```
CLOSED: Implement JWT | 3 file(s) in src [auth.py, jwt.py] DELTA:+140/-12 | TEST:45
WIP: Refactor DB | 2 file(s) in src [db.py] DELTA:+50/-10
3 file(s) in src [auth.py] DELTA:+100/-20 | TEST:30 | NEW:src/auth.py DEPS:+reqwest,-hyper
```

Every metric is regex-extracted from the diff. No LLM, no guessing.

### What Gets Extracted

| Metric | Source | Example |
|--------|--------|---------|
| Task transitions | Markdown/text checkbox diffs, usually task notes or local task state | `CLOSED:`, `WIP:` |
| File counts | `git diff --numstat` | `3 file(s)` |
| Changed dirs | Top-level directories | `in src,tests` |
| Line deltas | `git diff --numstat` | `DELTA:+140/-12` |
| Test lines | Files matching `*test*`, `*spec*` | `TEST:45` |
| New files | `git diff --diff-filter=A` | `NEW:src/auth.py` |
| Deleted files | `git diff --diff-filter=D` | `DEL:src/old.py` |
| Deps changed | `Cargo.toml` / `package.json` diff | `DEPS:+reqwest,-hyper` |
| Binary files | `git diff --numstat` with `/- -` | `BIN:1` |

### Why Deterministic Over AI

| Aspect | AI Commit | Deterministic Commit |
|--------|-----------|---------------------|
| Queryability | Can't grep intent | `git log --grep="JWT"` |
| Hallucination | Possible | None |
| Compute cost | High (per commit) | Zero |
| Downstream value | Low (parse prose) | High (structured data) |

## Shared Libraries

The CLI binaries are wrappers. Shared logic lives in a sibling repo:

```
dracon-libs/                (required for building)
├── services/ai/            ← AI adapters, router, lanes
└── tools/sync/dracon-git/  ← git operations library
```

`dracon-libs` must be checked out as a sibling to `dracon-utilities`. Only the CLI binaries get installed.

## Coordination: IndexLock

Both dracon-sync and dracon-warden use `.git/index.lock` to coordinate with git's own checkout process. This prevents the race condition where warden/sync write working-tree files while git is mid-checkout.

- `IndexLock::acquire()` — blocks until the lock is available
- `IndexLock::bypass()` — for explicit user operations (`once`, `repair`)
- Uses `O_EXCL` (atomic create-new) — no TOCTOU race
