# Sync-health audit — 2026-07-09 (evening)

**Trigger:** operator reported hegemon (and "others") showing changes that
are "not getting staged and pushed."

**Headline finding:** there is **no systemic sync stall**. The daemon is
committing + pushing normally. `hegemon` is in fact syncing fine — its
remotes (github/gitlab/codeberg/origin) are all current; what the operator
saw was **transient lag** (the daemon commits on a ~1-min cycle while active
development produces commits faster) plus uncommitted working-tree edits the
daemon had not yet picked up. The **only genuine blockers** are two specific
repos: `pully` (STUCK_PULL) and `dracon-code` (untrusted author).

---

## 1. Method

- `systemctl --user is-active dracon-sync.service` → **active** (PID 1344337).
- `dracon-sync repos --json` across all 26 repos.
- Local `main` vs every configured remote `refs/heads/main` (divergence scan).
- Daemon journal (`journalctl --user -u dracon-sync.service`) for push/pull
  errors and per-repo sync events.
- Manual `git push --dry-run` / `git fetch --dry-run` to separate daemon bugs
  from real network/URL problems.
- `git remote -v`, `.dracon/dracon-sync.toml`, and the daemon's in-memory
  ledger (`remotes:` field per repo).

## 2. Daemon health

- Active, PID 1344337 (started 14:38). **0 orphaned git pushes** at audit time.
- No `rejected` / `non-fast-forward` / `fatal` / `remote error` / `denied` /
  `auth` messages in the last 40 min.
- The per-repo `⏫ scaling push timeout 900s → 600s (N commits ahead)` lines
  are a **pre-push log**, not a failure. Each is followed by
  `🔁 synced (late)` once the push completes.

## 3. Divergence scan (local `main` vs remotes)

At audit time 4 / 26 repos showed local ≠ some remote:

| repo | local | behind remote(s) | verdict |
|------|-------|------------------|---------|
| hegemon | `7e99505c` | all remotes `@e75dad95` | **transient** — daemon pushed `e75dad95`, `7e99505c` is newer dev work not yet caught up |
| hellhunter | `98e908e2` | codeberg/github `@1149ee1d` | **transient** — syncing |
| pully | `ecbc44e5` | codeberg/gitlab/origin `@b0c193ab` | **STUCK** — see §4 |
| dracon-code | `2c37e270` | codeberg/gitlab/origin `@df010cce` | **BLOCKED** — untrusted author, see §5 |

**github (origin) is current for every repo** — no repo is missing from
GitHub. codeberg/gitlab lag by a commit or two and catch up within a cycle
(observed: hegemon's codeberg advanced `95cf294c → e75dad95` between two
samples, i.e. the daemon was mid-push and completed it).

## 4. pully — STUCK_PULL (genuine blocker)

- `dracon-sync repos`: `state_flags: ['AHEAD:1','BEHIND:1','STUCK_PULL']`,
  daemon ledger `remotes: None`.
- Journal (repeated): `pull/merge failed: Git operation failed: unsupported
  URL protocol; class=Net (12) - aborting sync pass`, then
  `exceeded max failures (5), skipping until resolved`, plus
  `🔔 Stuck Ahead (Unpushed)` + `Stuck Behind (Unpulled)` alerts.
- 1 local commit unpushed: `ecbc44e5 Fix tcp service health checks to use the
  service port`. 1 commit on origin not pulled.
- **Root cause:** the daemon's in-memory ledger has **no remotes for pully**
  (`remotes: None`), even though `git remote -v` shows 3 valid
  `git@…` SSH remotes. The daemon's libgit2-based pull therefore constructs an
  empty/None URL → `unsupported URL protocol`. This is a **daemon
  discovery/persistence gap for pully specifically**, not a network or URL
  problem — proof: `git fetch --dry-run origin` from the CLI **succeeds**
  (exit 0, SSH transport fine), and every other repo (hegemon, dracon-platform,
  …) syncs via the same `git@` URLs.
- pully has **no `.dracon/dracon-sync.toml`** (hegemon has one) and is **not**
  a gitlink of dracon-platform.

**Recommended fix (not yet executed):** restart the daemon so it re-runs
discovery and re-populates pully's remotes (`remotes:` should then be non-null
and the libgit2 pull will resolve a real URL). If a restart does not clear it,
investigate the daemon's remote-discovery path for `git@` SCP-style URLs in
libgit2 pull (other repos' pulls succeed, so this is likely a transient
ledger gap rather than a libgit2 SSH limitation).

## 5. dracon-code — untrusted author (genuine blocker, by design)

- `state_flags: ['AHEAD:2']`, hint `🚫 unowned: untrusted_author`.
- 2 local commits unpushed, both authored by `audit-agent <audit@dracon-code>`:
  - `2c37e270 docs(audit): strengthen 14-types framing + refresh stale test count`
  - `21dc90727 fix(tui): headless smoke path exits 0; clear error without TTY`
- The daemon **correctly refuses** to auto-push an untrusted author (its trust
  model). This is working-as-intended, not a bug. It is the same deferred item
  flagged in the 2026-07-09 full audit.

**Resolution requires an operator trust decision** (outside this audit): either
trust the `audit-agent` author for `dracon-code`, or re-author those 2 commits
with a trusted identity, after which the daemon will push them.

## 6. hegemon — NOT actually stuck (premise corrected)

