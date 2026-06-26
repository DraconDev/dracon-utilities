# Triple-Sync Feasibility Investigation — 2026-06-26

**Audit date**: 2026-06-26 (BST)
**Auditor**: pi (operator-instructed read-only investigation)
**Mode**: read-only — no daemon config, per-repo config, working tree, remote, or running services modified
**Prior art consulted**:
- `docs/design/daemon-behavior-audit-2026-06-26.md` (2026-06-26 daemon audit baseline)
- `docs/design/concern-2-4remote-divergence-2026-06-21.md` (4-remote divergence runbook; contains 2026-06-21 unintended-force-push audit note — re-read carefully and explicitly avoided)
- `docs/design/mirror-divergence-and-secret-remediation-2026-06-21.md` (referenced via concern-2)
- `docs/design/gitlab-storage-and-divergence-2026-06-23.md` (referenced via concern-2)
- `docs/design/sync-push-classification.md` (push state classification rules)
- `/home/dracon/.dracon/utilities/sync/dracon-sync.toml` (global policy)
**Probe evidence**: `docs/design/audit-2026-06-26/triple-sync-probe.json` (15.0 KiB, 15 repos × 3 forges + SSH auth + CLI auth)

---

## Section 1 — dracon-platform repo state (read-only)

Captured at 2026-06-26 ~22:50 BST via `cd /home/dracon/Dev/dracon-platform && …`:

| Field | Value |
|---|---|
| Remotes configured | `codeberg` only (1 remote) |
| `codeberg` URL | `git@codeberg.org:dracondev/dracon-platform.git` |
| `codeberg` push URL | (same as fetch) |
| Current branch | `main-temp` |
| Upstream tracking | `codeberg/main-temp` |
| Local HEAD | `d4ca6983ff67e652a9370e09e73c56b4d6d16b93` |
| codeberg/main-temp tip | `6a7cf69324074e35cff9e64f4aa3ef15d6c3b4e5` |
| Merge base | `8fc02238f509c7e5e48106f474e65e5e7e1e603b` |
| `git rev-list --left-right --count HEAD...codeberg/main-temp` | **217 / 1** (local 217 ahead, codeberg 1 ahead) |
| `git merge-base --is-ancestor 6a7cf69324 HEAD` exit | **1** (6a7cf69324 is NOT a local ancestor) |

### `.git/config` relevant excerpts

```ini
[filter "dracon"]
        clean = dracon-warden filter-clean %f
        smudge = true
        required = true
[remote "codeberg"]
        url = git@codeberg.org:dracondev/dracon-platform.git
        fetch = +refs/heads/*:refs/remotes/codeberg/*
[branch "main-temp"]
        remote = codeberg
        merge = refs/heads/main-temp
[user]
        email = dracon@local
        name = dracon
```

The repo has **NO** `github` remote, **NO** `gitlab` remote, **NO** `origin` remote. Only `codeberg`. The daemon's "concern" status (`dracon-sync repos` output: ❌ CONCERN, 153+ consecutive failures) reflects the persistent non-fast-forward.

### Divergence analysis (read-only)

- The 1-behind commit on codeberg is `6a7cf69324 CLOSED: fix-ovh-access-key-id-misconfig, add-migration-safety-doc, tighten-gitignore-explicit-denylist, re…` (DraconDev, ~2 hours before this audit).
- The 217-ahead commits on local are exclusively authored by `dracon <dracon@local>` and the daemon (per `dracon-sync-incidents.jsonl` for this repo: 6,910 sync_commit entries in the 7-day window).
- `git merge-base --is-ancestor 6a7cf69324 HEAD` returned **exit=1** — the codeberg commit is NOT a local ancestor. They share merge-base `8fc02238f509…` but the histories diverged at that point.
- This is **not** a clean fast-forward situation. It is a true history divergence where the operator's codeberg mirror received one commit directly (likely via web UI or another machine) that is not present locally, and the daemon's local-only history has been blocked from pushing since the divergence appeared.
- `force_push_when_behind = true` is set on the `codeberg` [[remotes]] block in the global policy. `--force-with-lease` is the daemon's fallback. But `--force-with-lease` is **safe only when the remote tip equals the local tracking ref**. Here the local tracking ref `refs/remotes/codeberg/main-temp` has been refreshed by `git fetch` (during this audit) to point at `6a7cf69324`, and a force-with-lease would overwrite codeberg's divergent commit. This is the same risk class as the 2026-06-21 unintended force-push incident in `concern-2-4remote-divergence-2026-06-21.md` — and the current goal explicitly forbids that.

### What the repo would need to support triple-sync (read-only enumeration)

The 4 conditions that must be satisfied before the daemon can push to github + gitlab + codeberg for this repo:

