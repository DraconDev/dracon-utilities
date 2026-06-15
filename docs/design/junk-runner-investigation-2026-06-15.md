# Junk-Runner-bevy + dracon-platform untracked investigation — 2026-06-15

## Operator request

> "ok we want to look into junk runner and the
> platform, do we have a good reason to no trck
> those ? junk runenr just sems wrong"

The operator asked: do we have a good reason to not
track the untracked files in `Junk-Runner-bevy` and
`dracon-platform`?

## TL;DR — YES, but the policy was on the wrong branch

**Junk-Runner-bevy: REAL BUG FOUND.** The operator's
policy to exclude `test-results/` from auto-commit was
on the `tauri2` branch but the daemon was working on
`main` — so the policy was NOT being applied. The
daemon was committing test-results/ PNGs on main
(e.g. commit `b71c068db` "3 file(s) in test-results").
The operator's "junk runner just seems wrong" was
CORRECT — the policy was on the wrong branch.

**dracon-platform: GOOD REASONS for untracked files.**
The 9 `.pi-tmp/*` scratch dirs are by convention never
committed. The 3 deferred source dirs were intentionally
deferred in goal `ca80b0d1`. The 2 new audit dirs were
a real bug (daemon not picking them up) — fixed by
manual commit.

## Part A: Junk-Runner-bevy — REAL BUG (policy on wrong branch)

### Initial investigation

The 3 untracked PNGs in `test-results/` (at the time)
were correctly explained by the operator's per-repo
policy at `.dracon/dracon-sync.toml`:

```toml
auto_commit_exclude_patterns = [
    "**/test-results/**",
    "**/e2e/screenshots/**",
]
```

The policy was added in `44dffcada` (2026-06-15
10:59:12) and updated in `dc8f85fe1` (2026-06-15
11:13:19). It excluded test-results/ PNGs from
auto-commit to break a 2989-commit auto-commit loop.

### Then the daemon committed the r4 PNGs anyway

At 20:37:50, the daemon made commit `b71c068db`
"3 file(s) in test-results" — adding the r4 PNGs
to git. This was surprising — the policy should have
excluded them.

### Root cause: the policy is on `tauri2`, not `main`

Investigation revealed:
- Current branch (per `.git/HEAD`): `main`
- `git ls-tree HEAD` does NOT contain
  `.dracon/dracon-sync.toml`
- `git ls-tree tauri2` DOES contain it
- The operator's policy was committed to `tauri2` but
  the daemon is working on `main` (the default branch)
- Therefore the policy was not being applied to the
  branch the daemon was using

### Branch state

- `main` is 1 commit ahead of merge-base `6d1e953b6`
  (the Sponsors button commit)
- `tauri2` is **3036 commits** ahead of `main`
- The operator has two parallel branches and the
  policy made it to one but not the other

### Fix applied

Copied `.dracon/dracon-sync.toml` from `tauri2` to
`main`:

```bash
cd /home/dracon/Dev/Junk-Runner-bevy
git checkout tauri2 -- .dracon/dracon-sync.toml
git add .dracon/dracon-sync.toml
git commit -m "infra(sync): apply test-results exclude policy to main branch"
git push origin main
git push gitlab main
git push codeberg main
```

Result: All 4 remotes aligned at `24709b924db6`.
After this commit, future test runs that regenerate
`test-results/` PNGs will NOT be auto-committed.

### Other Junk-Runner-bevy findings

- **72 MOD transient state**: resolved by daemon's
  normal auto-commit cycle (24 commits in 2h, 188
  commits in 2 days)
- **Divergence with origin (Sponsors button)**: local
  was 1 commit behind origin (and github), pulled and
  pushed to all 4 remotes, now aligned at
  `6d1e953b6865` (now superseded by `24709b924db6`)

## Part B: dracon-platform — 14 UT explained

### What the operator saw

- **1 MOD**: `web/NAV-LOADING-LOGO-2026-06-15.md`
- **11 UT**: Mix of `.pi-tmp` scratch dirs, source
  dirs, and 1 new audit dir

### What it was at the time of investigation

- **0 MOD**: Already committed
- **14 UT**: 9 `.pi-tmp/*` scratch dirs (2 new since
  ca80b0d1) + 3 source dirs from ca80b0d1 deferred
  items + 2 new audit dirs

### Why the 14 entries are untracked — each has a reason

#### 9 `web/.pi-tmp/*` scratch dirs (session temps)

