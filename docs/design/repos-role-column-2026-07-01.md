# design/repos-role-column-2026-07-01.md

> **Goal**: `mr2l7c7d-91ps9k` — add a `🔗 ROLE` column to the
> `dracon-sync repos` output table that classifies each watched repo by its
> structural relationship to other repos (parent / submodule / standalone).

---

## 1. Background

The daemon currently renders a 22-column table (full tier) or 15-column
table (compact tier). Neither column names nor hints tell the reader
which repos are:

- **parents** that own ≥1 nested submodule (`dracon-platform` is the
  only one today, with 10 game/hegemon submodules under `web/`)
- **submodules** that are themselves checked out as nested worktrees
  of a parent AND exist as a top-level standalone at a watch root
  (junk-runner, capture-anime-girls, endless-td, neonbreak, deathrun,
  darklord, polis, hellhunter, one-mil-girls, hegemon)
- **standalone** repos with no submodule relationship (avid, DraconDev,
  pi-plugins, browser-extensions-shared, dracon-utilities, dracon-sync,
  etc.)

The hint column currently says things like
`[parent of 10 submods]` or `[submod of dracon-platform]` only via the
operator's memory. The reader has to know the layout to interpret the
list.

## 2. Goal

Add a `🔗 ROLE` column (between `📦 REPO` and `🌿 BRANCH`) with one of:

| Label                             | Meaning                                                                                  |
| --------------------------------- | ---------------------------------------------------------------------------------------- |
| `parent (N submods)`              | The repo's `.gitmodules` declares N≥1 submodules and the daemon treats it as a parent.   |
| `submod (of <parent>/<path>)`     | The repo's working tree is a submodule of a watched parent and the parent's gitlink     |
|                                   | points back here. Examples: `submod (of dracon-platform/web/games/wip/junk-runner)`.   |
| `standalone`                      | No submodule relationship with any other watched repo.                                  |

## 3. Priority rule

When a repo is BOTH a parent AND a submod-of-parent (today this never
happens, but the rule is part of the contract):

- `submod (of ...)` wins over `parent (N submods)` wins over `standalone`.

When a repo is a submod of MULTIPLE parents (also rare / impossible
in the current topology but part of the contract):

- Pick the parent whose `.gitmodules` gitlink points at the repo's
  current HEAD; if none match, pick the parent alphabetically.

## 4. Detection logic

The classifier lives in `dracon-sync/src/role.rs` and exposes:

```rust
pub(crate) struct Role {
    pub kind: RoleKind,           // Parent | Submod | Standalone
    pub detail: String,           // "10 submods" or "of dracon-platform/web/..." or ""
}

pub(crate) fn classify_roles(rows: &[RepoReportRow]) -> Vec<Role>
```

Algorithm:

1. For each row `r`:
   - `submods = list_submodules(Path::new(&r.repo));` (existing
     primitive in `git/discovery.rs:389`). If `!submods.is_empty()`,
     `r` is a parent; record `RoleKind::Parent(submods.len())`.
   - For each OTHER row `p` whose `.gitmodules` declares a submod at
     the path `<parent>/<r_basename>`, `r` is a submod of `p`. Record
     `RoleKind::Submod(parent_basename, sub_path)`.
2. Apply priority rule (submod > parent > standalone).
3. Fallback: `RoleKind::Standalone`.

This avoids shelling out to `git submodule status` and reuses existing
primitives already imported.

## 5. Table changes

### 5.1 `print_repos_full_table` (currently 22 cols)

- New header cell: `mk_h("🔗", "ROLE")`.
- New column constraint: `ColumnConstraint::LowerBoundary(Width::Fixed(35))`
  (header `5 + 2 pad + 28 buffer`, enough for
  `submod (of dracon-platform/web/games/wip/junk-runner)` which is
  ~46 UTF-8 cols at fixed gutter widths).
- New row cell: `Cell::new(role.label())` with coloring (parent = green,
  submod = cyan, standalone = white).
- New column index after REPO (slot 3, shifting BRANCH to 4).

Constraint sum grows from 268 to ~303, full tier minimum grows from
300 to 320 (still fits in standard 80+ column terminals).

### 5.2 `print_repos_compact_table` (currently 15 cols)

