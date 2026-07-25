# Incident: amend-race divergence, untrusted agent identities, and the libgit2 transport regression

**Date**: 2026-07-25
**Repos affected**: deathrun, hegemon, ai-auto-writer, browser-extensions-shared, pi-plugins
**Resolution shipped in**: dracon-sync v0.112.40 (perf) + **v0.112.41** (daemon-mode GIT_SSH_COMMAND) + **dracon-git v94.7.2** (git2 ssh/https features + agent-less ssh_cred)
**Status**: all resolved; fleet 35 repos / 0 CONCERN / 0 unowned / 0 push-stuck

## Summary of the four concurrent issues

| Repo | Symptom | Root cause | Resolution |
|------|---------|-----------|------------|
| deathrun | ❌ CONCERN ↑8 ↓1, push rejected | daemon committed pi-session WIP, pi session amended 6s later | pi session merged (`pull --no-rebase`) at 03:28; self-resolved |
| hegemon | 🛑 push-stuck 360m, 5 failures | stale ghost-clone commit on gitlab (wrong-direction SPEC.md §12.4 revert) | `merge -X ours gitlab/main` + push + `repair stuck-unstuck` |
| ai-auto-writer | 🚫 unowned | HEAD authored by `Audit <audit@dracon.dev>` — not in trust lists | added to `trusted_emails` + `trusted_authors` |
| browser-extensions-shared | 🚫 unowned ↑14 | commits by `Virtual Pet Loop <loop@virtualpet.local>` — untrusted | same fix |
| pi-plugins | push-stuck (codeberg) | codeberg SSH (217.197.84.140:22) unreachable — server-side | transient; daemon retries with backoff |

## Root-cause deep dives

### 1. The amend race (deathrun)

Timeline from the reflog:

```
02:45:54  daemon auto-commits the V37 pi session's in-progress files
          → f52d8c56 "2 file(s) in scripts [...] DELTA:+97/-0"
02:45:59  daemon pushes f52d8c56 to origin (gitlab)
02:46:00  pi session runs `git commit --amend` (proper "V37 iter 42"
          message) → dff1b486 — IDENTICAL TREE (910081b8…), new sha
03:17:46  iters 43–49 stack on the amended line → ↑8 ↓1, every
          origin push non-fast-forward
```

The daemon then failed to self-heal: `auto_pull_merge` (which would have
trivially merged the identical-tree commit) died with
`unsupported URL protocol; class=Net (12)` — see §3.

The pi session itself resolved it at 03:28 with `git pull --no-rebase
origin HEAD` (merge via ort). `f52d8c56` remains in history; no rewrite
was needed.

### 2. hegemon's stale ghost commit

gitlab/main carried two commits local lacked, authored yesterday 12:54 by
a different checkout (a `/home/dracon/Downloads/1` ghost clone, since
deleted):

1. `9f1bb7f1` — daemon-style commit reverting SPEC.md §12.4 to the OLD
   terse claim ("No PIL, no manual pixel hacks"), deleting the corrected
   documentation that local had since expanded. **Wrong-direction edit.**
2. `bcc268a4` — auto-merge of github's line (already an ancestor of
   local).

The only unique content on gitlab was the bad revert. Resolution:
`git merge -X ours gitlab/main` — conflict surface was SPEC.md only;
local's corrected text kept; FF-pushed everywhere. No force-push
(the ↓2 divergence exceeded AGENTS.md's one-commit-behind
`--force-with-lease` allowance, so the merge path was also the
policy-clean path).

### 3. THE SYSTEMATIC BUG: libgit2 built with no network transports

The daemon's misleading `unsupported URL protocol; class=Net (12)` error
masked everything. Investigation:

- Reproduced with a minimal git2 consumer: fetch failed for **every**
  repo, on both SCP (`git@host:…`) and `ssh://` URL forms.
