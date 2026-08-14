# AGENTS.md — how we work in dracon-utilities

> **Audience**: AI agents and human operators working in this
> repository. This file documents the durable, ongoing
> behaviors of the `dracon-sync` daemon — what to expect
> and how to interact with it.

## Repository architecture (READ THIS FIRST)

`dracon-utilities` is a **meta-only repo**. It tracks **no Rust
source** — only meta files: `AGENTS.md`, `CHANGELOG.md`, the audit
docs (`AUDIT-*.md`, `AUDIT_REPOS_*.md`, archived `release-notes-v0.112.*.md`),
`.cargo/config.toml`, the workspace `Cargo.toml`/`Cargo.lock`, and
`.pi/goals/**`.

The 3 utilities live in **nested standalone git repos** under this
directory, each with its own `.git/`, its own remotes
(codeberg/github/gitlab), its own history, tags, and CHANGELOG:

- `dracon-sync/` → `codeberg:dracondev/dracon-sync-background-auto-commit-multi-remote`
- `dracon-system/` → `codeberg:dracondev/dracon-system-disk-process-guard-doctor`
- `dracon-warden/` → `codeberg:dracondev/dracon-warden-secret-encrypt-age-git-filter`

The parent `Cargo.toml` is a plain `[workspace]` manifest listing the
three utility crates plus the in-tree `dracon-warden/src/security`
workspace member, so the AGENTS.md test-discipline commands (`cargo
build --release --locked`, `cargo test --workspace --locked`, `cargo
deny check`) work from the monorepo root by path — it does **not**
submodule or symlink the source. Because the source is in the nested repos, `git status` at
the parent shows `?? dracon-sync/` etc. as "untracked": these are the
nested repos themselves, **not** lost files. To edit a utility, `cd`
into its nested directory and work there; the daemon commits each
nested repo independently.

This mirrors the `dracon-platform/web/games/<name>/` nested-on-`main`
submodule design documented below, but here the crates are full
standalone repos rather than git submodules.
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

