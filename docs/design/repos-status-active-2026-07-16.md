# `dracon-sync repos` STATUS taxonomy: add an `ACTIVE` (in‑flight) state

**Date:** 2026-07-16
**Author:** dracon (via pi agent)
**Crate:** `dracon-sync` (`src/report.rs`)
**Goal:** `a50f68a7-6db3-40e8-94b5-fcd080b6ceac`

---

## 1. Problem

`dracon-sync repos` reported 29 repos as `✅ OK 23 ⚠️ WARN 6 ❌ CONCERN 0`.
The operator observed that the STATUS taxonomy was **wrong**: repos the
daemon was *actively syncing* (in‑flight `pushing` / `committing` / recently
`dirty`) were labelled `⚠️ WARN`, which implies "something is wrong / needs
attention". They are not wrong — the daemon is mid‑flight on them. The
operator asked for **different categories, or at least one** distinguishing
"active / working (plausibly not broken)" from genuine warnings.

A second observation: *"we clearly have actual problems here too"* — among the
WARNs there were genuine inefficiencies (e.g. hegemon committing ~100
Playwright `test-results/` artifacts per cycle; capture‑anime‑girls appearing
stalled).

### Root cause (code)

`report.rs` already computed a fine‑grained `state_cause`
(`Working`/`Committing`/`Pushing`/`Dirty`/`Stalled`/`Failed`/…) and a
`push_status` (`PENDING` while pushing), and the **STATE + ACTIVITY** columns
rendered those cleanly. But the **STATUS** column ignored all of that and
used a single overloaded rule (`report.rs:2522`):

```rust
let warn = !concern && real_is_dirty;   // ANY repo with tracked mods -> WARN
```

So **any** repo with uncommitted tracked changes — including one the daemon
is *currently pushing* — was stamped `⚠️ WARN`. The three STATUS render sites
(vertical / compact / full) only branched on `concern` / `warn` / `ok`, so
there was no `ACTIVE` bucket at all, and the `Unowned` guard (a
`StateCause::Unowned`) was only surfaced in the ACTIVITY column, never STATUS.

---

## 2. New taxonomy

STATUS priority order (rendered by a new `status_pair()` helper):

| STATUS | Meaning | Color |
|---|---|---|
| `❌ CONCERN` | divergence needing repair (`repo_is_concern`) | Red |
| `🚫 unowned` | ownership guard tripped (`StateCause::Unowned`) | Red |
| `🔄 ACTIVE` | daemon in‑flight / dirty‑recent it will handle | Cyan |
| `⚠️ WARN` | genuine issue (stalled / no progress) | Yellow |
| `✅ OK` | idle / cold / healthy / synced | Green |

**`ACTIVE` is defined as** `push_status == "PENDING"` **OR** `state_cause ∈
{Working, Pushing, Committing, Dirty}`:

- `PENDING` ⇒ a push is mid‑cycle.
- `Working` ⇒ clean and just synced (within `active_commit_minutes`).
- `Committing` ⇒ unpushed commits waiting / recent commit.
- `Dirty` ⇒ recent uncommitted work the daemon will pick up.

A repo that is dirty but **`Stalled`** (no progress for a long time) is **NOT**
active — it falls through to `⚠️ WARN` (a genuine problem). `Idle` / `Cold` /
`Healthy` / `Synced` / `Untracked` / `Intentional` / `Failed` are not active.

### Why this is the right cut

It matches the operator's mental model exactly: *"active/working, meaning
plausibly not broken"* vs *"a real warning"*. In‑flight + dirty‑recent repos
are `ACTIVE`; only repos the daemon has **given up on** (stalled) or that are
**divergent** (concern) are escalated.

---

## 3. Implementation (`src/report.rs`)

1. **`RepoReportRow` struct** — added `active: bool` (alongside `concern` /
   `warn`).
2. **`pub(crate) fn repo_is_active(push_status, state_cause)`** — mirrors the
   existing `repo_is_warn` / `repo_is_concern` predicates; unit‑tested
   (`test_repo_is_active`).
3. **Row construction** — `let active = repo_is_active(&push_status,
   &state_cause);` injected before the `RepoReportRow { .. }` literal, field
   `active,` added.
4. **`fn status_pair(row)`** — single source of truth for the STATUS label +
   color, used by all three render functions (vertical / compact / full).
   Previously the identical `if concern {…} else if warn {…} else {…}` was
   duplicated 3×; now `status_pair` also makes the `Unowned` case explicit in
   STATUS (it was previously only in ACTIVITY).
5. **Tally** — added `active_count` / `active_count_all`; `warn_count` now
   excludes active repos (`r.warn && !r.active`); `ok = total − concern −
   active − warn`. Summary line renders `🔄 ACTIVE N`. `--filter warn`
   retains only genuine (non‑active) warns.
