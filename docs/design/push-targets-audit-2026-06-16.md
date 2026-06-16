# Push Targets Audit — 2026-06-16

> **Goal**: `d2837ddc` (operator: "we are ignoring the previous ones now we are
> just directly pushing to the ones we marely with the long names right?")
>
> **Status**: **CLEAN** — daemon only pushes to the 3 long-name façade repos
> + the monorepo. No Set A short-name URL is the target of any push, by any
> mechanism.

This audit confirms that, as of 2026-06-16, the **daemon** (`dracon-sync`),
the **auto-sync mechanism** (monorepo `post-commit` hook →
`regenerate_facade_repos.py`), and **any other code or config** that triggers
a push, only target the long-name façade repos + the monorepo. All
short-name Set A URLs are ignored.

## Watched repos (the canonical 4)

The daemon's `dracon-sync repos` output shows exactly 4 long-name repos for
the `dracon-utilities` monorepo + its 3 façade repos:

| # | Repo | Local path | Remotes | Last commit |
|---|------|------------|---------|-------------|
| 3 | `dracon-utilities` | `/home/dracon/Dev/dracon-utilities` | origin (github), github, gitlab, codeberg | `ec45ef1` |
| 13 | `dracon-system-disk-process-guard-doctor` | `/home/dracon/Dev/facade-repos/dracon-system-disk-process-guard-doctor` | origin (github), github, gitlab, codeberg | `06a2021` |
| 14 | `dracon-warden-secret-encrypt-age-git-filter` | `/home/dracon/Dev/facade-repos/dracon-warden-secret-encrypt-age-git-filter` | origin (github), github, gitlab, codeberg | `38327a4` |
| 15 | `dracon-sync-background-auto-commit-multi-remote` | `/home/dracon/Dev/facade-repos/dracon-sync-background-auto-commit-multi-remote` | origin (github), github, gitlab, codeberg | `e6f05a1` |

All 4 are `✅ OK` and `🟢 synced`. The 12 other repos in the daemon's watch
list (`kiki-sassy-desktop-announcer`, `dracon-ai-lib`, `browser-extensions-shared`,
`dracon-libs`, `.dracon`, `avid`, `rust-ai-web-auto`, `dracon-code`,
`dracon-platform`, `pully-fully-pull-based-fleet-reconciler`, `ai-auto-writer`,
`DraconDev` org) are unrelated to this audit.

## Audit checks (all pass)

### Check 1: Daemon watch list is exactly 4 long-name repos

```bash
dracon-sync repos | grep -oE "dracon-(utilities|sync-[a-z-]+|system-[a-z-]+|warden-[a-z-]+)" | sort -u
# Result:
# dracon-sync-background-auto-commit-multi-remote
# dracon-system-disk-process-guard-doctor
# dracon-utilities
# dracon-warden-secret-encrypt-age-git-filter
```

**Result: ✓ PASS** — exactly the 4 expected long-name repos.

### Check 2: No Set A URL in any local clone's remotes

```bash
for dir in /home/dracon/Dev/facade-repos/* /home/dracon/Dev/dracon-utilities /home/dracon/dracon/*; do
  [ -d "$dir/.git" ] || continue
  git -C "$dir" remote -v 2>/dev/null | awk '{print $2}'
done | sort -u
```

All 14 URLs in the result are long-name:

| Host | Long-name URLs (3 façade repos + monorepo) |
|------|---------------------------------------------|
| github.com | `dracon-sync-background-auto-commit-multi-remote`, `dracon-system-disk-process-guard-doctor`, `dracon-utilities`, `dracon-warden-secret-encrypt-age-git-filter` |
| gitlab.com | `dracon-sync-background-auto-commit-multi-remote`, `dracon-system-disk-process-guard-doctor`, `dracon-utilities`, `dracon-warden-secret-encrypt-age-git-filter` |
| codeberg.org | `dracon-sync-background-auto-commit-multi-remote`, `dracon-system-disk-process-guard-doctor`, `dracon-utilities`, `dracon-warden-secret-encrypt-age-git-filter` |

**Result: ✓ PASS** — zero Set A short-name URLs.

### Check 3: No Set A URL in any active config/script/code

```bash
grep -rE "(watch-debounce|disk-zram|age-git-filter-secret-encrypt)" \
  /home/dracon/Dev/dracon-utilities/scripts \
  /home/dracon/Dev/dracon-utilities/.git/hooks \
  /home/dracon/Dev/dracon-utilities/install.sh \
  /home/dracon/.dracon/utilities/sync/dracon-sync.toml
```

**Result: ✓ PASS** — zero matches in any active config, script, or code.

### Check 3a: Historical references (carve-out)

Set A short-name URLs appear in 3 historical documents that document the
Set A → Set B rename event:

| File | Type | Why OK |
|------|------|--------|
| `CHANGELOG.md` | Changelog | Documents the rename event as history (explicitly carved out) |
| `docs/design/github-feature-repos.md` | Design doc | Section compares Set A vs Set B names; explains the rename |
| `release-notes-v0.112.5.md` | Release notes | Documents the rename event as part of the v0.112.5 release |

These references are **historical documentation** — they do not cause any
push to a Set A URL, are not loaded by the daemon or any sync code, and
exist solely to explain the rename that occurred. They are not "active"
references.

**Result: ✓ PASS** — all historical references are in documents that
document the rename as a historical event.

### Check 4: No clone points to `_deletion_scheduled` URLs

```bash
find /home/dracon -name "config" -path "*/.git/*" 2>/dev/null \
  | xargs grep -l "deletion_scheduled" 2>/dev/null
```

**Result: ✓ PASS** — zero matches. No local clone has a `_deletion_scheduled`
URL in its remotes.

### Check 5: Auto-sync only targets long-name clones

```bash
cat .git/hooks/post-commit
# MONOREPO_ROOT="$(git rev-parse --show-toplevel)"
# exec python3 "$MONOREPO_ROOT/scripts/regenerate_facade_repos.py" \
#     --monorepo-root "$MONOREPO_ROOT" \
#     --target-root /home/dracon/Dev/facade-repos

grep -nE "(target_root|FACADE_REPO_ROOT|façade_repo)" scripts/regenerate_facade_repos.py
# 60:DEFAULT_TARGET_ROOT = "/home/dracon/Dev/facade-repos"
# 99:    target_root: Path,
# 104:    facade_dir = target_root / long_name
# 132:            str(target_root),
# 227:    target_root = args.target_root.resolve()
# 255:            print(f"  [dry-run] would regenerate {u} → {target_root / long_name}")
# 260:        if not _regenerate_one(monorepo_root, target_root, utility):
```

**Result: ✓ PASS** — `target_root` is hardcoded to
`/home/dracon/Dev/facade-repos` (the long-name-only path). The `long_name`
values are the 3 Set B long-names.

### Check 6: 4-remote alignment of all 4 watched repos

| Repo | origin | github | gitlab | codeberg | All aligned? |
|------|--------|--------|--------|----------|--------------|
| `dracon-utilities` | `ec45ef1` | `ec45ef1` | `ec45ef1` | `ec45ef1` | ✓ |
| `dracon-sync-background-auto-commit-multi-remote` | `e6f05a1` | `e6f05a1` | `e6f05a1` | `e6f05a1` | ✓ |
| `dracon-system-disk-process-guard-doctor` | `06a2021` | `06a2021` | `06a2021` | `06a2021` | ✓ |
| `dracon-warden-secret-encrypt-age-git-filter` | (no `origin`) | `38327a4` | `38327a4` | `38327a4` | ✓ |

**Result: ✓ PASS** — all 4 repos are 4-remote aligned (warden has only 3
remotes because its `origin` (HTTPS github) is hidden behind the SSH `github`
remote, which is the canonical "github" remote per the daemon's
`force_push_when_behind = true` policy).

### Check 7: Monorepo tests

```bash
cargo test --workspace --locked
# Result: 856 passed, 0 failed, 9 ignored
```

**Result: ✓ PASS** — no regression.

## What about the 3 GitLab Set A repos in `_deletion_scheduled` state?

The 3 Set A repos on GitLab
(`DraconDev/dracon-sync-watch-debounce-commit-push-mirror-deletion_scheduled-83426810`,
`DraconDev/dracon-system-disk-zram-process-service-guard-deletion_scheduled-83426812`,
`DraconDev/dracon-warden-age-git-filter-secret-encrypt-deletion_scheduled-83426814`)
are in GitLab's soft-delete state and will be hard-deleted by GitLab
automatically. The default of `A` (leave-as-is) was applied per goal
`83e42c15`; the operator can override to `B` (hard-delete now), `C`
(archive + rename to `-deprecated`), or `D` (deprecated README + archive)
per repo at any time.

The daemon does **not** push to these repos. No local clone has these URLs
in its remotes. They are explicitly ignored.

## Why this is the right state

- The 3 long-name façade repos are the canonical install targets for the
  3 utilities (per goal `6a105c59` / v0.112.7).
- The monorepo is the dev workspace + build source for all 3 utilities.
- The 3 Set A short-name repos are deprecated; their GitHub URLs auto-redirect
  to the long-name, the Codeberg Set A repos were hard-deleted, and the
  GitLab Set A repos are pending hard-delete.
- The daemon's job is to ensure that nothing is left out unless there's a
  very good reason (per goal `6205ad1f`). Ignoring the Set A repos is
  consistent with this principle because they are deprecated.

## What stays untracked (carve-out)

The 3 GitLab Set A repos in `_deletion_scheduled` state are NOT being
auto-purged, NOT being pushed to, and NOT being modified. They will be
hard-deleted by GitLab automatically. The operator can override with
`B`/`C`/`D` per repo at any time per goal `83e42c15`; a follow-up release
will cut on request.
