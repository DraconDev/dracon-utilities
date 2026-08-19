# Docs index — dracon-utilities

A map of the design docs, audits, and process records in this repo.
The commit-all policy means these are all in git history; this index
is so you don't have to read 116 files to find the one you need.
**Convention**: `docs/design/` = durable design + investigation docs;
`docs/archive/` = superseded process iterations (kept for history);
root `*.md` = canonical audits, release notes, and core files.

> For how we work day-to-day (commit policy, daemon behavior,
> forbidden actions, test discipline), read `../AGENTS.md` first —
> it is the operator-facing authority, not this index.

---

## Core files (root)

| File | What it is |
|---|---|
| `AGENTS.md` | Operator + agent authority: commit policy, daemon behavior, forbidden actions, test discipline |
| `CHANGELOG.md` | Meta-repo changelog (all three utilities + daemon) |
| `README.md` | Repo overview |
| `CONTRIBUTING.md` | How to contribute |
| `SECURITY.md` | Security policy |
| `UTILITY_BOUNDARIES.md` | Where one utility ends and another begins |

## Top-level docs (`docs/`)

| File | What it is |
|---|---|
| `docs/ARCHITECTURE.md` | The 3 CLI binaries + systemd service layout |
| `docs/OPERATIONS.md` | Systemd services, incident response, troubleshooting |
| `docs/ROADMAP.md` | Documentation roadmap |
| `docs/README.md` | This index |
| `docs/design/` | Durable design + investigation docs (indexed below) |
| `docs/archive/` | Superseded process iterations (kept for history) |

## Canonical audits (root)

| File | What it is |
|---|---|
| `AUDIT-3-UTILITIES-2026-07-10.md` | The original 3-utility audit — **load-bearing**: ~24 code comments cite its CONCERN #4/#6 |
| `AUDIT_FULL_2026-07-18.md` | Full audit (daemon + all repos) → remediated in v0.112.19–21 |
| `AUDIT_FULL_2026-07-18-POSTFIX.md` | Post-remediation recheck of the 07-18 audit |
| `AUDIT_FULL_2026-07-21.md` | The audit that drove v0.112.31–34 (failure-visibility + warden + MEDIUM sweep) |
| `AUDIT_REPOS_2026-07-10.md`, `AUDIT_REPOS_2026-07-17.md` | Fleet repo-health audits |

## Policy & principles

| File | What it is |
|---|---|
| `commit-all-policy-2026-06-15.md` | The commit-all policy statement |
| `commit-all-policy-durable-2026-06-15.md` | The durable *code change* implementing it (5 design docs cite this) |
| `commit-all-principle-2026-06-16.md` | The operator's framing ("nothing left out unless very good reason") |
| `hygiene-what-to-ignore-2026-07-05.md` | What belongs in `.gitignore` vs committed |
| `warden-hygiene-defaults-2026-08-18.md` | Warden's narrow machine-local hygiene defaults and local-build release handoff |
| `codeberg-public-only-policy-2026-07-17.md` | Codeberg = public mirrors only (quota posture) |
| `codeberg-quota-leak-fix-2026-07-13.md` | The 85 GiB quota leak + forward-only fix |
| `pi-tmp-persist-policy-2026-06-16.md` | pi-tmp persistence policy |

## Architecture

| File | What it is |
|---|---|
| `nested-on-main-architecture-2026-07-02.md` | The current submodule-on-main design (canonical) |
| `daemon-standalone-removal-2026-07-01.md` | Why the standalone worktree layout was eliminated |
| `big-repo-storage-strategy.md` | Big-repo storage approach |
| `binary-asset-strategy-2026-07-03.md` | Binary assets in git (LFS vs bucket) |
| `lfs-vs-bucket-vs-grow-2026-07-03.md` | The LFS/bucket/grow decision |
| `triple-sync-feasibility-2026-06-26.md` | 3-forge mirror feasibility |
| `submodule-pain-explanation-2026-07-03.md` | Why submodules hurt + how we avoid the pain |
| `correction-and-monorepo-options-2026-07-03.md` | Monorepo options after the correction |
| `architecture-debate-was-distraction-2026-07-03.md` | Why the architecture debate was a distraction |
| `standalone-utility-repos-2026-06-21.md` | Standalone utility repos design |
| `daemon-behavior-audit-2026-06-26.md` | Daemon behavior deep-dive |
| `daemon-settling-2026-06-20.md` | Daemon settling behavior |
| `empty-repo-auto-create-fix-2026-07-21.md` | Empty-repo bootstrap + never-pushed detection (v0.112.29/30) |
| `github-feature-repos.md` | GitHub-specific repo features |

## Audits (daemon + fleet, in `docs/design/`)

