# Nested standalone re-migration (2026-09-11)

Operator decision: `dracon-utilities` is a parent repo; `dracon-sync/`,
`dracon-system/`, `dracon-warden/` are **independent nested git repos**
(own `.git`, remotes, history, tags) — explicitly **no submodules**.
This reverses the 2026-08-22 subtree-monorepo conversion (which itself
had replaced one day of submodules). Q&A answers: independent nested
repos / subtree-split history / all forges live / all three at once.

## Method (no force-push anywhere)

For each utility: base = its GitHub mirror tip (proven to contain the
codeberg and gitlab tips via `merge-base --is-ancestor`, so every push
is fast-forward):

| utility | base (github tip) | codeberg tip | gitlab tip |
|---------|-------------------|--------------|------------|
| sync | `549283e2` | `18dba43b` (ancestor) | `549283e2` (equal) |
| system | `88a65daa` | `dfd2664f` (ancestor) | `65786d3d` (ancestor) |
| warden | `86180069` | `447dd851` (ancestor) | `9c6a4728` (ancestor) |

1. `git subtree split -P <dir>` on the parent (full prefix history).
2. Graft point = oldest split commit whose tree equals the base tree
   (sync `341f44b2`, system `b277214d`, warden `b030c871`).
3. Fresh github clone + `rebase --onto <base> <graft> <split-tip>`
   (sync 67, system 120, warden 82 commits — zero conflicts).
4. Invariant verified at every step: replayed-tip tree == parent
   `HEAD:<dir>` tree == live worktree (`git status` clean after
   seating `.git`).
5. Result: original pre-08-22 SHAs preserved, post-08-22 monorepo work
   replayed on top, mirrors fast-forward. `refs/mirrors/*` and
   `split-*` temp refs deleted from the parent afterwards.

## Tag mapping (tree-match, annotated, prefixed convention restored)

Monorepo-era unprefixed tags were re-created prefixed in nested repos:

| nested tag | target | note |
|------------|--------|------|
| `dracon-sync-v0.113.53` | `549283e2` (base) | release touched parent root only; content identical |
| `dracon-sync-v0.113.54` | `ec27d39b` | |
| `dracon-sync-v0.113.55` | `76dd50ed` | |
| `dracon-system-v0.112.40` | `66a45a24` | |
| `dracon-warden-v0.113.6` | `73fb6a70` | |

Pre-freeze tags came with the github clone. Monorepo keeps its tags
as a historical record.

## Updated for the layout

- Nested READMEs: "frozen mirrors" → live-repo framing, standalone
  clone/build instructions, standalone raw/blob URLs (heroes included).
- Parent `ci.yml`: every job checks out the three nested repos
  (`path: dracon-sync|system|warden`, `ref: main`) — a bare parent
  clone has no utility source.
- `AGENTS.md` architecture section rewritten.
- Parent `.gitignore` already had the nested-standalone entries with
  the matching comment — no change needed; `git rm -r --cached`
  activated them (167 paths).
- `scripts/release.sh` (all three): `RELPFX` empty-prefix support,
  `TAG="${CRATE_NAME}-v${VERSION}"`, crate-name release titles,
  banner URL derived from the release remote (was hardcoded to the
  parent repo), `--abort` removes non-ignored untracked notes too
  (real standalone-mode bug: the ignored-only scan came free in the
  monorepo via the parent dir-ignore).
- Standalone `Cargo.lock` files refreshed (sync → v0.113.55, warden →
  v0.113.6; system was current) via isolated worktree `cargo check`.
- `dracon-sync/scripts/test_release_dry_run.sh`: fixture reworked to
  standalone layout, green (`sync release dry-run regression tests: ok`).
- `dracon-warden once` run in each nested repo (local filter config).

## Known state / follow-ups

- **Codeberg pushes rejected fleet-wide** (`Forgejo: Quota exceeded`,
  pre-existing 85 GiB quota posture). github+gitlab are live on all
  three utilities; the daemon retries codeberg via its normal
  push-stuck handling. No action beyond the quota work itself.
- **System/warden heavy release tests still assume the monorepo**:
  `dracon-system/scripts/test_release_standalone.sh` (clones the
  parent and expects `dracon-system/` inside),
  `test_release_pipeline.sh`, `test_release_abort.sh`, and
  `dracon-warden/scripts/test_release_dry_run.sh` (monorepo fixtures).
  Rework to standalone fixtures (sync's test is the template) and
  validate before the next release cut.
- **Parent CI not yet observed green** on the new layout (nested
  checkouts added but no push has run through all jobs since).
- `scripts/verify-install.sh` copies were not re-validated for
  standalone paths (warden README still references the parent-prefixed
  form from a full checkout, which is correct there).
- Future per-release notes land at nested repo roots (release.sh step
  4, unchanged); the parent archive (`docs/archive/release-notes/`)
  stays a historical record, not a process step.
