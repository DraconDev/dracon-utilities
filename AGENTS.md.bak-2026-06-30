# AGENTS.md — how we work in dracon-utilities

> **Audience**: AI agents and human operators working in this
> repository. This file documents the durable, ongoing
> behaviors of the `dracon-sync` daemon — what to expect
> and how to interact with it.

## Commit policy (the most important section)

**Default behavior (since 2026-06-17, after
`pi-tmp-persist-policy-2026-06-16.md`)**: the daemon
**commits ALL untracked files by default** —
`untracked_exclude_patterns = []` in the global config.
The only things the daemon refuses to auto-commit are:

1. **Files larger than 100 MiB** (`max_stage_file_bytes`)
2. **Things git already ignores** (`.gitignore` rules)
3. **Per-repo opt-outs** (only when a specific repo
   sets `untracked_exclude_patterns` in its
   `.dracon/dracon-sync.toml`)

### What gets committed automatically

Everything else, including:
- User notes (`NOTE.md`, `notes.md`, `scratch.md`)
- Audit evidence (`audit/`, `evidence/`, `screenshots/`)
- Media files (`*.png`, `*.jpg`, `*.mp4`, `*.mov`)
- Logs and database files (`*.log`, `nohup.out`,
  `*.sqlite`, `*.db`)
- Session-scratch files (`.pi-tmp/`, `scratch/`, `tmp/`,
  `.demon/`, `.sisyphus/`, `.ralph/`)
- Source code, docs, configs, scripts, tests

### Operator's framing (the why)

> "the most sensible thing is that we have a global
> rule, and unless it's something that would be very
> wrong to put on the repo we put it there. i think
> all untracked excludes arguably are wrong. just
> because they are short lived files doesn't mean
> we shouldn't put them there."

The old list (`**/scratch/**`, `**/pi-tmp/**`, etc.)
conflated "short-lived" with "very wrong to commit".
They are not the same thing. Short-lived files are
valid git content: the user/agent can `rm` them from
the working tree when they're done, and the daemon
will commit the deletion. If the user wants to
recover, the file is in git history.

Things that ARE very wrong to commit (handled
elsewhere, NOT by `untracked_exclude_patterns`):
- **Secrets in plaintext** → warden owns the
  encryption flow
- **Files > 100 MiB** → `max_stage_file_bytes = 104857600`
- **Build artifacts** (node_modules/, target/,
  build/, dist/) → already in `.gitignore`

### Size limit

Files larger than **100 MiB** (`104857600` bytes) are
NOT auto-staged. This is the hard exclusion threshold.

### Push timeouts

`push_op_timeout_secs = 300` (CHANGED 2026-06-17 from 60).
This matches the daemon's own code default
(`default_push_op_timeout_secs` in `dracon-sync/src/policy.rs`)
and gives a 5x safety margin over the v0.112.10 measured >60s
push time for a 23-file PNG-heavy commit. Per-remote timeouts
(e.g. 60s for github, 300s for gitlab/codeberg) would be more
precise but require a daemon code change to add the field to
`RemoteConfig`; deferred to a follow-up daemon release. The
global 300s is wasteful for github (which never takes more
than a few seconds) but harmless — the daemon times out via
process kill, not via waiting. See
`docs/design/push-timeout-fix-2026-06-17.md` for the full
data, rationale, and runbook.

### Debounce window (untracked files)

The daemon has a **3-second debounce** before processing a
file change, plus the time to `git add` + `git commit` + push
(typically 3-6 seconds). This means **a file may appear
untracked for 3-49 seconds** between creation and the
daemon's auto-commit:

- **Low churn** (no other files in the same repo): 3-9 seconds
- **High churn** (many files committed in parallel, e.g., a
  Playwright smoke-out PNG batch): up to 49 seconds

This is **normal daemon behavior**, not a bug. The "untracked"
status in `git status` during this window is the working tree
state, not a daemon refusal to commit. If a file is untracked
for **> 2 minutes**, investigate:
1. `journalctl --user -u dracon-sync.service --since "2m ago"`
2. Check `git status` and the per-repo `.gitignore`
3. Check the global config: `untracked_exclude_patterns` (should be `[]`)
4. Check the per-repo `.dracon/dracon-sync.toml` for
   `auto_commit_exclude_patterns`

Audit evidence: `docs/design/untracked-audit-2026-06-17.md`

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