| File | What it is |
|---|---|
| `daemon-behavior-audit-2026-06-26.md` | Daemon behavior audit |
| `full-audit-2026-07-03.md` | Full audit 07-03 (the `followup-tasklist` derives from it) |
| `full-audit-2026-07-05.md` | 26-repo push-health audit (cited by `hegemon-state-investigation`) |
| `full-audit-2026-07-09.md` | Wide-lens follow-up (daemon source + all repos) |
| `full-push-audit-2026-07-02.md` | Push-to-all audit |
| `repo-discovery-audit-2026-07-09.md` | Repo-discovery audit (fixed 3 daemon defects) |
| `sync-health-audit-2026-07-09.md` | Sync health audit |
| `sync-stall-audit-2026-07-09.md` | Sync stall audit (2 GiB pack stall) |
| `untracked-audit-2026-06-17.md` | Untracked-files audit (cited by AGENTS.md) |
| `untrackeds-audit-2026-07-09.md` | Untracked follow-up audit |
| `push-targets-audit-2026-06-16.md` | Push-target audit |
| `repos-speed-audit-2026-07-04.md` | `repos` performance audit |
| `scan-bloat-review-2026-07-15.md` | scan-bloat review (codeberg quota) |
| `post-migration-audit-2026-07-03.md` | Post-migration audit |
| `all-green-investigation-2026-06-15.md` | The all-green investigation |
| `final-audit-2026-06-16.md` | Final audit 06-16 |

## Incidents & fixes (per-repo and daemon)

### deathrun + size/bloat
| File | What it is |
|---|---|
| `audit-screenshot-bloat-deathrun-2026-07-23.md` | **deathrun 2 GiB fix (orphan cutover) + the probe-bug correction** |
| `platform-repo-bloat-investigation-2026-06-24.md` | Platform repo-bloat investigation |
| `dracon-platform-16gib-analysis-2026-07-07.md` | dracon-platform 16 GiB analysis |
| `auto-create-size-investigation-2026-06-27.md` | Auto-create size investigation |

### Push / sync incidents
| File | What it is |
|---|---|
| `push-stuck-resolution-2026-06-27.md` | Push-stuck resolution (cited by AGENTS.md) |
| `push-timeout-fix-2026-06-17.md` | Push timeout fix (cited by AGENTS.md) |
| `sync-push-classification.md` | Push rejection classification (cited by AGENTS.md) |
| `source-encryption-incident-2026-06-15.md` | Source encryption incident (cited by AGENTS.md) |
| `dracon-libs-deletion-2026-06-15.md` | dracon-libs symlink deletion (cited by AGENTS.md) |
| `dracon-platform-untracked-commit-2026-06-15.md` | dracon-platform untracked commit (cited by AGENTS.md) |
| `junk-runner-investigation-2026-06-15.md` | junk-runner policy drift (cited by AGENTS.md) |
| `ownership-investigation-2026-06-15.md` | Repo ownership analysis (cited by AGENTS.md) |
| `warden-plaintext-sibling.md` | Warden plaintext sibling handling (cited by AGENTS.md) |
| `platform-push-stuck-resolution-2026-06-26.md` | Platform push-stuck resolution |
| `dracon-utilities-push-stuck-2026-06-29.md` | dracon-utilities push-stuck |
| `push-stuck-render-investigation-2026-06-29.md` | Push-stuck render investigation |
| `no-origin-concern-ssh-2026-06-20.md` | No-origin concern (SSH migration) |
| `mirror-divergence-and-secret-remediation-2026-06-21.md` | Mirror divergence + secret remediation |
| `mirror-only-push-and-empty-repo-remotes-2026-06-20.md` | Mirror-only push + empty-repo remotes |
| `gitlab-storage-and-divergence-2026-06-23.md` | GitLab storage + divergence |

### Per-repo investigations (kiki-sassy, hegemon, dracon-platform, junk-runner, capture-anime-girls)
| File | What it is |
|---|---|
| `kiki-sassy-decision-handoff-2026-06-15.md` | kiki-sassy decision handoff |
| `kiki-sassy-deep-investigation-2026-06-16.md` | kiki-sassy deep investigation |
| `kiki-sassy-followups-2026-06-16.md` | kiki-sassy follow-ups |
| `kiki-sassy-merge-resolution-2026-06-16.md` | kiki-sassy merge resolution |
| `hegemon-autoignore-2026-07-05.md` | hegemon anti-rebloat (`.state-recon`) |
| `hegemon-backup-cruft-comparison-2026-07-05.md` | hegemon backup cruft comparison |
| `hegemon-github-fit-audit-2026-07-09.md` | hegemon github fit audit |
| `hegemon-github-push-fix-2026-07-06.md` | hegemon github push fix |
| `hegemon-size-investigation-2026-07-05.md` | hegemon size investigation (dangling objects → gc) |
| `hegemon-state-investigation-2026-07-05.md` | hegemon state investigation |
| `junk-runner-fix-2026-06-15.md` | junk-runner fix |
| `capture-anime-girls-uncommitted-investigation-2026-07-06.md` | capture-anime-girls uncommitted investigation |
| `dracon-code-divergence-2026-06-16.md` | dracon-code divergence |
| `concern-1-dracon-platform-2026-06-21.md` | Concern #1: dracon-platform |
| `concern-2-4remote-divergence-2026-06-21.md` | Concern #2: 4-remote divergence |
| `concern-investigation-2026-06-16.md` | Concern investigation |
| `concern-repo-investigation-2026-06-21.md` | Concern repo investigation |
| `concerns-investigation-2026-07-18.md` | Concerns investigation (drove v0.112.19–21) |
| `auto-commit-junk-investigation-2026-07-01.md` | Auto-commit junk investigation |
| `auto-private-repo-fix-2026-07-15.md` | Auto-private-repo fix |

