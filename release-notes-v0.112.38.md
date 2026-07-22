# Release Notes — v0.112.38 (2026-07-22) — rich table default + per-repo detail

**Headline**: Operator-requested UX reshape of `dracon-sync repos`.
**825 daemon tests**, clippy + deny clean.

## The problem

The default `repos` view at <242 cols was the Vertical per-repo
block view (each repo as a ~12-line block) — too verbose for a
default. The operator wanted: a **rich table** as the default, plus
detail available **on demand** (a flag or a specific repo).

## What changed

### 1. New default: rich 6-column table

Plain `dracon-sync repos` now shows:

```
│ #  ┆ STATUS     ┆ REPO              ┆ ACTIVITY            ┆ PUSH       ┆ HINT                 │
│ 1  ┆ 🔄 ACTIVE  ┆ dracon-platform   ┆ ⏳ dirty 0m · 2 mod ┆ ✅ OK      ┆ daemon handles after…│
```

- **ACTIVITY** — the activity label with dirty counts inline
  (`⏳ dirty 1d · 101 stg + 2 ut`, `🟣 pushing 0m (1 ahead)`,
  `🟢 synced 9m`, `⚪ idle 2h`, `⚫ cold 3d`)
- **PUSH** — the dedicated push-state cell (✅ OK · 🟣 PENDING ·
  🛑 STUCK · ❌ FAIL)
- **HINT** — the actionable text
- Sorted by severity (concern → warn → active → clean)
- At ≥140 cols a **PUBLISH** column (origin/main) is added

### 2. Per-repo detail: `dracon-sync repos <name>`

`dracon-sync repos darklord` prints the full detailed block for ONE
repo: branch, publish, changes (mod/stg/ut), ahead/behind, push-to
mirrors, push state, last commit, pushed-when, activity, state,
hint. Unknown basename or ambiguity exits with code 2 and a helpful
message.

### 3. Unchanged

- `-s` / `--summary` — the 3-column glance view
- `--layout vertical` — the old per-repo block view (for ALL repos)
- `--layout compact` / `--layout full` — the 16/23-column detailed
  tables (auto-picked at 242+/315+ cols)

New `LayoutTier::Rich` variant + `print_repos_rich_table`;
`choose_layout_tier` returns Rich for <242 cols.

## Test discipline

- `cargo test --workspace --locked` ✅ **825 daemon** (2 tier tests
  updated to the new default), warden 83, security ~111, system 86
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean
- Live-verified: default table (no cell wrapping), `repos
  dracon-utilities` detail block, `-s`, `--layout vertical`
