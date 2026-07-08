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

> **REMOVED in draft 2026-06-30 (audit goal `mr0q2qx2-mvfs0c`)**:
> The single-line "NEVER" list below was misleading because the daemon's
> own auto-repair path (`dracon-sync/src/report.rs:3705` and
> `report_v2_snapshot.rs:3166`) calls
> `rewrite_ahead_paths()` (`dracon-sync/src/git/staging.rs:148-244`)
> which uses `git filter-repo --invert-paths --force` when
> `auto_repair_concerns = true` (default per
> `dracon-sync/src/policy.rs:1580`). The 2026-06-30 audit confirmed
> this code path has never fired on our repos (zero
> `backup/pre-sync-largeblob-fix-*` branches found), but the
> contradiction between "NEVER rewrite history" and the default
> config warranted replacement rather than ad-hoc violation tracking.

Each rule is now spelled out with the relevant context — including
the daemon-side enforcement (or absence thereof) — so future
operators know what is human-policy, what is daemon-enforced, and
what falls in between.

### For HUMAN operators (mirrors what the daemon does NOT do)

- **`git add .`** — never use it; always specify explicit paths.
  The daemon itself uses `git add -A -- <explicit-paths>` and
  `git add -A -f -- <explicit-paths>` (see
  `dracon-sync/src/sync.rs:858,859`) — a safer equivalent.
- **Force-push to repos with > 5 commits ahead** — daemon default
  is `force_push_when_behind = false` (`dracon-sync/src/git/mod.rs:684+`).
  `--force-with-lease` may be used for one-commit-behind divergences,
  and explicit operator overrides are tracked per-incident (see
  `docs/design/push-stuck-resolution-2026-06-27.md` for the override
  template).
- **Auto-commit secrets** (`.env`, `*.pem`, `*.key`, `*.age`,
  `secrets/**`) — warden's job per "What the daemon does NOT do"
  section above. The daemon respects `.gitignore` (via
  `git add --others --exclude-standard`) but does not have its own
  secret scanner.

### For BOTH humans AND daemon

- **Reconnect legacy private remotes** — there is no automated
  path for this. The daemon's `test_load_secret_or_legacy_pat_*`
  test in `dracon-sync/src/git/mod.rs:370+` covers a fallback
  to a legacy secrets dir at runtime, but no production code
  reconnects old remote URLs.

### For DAEMON only (cannot be violated without changing config)

- **History rewrite via `filter-repo --invert-paths --force`** —
  This is what the daemon does *automatically* during auto-repair
  when `auto_repair_concerns = true` (default) detects large blobs
  ahead. Code paths:
  - Function definition: `dracon-sync/src/git/staging.rs:152`
  - Call sites: `dracon-sync/src/report.rs:3705`,
    `dracon-sync/src/report_v2_snapshot.rs:3166`
  - Default: `dracon-sync/src/policy.rs:1580`
    (`auto_repair_concerns: true`)

  Audit history (2026-06-30): found 1 historical auto-rewrite
  backup branch in `/home/dracon/Dev/avid/` —
  `backup/pre-sync-largeblob-fix-1780417168` (branch tip
  332a456, dated 2026-06-02 17:14:29 +0100, **before** this
  AGENTS.md file existed). The branch is local-only (not pushed
  to codeberg/github/gitlab). The branch tip suggests the
  rewrite was about ML model files (`models/parakeet-ctc/onnx/`)
  being staged but exceeded a threshold.

  Since that single pre-AGENTS.md incident, **no further
  rewrites have fired** on any watched repo (verified by
  searching for `backup/pre-sync-largeblob-fix-*` branches:
  only the one in `avid` exists). The daemon creates a backup
  branch before each rewrite, so any new
  `backup/pre-sync-largeblob-fix-*` branch is a fault requiring
  operator review.

  To PREVENT the daemon from auto-rewriting on a specific repo:
  ```toml
  # .dracon/dracon-sync.toml
  auto_repair_concerns = false
  ```
  Use case: working in a `kiki-sassy` or `one-mil-girls`-style
  operator-owned repo where history is sacred.

### What history-rewrite means here

Filter-repo --invert-paths (or filter-branch --index-filter)
removes specified paths from every commit in the local repo
ahead of the remote, then force-pushes. Different from:

- **Orphan commit cutover** (`git checkout --orphan + git read-tree`):
  creates a NEW root commit on a separate branch, leaves the old
  history intact. Example: the 2026-06-30
  `migration-light/medium/heavy` cutovers in `dracon-platform`.
- **Local-only history operations** (rebase, reset, checkout):
  don't touch the remote until you push.

If you intend a filter-repo style rewrite, the daemon will do it
for you on any large-blob-ahead concern; you don't need to invoke
it manually. If you intend a rebase or orphan cutover, that's a
separate operation not covered by the daemon's auto-repair.

- **Delete operator-owned repos** (kiki-sassy, one-mil-girls)
  without explicit approval — there is no `daemon-sync rm`
  command. Repo deletion is a manual `gh`/`glab`/`codeberg-cli`
  action that requires operator authorization.

## Submodule standalone worktree design