### Untracked / dirty / daemon fixes
| File | What it is |
|---|---|
| `untracked-content-resolution-2026-06-15.md` | Untracked content resolution |
| `untracked-md-systemic-2026-06-16.md` | Untracked .md systemic |
| `excluded-dirty-state-2026-06-15.md` | Excluded dirty state |
| `dirty-files-investigation.md` | Dirty files investigation |
| `dirty-repos-followup-2026-07-09b.md` | Dirty repos follow-up |
| `warn-concern-followup-2026-07-09.md` | WARN/CONCERN follow-up |
| `daemon-auto-resolve-unmerged-2026-06-21.md` | Daemon auto-resolve-unmerged |
| `daemon-concerns-cleanup-2026-07-01.md` | Daemon concerns cleanup |
| `daemon-fd-limit-fix-2026-07-03.md` | Daemon fd-limit fix |
| `daemon-pi-dir-skip-bug-2026-06-22.md` | Daemon .pi-dir skip bug |
| `daemon-staging-fix-2026-06-19.md` | Daemon staging fix |
| `facade-repo-staleness-fix-2026-06-21.md` | Facade repo staleness fix |
| `revert-filters-2026-06-15.md` | Revert filters |
| `secret-scan-text-files-2026-06-16.md` | Secret-scan text files |
| `warden-hook-pi-goals-skip-2026-06-18.md` | Warden hook pi-goals skip |
| `owner-nixos-pub-tracking.md` | owner-nixos-pub tracking |

### `repos` table / report work
| File | What it is |
|---|---|
| `repos-state-cause.md` | `repos` state-cause model |
| `repos-status-active-2026-07-16.md` | `repos` ACTIVE status |
| `repos-status-ok-clean-2026-07-17.md` | `repos` OK/CLEAN status |
| `repos-no-push-concern-2026-07-16.md` | `repos` no-push concern |
| `repos-perf-fix-2026-07-15.md` | `repos` performance fix |
| `repos-role-column-2026-07-01.md` | `repos` ROLE column |
| `repos-table-fix-2026-07-18.md` | `repos` table fix |
| `repos-view-improvements-2026-07-06.md` | `repos` view improvements |
| `repo-remote-visibility-2026-06-27.md` | Remote visibility (v1) |
| `repo-remote-visibility-v2-2026-06-27.md` | Remote visibility (v2) |
| `repo-remote-visibility-v3-revert-2026-06-27.md` | Remote visibility (v3, the revert — **the settled approach**) |
| `dracon-sync-repos-vs-vscode-discrepancy-2026-06-21.md` | `repos` vs VS Code discrepancy |
| `dracon-sync-warn-investigation-2026-06-17.md` | `repos` WARN investigation |
| `dracon-platform-push-investigation-2026-06-15.md` | dracon-platform push investigation |
| `dracon-platform-cleanup-2026-06-16.md` | dracon-platform cleanup |
| `dracon-platform-pack-size-hint-fix-2026-07-07.md` | dracon-platform pack-size hint fix |
| `platform-stupid-amount-of-changes-2026-06-21.md` | Platform change-volume investigation |

## Release process

| File | What it is |
|---|---|
| `release-process-2026-06-21.md` | The release process |
| `crates-io-publish-2026-06-16.md` | crates.io publishing |
| `patch-to-git-tag-2026-07-18.md` | `[patch.crates-io]` → git-tag (dracon-git v94.7.1) |
| `release-notes-v0.112.*.md` | One per version — archived 2026-07-23 (content is in `CHANGELOG.md`); see `docs/archive/release-notes/` |
| `v2-card-design-snapshot-2026-06-16.md` | v2 card design snapshot |
| `keep-alive.md` | Keep-alive |
| `followup-tasklist-2026-07-03.md` | Follow-up tasklist (from full-audit-2026-07-03) |
| `cli-print-style.md` | CLI print style |

## Archive (`docs/archive/`)

Superseded process iterations kept for history (not for reading):
- `audits-2026-07/` — the 7 `AUDIT-3-UTILITIES-*` process iterations
  (FINAL, FULL, INDEPENDENT, RECHECK×2, RERUN, FILTER-REPO) from
  running the 2026-07-10 audit seven ways. The canonical original is
  `AUDIT-3-UTILITIES-2026-07-10.md` at root.
- `release-notes/` — 32 per-version release notes (v0.112.5–12, 15–39).
  Pure duplication of `CHANGELOG.md` entries; archived 2026-07-23. The
  v0.112.13 and v0.112.14 notes remain at `dracon-sync/`.
- `test_activity.md` — a 1-line scratch artifact.

---

*Generated 2026-07-23 (docs cleanup). If a doc you need is missing
here, it's in git history (commit-all policy) or `docs/archive/`.*