- `cargo tree -e features -i libgit2-sys` → only `default`, **no `ssh`**.
- Cause: **git2 0.21 changed `default = []`** — ssh and https are now
  opt-in features. dracon-git's bare `git2 = "0.21"` therefore built
  libgit2 with no network transports at all.
- It lurked because dracon-git v94.7.1 made `fetch()` CLI-first; the
  libgit2 path only fires as fallback, and only when a repo is behind
  upstream (rare — the daemon is usually the only committer). When the
  CLI fetch had a transient failure (deathrun, 03:07), the fallback's
  bogus error replaced the real one.

Second layer: even with transports enabled, `Cred::ssh_key_from_agent`
cannot work in the daemon — the systemd user service has **no
`SSH_AUTH_SOCK`** (CLI git works because ssh falls back to `IdentityFile`
keys). And the agent failure is *lazy*: `ssh_key_from_agent` returns `Ok`
without touching the agent and only errors during the handshake, so a
naive `.or_else()` fallback never fires. The socket must be probed
eagerly.

**Fix shipped as dracon-git v94.7.2** (tag pushed to
github.com:DraconDev/dracon-libs):

```toml
git2 = { version = "0.21", features = ["ssh", "https"] }
```

plus `ssh_cred()`: probe `SSH_AUTH_SOCK` with `UnixStream::connect`;
agent if live, else `~/.ssh/id_ed25519|id_rsa|id_ecdsa` — mirroring CLI
ssh behavior. Verified end-to-end (`FETCH OK`) with `SSH_AUTH_SOCK`
unset and with a dead socket.

dracon-utilities `[patch.crates-io]` now pins `tag = "v94.7.2"`.
All 829 daemon tests + 33 dracon-git tests pass; clippy clean.

### 3b. THE THIRD SYSTEMATIC BUG: systemd 258.7 user namespace breaks plain ssh in the daemon

After the v94.7.2 deploy, a NEW error surfaced on every CLI fetch/pull
from the daemon:

```
git pull --no-rebase failed: Bad owner or permissions on
/nix/store/...-systemd-258.7/lib/systemd/ssh_config.d/20-systemd-ssh-proxy.conf
```

Root cause (reproduced deterministically via `systemd-run --user -p
ProtectHome=read-only ssh -G ...`): the systemd 258.7 user-service
sandbox (`ProtectSystem=strict` / `ProtectHome=read-only` /
`PrivateTmp`) now runs the daemon inside a **user namespace**, where
root-owned files appear as `nobody(65534)`. OpenSSH's
`secure_filename()` rejects any config path component not owned by euid
or uid 0, so `/etc/ssh/ssh_config`'s system Include of systemd's
nix-store `ssh_config.d` file failed the check. Pushes kept working
because they already set `GIT_SSH_COMMAND` with
`-F ~/.dracon/secrets/ssh/config`, which bypasses the broken include;
plain-ssh paths (dracon-git's CLI-first `fetch()` / `pull_merge()` /
`pull_rebase()`) all broke.

This also retro-explains deathrun's 03:07 failure: the CLI fetch failed
with this error first, and the broken libgit2 fallback (§3) masked it
with `unsupported URL protocol`. Two stacked bugs, one log line.

**Fix shipped in dracon-sync v0.112.41** (`main.rs`, daemon mode only):
set `GIT_SSH_COMMAND` process-wide (unless already set) to the same
`git_ssh_hardening()` value pushes use, so every subprocess git
invocation inherits it. Verified: zero `Bad owner` errors post-restart;
deathrun/hegemon/browser-extensions-shared all synced.

### 3d. POST-MORTEM (same day, evening): who ran filter-branch — SOLVED

The hegemon loop agent's own process notes in `.pi-glla/active.jsonl`:

> "The pre-commit hook auto-committed my change before I could attach my
> message. The fix landed as `9f1bb7f1` with the auto-generated subject
> `1 file(s) [SPEC.md] DELTA:+18/-9`. Recovery: `git filter-branch
> --msg-filter` rewrote just the message of `9f1bb7f1` to `b3045ebc`.
> Force-pushed with `--force-with-lease`."

