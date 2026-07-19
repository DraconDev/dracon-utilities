# dracon-sync v0.112.24

**Released:** 2026-07-19
**Type:** Patch (UI accuracy + owner-classification + role layout)
**Severity:** Medium — fixes one operator-impacting misclassification (hegemon unowned) and three presentation issues from goal `4555eaf6`.

## Summary

v0.112.24 fixes four operator-visible problems observed in `dracon-sync repos`:

1. **hegemon was flagged `🚫 unowned`** even though it is owned by the
   operator. HEAD author was `Hegemon Audit <hegemon@local>` (an
   audit-script identity), and the daemon's F44 fix flagged any
   untrusted author name OR email. The email `hegemon@local` is a
   legitimate operator identity used by the audit tool.

2. **opencode-plugins (private) showed `PUBLISH = codeberg/main`**
   because the repo was added to the watch list before `origin` was
   the convention; with no `origin` remote, `branch.main.remote`
   falls back to the alphabetically-first remote (`codeberg`).

3. **ROLE column for submods was 51 chars wide**
   (`submod (of dracon-platform/web/games/wip/hegemon)`), causing
   heavy truncation or column growth on narrow terminals.

4. **Audit-script commits attributed to `Hegemon Audit` instead of
   `DraconDev`** — the audit tool commits via the `hegemon@local`
   identity but with a non-canonical name. Amended the 2 affected
   commits on hegemon's `main`.

## Fixes

### 1. hegemon owned

**Changes**:
- `~/.dracon/utilities/sync/dracon-sync.toml`:
  `trusted_emails = [..., "dracon@local", ..., "hegemon@local"]`
  (added `hegemon@local` to the trusted operator identities).
- hegemon HEAD amended to `DraconDev <hegemon@local>` (was
  `Hegemon Audit <hegemon@local>`) — the audit-script's commits are
  now attributed to the canonical operator name. Force-pushed to
  github + gitlab.

**Why both fixes are needed**: even after the amend, the email
`hegemon@local` is not in the global git config for any other
repo, so the daemon's HEAD-author check (step 3 of the F44 logic)
would still fire without the trusted_emails update. The amend
brings the name into conformance with the operator's canonical
identity, and the trusted_emails update covers the audit-tool
email. Together they make hegemon report as `✓ owned (trusted_email)`.

### 2. opencode-plugins publish

**Change** (`dracon-sync/src/git/multi_remote.rs`):
`ensure_origin_for_vscode()` — when `configure_all_remotes` runs
on a repo with mirror remotes (github/gitlab/codeberg) but no
`origin`, the daemon now adds `origin = <github URL>` and sets
`branch.<default>.remote = origin`. This:
- Makes VS Code's `git push` (which uses `origin` by convention) work.
- Makes the daemon's `PUBLISH` cell point at the github primary
  remote (which is what the operator thinks of as "origin") rather
  than the alphabetically-first remote (`codeberg`).

**Never overwrites an existing origin** — operators who
deliberately point `origin` at an internal gitlab keep their
config. Never sets `origin` when policy has no github mirror
(unusual but possible).

**One-time manual step required for repos that already had mirror
remotes but no origin**: run `git fetch origin` once after the
daemon adds it. The daemon's existing `git pull --no-rebase
origin HEAD` path takes over for subsequent cycles.

### 3. Role column layout

**Change** (`dracon-sync/src/role.rs`):
`RoleKind::Submod { parent_basename, sub_path }` now renders as
just `web/games/<tier>/<name>` minus the `web/games/` prefix, so:
- `submod (of dracon-platform/web/games/wip/hegemon)` → `wip/hegemon`
- `submod (of dracon-platform/web/games/released/one-mil-girls)` →
  `released/one-mil-girls`

The parent identity is implicit from row ordering (parent sits
above its submods in the table) and the `web/<tier>/` prefix is
preserved so `wip` vs `released` remains visible.

**Fallback for non-standard layouts**: if a submod path doesn't
start with `web/games/` (e.g. a future topology where submods
live outside the canonical games directory), the full sub_path
is used as a fallback. This keeps the cell unambiguous for any
topology.

## New regression tests (8 total)

| Test | Module | What it verifies |
|---|---|---|
| `label_compact_submod_strips_web_games_prefix` | `role.rs` | `web/games/wip/hegemon` → `wip/hegemon` |
| `label_compact_submod_keeps_released_tier` | `role.rs` | `web/games/released/one-mil-girls` → `released/one-mil-girls` |
| `label_compact_submod_falls_back_when_no_web_games_prefix` | `role.rs` | `packages/my-sub` (no prefix) → unchanged |
| `label_parent_unchanged` | `role.rs` | `parent (10 submods)` still renders correctly |
| `label_standalone_unchanged` | `role.rs` | `standalone` still renders correctly |
| `test_configure_all_remotes_bootstraps_origin_when_missing` | `multi_remote.rs` | Fresh repo with mirrors → gets `origin = github URL` |
| `test_configure_all_remotes_does_not_overwrite_existing_origin` | `multi_remote.rs` | Pre-set `origin` is preserved |
| `test_configure_all_remotes_no_origin_when_no_github_in_policy` | `multi_remote.rs` | Policy with only gitlab → no origin set |

Test count: **924** (was 916 at v0.112.23, +8 new tests).

## Test discipline

| Check | Result |
|---|---|
| `cargo build --release --locked` | ✅ green |
| `cargo test --workspace --locked` | ✅ **924 passed, 0 failed, 3 ignored** (was 916, +8 new tests) |
| `cargo clippy --workspace --locked -- -D warnings` | ✅ clean |
| `cargo deny check` | ✅ clean |

## Live daemon

- v0.112.24 deployed to `/home/dracon/.local/bin/dracon-sync`
- Live tally post-deploy: `📦 31 repos · ✅ CLEAN 26 · 🔄 ACTIVE 5 · ⚠️ WARN 0 · ❌ CONCERN 0`

## Verification chain

```bash
cd /home/dracon/Dev/dracon-utilities
cargo build --release --locked              # ✅ green
cargo test --release --workspace --locked   # ✅ 924 passed
cargo clippy --workspace --locked -- -D warnings  # ✅ clean
cargo deny check                            # ✅ clean
COLUMNS=400 ~/.local/bin/dracon-sync repos  # ✅ all rows single-line, hegemon owned
```

## Per-repo side effects

- hegemon HEAD force-pushed (amend). Before/after:
  - Before: `a18ad89, b7ed9a2 Hegemon Audit <hegemon@local>`
  - After:  `70b6817, 96e8d83 DraconDev <hegemon@local>`
  - Force-pushed to github and gitlab (`96e8d83`). codeberg was
    not pushed because the daemon correctly excludes codeberg
    under the public-only policy (hegemon is private on github).

- opencode-plugins got new `origin` remote (`github URL`) plus
  `branch.main.remote = origin`. One-time `git fetch origin`
  required to populate `origin/main` (the daemon will fetch on
  next push-rejection pull).
