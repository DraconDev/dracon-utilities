# `dracon-sync repos` STATUS label: rename `OK` → `CLEAN`

**Date:** 2026-07-17
**Author:** dracon (via pi agent)
**Crate:** `dracon-sync` (`src/report.rs`)
**Goal:** `d861d0ad-5083-467b-a62a-619d7a48f7ab`

---

## 1. Decision

The operator observed the `✅ OK` STATUS label "might" warrant renaming but
was unsure to what (and noted "it's not a bad name"). After scoping every
`OK` occurrence and checking for JSON consumers, candidate labels
(`CLEAN` / `SYNCED` / `HEALTHY` / `IDLE`) were presented via
`ask_user_question`. **The operator chose `CLEAN`.**

Result: the human‑facing STATUS label `✅ OK` is now **`✅ CLEAN`** across the
vertical / compact / full table layouts, the `📦 N repos` tally, and
`--legend`. Semantics unchanged: idle/cold + healthy + fully‑synced/clean.

---

## 2. Scope (what changed vs preserved)

### Changed (the STATUS label — 3 sites in `src/report.rs`)
1. `status_pair()` final `else` branch (line ~3009):
   `("✅ OK", Color::Green)` → `("✅ CLEAN", Color::Green)`.
2. Summary tally (line ~2944):
   `ansi("32", &format!("✅ OK {ok_count}"))` → `"✅ CLEAN {ok_count}"`.
3. `--legend` STATUS line (line ~2243):
   `✅ OK = idle/cold + healthy + synced` → `✅ CLEAN = idle/cold + healthy + synced`.

### Preserved (different semantics — deliberately NOT renamed)
- **PUSH‑column `✅ OK`** (`push_cell_label`, line ~3747:
  `"OK" => ("✅ OK", Color::Green)`). This is the *push* status (all
  PUSH‑TO remotes synced), a distinct concept from repo STATUS. Left as `OK`.
- **JSON `ok` key** (`RepoReportJson.ok: usize`, payload `ok: ok_count`).
  This is a **machine‑parseable token**; the prior STATUS‑taxonomy goal
  required STATUS tokens to stay stable for external consumers. A consumer
  check found **no** external parser of `dracon-sync repos --json`'s `ok`
  field (the `"ok"` strings in `dracon-system` are unrelated disk/link
  states). Kept as `ok` by default.
- **`state_flags` "OK"** (line ~1373: pushed when a repo has no special
  flags). This flag is **never rendered** in any of the three table layouts
  (verified — `.state_flags` has no render site) and is a different concept
  (per‑repo flag, not STATUS). Left unchanged.

---

## 3. Why `CLEAN`

The STATUS `OK` trigger is literally *a clean working tree + fully synced with
all remotes + no divergence + daemon not currently syncing*. `CLEAN` names
that condition precisely and reads unambiguously as "nothing to do here",
distinct from `ACTIVE` (daemon in‑flight) / `WARN` (genuine issue) /
`CONCERN` (divergence). Column width `Fixed(11)` comfortably fits
`✅ CLEAN` (8 cols).

---

## 4. Verification

- `cargo build --release --locked` → clean.
- `cargo deny check` → advisories / bans / licenses / sources **ok**.
- `cargo test --workspace --locked` → **673 passed, 0 failed** (no STATUS
  display test asserted the old string; existing `push_cell_label`/`flags`/
  `push_status: "OK"` tests still pass unchanged).
- `dracon-sync repos` tally: `✅ CLEAN N 🔄 ACTIVE M ⚠️ WARN 0 ❌ CONCERN K`
  (e.g. `✅ CLEAN 22 🔄 ACTIVE 5 ⚠️ WARN 0 ❌ CONCERN 2`).
- STATUS column renders `✅ CLEAN` for clean repos; `✅ OK` no longer appears
  in the STATUS column (PUSH‑column `✅ OK` is intact in its own column).
- `dracon-sync repos --legend` STATUS line documents `✅ CLEAN`.
- `dracon-sync repos --json` still emits `"ok": N` (machine token preserved);
  `ok + active + warn + concern == repos`.
- Daemon healthy; it auto‑committed the `report.rs` source change.

---

## 5. Residual notes

- The `state_flags` "OK" flag remains in the data model but is unused by the
  renderers; out of scope for this label change.
- If the operator later wants the JSON `ok` key renamed for symmetry, that is
  a separate, explicitly‑approved change (would require a consumer‑impact
  recheck). Not done here.