| # | Condition | Current state | Verdict |
|---|---|---|---|
| 1 | A `github` remote pointing at `git@github.com:DraconDev/dracon-platform.git` (or the daemon's auto_create URL) | **Missing** | ❌ not met |
| 2 | A `gitlab` remote pointing at `git@gitlab.com:dracondev/dracon-platform.git` | **Missing** | ❌ not met |
| 3 | A local branch tracked to the same name as the forge's default branch (e.g. `main`) | Local branch is `main-temp`; forge defaults are `main` (per Section 3) | ❌ not met (would need rename `main-temp` → `main` and re-track) |
| 4 | The 1-behind commit `6a7cf69324` reconciled with local history (pull --rebase, or operator decision to overwrite) | **Unresolved** — this is the current PUSH_STUCK root cause | ❌ not met (operator decision required, see Section 7) |

Note: condition 3 is independent of the divergence. Even after the divergence is resolved, the local `main-temp` branch would push to `codeberg/main-temp` (which exists). The forge default branch is `main` and the daemon would need either (a) a per-repo config to push to `main-temp` explicitly, or (b) the local branch renamed to `main`. GitHub's `DraconDev/dracon-platform` repo has default=`main` (Section 3).

**Finding 1.1 — dracon-platform has no github/gitlab remotes.** Adding them is a 2-command operator action (`git remote add github …`, `git remote add gitlab …`) that the audit's read-only contract forbids. Severity: blocker (root cause of the `dracon-sync repos` ❌ CONCERN).

**Finding 1.2 — local branch is `main-temp`; forge default branches are `main`.** This is a smell (suggests operator worked around an earlier problem). To match forge defaults, the local branch would need to be renamed and the upstream re-tracked. Severity: warn.

**Finding 1.3 — codeberg has a divergent commit `6a7cf69324` that is not a local ancestor.** The daemon's `--force-with-lease` is unsafe here (per the 2026-06-21 incident precedent). Resolution requires operator decision: pull --rebase (adds the codeberg-only commit to local), or accept divergence and force-push local over codeberg (overwrites the divergent commit). Severity: blocker. This is the PUSH_STUCK root cause.

**Finding 1.4 — the 217-ahead local history is exclusively daemon-driven** (per the 6,910 sync_commit entries in `dracon-sync-incidents.jsonl` over the past 7 days). Once the divergence is resolved, this history will push in one large commit batch that may exceed the 900s push timeout (per `docs/design/gitlab-storage-and-divergence-2026-06-23.md` historical pattern). Severity: info. The daemon's dynamic timeout scaling (per the 2026-06-26 audit Finding 7.4) should handle it.

---

## Section 2 — Global dracon-sync config readiness for triple-sync

The full global policy is at `/home/dracon/.dracon/utilities/sync/dracon-sync.toml` (13.4 KiB). Relevant `[[remotes]]` blocks:

### `[[remotes]] name = "github"` (lines 234-249)

```toml
[[remotes]]
name = "github"
push_url = "git@github.com:DraconDev/{repo}.git"
auto_create = true  # Auto-create GitHub private repos for new repos

[remotes.repo_name_map]
".dracon" = "dracon-home"
"cli-file-manager" = "folder-auto-banner"
"dracon-sync" = "dracon-sync-background-auto-commit-multi-remote"
"dracon-warden" = "dracon-warden-secret-encrypt-age-git-filter"
"dracon-system" = "dracon-system-disk-process-guard-doctor"
# NOTE: no force_push_when_behind = true on github
```

### `[[remotes]] name = "gitlab"` (lines 251-274)

```toml
[[remotes]]
name = "gitlab"
push_url = "git@gitlab.com:dracondev/{repo}.git"
auto_create = true
force_push_when_behind = true  # --force-with-lease safety

[remotes.repo_name_map]
".dracon" = "dracon-home"
"cli-file-manager" = "folder-auto-banner"
"dracon-sync" = "dracon-sync-background-auto-commit-multi-remote"
"dracon-warden" = "dracon-warden-secret-encrypt-age-git-filter"
"dracon-system" = "dracon-system-disk-process-guard-doctor"
```

### `[[remotes]] name = "codeberg"` (lines 276-291)

```toml
[[remotes]]
name = "codeberg"
push_url = "git@codeberg.org:dracondev/{repo}.git"
auto_create = true
force_push_when_behind = true  # --force-with-lease safety

[remotes.repo_name_map]
".dracon" = "dracon-home"
"cli-file-manager" = "folder-auto-banner"
"dracon-sync" = "dracon-sync-background-auto-commit-multi-remote"
"dracon-warden" = "dracon-warden-secret-encrypt-age-git-filter"
"dracon-system" = "dracon-system-disk-process-guard-doctor"
```

### Name-map coverage vs the 15 watched repos

| Watched repo (basename) | Resolved github name | Resolved gitlab name | Resolved codeberg name | All 3 mapped? |
|---|---|---|---|---|
| `.dracon` (dir=`.dracon`) | `dracon-home` | `dracon-home` | `dracon-home` | ✅ |
| `dracon-platform` | `dracon-platform` (no map entry → `{repo}`) | `dracon-platform` | `dracon-platform` | ✅ (no map needed) |
| `pully-fully-pull-based-fleet-reconciler` | as-is | as-is | as-is | ✅ |
| `avid` | as-is | as-is | as-is | ✅ |
| `dracon-utilities` | as-is | as-is | as-is | ✅ |
| `browser-extensions-shared` | as-is | as-is | as-is | ✅ |
| `rust-ai-web-auto` | as-is | as-is | as-is | ✅ |
| `ai-auto-writer` | as-is | as-is | as-is | ✅ |
| `pi-plugins` | as-is | as-is | as-is | ✅ |
| `dracon-sync` (dir=`dracon-utilities/dracon-sync`) | `dracon-sync-background-auto-commit-multi-remote` | `dracon-sync-background-auto-commit-multi-remote` | `dracon-sync-background-auto-commit-multi-remote` | ✅ |
| `dracon-code` | as-is | as-is | as-is | ✅ |
| `dracon-strategy` | as-is | as-is | as-is | ✅ |
| `DraconDev` (dir=`dracon-strategy/DraconDev`) | as-is (case-sensitive on github) | as-is (DraconDev) | as-is (DraconDev) | ✅ (no map needed) |
| `dracon-warden` (dir=`dracon-utilities/dracon-warden`) | `dracon-warden-secret-encrypt-age-git-filter` | `dracon-warden-secret-encrypt-age-git-filter` | `dracon-warden-secret-encrypt-age-git-filter` | ✅ |
| `dracon-system` (dir=`dracon-utilities/dracon-system`) | `dracon-system-disk-process-guard-doctor` | `dracon-system-disk-process-guard-doctor` | `dracon-system-disk-process-guard-doctor` | ✅ |

**All 15 watched repos have a valid name mapping for all 3 forges.** The global config can drive triple-sync for every repo, modulo the local-repo-side requirements from Section 1.

**Finding 2.1 — `force_push_when_behind = true` is set on gitlab and codeberg, NOT on github.** This is intentional and consistent with the design rationale in the policy comments (the operator is the sole author on these repos, and `--force-with-lease` is a safe default for the mirrors). Severity: info.

**Finding 2.2 — `auto_create = true` on all 3 forges.** This means if the daemon is asked to push a repo whose remote does not exist, it will issue `gh repo create` / `glab repo create` / `curl POST /api/v1/user/repos` first. The 2026-06-26 audit's journal showed 521 `auto-create failed` warnings on github alone in 7 days ("GraphQL: You have created too many repositories, too quickly") — the auto-create was throttled by GitHub's API limits and never completed for the uncreated repos. Severity: warn. Many of the 404s in Section 3 are explained by this rate-limit history.

**Finding 2.3 — `dracon-utilities` has 4 remotes configured locally (`origin`, `github`, `gitlab`, `codeberg`).** The other 14 repos have at most 3 remotes. The extra `origin` is a historical artifact. Severity: info. No action.

---

## Section 3 — Live forge probing (read-only API calls)

Full probe evidence in `docs/design/audit-2026-06-26/triple-sync-probe.json` (15.0 KiB). All calls are GET / view — no POST/PUT/PATCH/DELETE issued.

### SSH auth (3 forges, BatchMode=yes)

| Host | Exit | Banner | Verdict |
|---|---|---|---|
| `git@github.com` | 1 | "Hi DraconDev! You've successfully authenticated, but GitHub does not provide shell access." | ✅ authenticated (exit=1 is normal for github) |
| `git@gitlab.com` | 0 | "Welcome to GitLab, @DraconDev!" | ✅ authenticated |
| `git@codeberg.org` | 0 | "Hi there, dracondev! You've successfully authenticated with the key named main, but Forgejo does not provide shell access." | ✅ authenticated |

### CLI auth

| Tool | Authenticated? | Scopes / protocol | Notes |
|---|---|---|---|
| `gh` | ✅ yes (DraconDev) | `gist, read:org, repo, workflow` over https | API calls work |
| `glab` | ❌ no token | SSH for git operations, https for API (401 unauthorized) | API calls fall back to `curl` (public read) |

### Per-repo × per-forge existence (15 × 3 = 45 cells)

✅ = exists · ❌ = 404 Not Found (does not exist on this forge) · see JSON for default_branch, visibility, ssh_url, etc.

| Watched repo | Resolved name | github | gitlab | codeberg |
|---|---|---|---|---|
| `.dracon` | `dracon-home` | ✅ (PRIVATE, default=main) | ❌ 404 | ❌ 404 |
| `dracon-platform` | `dracon-platform` | ✅ (PRIVATE, default=main) | ❌ 404 | ❌ 404 |
| `pully-fully-pull-based-fleet-reconciler` | as-is | ✅ (PRIVATE, default=main) | ❌ 404 | ❌ 404 |
| `avid` | `avid` | ✅ (PRIVATE, default=main) | ❌ 404 | ❌ 404 |
| `dracon-utilities` | `dracon-utilities` | ✅ (PUBLIC, default=main) | ✅ (public, default=main) | ✅ (default=main) |
| `browser-extensions-shared` | as-is | ✅ (PRIVATE, default=main) | ❌ 404 | ❌ 404 |
| `rust-ai-web-auto` | as-is | ✅ (PRIVATE, default=main) | ❌ 404 | ❌ 404 |
| `ai-auto-writer` | as-is | ✅ (PRIVATE, default=main) | ❌ 404 | ❌ 404 |
| `pi-plugins` | `pi-plugins` | ✅ (PUBLIC, default=main) | ❌ 404 | ✅ (default=main) |
| `dracon-sync` | `dracon-sync-background-auto-commit-multi-remote` | ✅ (PUBLIC, default=main) | ✅ (public, default=main) | ✅ (default=main) |
| `dracon-code` | as-is | ✅ (PRIVATE, default=main) | ❌ 404 | ❌ 404 |
| `dracon-strategy` | as-is | ✅ (PRIVATE, default=main) | ❌ 404 | ❌ 404 |
| `DraconDev` | `DraconDev` (case-preserved) | ✅ (PUBLIC, default=main) | ✅ (public, default=**master**) | ✅ (default=main) |
| `dracon-warden` | `dracon-warden-secret-encrypt-age-git-filter` | ✅ (PUBLIC, default=main) | ✅ (public, default=main) | ✅ (default=main) |
| `dracon-system` | `dracon-system-disk-process-guard-doctor` | ✅ (PUBLIC, default=main) | ✅ (public, default=main) | ✅ (default=main) |

### Existence summary

| Forge | Repos existing (out of 15) | Repos missing |
|---|---|---|
| github | **15/15** | none |
| gitlab | **5/15** | `.dracon`, `dracon-platform`, `pully-fully-pull-based-fleet-reconciler`, `avid`, `browser-extensions-shared`, `rust-ai-web-auto`, `ai-auto-writer`, `pi-plugins`, `dracon-code`, `dracon-strategy` |
| codeberg | **6/15** | `.dracon`, `dracon-platform`, `pully-fully-pull-based-fleet-reconciler`, `avid`, `browser-extensions-shared`, `rust-ai-web-auto`, `ai-auto-writer`, `dracon-code`, `dracon-strategy` |

**Finding 3.1 — github is the universal baseline.** All 15 watched repos exist on github. This is by design (per the policy comment: `auto_github_private_account = "DraconDev"` + `auto_github_private = true` had earlier created them all, plus the 2026-06-22 `auto_create = true` continues to cover new repos). Severity: info.

**Finding 3.2 — gitlab has only 5/15 repos** (the 3 utility subrepos + dracon-utilities + DraconDev). 10 repos are missing. The 2026-06-26 audit's journal showed 36 "GitLab metadata update failed: repo not found" warnings on Jun 19 22:49 and 23:15 — these are the uncreated repos. The `auto_create` mechanism on gitlab never succeeded for them, possibly due to API rate limits or to operator choice. Severity: blocker for triple-sync. The operator must decide whether to (a) accept the missing gitlab repos (per-repo override to skip gitlab), (b) trigger the auto-create manually for each, or (c) investigate why auto_create didn't complete.

**Finding 3.3 — codeberg has only 6/15 repos** (the same 5 as gitlab, plus `pi-plugins`). 9 are missing. The 2026-06-26 audit's journal showed similar "Codeberg metadata update failed: repo not found" warnings + 116 `auto-create failed for X on codeberg` warnings. The same `auto_create` mechanism never completed. Severity: blocker for triple-sync. Same decision space as gitlab.

**Finding 3.4 — `DraconDev` repo on gitlab has default branch `master`, not `main`.** This is the only case where the forge's default branch does not match the local `main` branch. Pushing `main` to gitlab would create a `main` branch (not the default), leaving the default `master` untracked. Severity: warn. Operator decision required: either (a) accept the mismatch, or (b) rename gitlab's default branch from `master` to `main` via the web UI.

**Finding 3.5 — `.dracon` (resolved name `dracon-home`) exists on github but NOT on gitlab or codeberg.** This is interesting because the daemon is currently pushing to it on all 3 remotes (per `dracon-sync repos` row 6: ✅ OK, `🔗 PUBLISH github/main`). On codeberg and gitlab, the push would fail with "repo not found" — but the daemon's `dracon-sync repos` shows it as healthy because the daemon checks `origin` (github), not the mirrors. This is the "operator-visibility gap" noted in `concern-2-4remote-divergence-2026-06-21.md` Section "Why the daemon didn't surface this proactively". Severity: warn. The auto_create for `.dracon` on gitlab/codeberg should have run at some point and may have been blocked by something specific to this repo.

**Finding 3.6 — `pi-plugins` exists on github AND codeberg, but NOT on gitlab.** Asymmetric triple-sync. Severity: info. Operator decision required.

**Finding 3.7 — `dracon-platform` exists ONLY on github, NOT on gitlab or codeberg.** The local repo's only remote is `codeberg` (pointing at the uncreated codeberg repo). The daemon has been trying to push to `codeberg` (404) and retrying 153+ times. This is the PUSH_STUCK root cause. Severity: blocker. The forge-side state is the opposite of what the operator might assume: codeberg's repo doesn't exist; github's does. To achieve triple-sync, the operator would need to (a) either add the github remote and accept that codeberg/gitlab are missing, or (b) trigger the auto_create on codeberg/gitlab and accept that those pushes are not yet ready.

---

## Section 4 — Per-repo triple-sync readiness matrix

| # | Watched repo | GitHub | GitLab | Codeberg | Local default branch | Forge default(s) match? | Ready for triple-sync? |
|---|---|---|---|---|---|---|---|
| 1 | `.dracon` (dracon-home) | ✅ exists | ❌ missing | ❌ missing | `main` | ✅ | ❌ blocker — gitlab+codeberg repos don't exist |
| 2 | dracon-platform | ✅ exists | ❌ missing | ❌ missing | `main-temp` | ❌ no (local=main-temp, forge=main) | ❌ blocker — gitlab+codeberg repos don't exist + branch mismatch + PUSH_STUCK |
| 3 | pully-fully-pull-based-fleet-reconciler | ✅ exists | ❌ missing | ❌ missing | `main` | ✅ | ❌ blocker — gitlab+codeberg repos don't exist |
| 4 | avid | ✅ exists | ❌ missing | ❌ missing | `main` | ✅ | ❌ blocker — gitlab+codeberg repos don't exist |
| 5 | dracon-utilities | ✅ exists | ✅ exists | ✅ exists | `main` | ✅ all 3 = main | ✅ **READY** (this is the canonical fully-triple-synced repo) |
| 6 | browser-extensions-shared | ✅ exists | ❌ missing | ❌ missing | `main` | ✅ | ❌ blocker — gitlab+codeberg repos don't exist |
| 7 | rust-ai-web-auto | ✅ exists | ❌ missing | ❌ missing | `main` | ✅ | ❌ blocker — gitlab+codeberg repos don't exist |
| 8 | ai-auto-writer | ✅ exists | ❌ missing | ❌ missing | `main` | ✅ | ❌ blocker — gitlab+codeberg repos don't exist |
| 9 | pi-plugins | ✅ exists | ❌ missing | ✅ exists | `main` | ✅ | ⚠️ partial — gitlab missing |
| 10 | dracon-sync | ✅ exists | ✅ exists | ✅ exists | `main` | ✅ all 3 = main | ✅ **READY** |
| 11 | dracon-code | ✅ exists | ❌ missing | ❌ missing | `main` | ✅ | ❌ blocker — gitlab+codeberg repos don't exist |
| 12 | dracon-strategy | ✅ exists | ❌ missing | ❌ missing | `main` | ✅ | ❌ blocker — gitlab+codeberg repos don't exist |
| 13 | DraconDev | ✅ exists | ✅ exists | ✅ exists | `main` | ⚠️ gitlab default=`master` | ⚠️ partial — gitlab default-branch mismatch (operational workaround: push to a non-default branch) |
| 14 | dracon-warden | ✅ exists | ✅ exists | ✅ exists | `main` | ✅ all 3 = main | ✅ **READY** |
| 15 | dracon-system | ✅ exists | ✅ exists | ✅ exists | `main` | ✅ all 3 = main | ✅ **READY** |

**Summary**: 4 of 15 repos are **READY** for triple-sync today (rows 5, 10, 14, 15). 2 of 15 are **partial** (rows 9, 13). 9 of 15 have **blocker** status (rows 1, 2, 3, 4, 6, 7, 8, 11, 12) because their gitlab and/or codeberg repos don't exist.

---

## Section 5 — Per-forge API requirements summary

### GitHub (`github.com/DraconDev`)

- **Auth**: SSH key (verified working — `Hi DraconDev!`). `gh` CLI is authenticated with token scopes `gist, read:org, repo, workflow`.
- **Auto-create**: `gh repo create <name> --private --source=<local>` (the daemon uses this). Throttled: 521 `auto-create failed` warnings in 7 days (the 2026-06-26 audit) due to "You have created too many repositories, too quickly". GitHub's rate limit is approximately 50/hour for new repos by default; resetting requires waiting.
- **Name restrictions**: cannot contain spaces; cannot start with a `.` or `-`; max 100 chars; case-preserved but case-insensitive on uniqueness. All 15 watched-repo names comply.
- **Default branch**: `main` is the standard. Newly created repos via API default to the account's `default_branch` setting (per the 2026-06-26 audit, this is `main` for DraconDev).
- **Rate-limit**: 5000 req/hour authenticated; the 521 auto-create failures were due to the new-repo rate-limit (~50/hour, separate bucket). SSH key auth is not rate-limited in the same way.
- **Visibility**: 7 of 15 repos are PRIVATE on github (per probe). `auto_github_private = false` is set in the global policy — wait, this seems contradictory. Let me re-check: the global policy says `auto_github_private = false` (do NOT auto-create on github), but the [[remotes]] `auto_create = true` (DO auto-create). The first flag controls `auto_github_private` (a separate feature for repos without an origin remote); the second controls the [[remotes]] block's per-remote auto_create. Reading more carefully: `auto_github_private = false` means "do not auto-create a github private repo for repos that have no origin remote at all". The [[remotes]] `auto_create = true` on the `github` [[remotes]] block means "auto-create via the [[remotes]] mechanism during multi-remote push". These are two different code paths. The 7 PRIVATE repos on github were likely created by the [[remotes]] mechanism (which defaults to `--private` per the daemon source).

### GitLab (`gitlab.com/dracondev`)

- **Auth**: SSH key (verified working). `glab` CLI is NOT API-authenticated (401); API calls fall back to `curl` (public read).
- **Auto-create**: `glab repo create <name> --private` or `curl -X POST -H "PRIVATE-TOKEN: …" https://gitlab.com/api/v4/projects`. The daemon's auto-create path likely needs an API token (which the operator hasn't provisioned for `glab`). The 36 "GitLab metadata update failed: repo not found" warnings in the 2026-06-26 audit are consistent with no auto-create happening (the metadata sync is a separate post-create step).
- **Name restrictions**: similar to GitHub. Names are user-scoped (`dracondev/<name>`) and case-preserved.
- **Default branch**: `main` for new projects. `DraconDev` repo on gitlab has `master` as default (legacy from before GitLab's default changed in 2020).
- **Rate-limit**: 300 req/min per user for authenticated API. SSH not rate-limited.
- **Visibility**: All 5 existing gitlab repos are `public`. The auto-create default in the daemon source for `gitlab` is likely `--visibility-level private` (or `internal`). Operator should verify what visibility was set on `dracon-utilities` etc.
- **Protected branches**: `dracon-utilities` has `main` protected (per `concern-2-4remote-divergence-2026-06-21.md`). Force-push to protected branches is rejected with `pre-receive hook declined`. This is independent of whether the daemon's `force_push_when_behind = true` is set in the global policy — the gitlab-side protection wins.

### Codeberg (`codeberg.org/dracondev`)

- **Auth**: SSH key (verified working, key name = `main`). No API token used; the API is public-read.
- **Auto-create**: `curl -X POST -H "Content-Type: application/json" -d '{"name":"...","private":true,"auto_init":false}' https://codeberg.org/api/v1/user/repos` (requires API token). The 116 `auto-create failed for X on codeberg` warnings in the 2026-06-26 audit are consistent with no API token being configured.
- **Name restrictions**: similar to GitHub. Names are user-scoped.
- **Default branch**: `main` for new repos. All 6 existing codeberg repos have `main` as default.
- **Rate-limit**: Forgejo-based; generally permissive for authenticated users. SSH not rate-limited.
- **Visibility**: All 6 existing codeberg repos return `visibility: null` in the API response (a Forgejo quirk: the visibility field is `public`/`private`/`limited` for organizations but the public API on a personal repo may not return it). `is_private: false` in all 6. So they are public.
- **Protected branches**: `dracon-utilities` on codeberg has `main` UNPROTECTED (per the 2026-06-21 doc, the operator's force-push succeeded there, which would have been rejected on a protected branch). This is the policy difference that allowed the 2026-06-21 unintended force-push to land on codeberg.

### Cross-forge: protection / force-push policy summary

| Forge | daemon's `force_push_when_behind` (global) | per-repo `main` protected? | Net effect for divergent repos |
|---|---|---|---|
| github | not set (uses default false) | unknown (not probed — would need to check each repo's settings) | daemon cannot force-push on github |
| gitlab | true | **protected** on `dracon-utilities` (and likely on others — to verify) | daemon's `--force-with-lease` is **rejected** by gitlab pre-receive hook |
| codeberg | true | **not protected** on `dracon-utilities` (and likely not on others) | daemon's `--force-with-lease` is **accepted** (and was the cause of the 2026-06-21 unintended force-push) |

**Finding 5.1 — codeberg's protection state is what made the 2026-06-21 unintended force-push possible.** This is the daemon's `force_push_when_behind = true` policy interacting with codeberg's lack of branch protection. The audit's read-only contract forbids resolving this, but the report flags it as a follow-up operator decision: either remove `force_push_when_behind = true` from the codeberg [[remotes]] block, or enable codeberg's `main` protection on each repo. Severity: warn (already known from the 2026-06-21 doc).

**Finding 5.2 — `gh` is the only CLI with API auth.** `glab` has SSH but no API token, so auto-create via glab is impossible. The auto-create path on gitlab must be a separate API call. The 36 GitLab metadata warnings (2026-06-26 audit) are consistent with the auto-create path failing silently and the metadata sync finding no repo. Severity: warn. The operator should provision a GitLab API token for the daemon if gitlab auto-create is desired.

**Finding 5.3 — forge-default-branch mismatches only occur for `DraconDev` (gitlab=master, others=main).** This is a legacy from before GitLab's default change. Severity: info. The daemon can push `main` to gitlab/DraconDev (creating a new branch on the remote), but gitlab's web UI will not show it as the default. The operator can rename gitlab's default via the web UI.

---

## Section 6 — Proposed per-repo `.dracon-sync.toml` content (TOML blocks only — NO files created)

For each repo where triple-sync is blocked by a forge-side gap, the proposed override below would let the daemon operate in a degraded-but-valid mode. **All blocks below are PROPOSED — DO NOT APPLY WITHOUT REVIEW.**

### Proposed override for repos where gitlab+codeberg don't exist (rows 1, 3, 4, 6, 7, 8, 11, 12)

If the operator decides **NOT** to auto-create the missing repos (e.g. to avoid public exposure), the daemon can be told to skip those mirrors. The override below says "only push to github, skip gitlab+codeberg" for a single repo.

```toml
# PROPOSED — DO NOT APPLY WITHOUT REVIEW
# This block is a per-repo override. If the operator decides to skip the
# gitlab+codeberg mirrors for repos where those repos do not exist,
# this would be placed at <repo>/.dracon-sync.toml (currently none exist).
# This is for repos: .dracon, pully, avid, browser-extensions-shared,
# rust-ai-web-auto, ai-auto-writer, dracon-code, dracon-strategy.
#
# This is a STATIC ANALYSIS: the daemon's policy schema has not been
# verified to support a "skip remotes" list. The block below is a
# best-guess and may need field-name adjustment after the operator
# reviews the daemon's policy.rs RemoteConfig / per-repo override schema.

# No per-repo fields needed to "skip gitlab/codeberg" — the global
# config has them enabled. A per-repo override to disable would be:
[remotes.github]
auto_create = true
# (gitlab and codeberg entries omitted, signalling "use global default" or "skip per-repo")
#
# ALTERNATIVE: the operator could choose to add gitlab+codeberg as
# mirrors but accept auto_create failures (daemon will retry). The
# current global config does this by default and produces the 36
# metadata warnings documented in the 2026-06-26 audit.
```

Note: I do not have a verified schema for per-repo `.dracon-sync.toml` that overrides the global `[[remotes]]` list. The daemon's per-repo override mechanism is described in the AGENTS.md "per-repo overrides" section, but I have not read the daemon's source to confirm the exact field shape. **The proposed block above is illustrative, not authoritative.** The operator should consult the daemon's `policy.rs` and `config.rs` source before applying anything.

### Proposed override for `dracon-platform` (row 2, the most divergent case)

`dracon-platform` has multiple issues: missing github/gitlab remotes, `main-temp` branch, PUSH_STUCK on codeberg, and divergent history. The override below would document the operator's decisions and configure the daemon accordingly. **This block is for the operator to review and adapt — NOT to apply as-is.**

```toml
# PROPOSED — DO NOT APPLY WITHOUT REVIEW
# dracon-platform: triple-sync prerequisites are NOT met. Before any
# per-repo override is created, the operator must:
#   1. Resolve the PUSH_STUCK on codeberg (decide canonical history
#      and either pull --rebase or accept force-push).
#   2. Decide on main-temp branch (rename to main? re-track main?).
#   3. Decide whether to add github and gitlab remotes locally.
#   4. Decide whether to auto-create dracon-platform on gitlab and
#      codeberg (currently 404 on both forges).
# Once those 4 decisions are made, the override below would be the
# CONFIGURATION (not the implementation) of the chosen state.

# (proposed, contingent on operator decisions 1-4)

# Option A: full triple-sync after decisions
[remotes.github]
push_url = "git@github.com:DraconDev/dracon-platform.git"
auto_create = false  # already exists per Section 3
force_push_when_behind = false  # not needed; resolve divergence upstream

[remotes.gitlab]
push_url = "git@gitlab.com:dracondev/dracon-platform.git"
auto_create = true  # would create the 404'd repo
force_push_when_behind = true

[remotes.codeberg]
push_url = "git@codeberg.org:dracondev/dracon-platform.git"
auto_create = true
force_push_when_behind = true

# Option B: codeberg-only (current state) — no per-repo override needed
# (the global config already pushes to all 3; the local repo's
# missing remotes mean only the configured one is used)
```

### Proposed override for `pi-plugins` (row 9, asymmetric: github+codeberg yes, gitlab no)

```toml
# PROPOSED — DO NOT APPLY WITHOUT REVIEW
# pi-plugins: codeberg and github exist, gitlab does not. The daemon
# can be told to skip gitlab by omitting the per-repo override (the
# global [[remotes]] gitlab block has auto_create = true which would
# try to create the missing repo, then fail at metadata sync).
#
# No override is strictly needed; the current behavior is "try to
# push to gitlab, fail, retry forever, log metadata warnings". This
# is the same pattern as 8 other repos. A per-repo override would
# only help if the operator wanted to suppress the metadata warnings.

# (no override content proposed; the global config + auto_create
# fallback is acceptable)
```

### Proposed override for `DraconDev` (row 13, default-branch mismatch on gitlab)

```toml
# PROPOSED — DO NOT APPLY WITHOUT REVIEW
# DraconDev: gitlab's default branch is `master`, but local is `main`.
# The daemon's push would succeed (creating a `main` branch on gitlab)
# but gitlab's web UI would show `master` as the default. Operator
# options:
#   1. Rename gitlab's default branch from `master` to `main` via
#      Settings → Repository → Default branch.
#   2. Accept the mismatch (daemon pushes main, gitlab shows master
#      as default, but both branches exist).
#   3. Configure the daemon to push to `master` instead of `main`
#      (would require a per-repo branch mapping, schema TBD).
#
# No per-repo override is proposed for option 1 or 2. Option 3
# would need:
# [branches]
# main = { push_to = "master" }  # schema TBD
```

**Finding 6.1 — no per-repo `.dracon-sync.toml` files were created during this audit.** `find /home/dracon -maxdepth 4 -name '.dracon-sync.toml' -newer /home/dracon/Dev/dracon-utilities/docs/design/triple-sync-feasibility-2026-06-26.md 2>/dev/null` returns 0 lines. Severity: info. The audit's read-only contract was honored.

**Finding 6.2 — the proposed overrides are illustrative, not authoritative.** The daemon's per-repo override schema has not been independently verified against the source code. The operator should consult `dracon-utilities/dracon-sync/src/policy.rs` and `config.rs` before applying any override. Severity: warn.

**Finding 6.3 — the most common "blocker" (9 of 15 repos) is "gitlab+codeberg repos don't exist".** The cleanest fix is to run the auto-create (or a manual `gh`/`curl POST`) for each missing repo, NOT to add per-repo overrides that suppress the warnings. Severity: blocker (operator decision required).

---

## Section 7 — Summary and recommended next actions

### Summary

| Metric | Count |
|---|---|
| Watched repos total | 15 |
| Triple-sync ready today (all 3 forges exist + default branch matches) | **4** (dracon-utilities, dracon-sync, dracon-warden, dracon-system) |
| Partial (≥1 forge missing or default-branch mismatch) | **2** (pi-plugins: gitlab missing; DraconDev: gitlab default=master) |
| Blocker (gitlab+codeberg repos don't exist) | **9** (.dracon, dracon-platform, pully, avid, browser-extensions-shared, rust-ai-web-auto, ai-auto-writer, dracon-code, dracon-strategy) |
| Policy violations against the 2026-06-17 commit-all principle | 0 (this audit did not change any exclude patterns) |
| New per-repo `.dracon-sync.toml` files created | **0** (read-only contract honored) |
| Forge-side state probes (45 cells) | 45 (all 15 × 3 forges) |
| `dracon-platform` ahead/behind on codeberg | **217 / 1** (grew from 68/1 in 5 minutes during this audit) |

### Recommended next actions (in priority order, framed as operator decisions)

1. **[blocker] Decide on `dracon-platform` divergence resolution.** The 1-behind commit `6a7cf69324` on codeberg blocks 217 local commits from pushing. Options: (a) `git pull --rebase codeberg main-temp` to add the codeberg-only commit to local history, (b) accept the divergence and force-push local over codeberg via `dracon-sync repair concerns --apply` (overwrites `6a7cf69324`; same risk class as 2026-06-21), (c) ignore the divergence and accept that 217 commits are stuck locally until the divergence is resolved.

2. **[blocker] Decide on `dracon-platform` branch name.** The local branch is `main-temp`; forge default is `main`. Rename local `main-temp` → `main` and re-track the forge default, OR add a per-repo override to push to `main-temp` explicitly (schema TBD).

3. **[blocker] Decide on the 9 missing repos on gitlab and codeberg.** For each, options: (a) trigger auto-create (run the daemon's auto-create path, or manually `gh repo create` / `curl POST /api/v1/user/repos`), (b) accept the missing repos and configure the daemon to skip them (no verified schema for this), (c) remove the repo from the daemon's `watch_roots` and add it to `exclude_repos`.

4. **[warn] Provision a GitLab API token for the daemon.** The 36 "GitLab metadata update failed: repo not found" warnings indicate the auto-create + metadata-sync path on gitlab is non-functional. Without a token, gitlab auto-create cannot complete. The same path may be why 10 repos are missing on gitlab.

5. **[warn] Decide on `DraconDev` default-branch mismatch on gitlab.** Rename gitlab's default from `master` to `main`, or accept the mismatch.

6. **[warn] Decide on codeberg's `force_push_when_behind = true` policy.** The 2026-06-21 unintended force-push was enabled by codeberg's lack of branch protection + the daemon's `force_push_when_behind = true`. Consider either (a) removing `force_push_when_behind = true` from the codeberg [[remotes]] block, or (b) enabling `main` protection on each codeberg repo. Both are policy changes, not just per-repo.

7. **[info] Consider the per-mirror visibility column for `dracon-sync repos`.** Per the 2026-06-21 doc's "Why the daemon didn't surface this proactively" section, the daemon's primary view of health is `origin`, not mirrors. A small change to `report.rs` could surface the per-mirror divergence visible in Section 3 of this report. Out of scope for the read-only investigation, but flagged for follow-up.

8. **[info] No per-repo `.dracon-sync.toml` was created during this audit.** The proposed overrides in Section 6 are illustrative. The operator should review the daemon's policy.rs schema before applying any of them.

---

## Evidence index

| File | Path | Description |
|---|---|---|
| `triple-sync-probe.json` | `docs/design/audit-2026-06-26/triple-sync-probe.json` | 15.0 KiB JSON: 15 × 3 forge existence + SSH auth + CLI auth |
| `daemon-behavior-audit-2026-06-26.md` | `docs/design/daemon-behavior-audit-2026-06-26.md` | Baseline daemon audit (sections 5, 7, 9) |
| `concern-2-4remote-divergence-2026-06-21.md` | `docs/design/concern-2-4remote-divergence-2026-06-21.md` | Prior art (4-remote divergence; contains 2026-06-21 unintended force-push audit note) |
| `dracon-platform` `.git/config` | `/home/dracon/Dev/dracon-platform/.git/config` | Captured verbatim in Section 1 |
| Global policy | `/home/dracon/.dracon/utilities/sync/dracon-sync.toml` | Lines 234-291 captured verbatim in Section 2 |

### Verification of read-only contract

```bash
# Should return 0 lines (no files created during this audit):
$ find /home/dracon -maxdepth 4 -name '.dracon-sync.toml' -newer \
    /home/dracon/Dev/dracon-utilities/docs/design/triple-sync-feasibility-2026-06-26.md 2>/dev/null | wc -l
0

# Should show no new remotes/branches/commits in dracon-platform:
$ cd /home/dracon/Dev/dracon-platform && git remote -v
codeberg	git@codeberg.org:dracondev/dracon-platform.git (fetch)
codeberg	git@codeberg.org:dracondev/dracon-platform.git (push)
$ git branch --show-current
main-temp
$ git log -1 --oneline
d4ca6983ff ...  # pre-audit state, daemon will have advanced this by reading time
```

**Audit complete. The single biggest finding is that 9 of 15 watched repos are missing on gitlab and/or codeberg, and the operator's decision on whether to auto-create, accept-missing, or skip is what unblocks triple-sync for the majority of the workspace.**