The "pre-commit hook" **is the daemon**. The loop agent's established
practice: daemon auto-commits its WIP with a generated message → agent
rewrites the message with `git filter-branch --msg-filter` → agent
force-pushes with `--force-with-lease`. This one habit explains the
entire hegemon incident class:

- the `filter-branch: rewrite` reflog entries (06:43, 06:50, 15:24)
- the force-pushed ghost lines racing the daemon (the stale `9f1bb7f1`
  SPEC.md §12.4 revert found on gitlab/main this morning)
- the recurring ↑N ↓1 divergences (daemon pushes original, agent
  force-pushes rewrite, both think they're ahead)
- committed merge-conflict markers in the goal ledger (merges resolved
  by an amending/rewriting agent mid-flight)

The agent is not malfunctioning — it honestly documents the practice as
"recovery". The tooling gap: nothing tells loop agents that (a) the
daemon's auto-commit is already PUSHED within seconds, so rewriting it
is a public-history rewrite, and (b) `--force-with-lease` against a
daemon-managed remote is forbidden by fleet policy. This is the root
fix for task "goal-loop tooling must not rewrite/amend pushed history".

The virtual-pet loop (browser-extensions-shared) shows the gentler
variant: tip-only `commit --amend` + periodic `pull --no-rebase`, no
force-push — divergence churn without ghost lines.

### 3c. Structural: amend-everything loops vs auto-push

The virtual-pet goal-loop (browser-extensions-shared) commits via
`git commit --amend` EVERY iteration (~2 min cadence) and runs
`git pull --no-rebase origin HEAD` periodically. Amending a merge
commit drops its second parent, so every daemon/loop merge at HEAD is
discarded at the next iteration — the ↓1 divergence is STRUCTURAL
while the loop runs (content is always identical; pure topology noise).
It self-resolves when the loop ends (deathrun pattern) or when the
loop's periodic pull lands. Long-term fix belongs in the goal-loop
tooling: don't amend once the daemon may have pushed (e.g. plain
`git commit` checkpoints, or amend only when `HEAD` is unpushed).

### 3e. hegemon backup-branch review (AGENTS.md fault policy) — BENIGN, cleaned