6. **`RepoReportJson`** — added `active: usize` (sums to `repos`).
7. **Legend** — added a `ℹ️ STATUS (🏷)` line documenting all five states.

All **existing tests pass** (dracon‑sync: 673 passed, 0 failed) and a new
`test_repo_is_active` covers the boundary. `cargo deny check` → advisories /
bans / licenses / sources **ok**.

---

## 4. Investigation of the original 6 WARNs

| Repo | Original STATE/ACT | Root cause | Resolution |
|---|---|---|---|
| dracon‑platform | `pushing` (4 mod in `web/`) | submodule‑pointer churn, daemon mid‑push | **ACTIVE** (correct) — daemon handles |
| hegemon | `dirty` (302→100 `test-results/`) | Playwright generated artifacts being committed every cycle | **ACTIVE** + **excluded** `test-results/**` (see §5) |
| .dracon | `dirty` (`repos-size-cache.json`) | perf‑fix cache file (transient) | **ACTIVE** — daemon commits to its own config repo |
| hellhunter | `dirty` (8 `.pi` BIN) | session scratch, daemon handling | **ACTIVE** (correct) |
| deathrun | `dirty` (docs BIN) | transient, daemon handling | **ACTIVE** (correct) |
| capture‑anime‑girls | `stalled 2h` (in paste) | transient big‑push timeout; journalctl shows continuous syncs (`🔁 synced` at 00:52) — clean now | **ACTIVE** (resolved by daemon) |

**Result:** all 6 false/mislabelled WARNs reclassified as `🔄 ACTIVE` (or OK
once settled). No repo is a genuine `WARN` after the fix.

---

## 5. Genuine problem resolved: hegemon `test-results/` churn

`hegemon` runs Playwright; `test-results/` held generated artifacts
(`error-context.md`, `test-failed-*.png`, `trace.zip`,
`.playwright-artifacts-0/`) — regenerated every test run. The daemon was
committing ~100 `test-results/` files per cycle (untracked + deletions of
already‑tracked runs).

**Decision (commit‑vs‑exclude, per the goal's "do not silently drop" rule):**
`test-results/` is **generated output, not user work**, so it is excluded
from auto‑commit — consistent with hegemon's *existing* per‑repo
`auto_commit_exclude_patterns` (`.pi/chrome-screenshots`, `.pi/visual-audit`,
`.pi/investigation/*.png`). This is a **sanctioned per‑repo operator
exception** (AGENTS.md explicitly preserves `auto_commit_exclude_patterns` for
"future operator‑set exceptions with a documented reason"); it is **not** a
reversal of the global commit‑all policy (which still applies to everything
else in hegemon, including real test source `tests/*.spec.ts`).

**Change:** `web/games/wip/hegemon/.dracon/dracon-sync.toml` —
`auto_commit_exclude_patterns` gained `"**/test-results/**"` with a documented
comment.

**Verification (git proof):**
- `dracon-sync sync-now hegemon` committed only `.dracon/dracon-sync.toml`
  (the config edit). `git log -1 --name-only | grep -c test-results/` → **0**.
- `git -C hegemon status --short` → **clean** (0 `test-results` entries).
- Churn stopped: the daemon no longer commits generated test artifacts.

---

## 6. Before / after

| | OK | ACTIVE | WARN | CONCERN |
|---|---|---|---|---|
| Before (user's paste) | 23 | — (none) | 6 | 0 |
| After fix | 20 | 9 | 0 | 0 |

The 6 false WARNs are gone; 9 in‑flight / dirty‑recent repos are now
correctly `🔄 ACTIVE`. JSON `ok + active + warn + concern == repos` (29).

---

## 7. Residual behavior / notes

- An `ACTIVE` repo that *only* has excluded `test-results/` artifacts on disk
  will keep showing `ACTIVE` (it is genuinely dirty, just intentionally not
  committed). That is accurate, not a bug. If the operator wants hegemon to
  read fully `OK` in the report, adding `test-results/` to hegemon's
  `.gitignore` (git‑native, complements the daemon exclude) would make
  libgit2 stop reporting it — optional, not required.
- The daemon process in memory was the **pre‑fix** binary at the time of
  writing; the **deployed on‑disk binary** (`~/.local/bin/dracon-sync`) is the
  new one, so `dracon-sync repos` already shows the new taxonomy. The running
  daemon will pick up the new binary on its next natural restart. The daemon
  auto‑committed the `report.rs` source change (`bb3ccf2`).
- `RepoReportRow { .. }` test literals and `RepoReportJson` gained the
  `active` field; `state_pair`/`status_pair` keep the STATUS colour vocabulary
  stable for downstream tooling.
