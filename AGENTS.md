# AGENTS.md — how we work in dracon-utilities

> **Audience**: AI agents and human operators working in this
> repository. This file documents the durable, ongoing
> behaviors of the `dracon-sync` daemon — what to expect
> and how to interact with it.

## Commit policy (the most important section)

**Default behavior (since 2026-06-15, goal
`9aaf0b08` / `546d4f9c`)**: the daemon **commits all
untracked files by default**. Only files matching
genuine session-scratch patterns are kept untracked.

### What gets committed automatically

Everything else, including:
- User notes (`NOTE.md`, `notes.md`, `scratch.md`)
- Audit evidence (`audit/`, `evidence/`, `screenshots/`)
- Media files (`*.png`, `*.jpg`, `*.mp4`, `*.mov`)
- Logs and database files (`*.log`, `nohup.out`,
  `*.sqlite`, `*.db`)
- Source code, docs, configs, scripts, tests

### What stays untracked (super-good reasons)

These patterns are excluded from auto-staging:

```text
**/scratch/**, **/scratch-*, **/scratch_*
**/tmp/**, **/tmp-*
**/pi-tmp/**, **/.pi-tmp/**
**/research/scratch/**
.demon/**, .sisyphus/**, .ralph/**
```

These are session-scratch directories, agent session
state, and temp directories. They are ephemeral by
design.

### Size limit

Files larger than **100 MiB** (`104857600` bytes) are
NOT auto-staged. This is the hard exclusion threshold.

### Per-repo overrides

The per-repo `.dracon/dracon-sync.toml` can extend the
exclude list with `auto_commit_exclude_patterns`. Example
from `Junk-Runner-bevy`:

```toml
auto_commit_exclude_patterns = [
    "**/test-results/**",
    "**/e2e/screenshots/**",
]
```

This prevents the 2989-commit auto-commit loop that
crashed the daemon originally. The override mechanism
still works under the new global default.

> **REMOVED 2026-06-15 (goal `76ddaa7e`)**:
> The `auto_commit_exclude_patterns` for
> `**/test-results/**` and `**/e2e/screenshots/**`
> was removed from `Junk-Runner-bevy/.dracon/
> dracon-sync.toml` and the `reports/kdp-live-*.md`
> was removed from `rust-ai-web-auto/.dracon/
> dracon-sync.toml`. The operator's new policy is
> "commit all untracked" with NO per-repo
> exceptions. Per-repo override mechanism still
> works for future operator-set exceptions (with
> a documented reason in the .toml file).

## Commit-all principle (2026-06-16, goal `6205ad1f`)

The operator's stated principle:

> "git sync just has to make sure that nothing
> left out unless we have a very good reason to
> leave it out"

This means: the daemon's commit-all policy is the
correct default. **The ONLY valid reasons to leave
a file untracked are:**

1. **Scratch/temp dirs** (ephemeral by design):
   `**/scratch/**`, `**/pi-tmp/**`, `.demon/**`,
   `.sisyphus/**`, `.ralph/**`, etc.
2. **Size limit**: files larger than 100 MiB are
   not auto-staged
3. **Sensitive files**: `.env`, `*.pem`, `*.key`,
   `*.age`, `secrets/**` are NEVER auto-staged
   (warden's job to encrypt or block; the
   `.gitignore` rules in the daemon-managed block
   enforce this)
4. **Per-repo `auto_commit_exclude_patterns`**
   only when the operator has explicitly set them
   in `.dracon/dracon-sync.toml` with a documented
   reason in the file

Any file that is not in one of these categories
should be auto-staged and committed. If the daemon
sees an untracked file outside these categories,
that is a bug or a misconfigured override.

### What the daemon does NOT do

The daemon does NOT auto-stage files inside
gitignored directories (e.g., `target/`,
`node_modules/`, `build/`, `dist/`, `archives/`).
Those are already in `.gitignore` via the
`hygiene_patterns` in warden's config, and the
daemon respects `.gitignore` via
`git add --others --exclude-standard`.

### What the operator must NOT do

- **NEVER add a "NEVER auto-stage" rule to a
  per-repo `.dracon/dracon-sync.toml`** unless
  the rule has a documented good reason. The
  `browser-extensions-shared` "NEVER auto-stage
  the untracked markdown" constraint (from goal
  `76ddaa7e`) was REMOVED in goal `c19d21b8`
  because it was based on a misunderstanding:
  the untracked `.md` was a deliverable
  cross-linked from a tracked file.

## Investigation-first discipline

When investigating a state anomaly, **read all the
existing design docs first** before forming a hypothesis.
Recent design docs (in `docs/design/`) cover:

- `commit-all-policy-2026-06-15.md` — this policy
- `commit-all-principle-2026-06-16.md` — the operator's
  stated principle and the audit of every
  "preserve untracked" exception
- `dracon-libs-deletion-2026-06-15.md` — symlink deletion
- `junk-runner-investigation-2026-06-15.md` — Junk-Runner-bevy policy drift
- `dracon-platform-untracked-commit-2026-06-15.md` — what stays untracked in dracon-platform (and why)
- `sync-push-classification.md` — push rejection classification
- `source-encryption-incident-2026-06-15.md` — encryption incident
- `warden-plaintext-sibling.md` — warden plaintext sibling handling
- `ownership-investigation-2026-06-15.md` — repo ownership analysis

Design docs are durable. Re-read them.

## Daemon commands

- `dracon-sync repos` — live state of all watched repos
- `dracon-sync doctor` — diagnose a specific concern
- `dracon-sync repair-concerns --apply` — apply a fix for a known concern
- `systemctl --user status dracon-sync.service` — daemon health

## Forbidden actions

- **NEVER** use `git add .` — always explicit paths
- **NEVER** force-push to repos with > 5 commits ahead
- **NEVER** rewrite history
- **NEVER** reconnect legacy private remotes
- **NEVER** delete operator-owned repos (kiki-sassy,
  one-mil-girls) without explicit approval
- **NEVER** auto-commit `.env`, `*.pem`, `*.key`,
  `*.age`, `secrets/**`

## Test discipline

- `cargo test --workspace --locked` must pass
- `cargo build --release --locked` must succeed
- `cargo deny check` must be clean
- New code paths require unit tests
- Backwards compatibility with all previously added
  policy fields is required