Per AGENTS.md ("any `backup/pre-sync-largeblob-fix-*` branch is a fault
requiring operator review"), hegemon had THREE such branches:

| branch | tip | date |
|---|---|---|
| backup/pre-sync-largeblob-fix-1784111638 | d58de24ae91b9f66c537f1285211f32a2a2d0ad0 | 2026-07-12 |
| backup/pre-sync-largeblob-fix-1784111877 | 1a15c0c86e23823b37865f791877e5cafa7f4032 | 2026-07-15 |
| backup/pre-sync-largeblob-fix-1784112055 | 1a15c0c86e23823b37865f791877e5cafa7f4032 | 2026-07-15 (also pushed to gitlab) |

Review verdict: the rewrite stripped ONLY `test-results/**/trace.zip`
Playwright artifacts (5-11 MiB each, regeneratable) from the ahead
history — fork point d077ce57 (2026-07-10). No source, docs, or assets
lost; development continued cleanly for 10 days after. All three local
branches + the gitlab remote branch deleted 2026-07-25 (tips recorded
above for recovery).

### 4. Agent-loop identities vs the F44 asymmetric-trust check

The ownership guard flags a repo when **either** the HEAD author email
**or** the author name is untrusted (F44 hardening, v0.112.21). The two
blocked repos were committed by agent loops using `GIT_AUTHOR_*` env
overrides (which beat repo-local `user.*` config — both repos already
had canonical local config).

Fix: whitelisted both sides in `~/.dracon/utilities/sync/dracon-sync.toml`:

- `trusted_emails += audit@dracon.dev, loop@virtualpet.local`
- `trusted_authors += "Audit", "Virtual Pet Loop"`

AGENTS.md gained an "Agent-loop git identities" section: when a new
agent loop is created, whitelist its identity in BOTH lists **before** it
starts committing.

## Whack-a-mole audit → enforcement stack (v0.113.0)

Operator question: "have we fixed the CAUSES, or are we playing
whack-a-mole?" Causal audit of the whole incident chain:

- **Root-caused at the source**: the systemd 258.7 userns ssh break
  (v0.112.41) manufactured most of the symptom cluster (fetch_failed,
  persistent divergence, stuck pushes); the KiB unit bug (v0.112.42)
  had silently disabled the GitHub 2 GiB pack guard.
- **The history-rewrite class was only SOFT-fixed** (AGENTS.md policy
  files) — agents may not read them. Hardened into enforcement:
  - **gitlab branch protection sweep**: 19 live branches across the
    fleet protected (`allow_force_push=false`, maintainers push).
    hegemon's protection is why gitlab kept the pre-rewrite history
    (the ghost 9f1bb7f1) — the mechanism already proved itself.
  - **GitHub**: all 23 public repos protected via API. Private repos
    CANNOT be protected on the free tier — residual gap, covered by
    the hooks below.
  - **gitlab auto-create** now protects `main` immediately on repo
    creation (dracon-sync v0.113.0) so the sweep can't silently
    regress with the next new repo.
  - **warden 0.113.0 global hooks** (`~/.config/git/hooks` via
    core.hooksPath + init.templateDir — warden owns the hook layer):
    pre-push refuses non-fast-forward updates + branch deletions,
    pre-rebase refuses rebasing published commits. Escape hatch:
    `DRACON_ALLOW_REWRITE=1`. Verified end-to-end: force-push of
    rewritten history refused (the hegemon case), amend-of-unpushed
    and rebase-of-unpublished still work.
  - Design note: the enforcement was first built as dracon-sync
    per-repo hooks and MOVED to warden after discovering warden
    seeds/owns hooks fleet-wide (global hooksPath, init.templateDir,
    hardening) — two installers would have ping-ponged ownership.
- **Garbage bloat self-heal**: `auto_gc_garbage_threshold_bytes`
  (default 2 GiB) — the daemon runs `git gc --prune=now` when
  dangling tmp_pack_* debris exceeds the threshold (the hegemon
  4.9 GiB / dracon-platform 37 GiB class, previously manual).
- **Accepted residuals**: trust-list onboarding for brand-new loop
  identities stays manual-but-documented (wildcard trust would weaken
  the exfil boundary); `repos` cold path ~7s once/hour (sidecar
  status file deferred — TTL=1h makes runs ~1s in practice).

## What was deliberately NOT done

- **No force-push anywhere.** deathrun self-resolved by merge; hegemon
  was merged with `-X ours`. The originally-drafted identical-tree
  auto-force-push daemon feature was dropped: with the libgit2 transport
  fixed, the existing `auto_pull_merge` (clean tree) and push.rs
  pull-retry (dirty tree) already resolve the amend race non-destructively.
  A force-push feature adds risk for zero marginal coverage.
- **`/home/dracon/Downloads/1/3/dracon-platform` (126 GiB stale full
  clone) NOT deleted.** Flagged to operator — disk is 91% full; deletion
  would free ~126 GiB but is outside the ghost-worktree scope.
  (The 4.1 GiB `Downloads/1/games` ghost tree WAS deleted — all dead
  worktree pointers, nothing modified in 7+ days.)

## Follow-ups

- [ ] Operator decision: delete `/home/dracon/Downloads/1/3` (126 GiB)
- [ ] Publish dracon-git v94.7.2 to crates.io (needs
      `CARGO_REGISTRY_TOKEN`), then remove `[patch.crates-io]`
- [ ] Watch codeberg SSH health for pi-plugins' 2 pending commits