CHANGED 2026-07-02 (goal `mr3g843f-lajfpg`/`354fe3cb`): The standalone
worktree layout was eliminated for all 10 game/hegemon submodules of
`dracon-platform`. The canonical architecture is now:

- **Nested submodule checkout** at
  `/home/dracon/Dev/dracon-platform/web/games/<wip|released>/<name>/`:
  on branch `main` directly (NOT detached). This is the only
  worktree per shared gitdir — there is no standalone at
  `/home/dracon/Dev/<name>/`.

- **Daemon watches the nested path** directly. Auto-commit +
  auto-push happen from `/Dev/dracon-platform/web/games/<wip|released>/<name>/`.
  The parent (`dracon-platform`) tracks the submodule via a
  gitlink that advances in lockstep with the nested's `main` SHA.

- **Convergence invariant**: for each submodule, the parent's
  tracked gitlink SHA equals the shared gitdir's `refs/heads/main`
  SHA. The daemon enforces this via `stage_gitlink_updates`
  (writes the gitlink via `git update-index --cacheinfo` when
  the nested has advanced `main`).

- **Why no standalone**: as of 2026-07-02, all 10 /Dev/<name>/
  standalones have been removed (`git worktree remove --force`).
  The shared gitdirs (at `/Dev/dracon-platform/.git/modules/web-games-<name>/`)
  remain intact; only the second worktree per gitdir was removed.

  REMOVED 2026-07-08 (goal `730eaf2a`): the daemon's
  `materialize_pending_submodules` path had been RE-CREATING
  top-level standalones (`/Dev/darklord` on 2026-07-04,
  `/Dev/junk-runner` on 2026-07-08) whenever a nested submodule
  was detached from `main` — silently defeating the 2026-07-02
  migration. The materialization code (`materialize_submodule`
  in `sync.rs` and the off-`main` branch in
  `materialize_pending_submodules`) was removed; the daemon now
  only configures multi-remote push for nested-on-`main`
  submodules and NEVER creates a standalone worktree. The two
  re-created standalones were pruned via `git worktree remove
  --force`.

- **Migration history**:
  - 2026-07-01 (`mr1x7j5i-zioba9`): initial layout with
    `daemon-standalone` branch and standalones at /Dev/<name>/.
    See `docs/design/daemon-standalone-removal-2026-07-01.md`.
  - 2026-07-02 (`mr3g843f-lajfpg`): pilot migration of
    junk-runner (removed standalone, switched nested to main).
  - 2026-07-02 (`354fe3cb`): bulk migration of all 10 games.
    Discovered and fixed 3 daemon bugs:
    1. Detached-HEAD push failed with "destination is not a full
       refname" — fixed by using `HEAD:refs/heads/main` refspec
       when the worktree is detached.
    2. `current_branch` only checked `<repo>/.git/HEAD` (wrong
       for worktree-style checkouts) and accepted the literal
       "HEAD" from `git rev-parse --abbrev-ref HEAD` — fixed
       by reading the worktree's actual HEAD file and filtering
       "HEAD".
    3. `default_trusted_remote_hosts` was case-sensitive
       (lowercase `dracondev` only; SSH URLs use `DraconDev`)
       — fixed by adding uppercase entries.
  - 2026-07-08 (`730eaf2a`): removed the daemon's standalone
    materialization (`materialize_submodule` + the off-`main`
    branch of `materialize_pending_submodules`) that had been
    silently re-creating `/Dev/darklord` and `/Dev/junk-runner`
    since the 2026-07-02 migration. Pruned both re-created
    standalones. Nested-on-`main` submodule discovery +
    multi-remote config is unchanged.

- **No more `daemon-standalone` branch**: the buffer branch was
  removed during the 2026-07-01 migration. After the 2026-07-02
  bulk migration, the standalone worktrees themselves are gone.
  **hegemon's GitHub is now synced** (2026-07-08): the daemon's
  2GiB pack-size guard had skipped hegemon's GitHub push because
  `.git` was inflated to ~4.9 GiB by DANGLING objects (a divergent
  gitlab fetch done during reconciliation + pre-existing garbage),
  not by real history. `git gc --prune=now` dropped it to ~163 MiB,
  so the daemon now pushes hegemon `main` to GitHub natively — no
  history rewrite was needed. (Earlier drafts claimed hegemon's
  GitHub "remains empty" due to 2.4GiB of MP3 files; that was
  wrong — the 4.9 GiB was garbage, not assets.) See
  `docs/design/nested-on-main-architecture-2026-07-02.md` for
  the new architecture and migration log.

- **`fast_forward_daemon_standalone_to_main`** is a no-op stub
  preserved for backwards compatibility with existing call sites.
  No standalones exist (re-confirmed 2026-07-08, goal
  `730eaf2a`, after the materialization path was removed and the
  two re-created standalones were pruned), so the function is
  never invoked.

## Test discipline

- `cargo test --workspace --locked` must pass
- `cargo build --release --locked` must succeed
- `cargo deny check` must be clean
- New code paths require unit tests
- Backwards compatibility with all previously added
  policy fields is required
