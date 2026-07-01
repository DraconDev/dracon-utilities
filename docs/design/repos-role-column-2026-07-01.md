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
</invoke>