`push_op_timeout_secs = 900` — an OPERATOR bump, not the daemon's
code default (FIXED 2026-08-09, audit MEDIUM: this section previously
claimed it "matches the daemon's own code default"; it does not).
History: 60 (pre-2026-06-17) -> 300 (2026-06-17, which DID match the
daemon's code default) -> 900 (2026-06-23, after a 50-commit/5000+-
file gitlab push kept timing out at 300s; see
`~/.dracon/utilities/sync/dracon-sync.toml` comments for the full
rationale). The daemon's code default REMAINS 300
(`default_push_op_timeout_secs` in `dracon-sync/src/policy.rs`), so a
fresh deployment without the config override gets 300s — operators
tuning from this file must not treat 900 as the baseline. The 900
gives a generous safety margin over the v0.112.10 measured
>60s push time for a 23-file PNG-heavy commit. Per-remote timeouts
(e.g. 60s for github, 900s for gitlab/codeberg) would be more
precise but require a daemon code change to add the field to
`RemoteConfig`; deferred to a follow-up daemon release. The
global 900s is wasteful for github (which never takes more
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

### Excluded-path semantics (CHANGED 2026-07-22, v0.112.34)

`auto_commit_exclude_patterns` means **"don't auto-commit
these files"** — nothing more. After each commit, the
daemon UNSTAGES excluded files (so its own `git add -A`
doesn't sweep them into YOUR next manual commit) but
**preserves their worktree content**. Your edits to
excluded files stay on disk, visible in `git status` as
modified-unstaged.

Before v0.112.34, the daemon ran
`git restore --staged --worktree` on excluded files after
every commit — **silently deleting the operator's
uncommitted edits** (audit F1.16). That data-loss default
was wrong for a knob named "exclude from auto-commit".

Operators who WANT hygiene enforcement ("these files must
always equal HEAD") opt in **per-repo**:

```toml
# .dracon/dracon-sync.toml
revert_excluded_to_head = true
```

Destructive behavior requires an explicit opt-in; it is
never the silent default.

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
5. **Regeneratable audit frame dumps** (ADDED
   2026-07-23, deathrun bloat fix): `.pi/
   chrome-screenshots/` and `audit-*/screenshots/`
   are warden-`.gitignore`d fleet-wide (via
   `hygiene_patterns` in `dracon-warden.toml`).
   The audit `.md` REPORTS still go up (they are
   the deliverable); the frame dumps are
   regeneratable on demand and were the source of
   deathrun's 2.85 GiB pushable-branch bloat that
   tripped github's 2 GiB pack limit. See
   `docs/design/audit-screenshot-bloat-deathrun-2026-07-23.md`.

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
- `codeberg-quota-leak-fix-2026-07-13.md` — forward-only
  daemon pattern fix + `scan-bloat` discovery loop.
  Live state: 85 GiB used / 85 GiB grace quota. 9 DIR-level
  patterns in `default_untracked_exclude_patterns` prevent
  new leaks. Historical cleanup is deferred (would require
  git filter-repo + force-push across 17 repos); see the
  design doc for the full plan + risk analysis.
- `dracon-libs-deletion-2026-06-15.md` — symlink deletion
- `junk-runner-investigation-2026-06-15.md` — Junk-Runner-bevy policy drift
- `dracon-platform-untracked-commit-2026-06-15.md` — what stays untracked in dracon-platform (and why)
- `sync-push-classification.md` — push rejection classification
- `source-encryption-incident-2026-06-15.md` — encryption incident
- `warden-plaintext-sibling.md` — warden plaintext sibling handling
- `ownership-investigation-2026-06-15.md` — repo ownership analysis

Design docs are durable. Re-read them.

## Agent-loop git identities (path ownership)

Configured watch paths are owned by default. `owned = false` in a repo's
`.dracon/dracon-sync.toml` is the explicit opt-out; legacy `owned = true`
remains accepted but is no longer needed. A bad local identity, historical
author, or foreign `origin` is therefore a **warning** for a path-owned repo,
not an auto-commit/push gate. Pushes still go only to the configured operator
namespaces; foreign remotes are fetch-only and warned about.

Loops SHOULD still use a deliberate identity (`<repo>-dev` /
`<repo>@dracon.local`) so commit attribution stays useful. Add new identities
to `trusted_emails` and `trusted_authors` when appropriate, but the daemon
must not expand those lists automatically and must not manufacture commits to
restore activity totals. A repo outside a configured watch root retains the
legacy heuristic ownership check, and `owned = false` always blocks it.

## History-rewrite ENFORCEMENT stack (v0.113.0, 2026-07-25)

The no-rewrite policy below is now enforced, not just documented:

1. **warden global hooks** (`~/.config/git/hooks`, warden-owned via
   `core.hooksPath`): `pre-push` refuses
   non-fast-forward updates and branch deletions; `pre-rebase`
   refuses rebasing commits already on any remote. Escape hatch for
   deliberate operator rewrites: `DRACON_ALLOW_REWRITE=1`.
   `init.templateDir` is an operator-managed Git setting; warden does
   not claim ownership of it.
2. **gitlab branch protection**: every live main/master across the
   fleet is protected (`allow_force_push=false`, maintainers push);
   dracon-sync's gitlab auto-create protects `main` on creation.
3. **GitHub**: all public repos protected; private repos can't be
   (free tier) — the warden hooks are the mitigation there.
4. **Auto-gc**: the daemon runs `git gc --prune=now` when a repo's
   dangling garbage exceeds `auto_gc_garbage_threshold_bytes`
   (default 2 GiB, 0 disables) — self-heals the tmp_pack_* bloat
   class that inflated hegemon to 4.9 GiB and dracon-platform to
   37 GiB.

Details: `docs/design/incident-amend-race-and-trust-2026-07-25.md`
("Whack-a-mole audit" section).

## Agent loops MUST NOT rewrite history (2026-07-25 incident)

Loop agents working in daemon-watched repos must never `commit --amend`,
`rebase`, `filter-branch`/`filter-repo`, or force-push — the daemon pushes
auto-commits within seconds, so any rewrite races published history and
creates permanent divergence churn (hegemon's loop documented
`filter-branch --msg-filter` + `--force-with-lease` as its "recovery"
practice; browser-extensions-shared's loop amended every ~2 min).

Policy AGENTS.md files were added 2026-07-25 at:
- `dracon-platform/AGENTS.md` (covers all nested game repos)
- `browser-extensions-shared/AGENTS.md`

New repos that host agent loops should get the same file (copy the
"dracon-sync daemon: git-history rules for agent loops" section).
Evidence: endless-td's loop agent adapted correctly ON ITS OWN
("force-push to protected main is blocked") — explicit policy works.

## Daemon commands

- `dracon-sync repos` — live state of all watched repos
- `dracon-sync health` — check daemon health and repository health
- `dracon-sync repair concerns` — inspect known concerns (dry-run)
- `dracon-sync repair concerns --apply` — apply a fix for a known concern
- `systemctl --user status dracon-sync.service` — daemon health
- `dracon-sync pause` / `dracon-sync resume` — freeze/unfreeze sync
  (daemon keeps RUNNING, skips cycles; 24h TTL self-heals forgotten pauses)
- `dracon-sync maintenance -- <cmd...>` — pause → run command → ALWAYS
  resume (v0.113.44+). The sanctioned wrapper for git surgery on
  daemon-owned repos.

## Daemon quiesce policy (2026-08-07, v0.113.44)

**`systemctl --user stop dracon-sync.service` is BANNED for
remediation.** A manual stop has no backstop (`Restart=always` only
covers crashes), so a forgotten restart leaves the fleet unsynced
silently. This policy exists because the 2026-08-06 dracon-platform
remediation (merge --abort + rebase) used `systemctl stop`, and any
agent copying that procedure could leave the daemon down.

Sanctioned quiesce paths:

1. **`dracon-sync maintenance -- <cmd...>`** — pauses, runs the
   command, ALWAYS resumes even on failure, propagates the command's
   exit code. Use for single git operations (merge --abort, rebase,
   reset).
2. **`dracon-sync pause` / `dracon-sync resume`** — for interactive
   multi-step work. A forgotten `resume` self-heals: freeze markers
   older than 24h are auto-cleared by the daemon.

Why pause beats stop: the service never goes down (health stays
green), freeze takes effect within one pulse interval (default 1s),
and the 24h TTL makes "forgot to resume" self-correcting.

**Mechanical backstop**: `dracon-sync-watchdog.timer` (user systemd,
2-min period) restarts the service if it is inactive and no
`~/.dracon/dracon-sync.maintenance-hold` marker exists. Genuine
multi-minute downtime (release installs, hardware work) must touch
the hold marker first and remove it afterwards. See
`docs/design/daemon-quiesce-policy-2026-08-07.md`.

## Guard service resilience & memory limiting (2026-08-10, v0.112.36)

The Aug 9–10 incidents (swap thrash, ENOSPC Chrome crash) ran with
`dracon-system-guard.service` **disabled + inactive** — no guard was
watching. Three things changed:

1. **Guard watchdog**: `dracon-system-guard-watchdog.timer` (every
   2 min) restarts the guard if it is ever inactive. `Restart=always`
   only covers crashes, not manual stops or a disabled unit. Escape
   hatch: `touch ~/.dracon/dracon-system.maintenance-hold` (remove
   afterwards — nothing does it automatically).
2. **Memory-pressure limiter** (`auto_renice_on_memory`, default
   true): during warn/critical memory pressure the top-5 RSS offenders
   get graduated nice (4 GiB → 5, 8 GiB → 10), restored on recovery.
   Fixes the "system unresponsive" symptom (CPU starvation) without
   killing. The implementation applies this only when the service has
   `CAP_SYS_NICE`, because an unprivileged user service can lower priority
   but cannot restore it; otherwise it emits one diagnostic and leaves
   processes untouched. Whitelist via `process_exempt_names`.
3. **OOM-killer bias** (`bias_oom_on_pressure`, default true): during
   critical pressure offenders get `oom_score_adj` 250 so the kernel's
   last-resort kill picks them, not an innocent process. Never
   triggers a kill; never touches adj ≤ −500 (protected) processes.

Optional (default OFF): `cap_offenders_cpu_percent = N` hard-
throttles offenders to N% CPU via a transient user systemd unit
(CPUQuota) during critical pressure. CPU throttling never kills —
verified live 100% → ~51%. Memory caps are deliberately NOT offered:
a memory cap frees nothing and only kills (MemoryMax) or freezes
(MemoryHigh) the process — renice fixes the responsiveness symptom,
OOM bias steers the kill, CPUQuota tames a stuck busy-loop.

This is the operator-approved design from the 2026-08-10 discussion:
"deprioritize heavy consumers during pressure, whitelist what must
stay fast, never cap memory, steer the last-resort kill at offenders."

## Disk cleanup & credential discipline (2026-08-10)

Full writeup: `docs/design/disk-full-credentials-2026-08-10.md` (incident,
verification protocol, path/pattern lists). Short version:

- **Never delete without operator approval**: `~/.config/google-chrome/**`
  (and caches while Chrome runs), `~/.dracon/**`, `~/.ssh/**`, `~/.git-credentials`,
  `~/.netrc`, `~/.npmrc`, `~/.config/gh/hosts.yml`, KWallet/keyring dirs,
  `*.env`/`*.pem`/`*.key`/`*.age` anywhere in `~/Dev`. Trash too — the
  2026-08-10 scan found 665 credential-pattern matches in a 56 GB Trash.
- **Scan before bulk delete / Trash empty**: pattern list in the design doc
  (chrome/credential/password/secret/token/login data/.env/.pem/.key/.age/
  .git-credentials/.npmrc/hosts.yml). Review names only; never empty Trash
  without the scan.
- **Guard live builds before `rm -rf target/`**: check for `cargo`/`rustc`/
  `npm install`/`vite build` processes (dev servers are safe to ignore).
- **Chrome DBs are WAL-locked while Chrome runs**: copy main+`-wal`+`-shm` to
  /tmp and run `PRAGMA integrity_check` on the copy; `database is locked` on a
  live file is contention, not corruption. Chrome 148+ has no
  `os_crypt.encrypted_key` in `Local State` — the key is in KWallet; missing
  `portal`/keyring with a fresh `Local State` is the real loss signal.
- **Verify credentials after any cleanup**, read-only (protocol in the doc).

## Forbidden actions

> **REMOVED in draft 2026-06-30 (audit goal `mr0q2qx2-mvfs0c`)**:
> The single-line "NEVER" list below was misleading because the daemon's
> own auto-repair path (`dracon-sync/src/report.rs:3705`) calls
> `rewrite_ahead_paths()` (`dracon-sync/src/git/staging.rs:148-244`)
> which uses `git filter-repo --invert-paths --force` when
> `auto_repair_concerns = true` (default per
> `dracon-sync/src/policy.rs:476-477`). The 2026-06-30 audit confirmed
> this code path has never fired on our repos (zero
> `backup/pre-sync-largeblob-fix-*` branches found), but the
> contradiction between "NEVER rewrite history" and the default
> config warranted replacement rather than ad-hoc violation tracking.
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
  `dracon-sync/src/sync.rs:1079,1097`) — a safer equivalent.
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
  ahead. Code paths (current as of v0.113.4 — the v0.113.3
  SYNC-H6 fix moved from `filter-repo` to `git bundle` for backup
  capture + `--force-with-lease` for the force-push; the rewrite
  itself still uses `filter-repo --invert-paths --force`):
  - Function definition: `dracon-sync/src/git/staging.rs:152`
  - Call site: `dracon-sync/src/report.rs:3705`
  - Default: `dracon-sync/src/policy.rs:1956`
    (`auto_repair_concerns: true`, set via
    `#[serde(default = "default_true")]` so TOML-loaded configs
    without an explicit value get `true`; the bare
    `#[derive(Default)]` on `SyncPolicy` gives `false`)

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

  CHANGED 2026-07-29 (v0.113.10): the 2026-07-29 fleet cleanup
  (docs/design/stale-backup-branch-cleanup-2026-07-29.md)
  bundled + deleted all 8 historical `backup/*` branches
  (including avid's) — bundles live at
  `~/dracon/backups/stale-branch-bundles-20260729/`. The new
  opt-in janitor (`auto_prune_stale_backup_branches`, enabled
  fleet-wide 2026-07-29) now reaps future instances daily
  (bundle-first into `backup_dir/auto-prune/`, local + matching-
  tip remote deletion). The operator-review signal is NOT lost:
  every janitor deletion is `log_warn!`'d with repo, ref, tip,
  and bundle path — review the journal instead of the branch
  list. A branch surviving > 24h means either the janitor is
  disabled or its bundle failed (also logged).

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
- `cargo clippy --workspace --locked -- -D warnings` must be clean
- New code paths require unit tests
- Backwards compatibility with all previously added
  policy fields is required
- **Per-repo knobs need BOTH halves (enforced since
  v0.113.34)**: adding a `SyncPolicy` field that a repo may
  tune requires (1) the field, (2) an `Option<>` counterpart
  in `RepoPolicyOverride`, (3) merge resolution at the point
  of use (`repo_override.field.unwrap_or(policy.field)`,
  pattern: `auto_bump_versions`).
  `test_repo_override_field_coverage_tripwire` in
  `dracon-sync/src/policy.rs` fails `cargo test` if you add a
  field to either struct without deciding its per-repo story
  (add the override half, or list the name in
  `OVERRIDE_COVERAGE_GLOBAL_ONLY` /
  `OVERRIDE_COVERAGE_OVERRIDE_ONLY` with a reason). This
  exists because v0.113.29 added the SyncPolicy half of
  `build_artifact_cleanup` without the override half — the
  per-repo opt-out silently did nothing in production until
  v0.113.33.

## Recent audit-driven changes

### 2026-07-19 — post-v0.112.20 audit baseline (`AUDIT_FULL_2026-07-18-POSTFIX.md`, audit B8)

53+ findings across the daemon, warden, system, and meta-repo. All
**11 HIGH** (8 daemon + 3 warden) and 7 actionable **MEDIUM** findings
were remediated. Critical fixes:

- **F39** ownership substring bypass — daemon's primary safety guard
  against pushing to attacker infra is now tuple-atomic.
- **F40** `standard_files` path traversal — absolute/`..` paths now
  rejected by `validate_config`.
- **F41** `git_askpass_script` race window closed — atomic
  `O_EXCL|O_NOFOLLOW` + `mode(0o700)` + `AskpassScript` Drop guard.
- **F30** v0.112.19 table-width fix completed (was partial; test array
  never had ROLE).
- **F45/F46** test infra hardened — no more `mem::forget` TempDir leaks
  or racy `EnvRestorer::Drop`.

### v0.113.0 (2026-07-25) — enforcement stack

Warden hook enforcement moved from documentation to a hard gate:

- **gitlab auto-protect**: every live main/master across the fleet
  is protected (`allow_force_push=false`); dracon-sync auto-protects
  on `main` creation.
- **Warden pre-push / pre-rebase hooks** block non-fast-forward
  pushes and rebases of already-published commits (escape hatch:
  `DRACON_ALLOW_REWRITE=1`).
- **`auto_gc_garbage_threshold_bytes`** (default 2 GiB) — daemon
  self-heals the `tmp_pack_*` / dangling-object bloat class.

Documented in
[`docs/design/incident-amend-race-and-trust-2026-07-25.md`](docs/design/incident-amend-race-and-trust-2026-07-25.md)
("Whack-a-mole audit" section).

### v0.113.4 (2026-07-26) — full-audit remediation (`AUDIT_FULL_2026-07-26.md`)

Fleet-wide audit found 13 HIGH / 23 MEDIUM / ~30 LOW across all three
utilities + the meta-repo. Every HIGH was independently spot-checked
against source before acceptance. Remediation was split into 4
batches:

- **dracon-sync v0.113.2** (SYNC-H8 conflict helpers for nested
  submodule gitdirs; SYNC-H2 self-defeating backstop →
  `SyncOutcome::BackstopSkipped`; SYNC-H3 `maybe_auto_gc` async via
  `run_git_with_timeout` 600s + per-repo cooldown; SYNC-H1 quiet-
  daemon wedge valve reachable from daemon loop; SYNC-H7 bonus
  cat-file pipe deadlock fix).
- **dracon-warden v0.113.1** (WARDEN-H1 binary whole-file secrets
  corrupted by smudge UTF-8 lossy path → `smudge_with_security`
  tries `decrypt_whole_file_tag` first; WARDEN-H2 global pre-commit
  blocked ALL non-hardened repos → chains to repo-local hook + no-ops
  unless warden-managed (via `git config --local filter.dracon.clean`);
  WARDEN-H3 pre-rebase `head -100` newest-first miss → boundary-check
  via `git branch -r --contains`; WARDEN-M2 `\x27` → shell `'\''`
  idiom in pre-push secret scan regex).
- **dracon-sync v0.113.3** (SYNC-H6 `rewrite_ahead_paths` destroyed
  own backup / deleted origin / no force-push → bundle-file backup
  via `git bundle create <gitdir>/<name>.bundle HEAD --refs HEAD`,
  pre-rewrite capture, `--force-with-lease` anchored to pre-rewrite
  upstream-sha; M7 auto-pull explicit `refs/heads/<branch>` +
  `--no-edit` + `merge --abort` on failure).
- **dracon-sync v0.113.4** (SYNC-H4 visibility cache-poison on
  transient gh failure → `get_github_visibility_opt` skip-both-
  flips-and-cache on `None`; SYNC-H5 `standard_files` source path
  traversal at both `validate_config` and point-of-use, with new
  `is_safe_standard_file_path` helper that rejects raw-absolute and
  any `..` component but allows `~/...`).
- **dracon-system v0.112.34** (SYS-H1 guard daemon busy-looped
  forever after first interval → `elapsed` reset inside outer loop;
  SYS-H2 `link apply` could never fix a drifted symlink → in-sync
  short-circuit + direct `fs::remove_file(&link)` for drifted
  symlinks).
- **dracon-warden v0.113.2** (F0.1 follow-up — tag-push false-
  positive: BAD_AUTHORS scan now distinguishes branch pushes
  (`rev-list $local --not $remote`) from new-ref pushes
  (`rev-list $local --not --remotes`), so a test-identity commit
  already published via a prior branch push is no longer re-scanned
  on a later tag push).

Total workspace tests: **1038** (was 915 before v0.113.4).
Test breakdown: dracon-sync 847 (837 + 10 integration);
dracon-system 88; dracon-warden 103 (93 + 10 integration).

### 2026-08-14 — second-pass source audit

The current source was reviewed again against the deferred findings in
`AUDIT_FULL_2026-07-26.md`. This pass added regression coverage and fixed
bounded correctness issues in Git status/path parsing, failed-query handling,
SHA-256 object IDs, large-blob paths containing spaces, bounded Git stdin and
temporary-file cleanup, askpass cleanup, branch/ref validation, mirror error
attribution, atomic origin-gone ledger writes, Cargo version comments,
fail-closed future visibility timestamps, and several system cleanup/accounting
paths (mount-aware inode checks, lock truncation ordering, Unicode-safe event
formatting, and Nix reclaim reporting).

The example policy now documents the live `exclude_repos`, GitHub privacy,
stage-batch, and unpushed-alert settings. Historical design documents that no
longer describe the active nested-on-`main` layout or the current timeout
override are marked as superseded. Remaining low-priority design limitations
are recorded in the audit handoff rather than silently treated as fixed.

## `[patch.crates-io]` status

**RESOLVED 2026-08-08**: `dracon-git v94.7.2` was published to crates.io,
`[patch.crates-io]` removed from `Cargo.toml`, the dependency bumped to
`dracon-git = "94.7.2"`, and `deny.toml [sources].allow-git` cleared.

The patch existed since 2026-07-18 as a workaround for the libgit2
ssh-agent bug fixed in `dracon-git v94.7.2` (upgraded from v94.7.1 on
2026-07-25). Its removal is tied to an incident: `cargo publish` **strips
`[patch.crates-io]` from the published manifest**, so every `cargo install
--version`-style install silently built against crates.io dracon-git
94.7.0 (unpatched) — visible as phantom untracked counts (2026-08-08,
`docs/design/installed-binary-drops-patch-dracon-git-2026-08-08.md`).
Guard: `dracon-sync/scripts/verify-install.sh` (fixture check; wired into
`dracon-sync/scripts/release.sh` step 7 against the packaged artifact, and reminded
after every release).
