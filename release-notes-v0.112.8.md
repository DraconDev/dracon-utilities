# Release v0.112.8 — 2026-06-16

## Summary

This release is a **push-targets audit** that confirms the daemon and the
auto-sync mechanism only target the 3 long-name façade repos + the monorepo.
The Set A short-name repos (renamed in place on GitHub, hard-deleted on
Codeberg, in `_deletion_scheduled` state on GitLab) are explicitly ignored.

This is in direct response to the operator's feedback: "but make sure we are
ignoring the previous ones now we are just directly pushing to the ones we
marely with the long names right?" — Yes, confirmed: the daemon pushes only
to the 3 long-name façade repos + the monorepo.

## What was audited (and confirmed clean)

| Check | Result |
|-------|--------|
| Daemon watch list = 4 long-name repos (1 monorepo + 3 façade repos) | ✓ |
| No Set A URL in any local clone's remotes | ✓ |
| No Set A URL in any active config/script/code | ✓ |
| No local clone points to a `_deletion_scheduled` URL | ✓ |
| Auto-sync mechanism (post-commit hook + `regenerate_facade_repos.py`) only targets long-name clones | ✓ |
| All 4 watched repos are 4-remote aligned (github, gitlab, codeberg) | ✓ |
| Monorepo tests: 856 passed, 0 failed, 9 ignored | ✓ |

## The 4 canonical push targets

| # | Repo | Local path | Push targets |
|---|------|------------|--------------|
| 1 | `dracon-utilities` | `/home/dracon/Dev/dracon-utilities` | origin (github), github, gitlab, codeberg |
| 2 | `dracon-sync-background-auto-commit-multi-remote` | `/home/dracon/Dev/facade-repos/dracon-sync-background-auto-commit-multi-remote` | origin (github), github, gitlab, codeberg |
| 3 | `dracon-system-disk-process-guard-doctor` | `/home/dracon/Dev/facade-repos/dracon-system-disk-process-guard-doctor` | origin (github), github, gitlab, codeberg |
| 4 | `dracon-warden-secret-encrypt-age-git-filter` | `/home/dracon/Dev/facade-repos/dracon-warden-secret-encrypt-age-git-filter` | origin (github), github, gitlab, codeberg |

All 14 URLs across the 4 repos are long-name URLs. No Set A short-name URL
is the target of any push, by any mechanism (daemon, post-commit hook,
scaffold script, or any other code path).

## What is explicitly ignored (carve-out)

The 3 Set A repos on GitLab in `_deletion_scheduled` state are explicitly
ignored:

- `DraconDev/dracon-sync-watch-debounce-commit-push-mirror-deletion_scheduled-83426810`
- `DraconDev/dracon-system-disk-zram-process-service-guard-deletion_scheduled-83426812`
- `DraconDev/dracon-warden-age-git-filter-secret-encrypt-deletion_scheduled-83426814`

These are in GitLab's soft-delete state and will be hard-deleted by GitLab
automatically. The default of `A` (leave-as-is) was applied per goal
`83e42c15`; the operator can override to `B` (hard-delete now), `C`
(archive + rename to `-deprecated`), or `D` (deprecated README + archive)
per repo at any time.

## Historical references (carve-out)

Set A short-name URLs appear in 3 historical documents that document the
Set A → Set B rename event as history. These are NOT active references:

- `CHANGELOG.md` (multiple entries documenting the rename)
- `docs/design/github-feature-repos.md` (section comparing Set A vs Set B names)
- `release-notes-v0.112.5.md` (release notes documenting the rename as part of v0.112.5)

These references are not loaded by the daemon or any sync code, and exist
solely to explain the rename. They are explicitly carved out from the
"no Set A URL" rule.

## Version bumps

- Root workspace: `0.112.7` → `0.112.8` (patch-level, audit only)
- `dracon-sync`: `0.1.8` → `0.1.9`
- `dracon-system`: `0.2.3` → `0.2.4`
- `dracon-warden`: `0.3.3` → `0.3.4`

## What's in the box (since v0.112.7)

- `docs/design/push-targets-audit-2026-06-16.md` (new design doc with the
  full audit results)
- `CHANGELOG.md [0.112.8] / Investigated` entry documenting the audit
- Version bumps (no code changes)

## What's next

- The 3 GitLab Set A repos in `_deletion_scheduled` state will be hard-deleted
  by GitLab automatically. The operator can override to `B`/`C`/`D` per repo
  at any time per goal `83e42c15`; a follow-up release will cut on request.
- The daemon continues to auto-push monorepo + 3 façade repos on all 4
  remotes (post-commit hook + daemon sync is operational).