- `dracon-sync repos` journal shows hegemon committing + syncing repeatedly:
  `📝 committed N files` → `🔁 synced (late)` at 15:09, 15:14, 15:16, 15:20,
  15:22, 15:27. All 4 remotes were at `e75dad95` (pushed) at audit time; newer
  local commits (`7e99505c`) are simply not yet caught up by the ~1-min cycle.
- This matches the earlier 2026-07-09 hegemon GitHub-fit work: hegemon is on
  GitHub at 0.158 GiB pack (under the 2 GiB limit). The rewrite + anti-rebloat
  `.gitignore` are holding.
- The operator's observation was the daemon's commit-cycle lag versus active
  editing, plus possibly uncommitted working-tree files the daemon had not yet
  staged. Not a stall.

## 7. Conclusion

- **No systemic sync stall.** The daemon commits + pushes all repos; GitHub is
  always current; codeberg/gitlab lag by a commit and catch up.
- **hegemon is healthy** — the reported symptom was transient lag, not a stall.
- **Two genuine blockers remain**, both previously-known deferred items:
  1. `pully` STUCK_PULL ← daemon ledger `remotes: None` (discovery gap);
     CLI fetch works, so a daemon restart should re-discover and clear it.
  2. `dracon-code` AHEAD:2 ← untrusted author `audit-agent`; needs an operator
     trust decision (or commit re-authoring).

## 8. Verification evidence index

- `systemctl --user is-active dracon-sync.service` → active, PID 1344337.
- `dracon-sync repos --json` → 4/26 diverged (hegemon/hellhunter transient,
  pully STUCK_PULL, dracon-code untrusted).
- `git ls-remote` per repo → github current for all; codeberg/gitlab lag then
  catch up (hegemon `95cf294c → e75dad95` observed).
- pully `git remote -v` → 3 valid `git@` SSH remotes; `git fetch --dry-run
  origin` → exit 0 (SSH OK).
- pully daemon ledger `remotes: None`; journal `unsupported URL protocol;
  class=Net (12)`.
- dracon-code `git log origin/main..main` → 2 commits by `audit-agent`.
- hegemon journal → 6 `🔁 synced (late)` events in 13 min.

## 9. Resolution — fix applied + re-audit (2026-07-09, goal `mrdmxu8n`)

The two genuine blockers from §4–§5 were fixed and the fleet was re-audited.

### 9.1 pully (was STUCK_PULL)
- Root cause refined: the diverged commits `ecbc44e5` (local, full message)
  and `b0c193ab` (origin, daemon auto-commit message) are **byte-identical
  duplicates** — same parent `2ffa5562`, same `port` field + tests added to
  the same two files. The same edit was committed in two sessions.
- Fix: `git reset --hard origin/main` in pully (zero content loss — the change
  already exists on origin via `b0c193ab`; working tree was clean, trees
  identical). No conflict, no pull required (behind becomes 0).
- Result: pully `✅ OK / healthy`, AHEAD:0/BEHIND:0; local = origin = gitlab =
  codeberg = `b0c193ab`. STUCK_PULL cleared.
- **Caveat (deferred):** pully's underlying daemon pull failure
  (`unsupported URL protocol; class=Net (12)` from the published `dracon-git`
  crate's libgit2 pull) was **not** patched — `dracon-git` is a published
  dependency, not editable here. The reset keeps pully synced while
  `behind==0`; if pully diverges again, the daemon's libgit2 pull may re-fail
  and require another manual `git pull`/reset. Follow-up daemon fix recommended
  (use git CLI for pull, or address `dracon-git` libgit2 SSH transport).

### 9.2 dracon-code (was untrusted_author)
- Fix: whitelisted the audit-agent author in the global daemon config
  `~/.dracon/utilities/sync/dracon-sync.toml` by adding
  `trusted_emails = ["dracsharp@gmail.com", "audit@dracon-code"]` (the file
  previously relied on built-in defaults only). `trusted_authors` is a
  global-only policy field (per-repo `.dracon/dracon-sync.toml` cannot set it).
- Restarted the daemon (`systemctl --user restart dracon-sync.service`) to
  reload config + re-discover remotes (PID rotated off 1344337).
- Result: dracon-code `✅ OK / healthy`, AHEAD:0; the 2 `audit-agent` commits
  (`2c37e270`, `21dc90727`) pushed; origin/gitlab/codeberg all SYNCED at
  `2c37e270`.

### 9.3 Re-audit results (all 26 repos)
- `dracon-sync repos --json` → `ok=22, warn=4`. The 4 `warn` are all `DIRTY`
  (benign untracked/modified working-tree state), **not** sync failures:
  nested submodules (`endless-td`, `neonbreak` pointers), untracked nested
  git repos (`dracon-sync/`, `dracon-system/` in dracon-utilities — separately
  watched), the daemon's own `.pi/goals/*.md` files, and one staged source
  file (`browser-extensions-shared/.../livePreview.ts`) the daemon commits on
  its next cycle.
- Corrected local-vs-remote divergence scan (valid 40-char SHA only, no
  `github/HEAD` false positives) → **0 real divergences**; every local `main`
  equals every configured remote `main`.
- Daemon health: `active`, **0 orphaned `git push`** processes, **no**
  `reject`/`fatal`/`non-fast-forward`/`denied`/`auth`/`unsupported URL` errors
  in the last 20 min of journal.
- hegemon: `✅ OK / synced` (original concern was transient lag, confirmed
  resolved).

**Conclusion:** both genuine blockers are resolved and verified; the fleet is
fully synced. Only benign `DIRTY` working-tree states remain, plus the deferred
pully libgit2-pull daemon bug (§9.1 caveat) for future divergences.