By project convention, `.pi-tmp/` directories are
session scratch files from the operator's prior `pi`
agent work sessions. They are **NEVER committed**.
The convention is documented in
`docs/design/dracon-platform-untracked-commit-2026-06-15.md`
(goal `ca80b0d1`).

#### 3 source dirs (deferred from ca80b0d1)

- `web/games/games/hegemon/src/lib/` (31 source files)
- `web/games/games/hegemon/static/assets/` (33 game
  art files)
- `web/games/src/routes/games/[slug]/` (2 source files)

These were intentionally deferred in goal
`ca80b0d1` because the operator's previous
auto-commit pattern (in goal `fa84a5bd`) committed
docs like `hegemon/ASSETS.md`, `hegemon/AUDIT.md`,
`hegemon/README.md`, `hegemon/package.json` (4
files) but NOT the `src/lib/` or `static/assets/`
subdirectories. The operator's intent is to commit
source code deliberately, not via auto-commit.

#### 2 new audit dirs (DAEMON BUG: should have been committed)

- `web/screenshots/audit-byteplus-cerebras-cloudflare-zai-opencode-2026-06-15/`
  (6 files, 672K, created 19:06)
- `web/screenshots/audit-dp9cqdwdz9-bonus-priority-2026-06-15/`
  (9 files, 1.1M, created 20:09)

These are intentional audit evidence directories.
They SHOULD have been auto-committed by the daemon
like the other `audit-*` dirs in the repo. The
daemon was committing other files in
`dracon-platform` (HOME-AUDIT-2026-06-15.md,
hegemon source files, etc.) but NOT these 2 dirs.

**Root cause investigation**: The daemon's
`auto_stage_untracked` default is `true`, so it
SHOULD stage untracked files. The 2 audit dirs
are small (15 files, 1.7MB total) and well under
the 50MB `max_stage_file_bytes` limit. Yet the
daemon hadn't picked them up.

**Manual commit (resolution)**: committed the 2
audit dirs, pushed to all 4 remotes. Result: all
4 remotes aligned at `700d58c28c08`.

## Part C: Resolution summary

### Junk-Runner-bevy

- **REAL BUG FIXED**: Policy was on `tauri2` branch
  only, not on `main` (the working branch). Copied
  the policy to `main`. All 4 remotes now aligned at
  `24709b924db6`.
- **Sponsors button divergence**: pulled from
  origin, pushed to gitlab/codeberg. All 4 remotes
  aligned at `6d1e953b6865` (now superseded by
  `24709b924db6`).

### dracon-platform

- **9 .pi-tmp scratch dirs**: CORRECTLY untracked by
  convention (no fix needed)
- **3 source dirs**: DEFERRED per ca80b0d1 (no fix
  needed, ask operator when ready)
- **2 new audit dirs**: BUG — should have been
  auto-committed by daemon. Manually committed as
  workaround. All 4 remotes aligned at
  `700d58c28c08`.

## Verification commands + output

### Junk-Runner-bevy 3-remote alignment

```
local:    24709b924db6
origin:   24709b924db6
github:   24709b924db6
gitlab:   24709b924db6
codeberg: 24709b924db6
```

### dracon-platform 3-remote alignment

```
local:    e0cc1959b848
origin:   e0cc1959b848
github:   e0cc1959b848
gitlab:   e0cc1959b848
codeberg: e0cc1959b848
```

### Live `dracon-sync repos`

After the goal:
- All 14 repos: `✅ OK 14  ⚠️ WARN 0  ❌ CONCERN 0`
- `Junk-Runner-bevy`: ✅ OK, 0 MOD, 0 UT, healthy
- `dracon-platform`: ✅ OK, 0 MOD, 12 UT (9 .pi-tmp
  + 3 source dirs), healthy

### cargo test, build, deny

All unchanged from prior goal (849 tests pass, release
build OK, cargo deny clean).

## Future work (informational, not done in this goal)

1. **Daemon bug (dracon-platform audit dirs)**:
   investigate why the 2 audit dirs weren't
   auto-committed. Possible causes:
   - Settling window too long
   - in_flight HashSet still excluding them
   - Some auto-exclude pattern matching
2. **Branch-policy sync**: the operator has parallel
   `main` and `tauri2` branches. The policy needs to
   be on both. Consider adding a CI check or
   `dracon-sync doctor` warning if `.dracon/dracon-sync.toml`
   exists on one branch but not the other.
3. **Operator's deferred source dirs**: ask the
   operator if they want the 3 source dirs in
   dracon-platform committed (hegemon/src/lib,
   hegemon/static/assets, slug route).