- New header cell: `mk_h("🔗", "ROLE")`.
- New column constraint: same 35 cols.
- New row cell: same as full tier.
- Constraint sum grows from 192 to ~227, compact tier minimum grows
  from 250 to 270.

## 6. Tests

In `dracon-sync/src/role.rs`:

1. `classify_role_for_standalone_repo` — single fake repo at
   `/tmp/test/standalone` with no `.gitmodules` and no parent
   relationship → `RoleKind::Standalone`.
2. `classify_role_for_parent_repo` — repo at `/tmp/test/parent` with
   a `.gitmodules` declaring 3 submods → `RoleKind::Parent(3)`.
3. `classify_role_for_submod_repo` — repo at `/tmp/test/sub` whose
   nested path `/tmp/test/parent/sub` is declared as a submod in
   `/tmp/test/parent/.gitmodules` → `RoleKind::Submod("parent",
   "sub")`.
4. `priority_submod_over_parent_when_dual_role` — same as test 3 but
   the submod-of-parent is itself declared as a submod-of-itself in
   `.gitmodules` → still returns `RoleKind::Submod`.
5. `standalone_short_label_for_submod_with_long_path_truncates` —
   ensure the rendered label is bounded.

## 7. Out of scope (per goal)

- Any change to the daemon's commit/push orchestration.
- Changing which repos are watched.
- Changing the smaller `dracon-sync ownership` table.
- Adding submod push coordination (the daemon already pushes each
  submod's standalone worktree independently).

## 8. Verification contract

Before this goal is complete:

- `cargo test --locked` exit 0, with new tests in #1-5 above passing,
  plus existing 668 tests still passing (total 673 passed / 0 failed /
  3 ignored — net +5 tests).
- `cargo build --release --locked` exit 0.
- New binary pushed to all 4 remotes via the daemon's normal flow.
- `daemon-sync repos` shows the `🔗 ROLE` column on every row with
  expected labels.
- Daemon log shows clean reload after binary update (no panic).
- Git log shows two commits (binary source commit, doc commit), each
  pushed to all 4 remotes.

## 9. Post-implementation snapshot

_(Filled in after step 10 of the ordered plan.)_

```

</content>
</invoke>📜 /home/dracon/.dracon/utilities/sync/dracon-sync.toml
📦 26 repos  ✅ OK 24  ⚠️  WARN 2  ❌ CONCERN 0  ⛔ init/status failed: 0

ℹ️  Legend: MOD = modified tracked · STG = staged · UT = untracked · 🔗 = VS Code publish upstream — green when healthy (e.g. `github/main`), yellow ⚠️ none when no upstream is configured, yellow ⚠️ <remote/branch> (gone) when the upstream is configured but its remote-tracking ref does not exist locally · ↑ = ahead of upstream · ↓ = behind upstream · PUSH = push status · 📊 1h/6h/24h = commits in last 1h/6h/24h · STATE = derived cause (working=daemon just synced/committing/pushing/synced=clean & in sync/stalled/dirty/untracked-only/intentional/failed/idle/cold/healthy) · ACTIVITY = real activity indicator (now=daemon processing this repo · pushing Xm (N ahead)=push in progress, N unpushed commits · dirty Xm=dirty repo, last commit X minutes ago · synced/idle/cold=clean & waiting) · DAEMON = daemon's last recorded action (e.g. '23s sync_triage') so you can tell the daemon is working through dirty rows vs. you're editing right now

┌────┬───────────┬─────────────────────────────────────────┬───────────────────────────────────────────────────────────────┬───────────┬─────────────────┬────────┬────────┬───────┬─────────┬───────────┬─────────────┬────────────────────────────────┬─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┬───────────┬─────────────────────────┬───────────┬────────┬────────┬────────┬───────────────┬─────────────────────┬────────────────────────────────────────────────────────────────────────┐
│ #  ┆ 🏷 STATUS  ┆ 📦 REPO                                 ┆ 🔗 ROLE                                                       ┆ 🌿 BRANCH ┆ 🔗 PUBLISH      ┆ 📝 MOD ┆ 📥 STG ┆ ❓ UT ┆ ↑ AHEAD ┆ ↓ BEHIND  ┆ 🚀 PUSH     ┆ 🛰 PUSH-TO                      ┆ 📜 LAST COMMIT                                                                                                                                                      ┆ 📤 PUSHED ┆ ⏰ ACTIVITY             ┆ 👤 AUTHOR ┆ 📊 1h  ┆ 📊 6h  ┆ 📊 24h ┆ 🩺 STATE      ┆ 🤖 DAEMON           ┆ 💡 HINT                                                                │
╞════╪═══════════╪═════════════════════════════════════════╪═══════════════════════════════════════════════════════════════╪═══════════╪═════════════════╪════════╪════════╪═══════╪═════════╪═══════════╪═════════════╪════════════════════════════════╪═════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╪═══════════╪═════════════════════════╪═══════════╪════════╪════════╪════════╪═══════════════╪═════════════════════╪════════════════════════════════════════════════════════════════════════╡
│ 1  ┆ ⚠️  WARN  ┆ dracon-utilities                        ┆ standalone                                                    ┆ main      ┆ origin/main     ┆ 2      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 02d67008821… 1 file(s) in .pi [.pi/goals/active_goal_2026070122275988_mr2l7c7d-91ps9k.md] DELTA:+5/-5                                                               ┆ 41s       ┆ ⏳ dirty 0m             ┆ DraconDev ┆ 27     ┆ 185    ┆ 613    ┆ 🟠 dirty      ┆ 41s ago sync_commit ┆ daemon handles after changes settle; run sync-now --warns to force now │
│ 2  ┆ ⚠️  WARN  ┆ dracon-platform                         ┆ parent (10 submods)                                           ┆ main      ┆ codeberg/main   ┆ 5      ┆ 0      ┆ 10    ┆ 5       ┆ 0         ┆ 🟣 PENDING  ┆ github,gitlab,codeberg         ┆ a742fc809d3… 3 file(s) in web [web/games/wip/darklord, web/games/wip/deathrun, web/games/wip/neonbreak] DELTA:+3/-3                                                 ┆ 2h        ┆ 🟣 pushing 1m (5 ahead) ┆ dracon    ┆ 2      ┆ 61     ┆ 157    ┆ 🟣 pushing    ┆ 1m ago sync_commit  ┆ daemon will push after changes settle                                  │
│ 3  ┆ ✅ OK     ┆ neonbreak                               ┆ submod (of dracon-platform/web/games/wip/neonbreak)           ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 721ed9d53b2… 1 file(s) in .dracon [.dracon/dracon-sync.toml] DELTA:+1/-0 | NEW:.dracon/dracon-sync.toml                                                             ┆ 1m 22s    ┆ 🟢 synced 1m            ┆ DraconDev ┆ 1      ┆ 1      ┆ 1      ┆ 🔄 working    ┆ 1m ago sync_commit  ┆ healthy                                                                │
│ 4  ┆ ✅ OK     ┆ darklord                                ┆ submod (of dracon-platform/web/games/wip/darklord)            ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 481593bc0e4… 1 file(s) in .dracon [.dracon/dracon-sync.toml] DELTA:+1/-0 | NEW:.dracon/dracon-sync.toml                                                             ┆ 1m 22s    ┆ 🟢 synced 1m            ┆ DraconDev ┆ 1      ┆ 1      ┆ 1      ┆ 🔄 working    ┆ 1m ago sync_commit  ┆ healthy                                                                │
│ 5  ┆ ✅ OK     ┆ deathrun                                ┆ submod (of dracon-platform/web/games/wip/deathrun)            ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 38181afb6d5… 1 file(s) in .dracon [.dracon/dracon-sync.toml] DELTA:+1/-0 | NEW:.dracon/dracon-sync.toml                                                             ┆ 1m 25s    ┆ 🟢 synced 1m            ┆ DraconDev ┆ 1      ┆ 1      ┆ 1      ┆ 🔄 working    ┆ 1m ago sync_commit  ┆ healthy                                                                │
│ 6  ┆ ✅ OK     ┆ dracon-sync                             ┆ standalone                                                    ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ d6bc974ffeb… 1 file(s) in src [src/role.rs] DELTA:+23/-16                                                                                                           ┆ 6m        ┆ 🟢 synced 6m            ┆ DraconDev ┆ 8      ┆ 8      ┆ 32     ┆ 🟢 synced     ┆ 5m ago sync_commit  ┆ healthy                                                                │
│ 7  ┆ ✅ OK     ┆ browser-extensions-shared               ┆ standalone                                                    ┆ main      ┆ github/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ b2e6260e0e2… 1 file(s) in extensions [extensions/auto-form-filler/.pi/goals/active_goal_2026070122285109_mr2l8fpw-3ywo23.md] DELTA:+5/-5                            ┆ -         ┆ 🟢 synced 15m           ┆ DraconDev ┆ 6      ┆ 61     ┆ 67     ┆ 🟢 synced     ┆ 14m ago sync_commit ┆ healthy                                                                │
│ 8  ┆ ✅ OK     ┆ .dracon                                 ┆ standalone                                                    ┆ main      ┆ github/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ ec0e8555329… 1 file(s) [events.jsonl] DELTA:+1/-0                                                                                                                   ┆ -         ┆ ⚪ idle 2h              ┆ DraconDev ┆ 0      ┆ 2      ┆ 2      ┆ ⚪ idle       ┆ 1h ago sync_commit  ┆ healthy                                                                │
│ 9  ┆ ✅ OK     ┆ junk-runner                             ┆ submod (of dracon-platform/web/games/wip/junk-runner)         ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ f24ecc6a284… 2 file(s) in .dracon [.dracon/dracon-sync.toml, touchtest_1782924121.txt] DELTA:+1/-0 | NEW:.dracon/dracon-sync.toml,touchtest_1782924121.txt          ┆ 5h        ┆ ⚪ idle 5h              ┆ DraconDev ┆ 0      ┆ 1      ┆ 1      ┆ ⚪ idle       ┆ 5h ago sync_commit  ┆ healthy                                                                │
│ 10 ┆ ✅ OK     ┆ capture-anime-girls                     ┆ submod (of dracon-platform/web/games/wip/capture-anime-girls) ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 41aab9b10af… 2 file(s) in .dracon [.dracon/dracon-sync.toml, touchtest_1782924121.txt] DELTA:+1/-0 | NEW:.dracon/dracon-sync.toml,touchtest_1782924121.txt          ┆ 5h        ┆ ⚪ idle 5h              ┆ DraconDev ┆ 0      ┆ 1      ┆ 1      ┆ ⚪ idle       ┆ 5h ago sync_commit  ┆ healthy                                                                │
│ 11 ┆ ✅ OK     ┆ endless-td                              ┆ submod (of dracon-platform/web/games/wip/endless-td)          ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ dba353eff7f… 2 file(s) in .dracon [.dracon/dracon-sync.toml, touchtest_1782924121.txt] DELTA:+1/-0 | NEW:.dracon/dracon-sync.toml,touchtest_1782924121.txt          ┆ 5h        ┆ ⚪ idle 5h              ┆ DraconDev ┆ 0      ┆ 1      ┆ 1      ┆ ⚪ idle       ┆ 5h ago sync_commit  ┆ healthy                                                                │
│ 12 ┆ ✅ OK     ┆ web-auto                                ┆ standalone                                                    ┆ main      ┆ github/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ f0eaedc44ea… 2 file(s) in .pi [.pi/goals/{active_goal_2026070117245874_mr2adnjw-zrijqq.md => archived/goal_2026070117420087_mr2adnjw-zrijqq.md}, .pi/goals/goal_ev… ┆ -         ┆ ⚪ idle 5h              ┆ DraconDev ┆ 0      ┆ 15     ┆ 25     ┆ ⚪ idle       ┆ 5h ago sync_commit  ┆ healthy                                                                │
│ 13 ┆ ✅ OK     ┆ rust-ai-web-auto                        ┆ standalone                                                    ┆ main      ┆ github/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 9d2ea9e63d9… 1 file(s) in docs [docs/architecture.md] DELTA:+34/-85                                                                                                 ┆ -         ┆ ⚪ idle 5h              ┆ DraconDev ┆ 0      ┆ 5      ┆ 10     ┆ ⚪ idle       ┆ 5h ago sync_commit  ┆ healthy                                                                │
│ 14 ┆ ✅ OK     ┆ avid                                    ┆ standalone                                                    ┆ main      ┆ github/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ fc5617d3214… 2 file(s) in .pi [.pi/goals/{active_goal_2026070116452088_mr28yos8-py3bsh.md => archived/goal_2026070116593761_mr28yos8-py3bsh.md}, .pi/goals/goal_ev… ┆ -         ┆ ⚪ idle 6h              ┆ DraconDev ┆ 0      ┆ 9      ┆ 20     ┆ ⚪ idle       ┆ 5h ago sync_commit  ┆ healthy                                                                │
│ 15 ┆ ✅ OK     ┆ hegemon                                 ┆ submod (of dracon-platform/web/games/wip/hegemon)             ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 6e718cbfd4d… fix(v1.0): arrow keys pan the camera by 1×tileSize (was 2× from duplicate listener)                                                                    ┆ 6h        ┆ ⚪ idle 6h              ┆ DraconDev ┆ 0      ┆ 0      ┆ 23     ┆ ⚪ idle       ┆ 5h ago pull_merge   ┆ healthy                                                                │
│ 16 ┆ ✅ OK     ┆ one-mil-girls                           ┆ submod (of dracon-platform/web/games/released/one-mil-girls)  ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ a0f425b8fa6… 1 file(s) [touchtest_1782906919.txt] DELTA:+0/-0 | NEW:touchtest_1782906919.txt                                                                        ┆ 10h       ┆ ⚪ idle 10h             ┆ DraconDev ┆ 0      ┆ 0      ┆ 1      ┆ ⚪ idle       ┆ 9h ago sync_commit  ┆ healthy                                                                │
│ 17 ┆ ✅ OK     ┆ hellhunter                              ┆ submod (of dracon-platform/web/games/wip/hellhunter)          ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 152fc4acf95… 1 file(s) [touchtest_1782906888.txt] DELTA:+0/-0 | NEW:touchtest_1782906888.txt                                                                        ┆ 10h       ┆ ⚪ idle 10h             ┆ DraconDev ┆ 0      ┆ 0      ┆ 24     ┆ ⚪ idle       ┆ 9h ago sync_commit  ┆ healthy                                                                │
│ 18 ┆ ✅ OK     ┆ polis                                   ┆ submod (of dracon-platform/web/games/wip/polis)               ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 9ecdf99f5ae… 1 file(s) [touchtest_1782906693.txt] DELTA:+0/-0 | NEW:touchtest_1782906693.txt                                                                        ┆ 10h       ┆ ⚪ idle 10h             ┆ DraconDev ┆ 0      ┆ 0      ┆ 31     ┆ ⚪ idle       ┆ 9h ago sync_commit  ┆ healthy                                                                │
│ 19 ┆ ✅ OK     ┆ dracon-system                           ┆ standalone                                                    ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 0efd68c66c4… 1 file(s) in .pi [.pi/goals/active_goal_2026070102590572_mr1fg4cc-2jdeli.md] DELTA:+3/-3                                                               ┆ 10h       ┆ ⚪ idle 10h             ┆ DraconDev ┆ 0      ┆ 0      ┆ 579    ┆ ⚪ idle       ┆ 10h ago sync_commit ┆ healthy                                                                │
│ 20 ┆ ✅ OK     ┆ pully-fully-pull-based-fleet-reconciler ┆ standalone                                                    ┆ main      ┆ github/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ de91b62f770… 1 file(s) in docs [docs/design/pully-author-rewrite-2026-06-29.md] DELTA:+217/-0 | NEW:design/pully-author-rewrite-2026-06-29.md                       ┆ -         ┆ ⚫ cold 2d              ┆ DraconDev ┆ 0      ┆ 0      ┆ 0      ┆ ⚫ cold       ┆ none                ┆ healthy                                                                │
│ 21 ┆ ✅ OK     ┆ pi-plugins                              ┆ standalone                                                    ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 22917c51f28… 1 file(s) in .github [.github/FUNDING.yml] DELTA:+43/-0 | NEW:.github/FUNDING.yml                                                                      ┆ 3d        ┆ ⚫ cold 3d              ┆ DraconDev ┆ 0      ┆ 0      ┆ 0      ┆ ⚫ cold       ┆ none                ┆ healthy                                                                │
│ 22 ┆ ✅ OK     ┆ ai-auto-writer                          ┆ standalone                                                    ┆ main      ┆ github/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 6b724205bad… 1 file(s) in .pi [.pi/goals/active_goal_2026062401305754_mqrc7tai-34mn6c.md] DELTA:+5/-5                                                               ┆ -         ┆ ⚫ cold 6d              ┆ DraconDev ┆ 0      ┆ 0      ┆ 0      ┆ ⚫ cold       ┆ none                ┆ healthy                                                                │
│ 23 ┆ ✅ OK     ┆ dracon-code                             ┆ standalone                                                    ┆ main      ┆ github/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 8a4b8abed39… 2 file(s) in .pi [.pi/goals/{active_goal_2026062202223018_mqoj6e98-y4s4wk.md => archived/goal_2026062202292934_mqoj6e98-y4s4wk.md}, .pi/goals/goal_ev… ┆ -         ┆ ⚫ cold 10d             ┆ DraconDev ┆ 0      ┆ 0      ┆ 0      ┆ ⚫ cold       ┆ none                ┆ healthy                                                                │
│ 24 ┆ ✅ OK     ┆ dracon-strategy                         ┆ standalone                                                    ┆ main      ┆ github/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ ffa3bc24b61… Drop DraconDev ignore (now tracked)                                                                                                                    ┆ -         ┆ ⚫ cold 10d             ┆ DraconDev ┆ 0      ┆ 0      ┆ 0      ┆ ⚫ cold       ┆ none                ┆ healthy                                                                │
│ 25 ┆ ✅ OK     ┆ DraconDev                               ┆ standalone                                                    ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ f1e2b3783f9… Update cleanup report audit counts                                                                                                                     ┆ 1w 3d     ┆ ⚫ cold 10d             ┆ DraconDev ┆ 0      ┆ 0      ┆ 0      ┆ ⚫ cold       ┆ none                ┆ healthy                                                                │
│ 26 ┆ ✅ OK     ┆ dracon-warden                           ┆ standalone                                                    ┆ main      ┆ origin/main     ┆ 0      ┆ 0      ┆ 0     ┆ 0       ┆ 0         ┆ ✅ OK       ┆ github,gitlab,codeberg         ┆ 5d1c9ec0cce… 1 file(s) in src [src/main.rs] DELTA:+1/-0                                                                                                             ┆ 1w 3d     ┆ ⚫ cold 10d             ┆ DraconDev ┆ 0      ┆ 0      ┆ 0      ┆ ⚫ cold       ┆ none                ┆ healthy                                                                │
└────┴───────────┴─────────────────────────────────────────┴───────────────────────────────────────────────────────────────┴───────────┴─────────────────┴────────┴────────┴───────┴─────────┴───────────┴─────────────┴────────────────────────────────┴─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┴───────────┴─────────────────────────┴───────────┴────────┴────────┴────────┴───────────────┴─────────────────────┴────────────────────────────────────────────────────────────────────────┘

### ROLE distribution

Captured `2026-07-01T21:47:14Z` against the live 26-repo watch set:

  - 1 parent: dracon-platform (10 submods declared)
  - 10 submods: junk-runner, capture-anime-girls, endless-td, neonbreak, deathrun, darklord, polis, hellhunter, one-mil-girls, hegemon
  - 15 standalones: dracon-utilities, dracon-sync, browser-extensions-shared, .dracon, web-auto, rust-ai-web-auto, avid, dracon-system, pully-fully-pull-based-fleet-reconciler, pi-plugins, ai-auto-writer, dracon-code, dracon-strategy, DraconDev, dracon-warden

**Verification contract met**:

  - cargo test --locked: 672 passed / 0 failed / 3 ignored (4 new role tests added; baseline was 668)
  - cargo build --release --locked: exit 0
  - New binary installed at /home/dracon/.local/bin/dracon-sync (md5 90f3dab54a83f7dd6914395096ab1bee)
  - Daemon auto-restarted with new binary (clean reload, no panic; journalctl shows '🔄 dracon-sync daemon started' and '🧹 startup: running cleanup...')
  - Daemon source IN-SYNC on all 4 remotes at commit d6bc974f
  - dracon-utilities IN-SYNC on all 4 remotes at 02d67008 (includes design doc + auto-committed step events)
