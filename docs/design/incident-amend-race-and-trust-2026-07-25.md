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
