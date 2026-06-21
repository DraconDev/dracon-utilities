# Concern-repo investigation — 2026-06-21

Date: 2026-06-21
Related goal: `45e00159-72b1-4414-b60a-ba373a52ad27` (investigate possible concern repos).

## Scope

All 13 watched paths under the operator's `dracon-sync` policy
(`/home/dracon/.dracon`, `/home/dracon/Dev`, `/home/dracon/dracon`):

```
.dracon, ai-auto-writer, avid, browser-extensions-shared,
DraconDev, DraconDev-private, dracon-code, dracon-libs,
dracon-platform, dracon-strategy, dracon-utilities,
pully-fully-pull-based-fleet-reconciler, rust-ai-web-auto
```

(`DraconDev` no longer exists on disk; the daemon no longer watches it. It
appears only in stale incident-ledger entries from prior sessions.)

For every repo the investigation captured:

- `git remote -v`
- `git config --get-regexp '^branch\.'`
- `git status --short --branch`
- ahead/behind against each mirror (`github`, `gitlab`, `codeberg`)
- `git ls-remote <remote> refs/heads/main` reachability and HEAD hash
- `.git` size in MB
- `git submodule status`
- last 5 entries in the daemon incident ledger
- recent `journalctl` push-failure / warn patterns

Raw evidence per repo: `/tmp/dracon-evidence/<repo>.txt` (gathered by
`/tmp/gather.sh` on the investigation host).

## Findings

### ai-auto-writer — ✅ Healthy

- Remotes: `github`, `gitlab`, `codeberg` (SSH); no `origin`.
- `branch.main.remote = github`, `branch.main.merge = refs/heads/main`.
- All three mirrors at `ahead=0 behind=0`; remote HEADs match locally.
- `.git` 120 MB; no submodules.
- Incident ledger shows only normal `sync_commit` entries.

Action: none.

### avid — ✅ Healthy

- Same mirror layout as ai-auto-writer.
- All mirrors at `0/0`; remote HEADs match.
- `.git` 47 MB; no submodules.

Action: none.

### browser-extensions-shared — ✅ Healthy

- Mirror layout; one archived branch (`archive/multi-provider-byok-lib`)
  tracked against `codeberg` — local-only metadata, expected.
- All mirrors at `0/0`; remote HEADs match.
- `.git` 535 MB (large but bounded; pre-existing).

Action: none.

### dracon-code — ✅ Healthy

- Mirror layout; `branch.main.remote = github`.
- All mirrors at `0/0`; remote HEADs match.
- `.git` 217 MB.

Action: none.

### DraconDev-private — ✅ Healthy

- Mirror layout; `branch.main.remote = github`, `merge = refs/heads/main`.
- All mirrors at `0/0`; remote HEADs match (`2cec6194525…`).
- `.git` 2 MB; only 1 incident (the initial LICENSE + FUNDING commit).

Action: none. The earlier concern (no remotes, missing remote-tracking refs)
was resolved by `configure_standard_remotes_if_missing` and
`count_unpushed_vs_configured_remotes` (see `mirror-only-push-and-empty-repo-remotes-2026-06-20.md`).

### DraconDev — ⚠️ Gone, not currently watched

- `/home/dracon/Dev/DraconDev` does not exist.
- The daemon does not list it in `dracon-sync repos` (currently 12 repos).
- Stale incident-ledger entries reference the old path.

Action: none required. Old incidents are retention-managed; the daemon
skips vanished paths cleanly via the `if !repo.exists() { continue }` guard.

### dracon-libs — ✅ Healthy

- Mirror layout; `branch.main.remote = github`.
- All mirrors at `0/0`; remote HEADs match.
- `.git` 436 MB.

Action: none.

### dracon-platform — ⚠️ Latent concern (FIXED in this investigation)

- Both legacy HTTPS `origin` AND SSH mirrors (`github`/`gitlab`/`codeberg`).
- `branch.main.remote = origin`, `branch.main.merge = refs/heads/main`.
- All three SSH mirrors at `0/0`; remote HEADs match.
- `.git` 15 GB — the largest by far. Pushes are inherently slow; the
  daemon scales `push_op_timeout_secs` with ahead count (see `scale_push_timeout`).
- `rust-ai-web-auto` codeberg reachability returned `Connection closed by
  217.197.84.140 port 22` once during the snapshot — a transient SSH
  rejection; the daemon retries on the next cycle and pushes succeed via
  other mirrors.

**Root cause of the latent concern:** after every successful mirror push,
`refresh_publish_upstream` called `git fetch --prune origin main:refs/remotes/origin/main`.
For an HTTPS `origin` that fetch is slow and unreliable, and the follow-up
`git branch --set-upstream-to=origin/main` occasionally failed with
`fatal: cannot set up upstream`. The daemon logged `⚠️ failed to refresh
publish upstream for dracon-platform: set-upstream failed…` on every cycle,
producing noise without any functional break (the upstream config is already
set; `git status --branch` and VS Code see `origin/main` correctly).

**Fix applied in this investigation:**

1. `refresh_publish_upstream` now skips when the configured publish remote
   is `origin` AND the repo also has SSH mirrors (`github`/`gitlab`/`codeberg`).
   The publish-upstream config itself remains correct, VS Code is happy, and
   we stop triggering an HTTPS fetch every cycle.
2. Any remaining failure is logged at debug level (🐛) instead of `⚠️`.
3. Unit test `test_refresh_publish_upstream_skips_origin_when_ssh_mirrors_exist`
   covers the skip path.

After deploy, `journalctl --since "2 min ago"` shows zero
`refresh publish upstream` or `set-upstream failed` entries for
`dracon-platform`, while `git status --branch` still reports `origin/main`.

### dracon-strategy — ✅ Healthy

- Mirror layout (auto-configured by the daemon; see `mirror-only-push-and-empty-repo-remotes-2026-06-20.md`).
- All mirrors at `0/0`; remote HEADs match.
- `.git` 1 MB.

Action: none.

### dracon-utilities — ✅ Healthy

- Mirror layout; `branch.main.remote = github`.
- All mirrors at `0/0`; remote HEADs match.
- `.git` 38 MB.

Action: none.

### pully-fully-pull-based-fleet-reconciler — ✅ Healthy

- Mirror layout; `branch.main.remote = github`.
- All mirrors at `0/0`; remote HEADs match.
- `.git` 32 MB.

Action: none.

### rust-ai-web-auto — ✅ Healthy (transient codeberg blip noted)

- Mirror layout; `branch.main.remote = github`.
- `git ls-remote codeberg refs/heads/main` returned
  `Connection closed by 217.197.84.140 port 22` once during the snapshot.
  The same `codeberg` remote is healthy for every other repo; this is a
  single transient SSH rejection, not a configuration or credential issue.
- `.git` 14 MB.

Action: none. The daemon's push path retries on transient failures and
the other two mirrors provide redundancy.

### .dracon — ✅ Healthy

- Mirror layout with `repo_name_map` mapping `.dracon` → `dracon-home`.
- All mirrors at `0/0`; remote HEADs match (`c5c40d9a1a9e…`).
- `.git` 85 MB.

Action: none.

## Summary

| Repo | Health | Action |
| --- | --- | --- |
| ai-auto-writer | ✅ Healthy | none |
| avid | ✅ Healthy | none |
| browser-extensions-shared | ✅ Healthy | none |
| dracon-code | ✅ Healthy | none |
| DraconDev-private | ✅ Healthy | none |
| DraconDev | ⚠️ Gone | none (daemon already skips) |
| dracon-libs | ✅ Healthy | none |
| dracon-platform | ⚠️ Latent → FIXED | skip HTTPS `origin` refresh + debug-log failures |
| dracon-strategy | ✅ Healthy | none |
| dracon-utilities | ✅ Healthy | none |
| pully-fully-pull-based-fleet-reconciler | ✅ Healthy | none |
| rust-ai-web-auto | ✅ Healthy | none (transient codeberg SSH blip) |
| .dracon | ✅ Healthy | none |

## Post-investigation live state

```
📦 12 repos  ✅ OK 11  ⚠️  WARN 1  ❌ CONCERN 0  ⛔ init/status failed: 0
```

The single WARN is `dracon-platform` mid-push (`🟣 pushing 0m (1 ahead)`)
during evidence capture, not a concern.

Every watched repo with commits is `ahead=0 behind=0` against all three
mirrors (`github`, `gitlab`, `codeberg`), confirmed by direct
`git rev-list --count refs/remotes/<remote>/main..HEAD` checks.

## Validation evidence

- `cargo test -p dracon-sync --locked` → **589 passed, 3 ignored**
  (added `test_refresh_publish_upstream_skips_origin_when_ssh_mirrors_exist`).
- `cargo build --release --locked` → 0 errors.
- `cargo deny check` → clean.
- `./install.sh --upgrade --binaries-only` → binaries installed.
- `journalctl --since "2 min ago"` shows zero `refresh publish upstream`
  warnings for `dracon-platform` (the pre-fix investigation window showed
  dozens per minute).

## Non-goals / deferred

- `dracon-platform` `.git` is 15 GB. The operator has not asked for
  history rewriting; the daemon already scales push timeouts with the
  ahead count to avoid premature timeouts. No action.
- `browser-extensions-shared` (535 MB) and `dracon-libs` (436 MB) `.git`
  sizes are within normal ranges for the work each repo carries. No
  action.
- `DraconDev` is gone. Stale incident entries remain in the ledger but
  are retention-managed.